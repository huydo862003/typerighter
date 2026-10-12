//! Serialized formats for cross-session persistence.
//!
//! Three files are persisted:
//! - `dep-graph.bin`: The dependency graph (nodes, edges, fingerprints). No query result data.
//! - `query-cache.bin`: Cached query results, accessed lazily by offset.
//! - `interned-blobs.bin`: Deduplicated blobs (e.g. green tree nodes).

pub mod dep_graph;
pub mod interned_blobs;
pub mod query_cache;

use dep_graph::DepGraph;
use interned_blobs::InternedBlobs;
use query_cache::QueryCache;

use crate::dep_graph::DepNode;

/// The serialized form of the entire query storage
pub struct SerializedQueryStorage {
  pub dep_graph: DepGraph,
  pub query_cache: QueryCache,
  pub interned_blobs: InternedBlobs,
}

/// Cache entry counts by node type
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheStats {
  pub derived_queries: usize,
  pub derived_fields: usize,
  pub input_fields: usize,
  pub interned: usize,
  pub intern_blobs: usize,
}

impl SerializedQueryStorage {
  /// Count entries by node type
  pub fn stats(&self) -> CacheStats {
    let mut stats = CacheStats {
      derived_queries: 0,
      derived_fields: 0,
      input_fields: 0,
      interned: 0,
      intern_blobs: self.interned_blobs.records.len(),
    };
    for node in &self.dep_graph.nodes {
      match node {
        DepNode::DerivedQuery { .. } => stats.derived_queries += 1,
        DepNode::DerivedField { .. } => stats.derived_fields += 1,
        DepNode::InputField { .. } => stats.input_fields += 1,
        DepNode::Interned { .. } => stats.interned += 1,
        DepNode::Evicted => {}
      }
    }
    stats
  }
}
