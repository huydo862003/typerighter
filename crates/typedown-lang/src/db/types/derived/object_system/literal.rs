use std::collections::HashMap;

use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_bool_type, get_num_type, get_str_type};
use crate::db::types::{FuncSignature, LazyType, LiteralValue};

#[query_derived]
pub struct TdLiteralType {
  pub value: LiteralValue,
}

impl TdRuntimeObject for TdLiteralType<'_> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.display_name(db)
  }
}

impl TdStaticType for TdLiteralType<'_> {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    match self.value(db) {
      LiteralValue::Str(s) => format!("\"{}\"", s),
      LiteralValue::Num(n) => n,
      LiteralValue::Bool(b) => b.to_string(),
    }
  }

  fn runtime_type(&self, db: &TypedownDatabase) -> Option<TdTypeEnum> {
    Some(self.underlying_type(db))
  }

  fn static_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdTypeEnum> {
    self.underlying_type(db).static_vtable(db)
  }

  fn get_fields(&self, db: &TypedownDatabase) -> HashMap<String, LazyType> {
    self.underlying_type(db).get_fields(db)
  }

  fn lookup_field_type(&self, db: &TypedownDatabase, name: &str) -> Option<TdTypeEnum> {
    self.underlying_type(db).lookup_field_type(db, name)
  }

  fn index_type(&self, db: &TypedownDatabase, key_type: &TdTypeEnum) -> Option<FuncSignature> {
    self.underlying_type(db).index_type(db, key_type)
  }

  fn call_type(&self, db: &TypedownDatabase, arg_types: Vec<TdTypeEnum>) -> Option<FuncSignature> {
    self.underlying_type(db).call_type(db, arg_types)
  }
}

impl TdLiteralType<'_> {
  pub fn underlying_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    match self.value(db) {
      LiteralValue::Str(_) => get_str_type(db).into(),
      LiteralValue::Num(_) => get_num_type(db).into(),
      LiteralValue::Bool(_) => get_bool_type(db).into(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_types::{get_literal_type, get_object_type};
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn literal_type_delegates_static_operations_to_underlying_type() {
    let db = make_db();
    let lit_str = get_literal_type(&db, LiteralValue::Str("hello".to_string()));
    let str_type: TdTypeEnum = get_str_type(&db).into();

    // parent_type returns default Object for static types
    assert_eq!(lit_str.parent_type(&db), Some(get_object_type(&db).into()));

    // static operations delegate to underlying type
    assert_eq!(lit_str.static_vtable(&db), str_type.static_vtable(&db));
    assert_eq!(
      lit_str.lookup_field_type(&db, "nonexistent"),
      str_type.lookup_field_type(&db, "nonexistent")
    );
  }
}
