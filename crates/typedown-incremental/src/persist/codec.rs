use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::path::PathBuf;
use std::sync::Arc;

use dashmap::DashMap;
use typedown_types::either::Either;

use crate::persist::serialized::dep_graph::DepNodeIndex;
use crate::persist::stable::StableCompare;
use crate::persist::unstable::{FieldDecodable, FieldEncodable};
use crate::{DepId, QueryDatabase, QueryStorage};

pub struct Encoder<'a> {
  db: &'a dyn QueryDatabase,
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

  /// Register a DepId and get a stable DepNodeIndex for it.
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

  pub fn intern_blob<T: 'static>(&mut self, blob: Vec<u8>, hint: Option<usize>) -> u32 {
    let hint = hint.map(|id| (std::any::TypeId::of::<T>(), id));
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

  pub fn emit_raw(&self, buf: &mut Vec<u8>, bytes: &[u8]) -> usize {
    buf.extend_from_slice(bytes);
    bytes.len()
  }

  pub fn emit_u8(&self, buf: &mut Vec<u8>, v: u8) -> usize {
    self.emit_raw(buf, &[v])
  }
  pub fn emit_u16(&self, buf: &mut Vec<u8>, v: u16) -> usize {
    self.emit_raw(buf, &v.to_le_bytes())
  }
  pub fn emit_u32(&self, buf: &mut Vec<u8>, v: u32) -> usize {
    self.emit_raw(buf, &v.to_le_bytes())
  }
  pub fn emit_u64(&self, buf: &mut Vec<u8>, v: u64) -> usize {
    self.emit_raw(buf, &v.to_le_bytes())
  }
  pub fn emit_u128(&self, buf: &mut Vec<u8>, v: u128) -> usize {
    self.emit_raw(buf, &v.to_le_bytes())
  }
  pub fn emit_usize(&self, buf: &mut Vec<u8>, v: usize) -> usize {
    self.emit_u64(buf, v as u64)
  }
  pub fn emit_isize(&self, buf: &mut Vec<u8>, v: isize) -> usize {
    self.emit_i64(buf, v as i64)
  }
  pub fn emit_i8(&self, buf: &mut Vec<u8>, v: i8) -> usize {
    self.emit_raw(buf, &[v as u8])
  }
  pub fn emit_i16(&self, buf: &mut Vec<u8>, v: i16) -> usize {
    self.emit_raw(buf, &v.to_le_bytes())
  }
  pub fn emit_i32(&self, buf: &mut Vec<u8>, v: i32) -> usize {
    self.emit_raw(buf, &v.to_le_bytes())
  }
  pub fn emit_i64(&self, buf: &mut Vec<u8>, v: i64) -> usize {
    self.emit_raw(buf, &v.to_le_bytes())
  }
  pub fn emit_i128(&self, buf: &mut Vec<u8>, v: i128) -> usize {
    self.emit_raw(buf, &v.to_le_bytes())
  }
  pub fn emit_f64(&self, buf: &mut Vec<u8>, v: f64) -> usize {
    self.emit_raw(buf, &v.to_le_bytes())
  }
  pub fn emit_bool(&self, buf: &mut Vec<u8>, v: bool) -> usize {
    self.emit_u8(buf, v as u8)
  }
  pub fn emit_char(&self, buf: &mut Vec<u8>, v: char) -> usize {
    self.emit_u32(buf, v as u32)
  }
  pub fn emit_str(&self, buf: &mut Vec<u8>, v: &str) -> usize {
    self.emit_u32(buf, v.len() as u32) + self.emit_raw(buf, v.as_bytes())
  }
  pub fn emit_bytes(&self, buf: &mut Vec<u8>, v: &[u8]) -> usize {
    self.emit_u32(buf, v.len() as u32) + self.emit_raw(buf, v)
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

  /// Map a DepNodeIndex to a DepId. No-op if already set.
  pub fn set_dep_node_id(&self, index: DepNodeIndex, dep_id: DepId) {
    self.dep_id_table.entry(index).or_insert(dep_id);
  }

  pub fn get_dep_node_id(&self, index: DepNodeIndex) -> Option<DepId> {
    self.dep_id_table.get(&index).map(|e| *e.value())
  }

  /// Get the DepId for a node, triggering deserialization if not yet loaded.
  pub fn get_or_deserialize_dep_node_id(&self, index: DepNodeIndex) -> Option<DepId> {
    if let Some(dep_id) = self.get_dep_node_id(index) {
      return Some(dep_id);
    }
    let ctx = self.storage.deserialize_ctx.get()?;
    let node = &ctx.serialized.dep_graph.nodes[index as usize];
    let name = node.name();
    let node_field_index = node.field_index();
    for &idx in ctx.ingredients_by_name(&name) {
      if self.storage.ingredients[idx].field_index == node_field_index {
        return self.storage.ingredients[idx]
          .ingredient
          .deserialize(ctx, index);
      }
    }
    None
  }

  pub fn get_intern_blob(&self, index: u32) -> &[u8] {
    &self.intern_blobs[index as usize]
  }

  pub fn read_raw(&self, data: &mut &[u8], buf: &mut [u8]) -> usize {
    let len = buf.len();
    buf.copy_from_slice(&data[..len]);
    *data = &data[len..];
    len
  }

  pub fn read_u8(&self, data: &mut &[u8]) -> u8 {
    let mut buf = [0u8; 1];
    self.read_raw(data, &mut buf);
    buf[0]
  }
  pub fn read_u16(&self, data: &mut &[u8]) -> u16 {
    let mut buf = [0u8; 2];
    self.read_raw(data, &mut buf);
    u16::from_le_bytes(buf)
  }
  pub fn read_u32(&self, data: &mut &[u8]) -> u32 {
    let mut buf = [0u8; 4];
    self.read_raw(data, &mut buf);
    u32::from_le_bytes(buf)
  }
  pub fn read_u64(&self, data: &mut &[u8]) -> u64 {
    let mut buf = [0u8; 8];
    self.read_raw(data, &mut buf);
    u64::from_le_bytes(buf)
  }
  pub fn read_u128(&self, data: &mut &[u8]) -> u128 {
    let mut buf = [0u8; 16];
    self.read_raw(data, &mut buf);
    u128::from_le_bytes(buf)
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
    let mut buf = [0u8; 2];
    self.read_raw(data, &mut buf);
    i16::from_le_bytes(buf)
  }
  pub fn read_i32(&self, data: &mut &[u8]) -> i32 {
    let mut buf = [0u8; 4];
    self.read_raw(data, &mut buf);
    i32::from_le_bytes(buf)
  }
  pub fn read_i64(&self, data: &mut &[u8]) -> i64 {
    let mut buf = [0u8; 8];
    self.read_raw(data, &mut buf);
    i64::from_le_bytes(buf)
  }
  pub fn read_i128(&self, data: &mut &[u8]) -> i128 {
    let mut buf = [0u8; 16];
    self.read_raw(data, &mut buf);
    i128::from_le_bytes(buf)
  }
  pub fn read_f64(&self, data: &mut &[u8]) -> f64 {
    let mut buf = [0u8; 8];
    self.read_raw(data, &mut buf);
    f64::from_le_bytes(buf)
  }
  pub fn read_bool(&self, data: &mut &[u8]) -> bool {
    self.read_u8(data) != 0
  }
  pub fn read_char(&self, data: &mut &[u8]) -> char {
    char::from_u32(self.read_u32(data)).unwrap()
  }
  pub fn read_str(&self, data: &mut &[u8]) -> String {
    let len = self.read_u32(data) as usize;
    let mut buf = vec![0u8; len];
    self.read_raw(data, &mut buf);
    String::from_utf8(buf).unwrap()
  }
  pub fn read_bytes_owned(&self, data: &mut &[u8]) -> Vec<u8> {
    let len = self.read_u32(data) as usize;
    let mut buf = vec![0u8; len];
    self.read_raw(data, &mut buf);
    buf
  }
}

// Encodable / Decodable traits
pub trait Encodable {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder);
}

pub trait Decodable: Sized {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self;
}

/* Primitive implementations */

// u8
impl Encodable for u8 {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buf, *self);
  }
}
impl Decodable for u8 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_u8(data)
  }
}

// u16
impl Encodable for u16 {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u16(buf, *self);
  }
}
impl Decodable for u16 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_u16(data)
  }
}

// u32
impl Encodable for u32 {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buf, *self);
  }
}
impl Decodable for u32 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_u32(data)
  }
}

// u64
impl Encodable for u64 {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u64(buf, *self);
  }
}
impl Decodable for u64 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_u64(data)
  }
}

// usize (encoded as u64)
impl Encodable for usize {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_usize(buf, *self);
  }
}
impl Decodable for usize {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_usize(data)
  }
}

// u128
impl Encodable for u128 {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u128(buf, *self);
  }
}
impl Decodable for u128 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_u128(data)
  }
}

// isize
impl Encodable for isize {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_isize(buf, *self);
  }
}
impl Decodable for isize {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_isize(data)
  }
}

// i8
impl Encodable for i8 {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_i8(buf, *self);
  }
}
impl Decodable for i8 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_i8(data)
  }
}

// i16
impl Encodable for i16 {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_i16(buf, *self);
  }
}
impl Decodable for i16 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_i16(data)
  }
}

// i32
impl Encodable for i32 {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_i32(buf, *self);
  }
}
impl Decodable for i32 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_i32(data)
  }
}

// i64
impl Encodable for i64 {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_i64(buf, *self);
  }
}
impl Decodable for i64 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_i64(data)
  }
}

// i128
impl Encodable for i128 {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_i128(buf, *self);
  }
}
impl Decodable for i128 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_i128(data)
  }
}

// f64
impl Encodable for f64 {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_f64(buf, *self);
  }
}
impl Decodable for f64 {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_f64(data)
  }
}

// bool
impl Encodable for bool {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_bool(buf, *self);
  }
}
impl Decodable for bool {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_bool(data)
  }
}

// char
impl Encodable for char {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_char(buf, *self);
  }
}
impl Decodable for char {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_char(data)
  }
}

// str
impl Encodable for str {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_str(buf, self);
  }
}

// String
impl Encodable for String {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_str(buf, self);
  }
}
impl Decodable for String {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    decoder.read_str(data)
  }
}

// PathBuf
impl Encodable for PathBuf {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_str(buf, &self.to_string_lossy());
  }
}
impl Decodable for PathBuf {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    PathBuf::from(decoder.read_str(data))
  }
}

// Option
impl<T: Encodable> Encodable for Option<T> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      None => {
        encoder.emit_u8(buf, 0);
      }
      Some(val) => {
        encoder.emit_u8(buf, 1);
        val.encode_field(buf, encoder);
      }
    }
  }
}
impl<T: FieldDecodable> Decodable for Option<T> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    match decoder.read_u8(data) {
      0 => None,
      _ => Some(T::decode_field(data, decoder)),
    }
  }
}

// [T]
impl<T: FieldEncodable> Encodable for [T] {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buf, self.len() as u32);
    for item in self {
      item.encode_field(buf, encoder);
    }
  }
}

// &T
impl<T: FieldEncodable + ?Sized> Encodable for &T {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    (**self).encode_field(buf, encoder);
  }
}

// Vec
impl<T: FieldEncodable> Encodable for Vec<T> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buf, self.len() as u32);
    for item in self {
      item.encode_field(buf, encoder);
    }
  }
}
impl<T: FieldDecodable> Decodable for Vec<T> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let len = decoder.read_u32(data) as usize;
    (0..len).map(|_| T::decode_field(data, decoder)).collect()
  }
}

// Box
impl<T: FieldEncodable> Encodable for Box<T> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    (**self).encode_field(buf, encoder);
  }
}
impl<T: FieldDecodable> Decodable for Box<T> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    Box::new(T::decode_field(data, decoder))
  }
}

// ()
impl Encodable for () {
  fn encode(&self, _buf: &mut Vec<u8>, _encoder: &mut Encoder) {}
}
impl Decodable for () {
  fn decode(_data: &mut &[u8], _decoder: &Decoder) -> Self {}
}

// (A,) tuple
impl<A: FieldEncodable> Encodable for (A,) {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.0.encode_field(buf, encoder);
  }
}
impl<A: FieldDecodable> Decodable for (A,) {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    (A::decode_field(data, decoder),)
  }
}

// (A, B, C) tuple
impl<A: FieldEncodable, B: FieldEncodable, C: FieldEncodable> Encodable for (A, B, C) {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.0.encode_field(buf, encoder);
    self.1.encode_field(buf, encoder);
    self.2.encode_field(buf, encoder);
  }
}
impl<A: FieldDecodable, B: FieldDecodable, C: FieldDecodable> Decodable for (A, B, C) {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    (
      A::decode_field(data, decoder),
      B::decode_field(data, decoder),
      C::decode_field(data, decoder),
    )
  }
}

// (A, B) tuple
impl<A: FieldEncodable, B: FieldEncodable> Encodable for (A, B) {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.0.encode_field(buf, encoder);
    self.1.encode_field(buf, encoder);
  }
}
impl<A: FieldDecodable, B: FieldDecodable> Decodable for (A, B) {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    (
      A::decode_field(data, decoder),
      B::decode_field(data, decoder),
    )
  }
}

// HashMap
impl<K: FieldEncodable + StableCompare, V: FieldEncodable> Encodable for HashMap<K, V> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buf, self.len() as u32);
    let mut entries: Vec<(&K, &V)> = self.iter().collect();
    entries.sort_by(|(k1, _), (k2, _)| k1.stable_cmp(encoder.db(), k2));
    for (key, value) in entries {
      key.encode_field(buf, encoder);
      value.encode_field(buf, encoder);
    }
  }
}
impl<K: FieldDecodable + Eq + Hash, V: FieldDecodable> Decodable for HashMap<K, V> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let len = decoder.read_u32(data) as usize;
    let mut map = HashMap::with_capacity(len);
    for _ in 0..len {
      map.insert(
        K::decode_field(data, decoder),
        V::decode_field(data, decoder),
      );
    }
    map
  }
}

// HashSet
impl<V: FieldEncodable + StableCompare> Encodable for HashSet<V> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u32(buf, self.len() as u32);
    let mut entries: Vec<&V> = self.iter().collect();
    entries.sort_by(|v1, v2| v1.stable_cmp(encoder.db(), v2));
    for v in entries {
      v.encode_field(buf, encoder);
    }
  }
}
impl<V: FieldDecodable + Eq + Hash> Decodable for HashSet<V> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let len = decoder.read_u32(data) as usize;
    let mut map = HashSet::with_capacity(len);
    for _ in 0..len {
      map.insert(V::decode_field(data, decoder));
    }
    map
  }
}

// Either
impl<L: FieldEncodable, R: FieldEncodable> Encodable for Either<L, R> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      Either::Left(val) => {
        encoder.emit_u8(buf, 0);
        val.encode_field(buf, encoder);
      }
      Either::Right(val) => {
        encoder.emit_u8(buf, 1);
        val.encode_field(buf, encoder);
      }
    }
  }
}
impl<L: FieldDecodable, R: FieldDecodable> Decodable for Either<L, R> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    match decoder.read_u8(data) {
      0 => Either::Left(L::decode_field(data, decoder)),
      _ => Either::Right(R::decode_field(data, decoder)),
    }
  }
}
