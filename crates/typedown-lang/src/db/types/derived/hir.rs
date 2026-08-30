use crate::syntax::diagnostic::Diagnostic;
use crate::syntax::red::RedNode;
use strum::FromRepr;
use typedown_macros::{StableCompare, query_derived};

use crate::db::types::{File, Project};
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase,
  StableHash, StableHasher,
};

/// A lowered YAML value, source-tracked via its originating project, file, and red node.
#[query_derived]
pub struct HirValue<'db> {
  #[id]
  pub project: Project,
  #[id]
  pub file: File,
  #[id]
  pub node: RedNode,
  pub kind: HirValueKind<'db>,
  pub diagnostics: Vec<Diagnostic>,
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

impl<'db> StableHash for HirValueKind<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      HirValueKind::Str(v)
      | HirValueKind::Num(v)
      | HirValueKind::Math(v)
      | HirValueKind::Ident(v) => v.stable_hash(db, hasher),
      HirValueKind::Bool(v) => v.stable_hash(db, hasher),
      HirValueKind::Null => {}
      HirValueKind::Mapping(entries) => entries.stable_hash(db, hasher),
      HirValueKind::Sequence(items) => items.stable_hash(db, hasher),
      HirValueKind::Interpolated(parts) | HirValueKind::Markdown(parts) => {
        parts.stable_hash(db, hasher)
      }
      HirValueKind::Tag { tag, inner } => {
        tag.stable_hash(db, hasher);
        inner.stable_hash(db, hasher);
      }
      HirValueKind::Prefix { op, operand } | HirValueKind::Postfix { op, operand } => {
        op.stable_hash(db, hasher);
        operand.stable_hash(db, hasher);
      }
      HirValueKind::Binary { op, left, right } => {
        op.stable_hash(db, hasher);
        left.stable_hash(db, hasher);
        right.stable_hash(db, hasher);
      }
      HirValueKind::Call { callee, args } => {
        callee.stable_hash(db, hasher);
        args.stable_hash(db, hasher);
      }
      HirValueKind::Index { expr, indices } => {
        expr.stable_hash(db, hasher);
        indices.stable_hash(db, hasher);
      }
      HirValueKind::Closure { params, body } => {
        params.stable_hash(db, hasher);
        body.stable_hash(db, hasher);
      }
    }
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
