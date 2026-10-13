pub mod format;
pub mod ingredient;

use std::sync::atomic::Ordering;

use crate::serial::format::binary_files::SerializedQueryStorage;
use crate::serial::format::binary_files::dep_graph::DepNodeIndex;
use crate::serial::format::binary_files::dep_graph::{self as dep_graph_format, DepGraph};
use crate::serial::format::binary_files::interned_blobs::{
  self as interned_blobs_format, InternedBlobs,
};
use crate::serial::format::binary_files::query_cache::{BackingFile, QueryCache};
use crate::serial::ingredient::serde::{DeserializeContext, SerializeContext};
use crate::{DepId, EntryId, QueryDatabase};

pub trait Codec: Send + Sync {
  fn serialize(&self, ctx: &mut SerializeContext, entry_id: EntryId);
  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId>;
  fn promote_cached(&self, _ctx: &DeserializeContext) {}
  fn no_hash(&self) -> bool {
    false
  }
}

/// Extension of QueryDatabase that supports serialization
pub trait SerializableQueryDatabase: QueryDatabase {
  /// Serialize the current query storage into the serialized formats
  fn dump(&self) -> SerializedQueryStorage
  where
    Self: Sized,
  {
    let storage = unsafe { self.storage() };
    let mut ctx = SerializeContext::new(self);

    macro_rules! serialize_all {
      ($ingredients:expr) => {
        for ingredient in $ingredients.iter() {
          for entry_id in ingredient.entry_ids() {
            ingredient.serialize(&mut ctx, entry_id);
          }
        }
      };
    }

    // Promote unaccessed cached entries so they survive the dump
    if let Some(ctx) = storage.deserialize_ctx.get() {
      for query in storage.queries.iter() {
        query.promote_cached(ctx);
      }
      for field in storage.fields.iter() {
        field.promote_cached(ctx);
      }
    }

    // Serialize all ingredients: inputs -> interned -> queries -> fields
    serialize_all!(storage.inputs);
    serialize_all!(storage.interned);
    serialize_all!(storage.queries);
    serialize_all!(storage.fields);

    // Finalize
    let (nodes, query_cache_mmap, query_cache_file, intern_blobs) = ctx.finalize();

    // Build DepGraph
    let total_edge_count = nodes.iter().map(|node| node.edges().len() as u64).sum();
    let dep_graph = DepGraph {
      header: dep_graph_format::FileHeader::new(storage.revision.load(Ordering::Acquire) as u64),
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

    // Store the result
    SerializedQueryStorage {
      dep_graph,
      query_cache,
      interned_blobs,
    }
  }
}
