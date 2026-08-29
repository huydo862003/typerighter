//! On-disk format for the dependency graph (`dep-graph.bin`).
//! Based on rustc's dep-graph.bin
//!
//! Stores nodes, edges, and fingerprints. No query result data.
//! Fully decoded upfront into memory on load.
//!
//! ```text
//! [ FileHeader (8 bytes)       ]  magic "TDEP" + version
//! [ DepNode                    ]  tag + variant-specific data
//! [ DepNode                    ]
//! [ ...                        ]
//! [ FileFooter (16 bytes)      ]  total_node_count + total_edge_count
//! ```

use crate::Fingerprint;

pub type DepNodeIndex = u32;

/* File identification */
// Magic: 4 bytes
// Version: 4 bytes
// Together takes 8 bytes

pub const MAGIC: [u8; 4] = *b"TDEP"; // Magic bytes, like PNG/ELF magic bytes
pub const VERSION: u32 = 1;

/// File header (16 bytes).
#[derive(Debug, Clone, Copy)]
pub struct FileHeader {
  pub magic: [u8; 4],
  pub version: u32,
  pub revision: u64,
}

impl FileHeader {
  pub fn new(revision: u64) -> Self {
    FileHeader {
      magic: MAGIC,
      version: VERSION,
      revision,
    }
  }

  /// A best-effort check if the read file is valid
  pub fn is_valid(&self) -> bool {
    self.magic == MAGIC && self.version == VERSION
  }

  /// Serialize
  pub fn to_bytes(&self) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0..4].copy_from_slice(&self.magic);
    bytes[4..8].copy_from_slice(&self.version.to_le_bytes());
    bytes[8..16].copy_from_slice(&self.revision.to_le_bytes());
    bytes
  }

  pub fn from_bytes(bytes: &[u8; 16]) -> Self {
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let revision = u64::from_le_bytes(bytes[8..16].try_into().unwrap());
    FileHeader {
      magic,
      version,
      revision,
    }
  }
}

/// A node in the dep graph
#[derive(Debug, Clone)]
pub enum DepNode {
  /// A derived query invocation (e.g. `vault_config(project)`)
  DerivedQuery {
    name: Fingerprint,
    key: Fingerprint,
    value: Fingerprint,
    entry_id: u64,
    value_entry_id: u64,
    changed_at: u64,
    verified_at: u64,
    edges: Vec<u32>,
  },
  /// A derived struct field (e.g. `VaultConfigResult::version`)
  DerivedField {
    name: Fingerprint,
    field_index: u8,
    entry_id: u64,
    value: Fingerprint,
    changed_at: u64,
  },
  /// An input field (e.g. `File::handle`). Leaf node.
  InputField {
    name: Fingerprint,
    field_index: u8,
    entry_id: u64,
    value: Fingerprint,
    changed_at: u64,
  },
  /// An interned value (e.g. `LiteralValue`). Leaf node.
  Interned { name: Fingerprint, blob_index: u32 },
  /// Evicted by LRU, forces recomputation on next load
  Evicted,
}

// Tag bytes for serialization
// Distinguishing among 3 kind of dep nodes
const TAG_DERIVED_QUERY: u8 = 0;
const TAG_DERIVED_FIELD: u8 = 1;
const TAG_INPUT_FIELD: u8 = 2;
const TAG_INTERNED: u8 = 3;
const TAG_EVICTED: u8 = 4;

// Byte sizes
const TAG_SIZE: usize = std::mem::size_of::<u8>();
const FINGERPRINT_SIZE: usize = std::mem::size_of::<Fingerprint>();
const FIELD_INDEX_SIZE: usize = std::mem::size_of::<u8>();
const REVISION_SIZE: usize = std::mem::size_of::<u64>();
const EDGE_COUNT_SIZE: usize = std::mem::size_of::<u32>();
const EDGE_SIZE: usize = std::mem::size_of::<u32>();

impl DepNode {
  pub fn name(&self) -> Fingerprint {
    match self {
      DepNode::DerivedQuery { name, .. }
      | DepNode::DerivedField { name, .. }
      | DepNode::InputField { name, .. }
      | DepNode::Interned { name, .. } => *name,
      DepNode::Evicted => Fingerprint([0; 16]),
    }
  }

  pub fn value_fingerprint(&self) -> Fingerprint {
    match self {
      DepNode::DerivedQuery { value, .. }
      | DepNode::DerivedField { value, .. }
      | DepNode::InputField { value, .. } => *value,
      DepNode::Interned { .. } | DepNode::Evicted => {
        panic!("this node does not have a value fingerprint")
      }
    }
  }

  pub fn field_index(&self) -> Option<u8> {
    match self {
      DepNode::DerivedField { field_index, .. } | DepNode::InputField { field_index, .. } => {
        Some(*field_index)
      }
      DepNode::DerivedQuery { .. } | DepNode::Interned { .. } | DepNode::Evicted => None,
    }
  }

  pub fn changed_at(&self) -> u64 {
    match self {
      DepNode::DerivedQuery { changed_at, .. }
      | DepNode::DerivedField { changed_at, .. }
      | DepNode::InputField { changed_at, .. } => *changed_at,
      DepNode::Interned { .. } | DepNode::Evicted => 0,
    }
  }

  pub fn edges(&self) -> &[u32] {
    match self {
      DepNode::DerivedQuery { edges, .. } => edges,
      DepNode::DerivedField { .. }
      | DepNode::InputField { .. }
      | DepNode::Interned { .. }
      | DepNode::Evicted => &[],
    }
  }

  /// Serialize
  pub fn to_bytes(&self) -> Vec<u8> {
    let capacity = match self {
      DepNode::DerivedQuery { edges, .. } => {
        TAG_SIZE +
        FINGERPRINT_SIZE * 3 + // name + key + value
        REVISION_SIZE * 4 + // entry_id + value_entry_id + changed_at + verified_at
        EDGE_COUNT_SIZE +
        edges.len() * EDGE_SIZE
      }
      DepNode::DerivedField { .. } => {
        TAG_SIZE +
        FINGERPRINT_SIZE * 2 + // name + value
        FIELD_INDEX_SIZE +
        REVISION_SIZE + // entry_id
        REVISION_SIZE // changed_at
      }
      DepNode::InputField { .. } => {
        TAG_SIZE +
        FINGERPRINT_SIZE * 2 + // name + value
        FIELD_INDEX_SIZE +
        REVISION_SIZE + // entry_id
        REVISION_SIZE // changed_at
      }
      DepNode::Interned { .. } => {
        TAG_SIZE + // discriminant
        FINGERPRINT_SIZE + // name fingerprint
        4 // blob_index: u32
      }
      DepNode::Evicted => TAG_SIZE,
    };
    let mut bytes = Vec::with_capacity(capacity); // preallocated to avoid reallocation overhead
    match self {
      DepNode::DerivedQuery {
        name,
        key,
        value,
        entry_id,
        value_entry_id,
        changed_at,
        verified_at,
        edges,
      } => {
        bytes.push(TAG_DERIVED_QUERY);
        bytes.extend_from_slice(&name.0);
        bytes.extend_from_slice(&key.0);
        bytes.extend_from_slice(&value.0);
        bytes.extend_from_slice(&entry_id.to_le_bytes());
        bytes.extend_from_slice(&value_entry_id.to_le_bytes());
        bytes.extend_from_slice(&changed_at.to_le_bytes());
        bytes.extend_from_slice(&verified_at.to_le_bytes());
        bytes.extend_from_slice(&(edges.len() as u32).to_le_bytes());
        for edge in edges {
          bytes.extend_from_slice(&edge.to_le_bytes());
        }
      }
      DepNode::DerivedField {
        name,
        field_index,
        entry_id,
        value,
        changed_at,
      } => {
        bytes.push(TAG_DERIVED_FIELD);
        bytes.extend_from_slice(&name.0);
        bytes.push(*field_index);
        bytes.extend_from_slice(&entry_id.to_le_bytes());
        bytes.extend_from_slice(&value.0);
        bytes.extend_from_slice(&changed_at.to_le_bytes());
      }
      DepNode::InputField {
        name,
        field_index,
        entry_id,
        value,
        changed_at,
      } => {
        bytes.push(TAG_INPUT_FIELD);
        bytes.extend_from_slice(&name.0);
        bytes.push(*field_index);
        bytes.extend_from_slice(&entry_id.to_le_bytes());
        bytes.extend_from_slice(&value.0);
        bytes.extend_from_slice(&changed_at.to_le_bytes());
      }
      DepNode::Interned { name, blob_index } => {
        bytes.push(TAG_INTERNED);
        bytes.extend_from_slice(&name.0);
        bytes.extend_from_slice(&blob_index.to_le_bytes());
      }
      DepNode::Evicted => {
        bytes.push(TAG_EVICTED);
      }
    }
    bytes
  }

  /// Deserialize
  pub fn from_bytes(bytes: &[u8]) -> (Self, usize) {
    let tag = bytes[0]; // the variant of the dep node
    let mut pos = TAG_SIZE; // current decoded offset
    match tag {
      // Derived query node
      TAG_DERIVED_QUERY => {
        // fingerprint of the derived query's name
        let name = Fingerprint(bytes[pos..pos + FINGERPRINT_SIZE].try_into().unwrap());
        pos += FINGERPRINT_SIZE;

        // fingerprint of the derived query's args
        let key = Fingerprint(bytes[pos..pos + FINGERPRINT_SIZE].try_into().unwrap());
        pos += FINGERPRINT_SIZE;

        // fingerprint of the derived query's value
        let value = Fingerprint(bytes[pos..pos + FINGERPRINT_SIZE].try_into().unwrap());
        pos += FINGERPRINT_SIZE;

        let entry_id = u64::from_le_bytes(bytes[pos..pos + REVISION_SIZE].try_into().unwrap());
        pos += REVISION_SIZE;

        let value_entry_id =
          u64::from_le_bytes(bytes[pos..pos + REVISION_SIZE].try_into().unwrap());
        pos += REVISION_SIZE;

        // the revision when the value last changed
        let changed_at = u64::from_le_bytes(bytes[pos..pos + REVISION_SIZE].try_into().unwrap());
        pos += REVISION_SIZE;

        // the revision when last confirmed valid
        let verified_at = u64::from_le_bytes(bytes[pos..pos + REVISION_SIZE].try_into().unwrap());
        pos += REVISION_SIZE;

        // the number of edges connecting this dep node
        let edge_count =
          u32::from_le_bytes(bytes[pos..pos + EDGE_COUNT_SIZE].try_into().unwrap()) as usize;
        pos += EDGE_COUNT_SIZE;

        // decode all the edges
        let mut edges = Vec::with_capacity(edge_count);
        for _ in 0..edge_count {
          edges.push(u32::from_le_bytes(
            bytes[pos..pos + EDGE_SIZE].try_into().unwrap(),
          ));
          pos += EDGE_SIZE;
        }

        (
          DepNode::DerivedQuery {
            name,
            key,
            value,
            entry_id,
            value_entry_id,
            changed_at,
            verified_at,
            edges,
          },
          pos,
        )
      }
      // Derived query field
      TAG_DERIVED_FIELD => {
        let name = Fingerprint(bytes[pos..pos + FINGERPRINT_SIZE].try_into().unwrap());
        pos += FINGERPRINT_SIZE;

        let field_index = bytes[pos];
        pos += FIELD_INDEX_SIZE;

        let entry_id = u64::from_le_bytes(bytes[pos..pos + REVISION_SIZE].try_into().unwrap());
        pos += REVISION_SIZE;

        let value = Fingerprint(bytes[pos..pos + FINGERPRINT_SIZE].try_into().unwrap());
        pos += FINGERPRINT_SIZE;

        let changed_at = u64::from_le_bytes(bytes[pos..pos + REVISION_SIZE].try_into().unwrap());
        pos += REVISION_SIZE;

        (
          DepNode::DerivedField {
            name,
            field_index,
            entry_id,
            value,
            changed_at,
          },
          pos,
        )
      }
      // Input field
      TAG_INPUT_FIELD => {
        let name = Fingerprint(bytes[pos..pos + FINGERPRINT_SIZE].try_into().unwrap());
        pos += FINGERPRINT_SIZE;

        let field_index = bytes[pos];
        pos += FIELD_INDEX_SIZE;

        let entry_id = u64::from_le_bytes(bytes[pos..pos + REVISION_SIZE].try_into().unwrap());
        pos += REVISION_SIZE;

        let value = Fingerprint(bytes[pos..pos + FINGERPRINT_SIZE].try_into().unwrap());
        pos += FINGERPRINT_SIZE;

        let changed_at = u64::from_le_bytes(bytes[pos..pos + REVISION_SIZE].try_into().unwrap());
        pos += REVISION_SIZE;

        (
          DepNode::InputField {
            name,
            field_index,
            entry_id,
            value,
            changed_at,
          },
          pos,
        )
      }
      TAG_INTERNED => {
        let name = Fingerprint(bytes[pos..pos + FINGERPRINT_SIZE].try_into().unwrap());
        pos += FINGERPRINT_SIZE;

        let blob_index = u32::from_le_bytes(bytes[pos..pos + 4].try_into().unwrap());
        pos += 4;

        (DepNode::Interned { name, blob_index }, pos)
      }
      TAG_EVICTED => (DepNode::Evicted, pos),
      _ => panic!("unknown DepNode tag {tag}"),
    }
  }
}

/// File footer (16 bytes).
#[derive(Debug, Clone, Copy)]
pub struct FileFooter {
  pub total_node_count: u64, // 8 bytes
  pub total_edge_count: u64, // 8 bytes
}

impl FileFooter {
  /// Serialize
  pub fn to_bytes(&self) -> [u8; 16] {
    let mut bytes = [0u8; 16];

    // Encode the node count as little-endian & serialize
    bytes[0..8].copy_from_slice(&self.total_node_count.to_le_bytes());

    // Encode the edge count as little-endian & serialize
    bytes[8..16].copy_from_slice(&self.total_edge_count.to_le_bytes());

    bytes
  }

  /// Deserialize
  pub fn from_bytes(bytes: &[u8; 16]) -> Self {
    FileFooter {
      // Decode the node count from little-endian & deserialize
      total_node_count: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
      // Decode the edge count from little-endian & deserialize
      total_edge_count: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
    }
  }
}

/// The dep graph readily for serialization or deserialization into
pub struct DepGraph {
  pub header: FileHeader,
  pub nodes: Vec<DepNode>,
  pub footer: FileFooter,
}

impl DepGraph {
  pub fn node_count(&self) -> usize {
    self.nodes.len()
  }
}
