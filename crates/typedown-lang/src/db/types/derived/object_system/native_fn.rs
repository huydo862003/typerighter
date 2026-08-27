use strum::FromRepr;
use typedown_macros::StableCompare;

use super::TdObjectEnum;
use super::base::TdRuntimeObject;
use super::str::TdStrObj;
use crate::db::TypedownDatabase;
use crate::db::types::{HirValue, Project, RuntimeScope};
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, FieldDecodable, FieldEncodable, QueryDatabase,
  StableHash, StableHasher,
};

use crate::syntax::diagnostic::Diagnostic;

pub type NativeFn = fn(
  &TypedownDatabase,
  Project,
  Option<TdObjectEnum>,
  Vec<TdObjectEnum>,
) -> Result<TdObjectEnum, Vec<Diagnostic>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, StableCompare)]
#[repr(u8)]
pub enum NativeFnKind {
  ToStringMethod = 0,
}

impl StableHash for NativeFnKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    (*self as u8).stable_hash(db, hasher);
  }
}

impl Encodable for NativeFnKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buf, *self as u8);
  }
}

impl Decodable for NativeFnKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    NativeFnKind::from_repr(tag).expect("unknown NativeFnKind tag")
  }
}

impl NativeFnKind {
  pub fn resolve(self) -> NativeFn {
    match self {
      NativeFnKind::ToStringMethod => to_string_method,
    }
  }
}

fn to_string_method(
  db: &TypedownDatabase,
  _project: Project,
  this: Option<TdObjectEnum>,
  _args: Vec<TdObjectEnum>,
) -> Result<TdObjectEnum, Vec<Diagnostic>> {
  let Some(this) = this else {
    return Err(vec![]);
  };
  Ok(TdStrObj::new(db, this.to_display_string(db)).into())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub enum FnKind {
  Native(NativeFnKind),
  UserDefined(HirValue, RuntimeScope),
}

#[derive(FromRepr)]
#[repr(u8)]
enum FnKindTag {
  Native = 0,
  UserDefined = 1,
}

impl StableHash for FnKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      FnKind::Native(kind) => kind.stable_hash(db, hasher),
      FnKind::UserDefined(hir, runtime_scope) => {
        hir.stable_hash(db, hasher);
        runtime_scope.stable_hash(db, hasher);
      }
    }
  }
}

impl Encodable for FnKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      FnKind::Native(kind) => {
        encoder.emit_u8(buf, FnKindTag::Native as u8);
        kind.encode(buf, encoder);
      }
      FnKind::UserDefined(hir, runtime_scope) => {
        encoder.emit_u8(buf, FnKindTag::UserDefined as u8);
        hir.encode_field(buf, encoder);
        runtime_scope.encode_field(buf, encoder);
      }
    }
  }
}

impl Decodable for FnKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match FnKindTag::from_repr(tag).expect("unknown FnKind tag") {
      FnKindTag::Native => FnKind::Native(NativeFnKind::decode(data, decoder)),
      FnKindTag::UserDefined => FnKind::UserDefined(
        HirValue::decode_field(data, decoder),
        RuntimeScope::decode_field(data, decoder),
      ),
    }
  }
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::PathBuf;

  use super::*;
  use crate::db::QueryStorage;
  use crate::db::types::TdNumObj;

  fn make_db() -> (TypedownDatabase, Project) {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let project = Project::new(&db, PathBuf::from("/test"), HashMap::new());
    (db, project)
  }

  #[test]
  fn test_native_fn_optional_this() {
    let (db, project) = make_db();
    let native_fn = NativeFnKind::ToStringMethod.resolve();
    let num_obj: TdObjectEnum = TdNumObj::new(&db, 42.0).into();

    let result_with_this = native_fn(&db, project, Some(num_obj), vec![]);
    assert!(result_with_this.is_ok());
    assert_eq!(
      result_with_this.unwrap().to_display_string(&db),
      "42".to_string()
    );

    let result_no_this = native_fn(&db, project, None, vec![]);
    assert!(result_no_this.is_err());
  }
}
