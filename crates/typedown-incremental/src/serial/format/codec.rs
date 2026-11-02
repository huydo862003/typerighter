use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use typedown_types::either::Either;

use crate::serial::format::dep_graph::{DepNode, DepNodeIndex};
use crate::serial::format::stable::StableCompare;
use crate::{DepId, QueryDatabase, QueryStorage};

pub struct Encoder<'a> {
  pub db: &'a dyn QueryDatabase,
  intern_hints: HashMap<(std::any::TypeId, usize), u32>,
  intern_blobs: Vec<Vec<u8>>,
  intern_table: HashMap<Vec<u8>, u32>,
  dep_id_table: HashMap<DepId, DepNodeIndex>,
  next_dep_node_index: DepNodeIndex,
}

impl<'a> Encoder<'a> {
  pub fn new(db: &'a dyn QueryDatabase) -> Self {
    Self {
      db,
      intern_hints: HashMap::new(),
      intern_blobs: Vec::new(),
      intern_table: HashMap::new(),
      dep_id_table: HashMap::new(),
      next_dep_node_index: 0,
    }
  }

  /// Register a DepId and get a stable DepNodeIndex for it
  pub fn add_dep_id(&mut self, dep_id: DepId) -> DepNodeIndex {
    if let Some(&index) = self.dep_id_table.get(&dep_id) {
      return index;
    }
    let index = self.next_dep_node_index;
    self.next_dep_node_index += 1;
    self.dep_id_table.insert(dep_id, index);
    index
  }

  pub fn db(&self) -> &dyn QueryDatabase {
    self.db
  }

  pub fn dep_id_table(&self) -> &HashMap<DepId, DepNodeIndex> {
    &self.dep_id_table
  }

  pub fn finish(self) -> Vec<Vec<u8>> {
    self.intern_blobs
  }

  pub fn get_intern_blob_index_fast<T: 'static>(&self, hint: u32) -> Option<u32> {
    let key = (std::any::TypeId::of::<T>(), hint as usize);
    self.intern_hints.get(&key).copied()
  }

  // Insert blob to intern table if blob does not already exist
  // Hint can be used to speed up looking up
  // Note that equal blobs should ensure equal hints, else, you risk duplicating the blobs
  pub fn intern_blob<T: 'static>(&mut self, blob: Vec<u8>, hint: Option<u32>) -> u32 {
    let hint = hint.map(|id| (std::any::TypeId::of::<T>(), id as usize));
    if let Some(key) = hint
      && let Some(&index) = self.intern_hints.get(&key)
    {
      return index;
    }
    if let Some(&index) = self.intern_table.get(&blob) {
      if let Some(key) = hint {
        self.intern_hints.insert(key, index);
      }
      return index;
    }
    let index = self.intern_blobs.len() as u32;
    self.intern_table.insert(blob.clone(), index);
    self.intern_blobs.push(blob);
    if let Some(key) = hint {
      self.intern_hints.insert(key, index);
    }
    index
  }

  pub fn emit_raw(&self, buffer: &mut Vec<u8>, bytes: &[u8]) -> usize {
    buffer.extend_from_slice(bytes);
    bytes.len()
  }

  pub fn emit_u8(&self, buffer: &mut Vec<u8>, value: u8) -> usize {
    self.emit_raw(buffer, &[value])
  }
  pub fn emit_u16(&self, buffer: &mut Vec<u8>, value: u16) -> usize {
    self.emit_raw(buffer, &value.to_le_bytes())
  }
  pub fn emit_u32(&self, buffer: &mut Vec<u8>, value: u32) -> usize {
    self.emit_raw(buffer, &value.to_le_bytes())
  }
  pub fn emit_u64(&self, buffer: &mut Vec<u8>, value: u64) -> usize {
    self.emit_raw(buffer, &value.to_le_bytes())
  }
  pub fn emit_u128(&self, buffer: &mut Vec<u8>, value: u128) -> usize {
    self.emit_raw(buffer, &value.to_le_bytes())
  }
  pub fn emit_usize(&self, buffer: &mut Vec<u8>, value: usize) -> usize {
    self.emit_u64(buffer, value as u64)
  }
  pub fn emit_isize(&self, buffer: &mut Vec<u8>, value: isize) -> usize {
    self.emit_i64(buffer, value as i64)
  }
  pub fn emit_i8(&self, buffer: &mut Vec<u8>, value: i8) -> usize {
    self.emit_raw(buffer, &[value as u8])
  }
  pub fn emit_i16(&self, buffer: &mut Vec<u8>, value: i16) -> usize {
    self.emit_raw(buffer, &value.to_le_bytes())
  }
  pub fn emit_i32(&self, buffer: &mut Vec<u8>, value: i32) -> usize {
    self.emit_raw(buffer, &value.to_le_bytes())
  }
  pub fn emit_i64(&self, buffer: &mut Vec<u8>, value: i64) -> usize {
    self.emit_raw(buffer, &value.to_le_bytes())
  }
  pub fn emit_i128(&self, buffer: &mut Vec<u8>, value: i128) -> usize {
    self.emit_raw(buffer, &value.to_le_bytes())
  }
  pub fn emit_f64(&self, buffer: &mut Vec<u8>, value: f64) -> usize {
    self.emit_raw(buffer, &value.to_le_bytes())
  }
  pub fn emit_bool(&self, buffer: &mut Vec<u8>, value: bool) -> usize {
    self.emit_u8(buffer, value as u8)
  }
  pub fn emit_char(&self, buffer: &mut Vec<u8>, value: char) -> usize {
    self.emit_u32(buffer, value as u32)
  }
  pub fn emit_str(&self, buffer: &mut Vec<u8>, value: &str) -> usize {
    self.emit_u32(buffer, value.len() as u32) + self.emit_raw(buffer, value.as_bytes())
  }
  pub fn emit_bytes(&self, buffer: &mut Vec<u8>, value: &[u8]) -> usize {
    self.emit_u32(buffer, value.len() as u32) + self.emit_raw(buffer, value)
  }
}

pub struct Decoder {
  storage: Arc<QueryStorage>,
  intern_blobs: Arc<Vec<Vec<u8>>>,
  dep_id_table: DashMap<DepNodeIndex, DepId>,
}

impl Decoder {
  pub fn new(storage: Arc<QueryStorage>, intern_blobs: Arc<Vec<Vec<u8>>>) -> Self {
    Self {
      storage,
      intern_blobs,
      dep_id_table: DashMap::new(),
    }
  }

  pub fn storage(&self) -> &QueryStorage {
    &self.storage
  }

  /// Map a DepNodeIndex to a DepId
  /// No-op if already set
  pub fn set_dep_node_id(&self, index: DepNodeIndex, dep_id: DepId) {
    self.dep_id_table.entry(index).or_insert(dep_id);
  }

  pub fn get_dep_node_id(&self, index: DepNodeIndex) -> Option<DepId> {
    self.dep_id_table.get(&index).map(|entry| *entry.value())
  }

  // Get the DepId for a node, triggering deserialization if not yet loaded
  pub fn get_or_deserialize_dep_node_id(&self, index: DepNodeIndex) -> Option<DepId> {
    if let Some(dep_id) = self.get_dep_node_id(index) {
      return Some(dep_id);
    }
    let ctx = self.storage.deserialize_ctx.get()?;
    let node = &ctx.serialized.dep_graph.nodes[index as usize];
    let name = node.name();
    let node_field_index = node.field_index();

    match node {
      DepNode::InputField { .. } => {
        for &_index in ctx.inputs_by_name(&name) {
          let input = &self.storage.inputs[_index];
          if Some(input.field_index()) == node_field_index {
            return input.deserialize(ctx, index);
          }
        }
      }
      DepNode::Interned { .. } => {
        if let Some(&_index) = ctx.interned_by_name(&name).iter().next() {
          return self.storage.interned[_index].deserialize(ctx, index);
        }
      }
      DepNode::DerivedQuery { .. } => {
        if let Some(&_index) = ctx.queries_by_name(&name).iter().next() {
          return self.storage.queries[_index].deserialize(ctx, index);
        }
      }
      DepNode::DerivedField { .. } => {
        for &_index in ctx.fields_by_name(&name) {
          let field = &self.storage.fields[_index];
          if Some(field.field_index()) == node_field_index {
            return field.deserialize(ctx, index);
          }
        }
      }
      // Evicted means this node was not serialized (LRU-evicted or never accessed)
      // Return None so callers treat it as a cache miss and recompute
      DepNode::Evicted => {}
    }
    None
  }

  pub fn get_intern_blob(&self, index: u32) -> &[u8] {
    &self.intern_blobs[index as usize]
  }

  pub fn read_raw(&self, data: &mut &[u8], buffer: &mut [u8]) -> usize {
    let length = buffer.len();
    buffer.copy_from_slice(&data[..length]);
    *data = &data[length..];
    length
  }

  pub fn read_u8(&self, data: &mut &[u8]) -> u8 {
    let mut buffer = [0u8; 1];
    self.read_raw(data, &mut buffer);
    buffer[0]
  }
  pub fn read_u16(&self, data: &mut &[u8]) -> u16 {
    let mut buffer = [0u8; 2];
    self.read_raw(data, &mut buffer);
    u16::from_le_bytes(buffer)
  }
  pub fn read_u32(&self, data: &mut &[u8]) -> u32 {
    let mut buffer = [0u8; 4];
    self.read_raw(data, &mut buffer);
    u32::from_le_bytes(buffer)
  }
  pub fn read_u64(&self, data: &mut &[u8]) -> u64 {
    let mut buffer = [0u8; 8];
    self.read_raw(data, &mut buffer);
    u64::from_le_bytes(buffer)
  }
  pub fn read_u128(&self, data: &mut &[u8]) -> u128 {
    let mut buffer = [0u8; 16];
    self.read_raw(data, &mut buffer);
    u128::from_le_bytes(buffer)
  }
  pub fn read_usize(&self, data: &mut &[u8]) -> usize {
    self.read_u64(data) as usize
  }
  pub fn read_isize(&self, data: &mut &[u8]) -> isize {
    self.read_i64(data) as isize
  }
  pub fn read_i8(&self, data: &mut &[u8]) -> i8 {
    self.read_u8(data) as i8
  }
  pub fn read_i16(&self, data: &mut &[u8]) -> i16 {
    let mut buffer = [0u8; 2];
    self.read_raw(data, &mut buffer);
    i16::from_le_bytes(buffer)
  }
  pub fn read_i32(&self, data: &mut &[u8]) -> i32 {
    let mut buffer = [0u8; 4];
    self.read_raw(data, &mut buffer);
    i32::from_le_bytes(buffer)
  }
  pub fn read_i64(&self, data: &mut &[u8]) -> i64 {
    let mut buffer = [0u8; 8];
    self.read_raw(data, &mut buffer);
    i64::from_le_bytes(buffer)
  }
  pub fn read_i128(&self, data: &mut &[u8]) -> i128 {
    let mut buffer = [0u8; 16];
    self.read_raw(data, &mut buffer);
    i128::from_le_bytes(buffer)
  }
  pub fn read_f64(&self, data: &mut &[u8]) -> f64 {
    let mut buffer = [0u8; 8];
    self.read_raw(data, &mut buffer);
    f64::from_le_bytes(buffer)
  }
  pub fn read_bool(&self, data: &mut &[u8]) -> bool {
    self.read_u8(data) != 0
  }
  pub fn read_char(&self, data: &mut &[u8]) -> char {
    char::from_u32(self.read_u32(data)).unwrap()
  }
  pub fn read_str(&self, data: &mut &[u8]) -> String {
    let length = self.read_u32(data) as usize;
    let mut buffer = vec![0u8; length];
    self.read_raw(data, &mut buffer);
    String::from_utf8(buffer).unwrap()
  }
  pub fn read_bytes_owned(&self, data: &mut &[u8]) -> Vec<u8> {
    let length = self.read_u32(data) as usize;
    let mut buffer = vec![0u8; length];
    self.read_raw(data, &mut buffer);
    buffer
  }
}

// Encodable / Decodable traits
pub trait Encodable {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder);

  // Encode as a field inside another struct
  // Id types override to write just a dep index reference
  fn field_encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    self.encode(buffer, encoder);
  }
}

pub trait Decodable: Sized {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self;

  // Decode as a field inside another struct
  // Id types override to read a dep index reference
  fn field_decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    Self::decode(data, decoder)
  }
}

/* Primitive implementations */

// u8
impl Encodable for u8 {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buffer, *self);
  }
}
impl Decodable for u8 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_u8(data)
  }
}

// u16
impl Encodable for u16 {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u16(buffer, *self);
  }
}
impl Decodable for u16 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_u16(data)
  }
}

// u32
impl Encodable for u32 {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buffer, *self);
  }
}
impl Decodable for u32 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_u32(data)
  }
}

// u64
impl Encodable for u64 {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u64(buffer, *self);
  }
}
impl Decodable for u64 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_u64(data)
  }
}

// usize (encoded as u64)
impl Encodable for usize {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_usize(buffer, *self);
  }
}
impl Decodable for usize {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_usize(data)
  }
}

// u128
impl Encodable for u128 {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u128(buffer, *self);
  }
}
impl Decodable for u128 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_u128(data)
  }
}

// isize
impl Encodable for isize {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_isize(buffer, *self);
  }
}
impl Decodable for isize {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_isize(data)
  }
}

// i8
impl Encodable for i8 {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_i8(buffer, *self);
  }
}
impl Decodable for i8 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_i8(data)
  }
}

// i16
impl Encodable for i16 {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_i16(buffer, *self);
  }
}
impl Decodable for i16 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_i16(data)
  }
}

// i32
impl Encodable for i32 {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_i32(buffer, *self);
  }
}
impl Decodable for i32 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_i32(data)
  }
}

// i64
impl Encodable for i64 {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_i64(buffer, *self);
  }
}
impl Decodable for i64 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_i64(data)
  }
}

// i128
impl Encodable for i128 {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_i128(buffer, *self);
  }
}
impl Decodable for i128 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_i128(data)
  }
}

// f64
impl Encodable for f64 {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_f64(buffer, *self);
  }
}
impl Decodable for f64 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_f64(data)
  }
}

// bool
impl Encodable for bool {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_bool(buffer, *self);
  }
}
impl Decodable for bool {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_bool(data)
  }
}

// char
impl Encodable for char {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_char(buffer, *self);
  }
}
impl Decodable for char {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_char(data)
  }
}

// str
impl Encodable for str {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_str(buffer, self);
  }
}

// String
impl Encodable for String {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_str(buffer, self);
  }
}
impl Decodable for String {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_str(data)
  }
}

// PathBuf
impl Encodable for PathBuf {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    let string = self.to_str().expect("non-UTF-8 path in cache encode");
    encoder.emit_str(buffer, string);
  }
}
impl Decodable for PathBuf {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    PathBuf::from(decoder.read_str(data))
  }
}

// Option
impl<T: Encodable> Encodable for Option<T> {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      None => {
        encoder.emit_u8(buffer, 0);
      }
      Some(value) => {
        encoder.emit_u8(buffer, 1);
        value.field_encode(buffer, encoder);
      }
    }
  }
}
impl<T: Decodable> Decodable for Option<T> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    match decoder.read_u8(data) {
      0 => None,
      _ => Some(T::field_decode(data, decoder)),
    }
  }
}

// [T]
impl<T: Encodable> Encodable for [T] {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buffer, self.len() as u32);
    for item in self {
      item.field_encode(buffer, encoder);
    }
  }
}

// &T
impl<T: Encodable + ?Sized> Encodable for &T {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    (**self).field_encode(buffer, encoder);
  }
}

// Vec
impl<T: Encodable> Encodable for Vec<T> {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buffer, self.len() as u32);
    for item in self {
      item.field_encode(buffer, encoder);
    }
  }
}
impl<T: Decodable> Decodable for Vec<T> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let length = decoder.read_u32(data) as usize;
    (0..length)
      .map(|_| T::field_decode(data, decoder))
      .collect()
  }
}

// Box
impl<T: Encodable> Encodable for Box<T> {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    (**self).field_encode(buffer, encoder);
  }
}
impl<T: Decodable> Decodable for Box<T> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    Box::new(T::field_decode(data, decoder))
  }
}

// ()
impl Encodable for () {
  fn encode(&self, _buffer: &mut Vec<u8>, _encoder: &mut Encoder) {}
}
impl Decodable for () {
  fn decode(_data: &mut &[u8], _decoder: &Decoder) -> Self {}
}

// (A,) tuple
impl<A: Encodable> Encodable for (A,) {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    self.0.field_encode(buffer, encoder);
  }
}
impl<A: Decodable> Decodable for (A,) {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    (A::field_decode(data, decoder),)
  }
}

// (A, B, C) tuple
impl<A: Encodable, B: Encodable, C: Encodable> Encodable for (A, B, C) {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    self.0.field_encode(buffer, encoder);
    self.1.field_encode(buffer, encoder);
    self.2.field_encode(buffer, encoder);
  }
}
impl<A: Decodable, B: Decodable, C: Decodable> Decodable for (A, B, C) {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    (
      A::field_decode(data, decoder),
      B::field_decode(data, decoder),
      C::field_decode(data, decoder),
    )
  }
}

// (A, B) tuple
impl<A: Encodable, B: Encodable> Encodable for (A, B) {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    self.0.field_encode(buffer, encoder);
    self.1.field_encode(buffer, encoder);
  }
}
impl<A: Decodable, B: Decodable> Decodable for (A, B) {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    (
      A::field_decode(data, decoder),
      B::field_decode(data, decoder),
    )
  }
}

// BTreeMap, already sorted so no StableCompare needed for encode
impl<K: Encodable + Ord, V: Encodable> Encodable for BTreeMap<K, V> {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buffer, self.len() as u32);
    for (key, value) in self {
      key.field_encode(buffer, encoder);
      value.field_encode(buffer, encoder);
    }
  }
}
impl<K: Decodable + Ord, V: Decodable> Decodable for BTreeMap<K, V> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let length = decoder.read_u32(data) as usize;
    let mut map = BTreeMap::new();
    for _ in 0..length {
      map.insert(
        K::field_decode(data, decoder),
        V::field_decode(data, decoder),
      );
    }
    map
  }
}

// BTreeSet
impl<V: Encodable + Ord> Encodable for BTreeSet<V> {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buffer, self.len() as u32);
    for value in self {
      value.field_encode(buffer, encoder);
    }
  }
}
impl<V: Decodable + Ord> Decodable for BTreeSet<V> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let length = decoder.read_u32(data) as usize;
    let mut set = BTreeSet::new();
    for _ in 0..length {
      set.insert(V::field_decode(data, decoder));
    }
    set
  }
}

// HashMap
impl<K: Encodable + StableCompare, V: Encodable> Encodable for HashMap<K, V> {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buffer, self.len() as u32);
    let mut entries: Vec<(&K, &V)> = self.iter().collect();
    entries.sort_by(|(left_key, _), (right_key, _)| left_key.stable_cmp(encoder.db(), right_key));
    for (key, value) in entries {
      key.field_encode(buffer, encoder);
      value.field_encode(buffer, encoder);
    }
  }
}
impl<K: Decodable + Eq + Hash, V: Decodable> Decodable for HashMap<K, V> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let length = decoder.read_u32(data) as usize;
    let mut map = HashMap::with_capacity(length);
    for _ in 0..length {
      map.insert(
        K::field_decode(data, decoder),
        V::field_decode(data, decoder),
      );
    }
    map
  }
}

// HashSet
impl<V: Encodable + StableCompare> Encodable for HashSet<V> {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buffer, self.len() as u32);
    let mut entries: Vec<&V> = self.iter().collect();
    entries.sort_by(|left_value, right_value| left_value.stable_cmp(encoder.db(), right_value));
    for value in entries {
      value.field_encode(buffer, encoder);
    }
  }
}
impl<V: Decodable + Eq + Hash> Decodable for HashSet<V> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let length = decoder.read_u32(data) as usize;
    let mut map = HashSet::with_capacity(length);
    for _ in 0..length {
      map.insert(V::field_decode(data, decoder));
    }
    map
  }
}

// Either
impl<L: Encodable, R: Encodable> Encodable for Either<L, R> {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      Either::Left(value) => {
        encoder.emit_u8(buffer, 0);
        value.field_encode(buffer, encoder);
      }
      Either::Right(value) => {
        encoder.emit_u8(buffer, 1);
        value.field_encode(buffer, encoder);
      }
    }
  }
}
impl<L: Decodable, R: Decodable> Decodable for Either<L, R> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    match decoder.read_u8(data) {
      0 => Either::Left(L::field_decode(data, decoder)),
      _ => Either::Right(R::field_decode(data, decoder)),
    }
  }
}
