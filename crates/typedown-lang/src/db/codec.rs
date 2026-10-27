use std::hash::Hasher;

use crate::syntax::green::cache::with_green_cache;
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;
use crate::{
  db::types::{FileHandle, FileMetadata},
  syntax::green::GreenNode,
};

use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};

// GreenNode (interned)
fn encode_green_node(node: &GreenNode, encoder: &mut Encoder) -> u32 {
  let hint = Some(node.as_ptr() as u32);
  if let Some(index) = hint.and_then(|hint| encoder.get_intern_blob_index_fast::<GreenNode>(hint)) {
    return index;
  }

  if node.is_node() {
    let syntax_node = node.as_node().unwrap();
    let children = syntax_node.children();
    let mut child_indices = Vec::with_capacity(children.len());
    for child in children {
      child_indices.push(encode_green_node(child, encoder));
    }
    let mut blob = Vec::new();
    blob.push(0); // tag: node
    blob.extend_from_slice(&(syntax_node.kind() as u16).to_le_bytes());
    blob.extend_from_slice(&(child_indices.len() as u32).to_le_bytes());
    for index in &child_indices {
      blob.extend_from_slice(&index.to_le_bytes());
    }
    encoder.intern_blob::<GreenNode>(blob, hint)
  } else {
    let token = node.as_token().unwrap();
    let mut blob = Vec::new();
    blob.push(1); // tag: token
    blob.extend_from_slice(&(token.kind() as u16).to_le_bytes());
    let bytes = token.bytes();
    blob.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    blob.extend_from_slice(bytes);
    encoder.intern_blob::<GreenNode>(blob, hint)
  }
}

impl Encodable for GreenNode {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    let index = encode_green_node(self, encoder);
    encoder.emit_u32(buf, index);
  }
}

fn decode_green_blob(index: usize, decoder: &Decoder) -> GreenNode {
  let blob = decoder.get_intern_blob(index as u32);
  let tag = blob[0];
  let kind_value = u16::from_le_bytes(blob[1..3].try_into().unwrap());

  let kind = SyntaxKind::from_repr(kind_value).expect("unknown SyntaxKind");

  match tag {
    0 => {
      let child_count = u32::from_le_bytes(blob[3..7].try_into().unwrap()) as usize;
      let mut children = Vec::with_capacity(child_count);
      for index in 0..child_count {
        let offset = 7 + index * 4;
        let child_index = u32::from_le_bytes(blob[offset..offset + 4].try_into().unwrap()) as usize;
        let child = decode_green_blob(child_index, decoder);
        children.push(child);
      }
      let syntax_node = with_green_cache(|cache| cache.node(kind, &children));
      GreenNode::from_node(syntax_node)
    }
    1 => {
      let byte_len = u32::from_le_bytes(blob[3..7].try_into().unwrap()) as usize;
      let bytes = &blob[7..7 + byte_len];
      let token = with_green_cache(|cache| cache.token(kind, bytes));
      GreenNode::from_token(token)
    }
    _ => panic!("unknown GreenNode tag {tag}"),
  }
}

impl Decodable for GreenNode {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let index = decoder.read_u32(data) as usize;
    decode_green_blob(index, decoder)
  }
}

// RedNode
impl Encodable for RedNode {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.offset().encode(buf, encoder);
    let root = self.root();
    (*root).encode(buf, encoder);
  }
}

impl Decodable for RedNode {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let offset = usize::decode(data, decoder);
    let green = GreenNode::decode(data, decoder);
    let root_node = green.as_node().expect("RedNode root must be a node");
    let root = RedNode::new_root(root_node.clone());
    root.find_at_offset(offset).unwrap_or(root)
  }
}

// StableHash impls

impl StableHash for FileMetadata {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.mtime.stable_hash(db, hasher);
    self.ctime.stable_hash(db, hasher);
  }
}

impl StableHash for FileHandle {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      FileHandle::Path(path, metadata) => {
        path.stable_hash(db, hasher);
        metadata.stable_hash(db, hasher);
      }
      FileHandle::Content(path, content, metadata) => {
        path.stable_hash(db, hasher);
        content.stable_hash(db, hasher);
        metadata.stable_hash(db, hasher);
      }
    }
  }
}

// SyntaxKind

impl Encodable for SyntaxKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u16(buf, *self as u16);
  }
}

impl Decodable for SyntaxKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let value = decoder.read_u16(data);
    SyntaxKind::from_repr(value).expect("unknown SyntaxKind")
  }
}

impl StableHash for SyntaxKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, _db: &DB, hasher: &mut StableHasher) {
    Hasher::write_u16(hasher, *self as u16);
  }
}

#[cfg(test)]
mod tests {
  use crate::db::TypedownDatabase;
  use crate::syntax::diagnostic::TdDiagnostic;
  use crate::syntax::green::GreenNode;
  use crate::syntax::red::RedNode;
  use crate::syntax::syntax_kind::SyntaxKind;
  use proptest::prelude::*;
  use std::fmt::Debug;
  use std::path::PathBuf;
  use std::sync::Arc;
  use typedown_incremental::{Decodable, Encodable};
  use typedown_incremental::{Decoder, Encoder, QueryStorage};

  /// Check that encode and decode return the original value
  fn encode_decode_roundtrip<T: Encodable + Decodable + PartialEq + Debug>(v: &T) {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let mut buf = Vec::new();
    let mut enc = Encoder::new(&db);
    v.encode(&mut buf, &mut enc);
    let intern_blobs = enc.finish();
    let decoder = Decoder::new(Arc::new(db.storage.clone()), Arc::new(intern_blobs));
    let mut data: &[u8] = &buf;
    let decoded = T::decode(&mut data, &decoder);
    assert_eq!(*v, decoded);
  }

  // Boolean encode/decode
  #[test]
  fn encode_bool_false_correctly() {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let encoder = Encoder::new(&db);
    let mut buf = Vec::new();
    encoder.emit_bool(&mut buf, false);
    assert_eq!(buf, vec![0]);
  }

  #[test]
  fn encode_bool_true_correctly() {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let encoder = Encoder::new(&db);
    let mut buf = Vec::new();
    encoder.emit_bool(&mut buf, true);
    assert_eq!(buf, vec![1]);
  }

  #[test]
  fn decode_bool_false_correctly() {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let decoder = Decoder::new(Arc::new(db.storage.clone()), Arc::new(vec![]));
    let data: &[u8] = &[0];
    let mut data = data;
    assert!(!decoder.read_bool(&mut data));
  }

  #[test]
  fn decode_bool_true_correctly() {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let decoder = Decoder::new(Arc::new(db.storage.clone()), Arc::new(vec![]));
    let data: &[u8] = &[1];
    let mut data = data;
    assert!(decoder.read_bool(&mut data));
  }

  proptest! {
    #[test]
    fn bool_roundtrip(v in any::<bool>()) {
      encode_decode_roundtrip(&v);
    }
  }

  // Number encode/decode

  proptest! {
    #[test]
    fn char_rountrip(v in any::<char>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn u8_roundtrip(v in any::<u8>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn i8_roundtrip(v in any::<i8>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn i16_roundtrip(v in any::<i16>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn u16_roundtrip(v in any::<u16>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn i32_roundtrip(v in any::<i32>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn u32_roundtrip(v in any::<u32>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn i64_roundtrip(v in any::<i64>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn u64_roundtrip(v in any::<u64>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn i128_roundtrip(v in any::<i128>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn u128_roundtrip(v in any::<u128>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn usize_roundtrip(v in any::<usize>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn isize_roundtrip(v in any::<isize>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn f64_roundtrip(v in any::<f64>()) {
      encode_decode_roundtrip(&v);
    }
  }

  // String and collection encode/decode

  proptest! {
    #[test]
    fn string_roundtrip(v in any::<String>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn pathbuf_roundtrip(v in any::<PathBuf>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn vec_u8_roundtrip(v in proptest::collection::vec(any::<u8>(), 0..100)) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn vec_i32_roundtrip(v in proptest::collection::vec(any::<i32>(), 0..100)) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn vec_string_roundtrip(v in proptest::collection::vec(any::<String>(), 0..20)) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn option_u32_roundtrip(v in any::<Option<u32>>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn option_string_roundtrip(v in any::<Option<String>>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn vec_option_i64_roundtrip(v in proptest::collection::vec(any::<Option<i64>>(), 0..50)) {
      encode_decode_roundtrip(&v);
    }
  }

  // Compound types

  proptest! {
    #[test]
    fn unit_roundtrip(_v in proptest::strategy::Just(())) {
      encode_decode_roundtrip(&());
    }

    #[test]
    fn tuple1_roundtrip(v in any::<(i32,)>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn tuple2_roundtrip(v in any::<(i32, String)>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn tuple3_roundtrip(v in any::<(u8, bool, String)>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn box_roundtrip(v in any::<Box<i32>>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn either_roundtrip(v in prop_oneof![
      any::<i32>().prop_map(typedown_types::either::Either::<i32, String>::Left),
      any::<String>().prop_map(typedown_types::either::Either::<i32, String>::Right),
    ]) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn hashmap_roundtrip(v in proptest::collection::hash_map(any::<String>(), any::<i32>(), 0..20)) {
      encode_decode_roundtrip(&v);
    }
  }

  // Syntax nodes

  proptest! {
    #[test]
    fn syntax_kind_roundtrip(v in any::<SyntaxKind>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn green_node_roundtrip(v in any::<GreenNode>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn red_node_roundtrip(v in any::<RedNode>()) {
      encode_decode_roundtrip(&v);
    }

    #[test]
    fn diagnostic_roundtrip(v in any::<TdDiagnostic>()) {
      encode_decode_roundtrip(&v);
    }
  }
}
