//! Session directory management for incremental cache persistence
//!
//! Follows rustc's pattern: each session gets its own subdirectory with a lock file
//!
//! ```text
//! {cache_dir}/
//!   s-{timestamp_millis}-{random}-working/   <- active session (exclusive lock)
//!   s-{timestamp_millis}-{revision}/         <- finalized previous session
//! ```

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use fs2::FileExt;
use memmap2::Mmap;

use crate::SerializedQueryStorage;
use crate::dep_graph::DepGraph;
use crate::interned_blobs::InternedBlobs;
use crate::query_cache::{BackingFile, QueryCache};
use crate::serial::format::binary_files;

const DEP_GRAPH_FILE: &str = "dep-graph.bin";
const QUERY_CACHE_FILE: &str = "query-cache.bin";
const INTERNED_BLOBS_FILE: &str = "interned-blobs.bin";
const LOCK_FILE: &str = "lock";

/// An active cache session with an exclusive lock on its working directory
pub struct CacheSession {
  working_dir: PathBuf,
  lock_file: Option<File>,
}

impl CacheSession {
  /// A no-op session that does nothing on finalize
  pub fn empty() -> Self {
    CacheSession {
      working_dir: PathBuf::new(),
      lock_file: None,
    }
  }

  /// Open a cache session: load the most recent finalized session (if any),
  /// then create a new working directory for this session
  pub fn open(cache_dir: &Path) -> io::Result<(Self, Option<SerializedQueryStorage>)> {
    fs::create_dir_all(cache_dir)?;

    // Ignore everything in .local/ since it's all generated
    let local_dir = cache_dir.parent().expect("cache_dir has parent");
    let gitignore = local_dir.join(".gitignore");
    if !gitignore.exists() {
      let _ = fs::write(&gitignore, "*\n");
    }

    garbage_collect(cache_dir);

    // Find and load the most recent finalized session
    let serialized = find_latest_finalized(cache_dir).and_then(|dir| {
      let lock_file = File::open(dir.join(LOCK_FILE)).ok()?;
      lock_file.lock_shared().ok()?;
      let result = load_from_dir(&dir);
      let _ = lock_file.unlock();
      result
    });

    // Create a new working directory
    let timestamp = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or_default()
      .as_millis();
    let random: u32 = rand::random();
    let working_name = format!("s-{}-{:08x}-working", timestamp, random);
    let working_dir = cache_dir.join(working_name);
    fs::create_dir_all(&working_dir)?;

    let lock_path = working_dir.join(LOCK_FILE);
    let lock_file = File::create(&lock_path)?;
    lock_file.lock_exclusive()?;

    Ok((
      CacheSession {
        working_dir,
        lock_file: Some(lock_file),
      },
      serialized,
    ))
  }

  /// Write the serialized data and rename the working directory to finalized
  pub fn finalize(mut self, data: &SerializedQueryStorage, revision: u64) -> io::Result<()> {
    if self.lock_file.is_none() {
      return Ok(());
    }
    save_to_dir(&self.working_dir, data)?;

    // Windows requires all handles closed before renaming a directory
    drop(self.lock_file.take());

    // Rename working -> finalized (s-{timestamp}-{random}-working -> s-{timestamp}-{random}-{revision})
    let parent = self
      .working_dir
      .parent()
      .expect("working dir must have parent");
    // Strip the -working suffix and keep the random token to prevent same-millisecond collisions
    let base = self
      .working_dir
      .file_name()
      .and_then(|name| name.to_str())
      .and_then(|name| name.strip_suffix("-working"))
      .expect("working dir must end with -working");
    let finalized_name = format!("{}-{}", base, revision);
    let finalized_dir = parent.join(finalized_name);
    fs::rename(&self.working_dir, &finalized_dir)?;

    Ok(())
  }
}

// Extract (timestamp, revision) from a finalized session dir name (s-{timestamp}-{random}-{revision})
// Revision is the last dash-separated segment; used as a tiebreaker when two sessions share a timestamp
fn session_sort_key(path: &Path) -> (u128, u64) {
  let name = path
    .file_name()
    .and_then(|name| name.to_str())
    .and_then(|name| name.strip_prefix("s-"))
    .unwrap_or("");
  let mut parts = name.split('-');
  let timestamp: u128 = parts
    .next()
    .and_then(|string| string.parse().ok())
    .unwrap_or(0);
  let revision: u64 = parts
    .next_back()
    .and_then(|string| string.parse().ok())
    .unwrap_or(0);
  (timestamp, revision)
}

/// Find the most recent finalized session directory (not ending in "-working")
fn find_latest_finalized(cache_dir: &Path) -> Option<PathBuf> {
  let mut candidates: Vec<PathBuf> = fs::read_dir(cache_dir)
    .ok()?
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| {
      path.is_dir()
        && path
          .file_name()
          .and_then(|name| name.to_str())
          .is_some_and(|name| name.starts_with("s-") && !name.ends_with("-working"))
    })
    .collect();
  // Sort by (timestamp, revision) descending so the newest session is always first, even when two sessions complete within the same millisecond
  candidates.sort_unstable_by_key(|path| std::cmp::Reverse(session_sort_key(path)));
  candidates.into_iter().next()
}

/// Delete stale working directories and old finalized sessions
fn garbage_collect(cache_dir: &Path) {
  let entries: Vec<PathBuf> = fs::read_dir(cache_dir)
    .into_iter()
    .flatten()
    .filter_map(|entry| entry.ok())
    .map(|entry| entry.path())
    .filter(|path| {
      path.is_dir()
        && path
          .file_name()
          .and_then(|name| name.to_str())
          .is_some_and(|name| name.starts_with("s-"))
    })
    .collect();

  // Delete stale working directories (owner crashed)
  for dir in &entries {
    let name = dir.file_name().and_then(|name| name.to_str()).unwrap_or("");
    if !name.ends_with("-working") {
      continue;
    }
    let lock_path = dir.join(LOCK_FILE);
    if let Ok(file) = File::open(&lock_path) {
      if file.try_lock_exclusive().is_ok() {
        let _ = file.unlock();
        let _ = fs::remove_dir_all(dir);
      }
    } else {
      // No lock file means no owner
      let _ = fs::remove_dir_all(dir);
    }
  }

  // Keep only the most recent finalized session
  let mut finalized_path: Vec<&PathBuf> = entries
    .iter()
    .filter(|path| {
      path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| !name.ends_with("-working"))
    })
    .collect();
  finalized_path.sort_unstable_by_key(|path| std::cmp::Reverse(session_sort_key(path)));
  for old_path in finalized_path.into_iter().skip(1) {
    let lock_path = old_path.join(LOCK_FILE);
    if let Ok(file) = File::open(&lock_path)
      && file.try_lock_exclusive().is_ok()
    {
      let _ = file.unlock();
      let _ = fs::remove_dir_all(old_path);
    }
  }
}

// File I/O for the three .bin formats

fn save_to_dir(dir: &Path, data: &SerializedQueryStorage) -> io::Result<()> {
  write_dep_graph(&dir.join(DEP_GRAPH_FILE), &data.dep_graph)?;
  write_query_cache(&dir.join(QUERY_CACHE_FILE), &data.query_cache)?;
  write_interned_blobs(&dir.join(INTERNED_BLOBS_FILE), &data.interned_blobs)?;
  Ok(())
}

fn load_from_dir(dir: &Path) -> Option<SerializedQueryStorage> {
  let dep_graph = read_dep_graph(&dir.join(DEP_GRAPH_FILE))?;
  let query_cache = read_query_cache(&dir.join(QUERY_CACHE_FILE))?;
  let interned_blobs = read_interned_blobs(&dir.join(INTERNED_BLOBS_FILE))?;
  Some(SerializedQueryStorage {
    dep_graph,
    query_cache,
    interned_blobs,
  })
}

fn write_dep_graph(path: &Path, graph: &DepGraph) -> io::Result<()> {
  let mut file = BufWriter::new(File::create(path)?);
  file.write_all(&graph.header.to_bytes())?;
  for node in &graph.nodes {
    file.write_all(&node.to_bytes())?;
  }
  file.write_all(&graph.footer.to_bytes())?;
  file.flush()
}

fn read_dep_graph(path: &Path) -> Option<DepGraph> {
  let data = fs::read(path).ok()?;
  if data.len() < 32 {
    return None;
  }
  let header = binary_files::dep_graph::FileHeader::from_bytes(data[..16].try_into().ok()?);
  if !header.is_valid() {
    return None;
  }
  let footer =
    binary_files::dep_graph::FileFooter::from_bytes(data[data.len() - 16..].try_into().ok()?);
  let mut position = 16;
  let node_end = data.len() - 16;
  let mut nodes = Vec::with_capacity(footer.total_node_count as usize);
  for _ in 0..footer.total_node_count {
    if position >= node_end {
      return None;
    }
    let (node, consumed) = binary_files::dep_graph::DepNode::from_bytes(&data[position..]);
    position += consumed;
    nodes.push(node);
  }
  Some(DepGraph {
    header,
    nodes,
    footer,
  })
}

fn write_query_cache(path: &Path, cache: &QueryCache) -> io::Result<()> {
  if fs::hard_link(&cache.backing_path, path).is_ok() {
    return Ok(());
  }
  fs::write(path, &*cache.mmap)
}

fn read_query_cache(path: &Path) -> Option<QueryCache> {
  let file = File::open(path).ok()?;
  let mmap = unsafe { Mmap::map(&file).ok()? };
  QueryCache::new(mmap, path.to_path_buf(), BackingFile::Disk(file))
}

fn write_interned_blobs(path: &Path, blobs: &InternedBlobs) -> io::Result<()> {
  let mut file = BufWriter::new(File::create(path)?);
  file.write_all(&blobs.header.to_bytes())?;
  for record in &blobs.records {
    let len = record.len() as u32;
    file.write_all(&len.to_le_bytes())?;
    file.write_all(record)?;
  }
  file.write_all(&blobs.footer.to_bytes())?;
  file.flush()
}

fn read_interned_blobs(path: &Path) -> Option<InternedBlobs> {
  let data = fs::read(path).ok()?;
  if data.len() < 24 {
    return None;
  }
  let header = binary_files::interned_blobs::FileHeader::from_bytes(data[..8].try_into().ok()?);
  if !header.is_valid() {
    return None;
  }
  let footer =
    binary_files::interned_blobs::FileFooter::from_bytes(data[data.len() - 16..].try_into().ok()?);
  let mut position = 8;
  let record_end = data.len() - 16;
  let mut records = Vec::with_capacity(footer.total_node_count as usize);
  for _ in 0..footer.total_node_count {
    if position + 4 > record_end {
      return None;
    }
    let len = u32::from_le_bytes(data[position..position + 4].try_into().ok()?) as usize;
    position += 4;
    if position + len > record_end {
      return None;
    }
    records.push(data[position..position + len].to_vec());
    position += len;
  }
  Some(InternedBlobs {
    header,
    records,
    footer,
  })
}
