use super::FileRedNode;
use crate::syntax::diagnostic::Diagnostic;
use strum::FromRepr;
use typedown_macros::{StableCompare, query_derived};

use crate::db::types::Project;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};

/// A lowered YAML value, source-tracked via its originating project and file red node
#[query_derived(custom_hash)]
pub struct HirValue<'db> {
  #[id]
  pub project: Project,
  #[id]
  pub node: FileRedNode,
  pub kind: HirValueKind<'db>,
  pub diagnostics: Vec<Diagnostic>,
}

// Only hash the #[id] fields since kind and diagnostics are deterministic from (project, node)
impl<'db> StableHash for HirValue<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    let storage = unsafe { db.storage() };
    let cache_key = (Self::ingredient_start_index(), self.0);
    if let Some(cached) = storage.derived_fingerprints.get(&cache_key) {
      ::std::hash::Hasher::write(hasher, &cached.0);
      return;
    }
    let mut inner_hasher = StableHasher::new();
    Self::try_project(*self, db).stable_hash(db, &mut inner_hasher);
    Self::try_node(*self, db).stable_hash(db, &mut inner_hasher);
    let fingerprint = typedown_incremental::Fingerprint::from_hasher(inner_hasher);
    storage.derived_fingerprints.insert(cache_key, fingerprint);
    ::std::hash::Hasher::write(hasher, &fingerprint.0);
  }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, StableCompare)]
pub enum HirValueKind<'db> {
  Str(String),
  Num(String),
  Math(String),
  Bool(bool),
  Null,
  Ident(String),
  Mapping(Vec<(String, HirValue<'db>)>),
  Sequence(Vec<HirValue<'db>>),
  Interpolated(Vec<InterpolatedPart<'db>>),
  Markdown(Vec<InterpolatedPart<'db>>),
  Tag {
    tag: Box<HirValue<'db>>,
    inner: Box<HirValue<'db>>,
  },
  Prefix {
    op: String,
    operand: Box<HirValue<'db>>,
  },
  Postfix {
    op: String,
    operand: Box<HirValue<'db>>,
  },
  Binary {
    op: String,
    left: Box<HirValue<'db>>,
    right: Box<HirValue<'db>>,
  },
  Call {
    callee: Box<HirValue<'db>>,
    args: Vec<HirValue<'db>>,
  },
  Index {
    expr: Box<HirValue<'db>>,
    indices: Vec<HirValue<'db>>,
  },
  Closure {
    params: Vec<String>,
    body: Box<HirValue<'db>>,
  },
}

// Discriminant-only: the key already captures (project, file_red_node) so HIR kind is deterministic
impl<'db> StableHash for HirValueKind<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, _db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(_db, hasher);
  }
}

impl StableHash for InterpolatedPart<'_> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      InterpolatedPart::Literal(s) => s.stable_hash(db, hasher),
      InterpolatedPart::Expr(hir) => hir.stable_hash(db, hasher),
    }
  }
}

#[derive(FromRepr)]
#[repr(u8)]
enum HirValueKindTag {
  Str = 0,
  Num = 1,
  Math = 2,
  Bool = 3,
  Null = 4,
  Ident = 5,
  Mapping = 6,
  Sequence = 7,
  Interpolated = 8,
  Markdown = 9,
  Tag = 10,
  Prefix = 11,
  Postfix = 12,
  Binary = 13,
  Call = 14,
  Index = 15,
  Closure = 16,
}

#[derive(FromRepr)]
#[repr(u8)]
enum InterpolatedPartTag {
  Literal = 0,
  Expr = 1,
}

impl<'db> Encodable for HirValueKind<'db> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      HirValueKind::Str(val) => {
        encoder.emit_u8(buf, HirValueKindTag::Str as u8);
        val.encode(buf, encoder);
      }
      HirValueKind::Num(val) => {
        encoder.emit_u8(buf, HirValueKindTag::Num as u8);
        val.encode(buf, encoder);
      }
      HirValueKind::Math(val) => {
        encoder.emit_u8(buf, HirValueKindTag::Math as u8);
        val.encode(buf, encoder);
      }
      HirValueKind::Bool(val) => {
        encoder.emit_u8(buf, HirValueKindTag::Bool as u8);
        val.encode(buf, encoder);
      }
      HirValueKind::Null => {
        encoder.emit_u8(buf, HirValueKindTag::Null as u8);
      }
      HirValueKind::Ident(val) => {
        encoder.emit_u8(buf, HirValueKindTag::Ident as u8);
        val.encode(buf, encoder);
      }
      HirValueKind::Mapping(entries) => {
        encoder.emit_u8(buf, HirValueKindTag::Mapping as u8);
        entries.encode(buf, encoder);
      }
      HirValueKind::Sequence(items) => {
        encoder.emit_u8(buf, HirValueKindTag::Sequence as u8);
        items.encode(buf, encoder);
      }
      HirValueKind::Interpolated(parts) => {
        encoder.emit_u8(buf, HirValueKindTag::Interpolated as u8);
        parts.encode(buf, encoder);
      }
      HirValueKind::Markdown(parts) => {
        encoder.emit_u8(buf, HirValueKindTag::Markdown as u8);
        parts.encode(buf, encoder);
      }
      HirValueKind::Tag { tag, inner } => {
        encoder.emit_u8(buf, HirValueKindTag::Tag as u8);
        tag.encode(buf, encoder);
        inner.encode(buf, encoder);
      }
      HirValueKind::Prefix { op, operand } => {
        encoder.emit_u8(buf, HirValueKindTag::Prefix as u8);
        op.encode(buf, encoder);
        operand.encode(buf, encoder);
      }
      HirValueKind::Postfix { op, operand } => {
        encoder.emit_u8(buf, HirValueKindTag::Postfix as u8);
        op.encode(buf, encoder);
        operand.encode(buf, encoder);
      }
      HirValueKind::Binary { op, left, right } => {
        encoder.emit_u8(buf, HirValueKindTag::Binary as u8);
        op.encode(buf, encoder);
        left.encode(buf, encoder);
        right.encode(buf, encoder);
      }
      HirValueKind::Call { callee, args } => {
        encoder.emit_u8(buf, HirValueKindTag::Call as u8);
        callee.encode(buf, encoder);
        args.encode(buf, encoder);
      }
      HirValueKind::Index { expr, indices } => {
        encoder.emit_u8(buf, HirValueKindTag::Index as u8);
        expr.encode(buf, encoder);
        indices.encode(buf, encoder);
      }
      HirValueKind::Closure { params, body } => {
        encoder.emit_u8(buf, HirValueKindTag::Closure as u8);
        params.encode(buf, encoder);
        body.encode(buf, encoder);
      }
    }
  }
}

impl<'db> Decodable for HirValueKind<'db> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match HirValueKindTag::from_repr(tag).expect("unknown HirValueKind tag") {
      HirValueKindTag::Str => HirValueKind::Str(String::decode(data, decoder)),
      HirValueKindTag::Num => HirValueKind::Num(String::decode(data, decoder)),
      HirValueKindTag::Math => HirValueKind::Math(String::decode(data, decoder)),
      HirValueKindTag::Bool => HirValueKind::Bool(bool::decode(data, decoder)),
      HirValueKindTag::Null => HirValueKind::Null,
      HirValueKindTag::Ident => HirValueKind::Ident(String::decode(data, decoder)),
      HirValueKindTag::Mapping => HirValueKind::Mapping(Vec::decode(data, decoder)),
      HirValueKindTag::Sequence => HirValueKind::Sequence(Vec::decode(data, decoder)),
      HirValueKindTag::Interpolated => HirValueKind::Interpolated(Vec::decode(data, decoder)),
      HirValueKindTag::Markdown => HirValueKind::Markdown(Vec::decode(data, decoder)),
      HirValueKindTag::Tag => HirValueKind::Tag {
        tag: Box::decode(data, decoder),
        inner: Box::decode(data, decoder),
      },
      HirValueKindTag::Prefix => HirValueKind::Prefix {
        op: String::decode(data, decoder),
        operand: Box::decode(data, decoder),
      },
      HirValueKindTag::Postfix => HirValueKind::Postfix {
        op: String::decode(data, decoder),
        operand: Box::decode(data, decoder),
      },
      HirValueKindTag::Binary => HirValueKind::Binary {
        op: String::decode(data, decoder),
        left: Box::decode(data, decoder),
        right: Box::decode(data, decoder),
      },
      HirValueKindTag::Call => HirValueKind::Call {
        callee: Box::decode(data, decoder),
        args: Vec::decode(data, decoder),
      },
      HirValueKindTag::Index => HirValueKind::Index {
        expr: Box::decode(data, decoder),
        indices: Vec::decode(data, decoder),
      },
      HirValueKindTag::Closure => HirValueKind::Closure {
        params: Vec::decode(data, decoder),
        body: Box::decode(data, decoder),
      },
    }
  }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, StableCompare)]
pub enum InterpolatedPart<'db> {
  Literal(String),
  Expr(HirValue<'db>),
}

impl<'db> Encodable for InterpolatedPart<'db> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      InterpolatedPart::Literal(s) => {
        encoder.emit_u8(buf, InterpolatedPartTag::Literal as u8);
        s.encode(buf, encoder);
      }
      InterpolatedPart::Expr(hir) => {
        encoder.emit_u8(buf, InterpolatedPartTag::Expr as u8);
        hir.field_encode(buf, encoder);
      }
    }
  }
}

impl<'db> Decodable for InterpolatedPart<'db> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match InterpolatedPartTag::from_repr(tag).expect("unknown InterpolatedPart tag") {
      InterpolatedPartTag::Literal => InterpolatedPart::Literal(String::decode(data, decoder)),
      InterpolatedPartTag::Expr => InterpolatedPart::Expr(HirValue::field_decode(data, decoder)),
    }
  }
}
