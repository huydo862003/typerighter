//! Session directory management for incremental cache persistence.
//!
//! Follows rustc's pattern: each session gets its own subdirectory with a lock file.
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

use super::serialized::SerializedQueryStorage;
use super::serialized::dep_graph::{self as dep_graph_fmt, DepGraph};
use super::serialized::interned_blobs::{self as interned_blobs_fmt, InternedBlobs};
use super::serialized::query_cache::{BackingFile, QueryCache};

const DEP_GRAPH_FILE: &str = "dep-graph.bin";
const QUERY_CACHE_FILE: &str = "query-cache.bin";
const INTERNED_BLOBS_FILE: &str = "interned-blobs.bin";
const LOCK_FILE: &str = "lock";

/// An active cache session with an exclusive lock on its working directory.
pub struct CacheSession {
  working_dir: PathBuf,
  lock_file: Option<File>,
}

impl CacheSession {
  /// A no-op session that does nothing on finalize.
  pub fn empty() -> Self {
    CacheSession {
      working_dir: PathBuf::new(),
      lock_file: None,
    }
  }

  /// Open a cache session: load the most recent finalized session (if any),
  /// then create a new working directory for this session.
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

  /// Write the serialized data and rename the working directory to finalized.
  pub fn finalize(mut self, data: &SerializedQueryStorage, revision: u64) -> io::Result<()> {
    if self.lock_file.is_none() {
      return Ok(());
    }
    save_to_dir(&self.working_dir, data)?;

    // Windows requires all handles closed before renaming a directory
    drop(self.lock_file.take());

    // Rename working -> finalized
    let parent = self
      .working_dir
      .parent()
      .expect("working dir must have parent");
    let timestamp = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or_default()
      .as_millis();
    let finalized_name = format!("s-{}-{}", timestamp, revision);
    let finalized_dir = parent.join(finalized_name);
    fs::rename(&self.working_dir, &finalized_dir)?;

    Ok(())
  }
}

/// Find the most recent finalized session directory (not ending in "-working").
fn find_latest_finalized(cache_dir: &Path) -> Option<PathBuf> {
  let mut candidates: Vec<PathBuf> = fs::read_dir(cache_dir)
    .ok()?
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| {
      p.is_dir()
        && p
          .file_name()
          .and_then(|n| n.to_str())
          .is_some_and(|n| n.starts_with("s-") && !n.ends_with("-working"))
    })
    .collect();
  // Sort by name descending (timestamp is first and use the same digits, so lexicographic order works)
  candidates.sort_unstable_by(|a, b| b.cmp(a));
  candidates.into_iter().next()
}

/// Delete stale working directories and old finalized sessions.
fn garbage_collect(cache_dir: &Path) {
  let entries: Vec<PathBuf> = fs::read_dir(cache_dir)
    .into_iter()
    .flatten()
    .filter_map(|e| e.ok())
    .map(|e| e.path())
    .filter(|p| {
      p.is_dir()
        && p
          .file_name()
          .and_then(|n| n.to_str())
          .is_some_and(|n| n.starts_with("s-"))
    })
    .collect();

  // Delete stale working directories (owner crashed)
  for dir in &entries {
    let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if !name.ends_with("-working") {
      continue;
    }
    let lock_path = dir.join(LOCK_FILE);
    if let Ok(f) = File::open(&lock_path) {
      if f.try_lock_exclusive().is_ok() {
        let _ = f.unlock();
        let _ = fs::remove_dir_all(dir);
      }
    } else {
      // No lock file means no owner
      let _ = fs::remove_dir_all(dir);
    }
  }

  // Keep only the most recent finalized session
  let mut finalized: Vec<&PathBuf> = entries
    .iter()
    .filter(|p| {
      p.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| !n.ends_with("-working"))
    })
    .collect();
  finalized.sort_unstable_by(|a, b| b.cmp(a));
  for old in finalized.into_iter().skip(1) {
    let lock_path = old.join(LOCK_FILE);
    if let Ok(f) = File::open(&lock_path)
      && f.try_lock_exclusive().is_ok()
    {
      let _ = f.unlock();
      let _ = fs::remove_dir_all(old);
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
  let mut f = BufWriter::new(File::create(path)?);
  f.write_all(&graph.header.to_bytes())?;
  for node in &graph.nodes {
    f.write_all(&node.to_bytes())?;
  }
  f.write_all(&graph.footer.to_bytes())?;
  f.flush()
}

fn read_dep_graph(path: &Path) -> Option<DepGraph> {
  let data = fs::read(path).ok()?;
  if data.len() < 32 {
    return None;
  }
  let header = dep_graph_fmt::FileHeader::from_bytes(data[..16].try_into().ok()?);
  if !header.is_valid() {
    return None;
  }
  let footer = dep_graph_fmt::FileFooter::from_bytes(data[data.len() - 16..].try_into().ok()?);
  let mut pos = 16;
  let node_end = data.len() - 16;
  let mut nodes = Vec::with_capacity(footer.total_node_count as usize);
  for _ in 0..footer.total_node_count {
    if pos >= node_end {
      return None;
    }
    let (node, consumed) = dep_graph_fmt::DepNode::from_bytes(&data[pos..]);
    pos += consumed;
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
  let f = File::open(path).ok()?;
  let mmap = unsafe { Mmap::map(&f).ok()? };
  QueryCache::new(mmap, path.to_path_buf(), BackingFile::Disk(f))
}

fn write_interned_blobs(path: &Path, blobs: &InternedBlobs) -> io::Result<()> {
  let mut f = BufWriter::new(File::create(path)?);
  f.write_all(&blobs.header.to_bytes())?;
  for record in &blobs.records {
    let len = record.len() as u32;
    f.write_all(&len.to_le_bytes())?;
    f.write_all(record)?;
  }
  f.write_all(&blobs.footer.to_bytes())?;
  f.flush()
}

fn read_interned_blobs(path: &Path) -> Option<InternedBlobs> {
  let data = fs::read(path).ok()?;
  if data.len() < 24 {
    return None;
  }
  let header = interned_blobs_fmt::FileHeader::from_bytes(data[..8].try_into().ok()?);
  if !header.is_valid() {
    return None;
  }
  let footer = interned_blobs_fmt::FileFooter::from_bytes(data[data.len() - 16..].try_into().ok()?);
  let mut pos = 8;
  let record_end = data.len() - 16;
  let mut records = Vec::with_capacity(footer.total_node_count as usize);
  for _ in 0..footer.total_node_count {
    if pos + 4 > record_end {
      return None;
    }
    let len = u32::from_le_bytes(data[pos..pos + 4].try_into().ok()?) as usize;
    pos += 4;
    if pos + len > record_end {
      return None;
    }
    records.push(data[pos..pos + len].to_vec());
    pos += len;
  }
  Some(InternedBlobs {
    header,
    records,
    footer,
  })
}
