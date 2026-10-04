use strum::FromRepr;
use typedown_macros::StableCompare;

use super::TdObjEnum;
use super::base::TdRuntimeObj;
use super::string::TdStringObj;
use crate::db::TypedownDatabase;
use crate::db::types::{HirValue, Project, RuntimeScope};
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};

use crate::syntax::diagnostic::Diagnostic;

pub type NativeFunc<'db> = fn(
  &'db TypedownDatabase,
  Project,
  Option<TdObjEnum<'db>>,
  Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<Diagnostic>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, StableCompare)]
#[repr(u8)]
pub enum NativeFuncKind {
  ToStringMethod = 0,
}

impl StableHash for NativeFuncKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    (*self as u8).stable_hash(db, hasher);
  }
}

impl Encodable for NativeFuncKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buf, *self as u8);
  }
}

impl Decodable for NativeFuncKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    NativeFuncKind::from_repr(tag).expect("unknown NativeFnKind tag")
  }
}

impl NativeFuncKind {
  pub fn resolve<'db>(self) -> NativeFunc<'db> {
    match self {
      NativeFuncKind::ToStringMethod => to_string_method,
    }
  }
}

fn to_string_method<'db>(
  db: &'db TypedownDatabase,
  _project: Project,
  this: Option<TdObjEnum<'db>>,
  _args: Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<Diagnostic>> {
  let Some(this) = this else {
    return Err(vec![]);
  };
  Ok(TdStringObj::new(db, this.to_display_string(db)).into())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub enum FuncKind<'db> {
  Native(NativeFuncKind),
  UserDefined(HirValue<'db>, RuntimeScope<'db>),
}

#[derive(FromRepr)]
#[repr(u8)]
enum FuncKindTag {
  Native = 0,
  UserDefined = 1,
}

impl<'db> StableHash for FuncKind<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      FuncKind::Native(kind) => kind.stable_hash(db, hasher),
      FuncKind::UserDefined(hir, runtime_scope) => {
        hir.stable_hash(db, hasher);
        runtime_scope.stable_hash(db, hasher);
      }
    }
  }
}

impl<'db> Encodable for FuncKind<'db> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      FuncKind::Native(kind) => {
        encoder.emit_u8(buf, FuncKindTag::Native as u8);
        kind.encode(buf, encoder);
      }
      FuncKind::UserDefined(hir, runtime_scope) => {
        encoder.emit_u8(buf, FuncKindTag::UserDefined as u8);
        hir.field_encode(buf, encoder);
        runtime_scope.field_encode(buf, encoder);
      }
    }
  }
}

impl<'db> Decodable for FuncKind<'db> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match FuncKindTag::from_repr(tag).expect("unknown FnKind tag") {
      FuncKindTag::Native => FuncKind::Native(NativeFuncKind::decode(data, decoder)),
      FuncKindTag::UserDefined => FuncKind::UserDefined(
        HirValue::field_decode(data, decoder),
        RuntimeScope::field_decode(data, decoder),
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
  use crate::db::fixtures::make_number_obj;

  fn make_db() -> (TypedownDatabase, Project) {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let project = Project::new(&db, PathBuf::from("/test"), HashMap::new());
    (db, project)
  }

  #[test]
  fn test_native_func_optional_this() {
    let (db, project) = make_db();
    let native_func = NativeFuncKind::ToStringMethod.resolve();
    let number_obj: TdObjEnum = make_number_obj(&db, 42.0_f64.to_bits()).into();

    let result_with_this = native_func(&db, project, Some(number_obj), vec![]);
    assert!(result_with_this.is_ok());
    assert_eq!(
      result_with_this.unwrap().to_display_string(&db),
      "42".to_string()
    );

    let result_no_this = native_func(&db, project, None, vec![]);
    assert!(result_no_this.is_err());
  }
}
