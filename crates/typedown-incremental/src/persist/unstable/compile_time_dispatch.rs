// TIL: We use nightly `specialization` to simulate C++ `if constexpr` for compile-time type dispatch
// - FieldEncodable/FieldDecodable dispatch between query struct IDs (as DepNodeIndex) and plain types
// - Stable Rust has no way to do this: autoref specialization fails because Id and Encodable overlap

use crate::{Decodable, Decoder, Encodable, Encoder, Id, TOMBSTONE_ENTRY_ID};

/// Encode a field value. For query struct IDs (Id types), encodes as DepNodeIndex.
/// For plain types, delegates to Encodable::encode.
pub trait FieldEncodable {
  fn encode_field(&self, buf: &mut Vec<u8>, encoder: &mut Encoder);
}

impl<T: Encodable> FieldEncodable for T {
  default fn encode_field(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.encode(buf, encoder);
  }
}

impl<T: Id + Encodable + Into<usize>> FieldEncodable for T {
  fn encode_field(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    let index = encoder.add_dep_id(self.as_id());
    encoder.emit_u32(buf, index);
  }
}

/// Decode a field value. For query struct IDs (Id types), decodes from DepNodeIndex.
/// For plain types, delegates to Decodable::decode.
pub trait FieldDecodable: Sized {
  fn decode_field(data: &mut &[u8], decoder: &Decoder) -> Self;
}

impl<T: Decodable> FieldDecodable for T {
  default fn decode_field(data: &mut &[u8], decoder: &Decoder) -> Self {
    T::decode(data, decoder)
  }
}

impl<T: Id + Decodable + From<usize>> FieldDecodable for T {
  fn decode_field(data: &mut &[u8], decoder: &Decoder) -> Self {
    let index = decoder.read_u32(data);
    let entry_id = decoder
      .get_or_deserialize_dep_node_id(index)
      .map(|dep_id| dep_id.1)
      .unwrap_or(TOMBSTONE_ENTRY_ID);
    Self::from(entry_id)
  }
}
