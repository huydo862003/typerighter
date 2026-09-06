use super::storage::QueryStorage;
use crate::SerializeContext;
use crate::persist::serialized::SerializedQueryStorage;
use crate::persist::serialized::dep_graph::{self as dep_graph_format, DepGraph};
use crate::persist::serialized::interned_blobs::{self as interned_blobs_format, InternedBlobs};
use crate::persist::serialized::query_cache::{BackingFile, QueryCache};
use std::any::Any;

pub trait QueryDatabase: Any {
  #[doc(hidden)]
  unsafe fn storage(&self) -> &QueryStorage;

  #[doc(hidden)]
  unsafe fn storage_mut(&mut self) -> &mut QueryStorage;
}

/// Extension of QueryDatabase that supports serialization.
pub trait SerializableQueryDatabase: QueryDatabase {
  /// Serialize the current query storage into the serialized formats.
  fn dump(&self) -> SerializedQueryStorage
  where
    Self: Sized,
  {
    let storage = unsafe { self.storage() };
    let mut ctx = SerializeContext::new(self);

    // Serialize all ingredients, respecting no_hash
    // NOTE: If we're intending to lazily load the deps
    // instead of eagerly loading the deps like we're doing
    // We must perform cache promotion here
    for entry in storage.ingredients.iter() {
      let ingredient = &entry.ingredient;
      for entry_id in ingredient.entry_ids().collect::<Vec<_>>() {
        ingredient.serialize(&mut ctx, entry_id);
      }
    }

    // Finalize
    let (nodes, query_cache_mmap, query_cache_file, intern_blobs) = ctx.finalize();

    // Build DepGraph
    let total_edge_count = nodes.iter().map(|n| n.edges().len() as u64).sum();
    let dep_graph = DepGraph {
      header: dep_graph_format::FileHeader::new(
        storage.revision.load(std::sync::atomic::Ordering::Acquire) as u64,
      ),
      footer: dep_graph_format::FileFooter {
        total_node_count: nodes.len() as u64,
        total_edge_count,
      },
      nodes,
    };

    // Build QueryCache from mmap
    let temp_path = query_cache_file.into_temp_path();
    let backing_path = temp_path.to_path_buf();
    let query_cache = QueryCache::new(query_cache_mmap, backing_path, BackingFile::Temp(temp_path))
      .expect("Failed to construct QueryCache from serialized data");

    // Build InternedBlobs
    let total_byte_size: u64 = intern_blobs.iter().map(|b| b.len() as u64).sum();
    let interned_blobs = InternedBlobs {
      header: interned_blobs_format::FileHeader::new(),
      footer: interned_blobs_format::FileFooter {
        total_node_count: intern_blobs.len() as u64,
        total_byte_size,
      },
      records: intern_blobs,
    };

    SerializedQueryStorage {
      dep_graph,
      query_cache,
      interned_blobs,
    }
  }
}
