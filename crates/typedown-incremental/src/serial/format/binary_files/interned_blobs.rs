//! On-disk format for interned blobs (`interned-blobs.bin`).
//! Based on rustc's interned node deduplication strategy.
//!
//! Stores deduplicated blobs (e.g. green tree nodes: tokens and inner nodes).
//! Each blob is stored once & references use indices into this table.
//!
//! ```text
//! [ FileHeader (8 bytes)       ]  magic "TDIB" + version
//! [ NodeRecord                 ]  tag + kind + payload
//! [ NodeRecord                 ]
//! [ ...                        ]
//! [ FileFooter (16 bytes)      ]  total_node_count + total_byte_size
//! ```
//!
//! Two record types:
//! - Token (tag = 0): SyntaxKind + text bytes
//! - Node  (tag = 1): SyntaxKind + child_count + child indices
//!
//! Child indices refer to earlier entries in this file.
//! Decoded upfront into a Vec on load.

/* File identification */
// Magic: 4 bytes
// Version: 4 bytes
// Together takes 8 bytes

pub const MAGIC: [u8; 4] = *b"TDIB";
pub const VERSION: u32 = 1;

// Record tags
const TAG_TOKEN: u8 = 0;
const TAG_NODE: u8 = 1;

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
    bytes[0..4].copy_from_slice(&self.magic); // Char need not further endian processing
    bytes[4..8].copy_from_slice(&self.version.to_le_bytes()); // Encode as little endian
    bytes
  }

  pub fn from_bytes(bytes: &[u8; 8]) -> Self {
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    FileHeader { magic, version }
  }
}

/// A record in the interned nodes file.
#[derive(Debug, Clone)]
pub enum NodeRecord {
  /// A token leaf node: SyntaxKind + text bytes.
  Token { kind: u16, text: Vec<u8> },
  /// An inner node: SyntaxKind + children (indices into this file's node table).
  Node { kind: u16, children: Vec<u32> },
}

// Byte sizes
const TAG_SIZE: usize = std::mem::size_of::<u8>();
const KIND_SIZE: usize = std::mem::size_of::<u16>();
const LENGTH_SIZE: usize = std::mem::size_of::<u32>();
const INDEX_SIZE: usize = std::mem::size_of::<u32>();

impl NodeRecord {
  pub fn to_bytes(&self) -> Vec<u8> {
    let capacity = match self {
      // Token node record
      NodeRecord::Token { text, .. } => {
        TAG_SIZE + // The variant of this node record
        KIND_SIZE + // The kind of this token
        LENGTH_SIZE + text.len() // The text of the token
      }
      // Node node record
      NodeRecord::Node { children, .. } => {
        TAG_SIZE + // The variant of this node record
        KIND_SIZE + // The kind of this node
        LENGTH_SIZE + // The number of the children
        children.len() * INDEX_SIZE // The list of children
      }
    };
    let mut bytes = Vec::with_capacity(capacity);
    match self {
      // Token node record
      NodeRecord::Token { kind, text } => {
        bytes.push(TAG_TOKEN);
        bytes.extend_from_slice(&kind.to_le_bytes());
        bytes.extend_from_slice(&(text.len() as u32).to_le_bytes());
        bytes.extend_from_slice(text);
      }
      // Node node record
      NodeRecord::Node { kind, children } => {
        bytes.push(TAG_NODE);
        bytes.extend_from_slice(&kind.to_le_bytes());
        bytes.extend_from_slice(&(children.len() as u32).to_le_bytes());
        for child in children {
          bytes.extend_from_slice(&child.to_le_bytes());
        }
      }
    }
    bytes
  }

  pub fn from_bytes(bytes: &[u8]) -> (Self, usize) {
    let tag = bytes[0];
    let mut pos = TAG_SIZE;
    match tag {
      // Token node record
      TAG_TOKEN => {
        // The token kind
        let kind = u16::from_le_bytes(bytes[pos..pos + KIND_SIZE].try_into().unwrap());
        pos += KIND_SIZE;

        // The token text length
        let text_len =
          u32::from_le_bytes(bytes[pos..pos + LENGTH_SIZE].try_into().unwrap()) as usize;
        pos += LENGTH_SIZE;

        // The token text
        let text = bytes[pos..pos + text_len].to_vec();
        pos += text_len;

        (NodeRecord::Token { kind, text }, pos)
      }
      // Node node record
      TAG_NODE => {
        // The node kind
        let kind = u16::from_le_bytes(bytes[pos..pos + KIND_SIZE].try_into().unwrap());
        pos += KIND_SIZE;

        // The node child count
        let child_count =
          u32::from_le_bytes(bytes[pos..pos + LENGTH_SIZE].try_into().unwrap()) as usize;
        pos += LENGTH_SIZE;

        // The node children list
        let mut children = Vec::with_capacity(child_count);
        for _ in 0..child_count {
          children.push(u32::from_le_bytes(
            bytes[pos..pos + INDEX_SIZE].try_into().unwrap(),
          ));
          pos += INDEX_SIZE;
        }

        (NodeRecord::Node { kind, children }, pos)
      }
      _ => panic!("unknown NodeRecord tag {tag}"),
    }
  }
}

/// File footer.
#[derive(Debug, Clone, Copy)]
pub struct FileFooter {
  /// Total number of node records in the file
  pub total_node_count: u64,
  /// Total byte size of all node records excluding header and footer
  pub total_byte_size: u64,
}

impl FileFooter {
  pub fn to_bytes(&self) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    bytes[0..8].copy_from_slice(&self.total_node_count.to_le_bytes());
    bytes[8..16].copy_from_slice(&self.total_byte_size.to_le_bytes());
    bytes
  }

  pub fn from_bytes(bytes: &[u8; 16]) -> Self {
    FileFooter {
      total_node_count: u64::from_le_bytes(bytes[0..8].try_into().unwrap()),
      total_byte_size: u64::from_le_bytes(bytes[8..16].try_into().unwrap()),
    }
  }
}

/// The interned blobs readily for serialization or deserialization into
pub struct InternedBlobs {
  pub header: FileHeader,
  pub records: Vec<Vec<u8>>,
  pub footer: FileFooter,
}

impl InternedBlobs {
  pub fn node_count(&self) -> usize {
    self.records.len()
  }
}
