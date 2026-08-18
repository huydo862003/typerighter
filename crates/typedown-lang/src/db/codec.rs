use crate::syntax::diagnostic::{Diagnostic, DiagnosticCode};
use crate::syntax::green::cache::with_green_cache;
use crate::syntax::green::node::SyntaxNode;
use crate::syntax::green::token::SyntaxToken;
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
  let hint = Some(node.as_ptr());
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
    for idx in &child_indices {
      blob.extend_from_slice(&idx.to_le_bytes());
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
  let kind_val = u16::from_le_bytes(blob[1..3].try_into().unwrap());

  let kind = SyntaxKind::from_repr(kind_val).expect("unknown SyntaxKind");

  match tag {
    0 => {
      let child_count = u32::from_le_bytes(blob[3..7].try_into().unwrap()) as usize;
      let mut children = Vec::with_capacity(child_count);
      for idx in 0..child_count {
        let offset = 7 + idx * 4;
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

// StableHash impls for syntax types

impl StableHash for SyntaxToken {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.kind().stable_hash(db, hasher);
    self.bytes().stable_hash(db, hasher);
  }
}

impl StableHash for SyntaxNode {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.kind().stable_hash(db, hasher);
    self.children().stable_hash(db, hasher);
  }
}

impl StableHash for GreenNode {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    if self.is_node() {
      std::hash::Hasher::write_u8(hasher, 0);
      self.as_node().unwrap().stable_hash(db, hasher);
    } else {
      std::hash::Hasher::write_u8(hasher, 1);
      self.as_token().unwrap().stable_hash(db, hasher);
    }
  }
}

impl StableHash for RedNode {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    (self.offset() as u64).stable_hash(db, hasher);
    (**self).stable_hash(db, hasher);
  }
}

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
    let val = decoder.read_u16(data);
    SyntaxKind::from_repr(val).expect("unknown SyntaxKind")
  }
}

impl StableHash for SyntaxKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, _db: &DB, hasher: &mut StableHasher) {
    std::hash::Hasher::write_u16(hasher, *self as u16);
  }
}

// Diagnostic

impl Encodable for Diagnostic {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buf, self.code() as u8);
    match self {
      Diagnostic::UnexpectedEof {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnexpectedChar {
        expected,
        encountered,
        start_offset,
        end_offset,
      } => {
        expected.encode(buf, encoder);
        encountered.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnterminatedString {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnterminatedInterpolation {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnterminatedCodeBlock {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnterminatedInlineCode {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnterminatedMathBlock {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnterminatedInlineMath {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MissingCodeBlockNewline {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MissingMathBlockNewline {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::InvalidChar {
        encountered,
        start_offset,
        end_offset,
      } => {
        encountered.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::InvalidUtf8 {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MixedIndentation {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::InconsistentIndentation {
        expected,
        encountered,
        start_offset,
        end_offset,
      } => {
        expected.encode(buf, encoder);
        encountered.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnmatchedDedent {
        indent,
        start_offset,
        end_offset,
      } => {
        indent.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MissingExponentDigits {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnexpectedTokensOnFrontmatterMarkerLine {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnexpectedContainerPropItem {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnexpectedContainerPropValue {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnexpectedContainerSlotSeparatorToken {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnclosedContainerPropBlock {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MissingContainerPropValueAfterEq { offset } => {
        offset.encode(buf, encoder);
      }
      Diagnostic::MissingFrontmatterMarker { offset } => {
        offset.encode(buf, encoder);
      }
      Diagnostic::MissingMarkdownHeadingHash {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MissingRequiredSpacesBetweenHashAndHeading {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MissingSyntaxNode {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnclosedLink {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnclosedBold {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnclosedItalic {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnclosedStrikethrough {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnclosedBoldItalic {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MismatchedItalicDelimiter {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MissingExpectMdPrefix {
        expected_prefix,
        start_offset,
        end_offset,
      } => {
        expected_prefix.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MissingTableSeparatorRow {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::TableColumnCountMismatch {
        expected,
        found,
        start_offset,
        end_offset,
      } => {
        expected.encode(buf, encoder);
        found.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::InsufficientBlockIndent {
        expected_more_than,
        found,
        start_offset,
        end_offset,
      } => {
        expected_more_than.encode(buf, encoder);
        found.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MissingVaultConfig { root_dir } => {
        root_dir.encode(buf, encoder);
      }
      Diagnostic::VaultConfigReadError { path, message } => {
        path.encode(buf, encoder);
        message.encode(buf, encoder);
      }
      Diagnostic::VaultConfigParseError {
        path,
        message,
        start_offset,
        end_offset,
      } => {
        path.encode(buf, encoder);
        message.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::VaultConfigEmpty { path } => {
        path.encode(buf, encoder);
      }
      Diagnostic::VaultConfigMissingField {
        path,
        field,
        start_offset,
        end_offset,
      } => {
        path.encode(buf, encoder);
        field.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::VaultConfigUnknownField {
        path,
        field,
        start_offset,
        end_offset,
      } => {
        path.encode(buf, encoder);
        field.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MissingSchemaField {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnresolvedSchema {
        name,
        start_offset,
        end_offset,
      } => {
        name.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::WrongTypeArgCount { expected, got } => {
        expected.encode(buf, encoder);
        got.encode(buf, encoder);
      }
      Diagnostic::NotCallable {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::WrongArgCount {
        expected,
        got,
        start_offset,
        end_offset,
      } => {
        expected.encode(buf, encoder);
        got.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::ArgTypeMismatch {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::FieldTypeMismatch {
        field,
        expected,
        start_offset,
        end_offset,
      } => {
        field.encode(buf, encoder);
        expected.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::NotIndexable {
        start_offset,
        end_offset,
      } => {
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::IndexTypeMismatch {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::TagTypeMismatch {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::OperandTypeMismatch {
        op,
        expected,
        start_offset,
        end_offset,
      } => {
        op.encode(buf, encoder);
        expected.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::MissingRequiredField {
        field,
        start_offset,
        end_offset,
      } => {
        field.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::ElementTypeMismatch {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::DuplicateKey {
        key,
        start_offset,
        end_offset,
      } => {
        key.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnresolvedFileRef {
        path,
        start_offset,
        end_offset,
      } => {
        path.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnresolvedIdentifier {
        name,
        start_offset,
        end_offset,
      } => {
        name.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::UnknownField {
        field,
        on_type,
        start_offset,
        end_offset,
      } => {
        field.encode(buf, encoder);
        on_type.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::IndexOutOfBounds {
        index,
        length,
        start_offset,
        end_offset,
      } => {
        index.encode(buf, encoder);
        length.encode(buf, encoder);
        start_offset.encode(buf, encoder);
        end_offset.encode(buf, encoder);
      }
      Diagnostic::NestedSchemaFile { path } => {
        path.encode(buf, encoder);
      }
      Diagnostic::VaultConfigInvalidValue {
        path,
        field,
        message,
        start_offset,
        end_offset,
      } => {
        path.encode(buf, encoder);
        field.encode(buf, encoder);
        message.encode(buf, encoder);
        encoder.emit_usize(buf, *start_offset);
        encoder.emit_usize(buf, *end_offset);
      }
      Diagnostic::InvalidCodeRangeIndicator {
        start_offset,
        end_offset,
      } => {
        encoder.emit_usize(buf, *start_offset);
        encoder.emit_usize(buf, *end_offset);
      }
      Diagnostic::DanglingParamList {
        start_offset,
        end_offset,
      } => {
        encoder.emit_usize(buf, *start_offset);
        encoder.emit_usize(buf, *end_offset);
      }
      Diagnostic::InvalidClosureParams {
        start_offset,
        end_offset,
      } => {
        encoder.emit_usize(buf, *start_offset);
        encoder.emit_usize(buf, *end_offset);
      }
      Diagnostic::UnclosedParamList {
        start_offset,
        end_offset,
      } => {
        encoder.emit_usize(buf, *start_offset);
        encoder.emit_usize(buf, *end_offset);
      }
      Diagnostic::InvalidSelfClosureParams {
        start_offset,
        end_offset,
      } => {
        encoder.emit_usize(buf, *start_offset);
        encoder.emit_usize(buf, *end_offset);
      }
    }
  }
}

impl Decodable for Diagnostic {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    let code = DiagnosticCode::from_repr(tag).expect("unknown DiagnosticCode tag");
    match code {
      DiagnosticCode::UnexpectedEof => {
        let expected = char::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnexpectedEof {
          expected,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnexpectedChar => {
        let expected = char::decode(data, decoder);
        let encountered = char::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnexpectedChar {
          expected,
          encountered,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnterminatedString => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnterminatedString {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnterminatedInterpolation => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnterminatedInterpolation {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnterminatedCodeBlock => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnterminatedCodeBlock {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnterminatedInlineCode => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnterminatedInlineCode {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnterminatedMathBlock => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnterminatedMathBlock {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnterminatedInlineMath => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnterminatedInlineMath {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MissingCodeBlockNewline => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MissingCodeBlockNewline {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MissingMathBlockNewline => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MissingMathBlockNewline {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::InvalidChar => {
        let encountered = char::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::InvalidChar {
          encountered,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::InvalidUtf8 => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::InvalidUtf8 {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MixedIndentation => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MixedIndentation {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::InconsistentIndentation => {
        let expected = char::decode(data, decoder);
        let encountered = char::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::InconsistentIndentation {
          expected,
          encountered,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnmatchedDedent => {
        let indent = usize::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnmatchedDedent {
          indent,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MissingExponentDigits => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MissingExponentDigits {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnexpectedTokensOnFrontmatterMarkerLine => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnexpectedTokensOnFrontmatterMarkerLine {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnexpectedContainerPropItem => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnexpectedContainerPropItem {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnexpectedContainerPropValue => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnexpectedContainerPropValue {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnexpectedContainerSlotSeparatorToken => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnexpectedContainerSlotSeparatorToken {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnclosedContainerPropBlock => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnclosedContainerPropBlock {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MissingContainerPropValueAfterEq => {
        let offset = usize::decode(data, decoder);
        Diagnostic::MissingContainerPropValueAfterEq { offset }
      }
      DiagnosticCode::MissingFrontmatterMarker => {
        let offset = usize::decode(data, decoder);
        Diagnostic::MissingFrontmatterMarker { offset }
      }
      DiagnosticCode::MissingMarkdownHeadingHash => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MissingMarkdownHeadingHash {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MissingRequiredSpacesBetweenHashAndHeading => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MissingRequiredSpacesBetweenHashAndHeading {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MissingSyntaxNode => {
        let expected = SyntaxKind::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MissingSyntaxNode {
          expected,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnclosedLink => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnclosedLink {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnclosedBold => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnclosedBold {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnclosedItalic => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnclosedItalic {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnclosedStrikethrough => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnclosedStrikethrough {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnclosedBoldItalic => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnclosedBoldItalic {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MismatchedItalicDelimiter => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MismatchedItalicDelimiter {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MissingExpectMdPrefix => {
        let expected_prefix = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MissingExpectMdPrefix {
          expected_prefix,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MissingTableSeparatorRow => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MissingTableSeparatorRow {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::TableColumnCountMismatch => {
        let expected = usize::decode(data, decoder);
        let found = usize::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::TableColumnCountMismatch {
          expected,
          found,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::InsufficientBlockIndent => {
        let expected_more_than = usize::decode(data, decoder);
        let found = usize::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::InsufficientBlockIndent {
          expected_more_than,
          found,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MissingVaultConfig => {
        let root_dir = String::decode(data, decoder);
        Diagnostic::MissingVaultConfig { root_dir }
      }
      DiagnosticCode::VaultConfigReadError => {
        let path = String::decode(data, decoder);
        let message = String::decode(data, decoder);
        Diagnostic::VaultConfigReadError { path, message }
      }
      DiagnosticCode::VaultConfigParseError => {
        let path = String::decode(data, decoder);
        let message = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::VaultConfigParseError {
          path,
          message,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::VaultConfigEmpty => {
        let path = String::decode(data, decoder);
        Diagnostic::VaultConfigEmpty { path }
      }
      DiagnosticCode::VaultConfigMissingField => {
        let path = String::decode(data, decoder);
        let field = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::VaultConfigMissingField {
          path,
          field,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::VaultConfigUnknownField => {
        let path = String::decode(data, decoder);
        let field = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::VaultConfigUnknownField {
          path,
          field,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MissingSchemaField => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MissingSchemaField {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnresolvedSchema => {
        let name = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnresolvedSchema {
          name,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::WrongTypeArgCount => {
        let expected = usize::decode(data, decoder);
        let got = usize::decode(data, decoder);
        Diagnostic::WrongTypeArgCount { expected, got }
      }
      DiagnosticCode::NotCallable => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::NotCallable {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::WrongArgCount => {
        let expected = usize::decode(data, decoder);
        let got = usize::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::WrongArgCount {
          expected,
          got,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::ArgTypeMismatch => {
        let expected = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::ArgTypeMismatch {
          expected,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::FieldTypeMismatch => {
        let field = String::decode(data, decoder);
        let expected = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::FieldTypeMismatch {
          field,
          expected,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::NotIndexable => {
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::NotIndexable {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::IndexTypeMismatch => {
        let expected = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::IndexTypeMismatch {
          expected,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::TagTypeMismatch => {
        let expected = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::TagTypeMismatch {
          expected,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::OperandTypeMismatch => {
        let op = String::decode(data, decoder);
        let expected = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::OperandTypeMismatch {
          op,
          expected,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::MissingRequiredField => {
        let field = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::MissingRequiredField {
          field,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::ElementTypeMismatch => {
        let expected = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::ElementTypeMismatch {
          expected,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::DuplicateKey => {
        let key = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::DuplicateKey {
          key,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnresolvedFileRef => {
        let path = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnresolvedFileRef {
          path,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnresolvedIdentifier => {
        let name = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnresolvedIdentifier {
          name,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnknownField => {
        let field = String::decode(data, decoder);
        let on_type = String::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::UnknownField {
          field,
          on_type,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::IndexOutOfBounds => {
        let index = usize::decode(data, decoder);
        let length = usize::decode(data, decoder);
        let start_offset = usize::decode(data, decoder);
        let end_offset = usize::decode(data, decoder);
        Diagnostic::IndexOutOfBounds {
          index,
          length,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::NestedSchemaFile => {
        let path = String::decode(data, decoder);
        Diagnostic::NestedSchemaFile { path }
      }
      DiagnosticCode::VaultConfigInvalidValue => {
        let path = String::decode(data, decoder);
        let field = String::decode(data, decoder);
        let message = String::decode(data, decoder);
        let start_offset = decoder.read_usize(data);
        let end_offset = decoder.read_usize(data);
        Diagnostic::VaultConfigInvalidValue {
          path,
          field,
          message,
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::InvalidCodeRangeIndicator => {
        let start_offset = decoder.read_usize(data);
        let end_offset = decoder.read_usize(data);
        Diagnostic::InvalidCodeRangeIndicator {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::DanglingParamList => {
        let start_offset = decoder.read_usize(data);
        let end_offset = decoder.read_usize(data);
        Diagnostic::DanglingParamList {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::InvalidClosureParams => {
        let start_offset = decoder.read_usize(data);
        let end_offset = decoder.read_usize(data);
        Diagnostic::InvalidClosureParams {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::UnclosedParamList => {
        let start_offset = decoder.read_usize(data);
        let end_offset = decoder.read_usize(data);
        Diagnostic::UnclosedParamList {
          start_offset,
          end_offset,
        }
      }
      DiagnosticCode::InvalidSelfClosureParams => {
        let start_offset = decoder.read_usize(data);
        let end_offset = decoder.read_usize(data);
        Diagnostic::InvalidSelfClosureParams {
          start_offset,
          end_offset,
        }
      }
    }
  }
}

impl StableHash for Diagnostic {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      Diagnostic::UnexpectedEof {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::UnexpectedChar {
        expected,
        encountered,
        start_offset,
        end_offset,
      } => {
        expected.stable_hash(db, hasher);
        encountered.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::InvalidChar {
        encountered,
        start_offset,
        end_offset,
      } => {
        encountered.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::InconsistentIndentation {
        expected,
        encountered,
        start_offset,
        end_offset,
      } => {
        expected.stable_hash(db, hasher);
        encountered.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::UnmatchedDedent {
        indent,
        start_offset,
        end_offset,
      } => {
        indent.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::MissingFrontmatterMarker { offset } => {
        offset.stable_hash(db, hasher);
      }
      Diagnostic::MissingSyntaxNode {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::MissingExpectMdPrefix {
        expected_prefix,
        start_offset,
        end_offset,
      } => {
        expected_prefix.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::TableColumnCountMismatch {
        expected,
        found,
        start_offset,
        end_offset,
      } => {
        expected.stable_hash(db, hasher);
        found.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::InsufficientBlockIndent {
        expected_more_than,
        found,
        start_offset,
        end_offset,
      } => {
        expected_more_than.stable_hash(db, hasher);
        found.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::MissingVaultConfig { root_dir } => {
        root_dir.stable_hash(db, hasher);
      }
      Diagnostic::VaultConfigReadError { path, message } => {
        path.stable_hash(db, hasher);
        message.stable_hash(db, hasher);
      }
      Diagnostic::VaultConfigParseError {
        path,
        message,
        start_offset,
        end_offset,
      } => {
        path.stable_hash(db, hasher);
        message.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::VaultConfigEmpty { path } => {
        path.stable_hash(db, hasher);
      }
      Diagnostic::VaultConfigMissingField {
        path,
        field,
        start_offset,
        end_offset,
      } => {
        path.stable_hash(db, hasher);
        field.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::VaultConfigUnknownField {
        path,
        field,
        start_offset,
        end_offset,
      } => {
        path.stable_hash(db, hasher);
        field.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::UnresolvedSchema {
        name,
        start_offset,
        end_offset,
      } => {
        name.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::WrongTypeArgCount { expected, got } => {
        expected.stable_hash(db, hasher);
        got.stable_hash(db, hasher);
      }
      Diagnostic::WrongArgCount {
        expected,
        got,
        start_offset,
        end_offset,
      } => {
        expected.stable_hash(db, hasher);
        got.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::ArgTypeMismatch {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::FieldTypeMismatch {
        field,
        expected,
        start_offset,
        end_offset,
      } => {
        field.stable_hash(db, hasher);
        expected.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::IndexTypeMismatch {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::TagTypeMismatch {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::OperandTypeMismatch {
        op,
        expected,
        start_offset,
        end_offset,
      } => {
        op.stable_hash(db, hasher);
        expected.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::MissingRequiredField {
        field,
        start_offset,
        end_offset,
      } => {
        field.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::ElementTypeMismatch {
        expected,
        start_offset,
        end_offset,
      } => {
        expected.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::DuplicateKey {
        key,
        start_offset,
        end_offset,
      } => {
        key.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::UnresolvedFileRef {
        path,
        start_offset,
        end_offset,
      } => {
        path.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::UnresolvedIdentifier {
        name,
        start_offset,
        end_offset,
      } => {
        name.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::UnknownField {
        field,
        on_type,
        start_offset,
        end_offset,
      } => {
        field.stable_hash(db, hasher);
        on_type.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::IndexOutOfBounds {
        index,
        length,
        start_offset,
        end_offset,
      } => {
        index.stable_hash(db, hasher);
        length.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::UnterminatedString {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnterminatedInterpolation {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnterminatedCodeBlock {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnterminatedInlineCode {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnterminatedMathBlock {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnterminatedInlineMath {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingCodeBlockNewline {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingMathBlockNewline {
        start_offset,
        end_offset,
      }
      | Diagnostic::InvalidUtf8 {
        start_offset,
        end_offset,
      }
      | Diagnostic::MixedIndentation {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingExponentDigits {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnexpectedTokensOnFrontmatterMarkerLine {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnexpectedContainerPropItem {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnexpectedContainerPropValue {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnexpectedContainerSlotSeparatorToken {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedContainerPropBlock {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingMarkdownHeadingHash {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingRequiredSpacesBetweenHashAndHeading {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedLink {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedBold {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedItalic {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedStrikethrough {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedBoldItalic {
        start_offset,
        end_offset,
      }
      | Diagnostic::MismatchedItalicDelimiter {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingTableSeparatorRow {
        start_offset,
        end_offset,
      }
      | Diagnostic::MissingSchemaField {
        start_offset,
        end_offset,
      }
      | Diagnostic::NotCallable {
        start_offset,
        end_offset,
      }
      | Diagnostic::NotIndexable {
        start_offset,
        end_offset,
      }
      | Diagnostic::InvalidCodeRangeIndicator {
        start_offset,
        end_offset,
      }
      | Diagnostic::DanglingParamList {
        start_offset,
        end_offset,
      }
      | Diagnostic::InvalidClosureParams {
        start_offset,
        end_offset,
      }
      | Diagnostic::InvalidSelfClosureParams {
        start_offset,
        end_offset,
      }
      | Diagnostic::UnclosedParamList {
        start_offset,
        end_offset,
      } => {
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::NestedSchemaFile { path } => {
        path.stable_hash(db, hasher);
      }
      Diagnostic::VaultConfigInvalidValue {
        path,
        field,
        message,
        start_offset,
        end_offset,
      } => {
        path.stable_hash(db, hasher);
        field.stable_hash(db, hasher);
        message.stable_hash(db, hasher);
        start_offset.stable_hash(db, hasher);
        end_offset.stable_hash(db, hasher);
      }
      Diagnostic::MissingContainerPropValueAfterEq { offset } => {
        offset.stable_hash(db, hasher);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use std::fmt::Debug;
  use std::path::PathBuf;

  use proptest::prelude::*;

  use typedown_incremental::{Decodable, Encodable};
  use typedown_incremental::{Decoder, Encoder, QueryStorage};

  use std::sync::Arc;

  use crate::db::TypedownDatabase;
  use crate::syntax::diagnostic::Diagnostic;
  use crate::syntax::green::GreenNode;
  use crate::syntax::red::RedNode;
  use crate::syntax::syntax_kind::SyntaxKind;

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
    fn diagnostic_roundtrip(v in any::<Diagnostic>()) {
      encode_decode_roundtrip(&v);
    }
  }
}
