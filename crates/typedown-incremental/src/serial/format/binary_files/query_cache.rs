//! On-disk format for the query result cache (`query-cache.bin`).
//!
//! Stores serialized return values of all cached queries, both derived and input.
//! Field projections (DerivedField, InputField) are not stored here. Their values are accessed via the parent query result.
//!
//! ```text
//! [ FileHeader (8 bytes)       ]  magic "TDQC" + version
//! [ result blob                ]  serialized query result
//! [ result blob                ]
//! [ ...                        ]
//! [ Footer                     ]  entry_count + Vec<(node_index, offset)>
//! [ footer_pos: u64            ]  last 8 bytes: byte offset of the Footer
//! ```
//!
//! On load, the footer is decoded upfront into a HashMap for O(1) lookup.
//! Result blobs stay in mmap and are decoded on demand.

use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;

use tempfile::TempPath;

/* File identification */
// Magic: 4 bytes
// Version: 4 bytes
// Together takes 8 bytes
pub const MAGIC: [u8; 4] = *b"TDQC";
pub const VERSION: u32 = 1;

/// File header (8 bytes).
#[derive(Debug, Clone, Copy)]
pub struct FileHeader {
  pub magic: [u8; 4],
  pub version: u32,
}

impl Default for FileHeader {
  fn default() -> Self {
    Self::new()
  }
}

impl FileHeader {
  pub fn new() -> Self {
    FileHeader {
      magic: MAGIC,
      version: VERSION,
    }
  }

  /// A best-effort check if the read file is valid
  pub fn is_valid(&self) -> bool {
    self.magic == MAGIC && self.version == VERSION
  }

  /// Serialize
  pub fn to_bytes(&self) -> [u8; 8] {
    let mut bytes = [0u8; 8];
    bytes[0..4].copy_from_slice(&self.magic);
    bytes[4..8].copy_from_slice(&self.version.to_le_bytes());
    bytes
  }

  /// Deserialize
  pub fn from_bytes(bytes: &[u8; 8]) -> Self {
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    FileHeader { magic, version }
  }
}

/// A footer entry
/// The footer contains an index table, mapping a node index in dep graph to it's corresponding blob's byte offset in the query cache
#[derive(Debug, Clone, Copy)]
pub struct FooterCacheEntry {
  /// Index into the dep graph's node array
  pub node_index: u32,
  /// Byte offset of the result blob from the start of the file
  pub offset: u64,
}

pub const CACHE_ENTRY_SIZE: usize = std::mem::size_of::<FooterCacheEntry>(); // 12

impl FooterCacheEntry {
  /// Serialize
  pub fn to_bytes(&self) -> [u8; CACHE_ENTRY_SIZE] {
    let mut bytes = [0u8; CACHE_ENTRY_SIZE];
    bytes[0..4].copy_from_slice(&self.node_index.to_le_bytes());
    bytes[4..12].copy_from_slice(&self.offset.to_le_bytes());
    bytes
  }

  /// Deserialize
  pub fn from_bytes(bytes: &[u8; CACHE_ENTRY_SIZE]) -> Self {
    FooterCacheEntry {
      node_index: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
      offset: u64::from_le_bytes(bytes[4..12].try_into().unwrap()),
    }
  }
}

pub enum BackingFile {
  /// From loading a .bin file on disk
  Disk(File),
  /// From dump via a named tempfile
  Temp(TempPath),
}

pub struct QueryCache {
  pub(crate) mmap: memmap2::Mmap,
  /// Index table: dep graph node index -> byte offset of the result blob.
  index: HashMap<u32, u64>,
  /// Path to the backing file, used for hard-linking during persist
  pub(crate) backing_path: PathBuf,
  /// Keeps the backing file alive so the mmap remains valid
  #[allow(dead_code)]
  backing: BackingFile,
}

impl QueryCache {
  pub fn new(mmap: memmap2::Mmap, backing_path: PathBuf, backing: BackingFile) -> Option<Self> {
    let data = &*mmap;
    let len = data.len();

    // Last 8 bytes: footer_pos
    let footer_pos = u64::from_le_bytes(data[len - 8..].try_into().ok()?) as usize;

    // Footer: entry_count (8 bytes) + entries
    let entry_count =
      u64::from_le_bytes(data[footer_pos..footer_pos + 8].try_into().ok()?) as usize;
    let entries_start = footer_pos + 8;

    let mut index = HashMap::with_capacity(entry_count);
    for idx in 0..entry_count {
      let offset = entries_start + idx * CACHE_ENTRY_SIZE;
      let entry =
        FooterCacheEntry::from_bytes(data[offset..offset + CACHE_ENTRY_SIZE].try_into().unwrap());
      index.insert(entry.node_index, entry.offset);
    }

    Some(QueryCache {
      mmap,
      index,
      backing_path,
      backing,
    })
  }

  /// Get the byte offset of a cached result by dep graph node index.
  pub fn offset_of(&self, node_index: u32) -> Option<u64> {
    self.index.get(&node_index).copied()
  }

  /// Get a decoder-ready byte slice starting at the cached result for a node.
  pub fn get(&self, node_index: u32) -> Option<&[u8]> {
    let offset = *self.index.get(&node_index)? as usize;
    Some(&self.mmap[offset..])
  }
}
