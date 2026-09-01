use std::collections::HashMap;

use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_bool_type, get_num_type, get_str_type};
use crate::db::types::{FuncSignature, LazyType, LiteralValue};

#[query_derived]
pub struct TdLiteralType<'db> {
  pub value: LiteralValue,
}

impl<'db> TdRuntimeObject<'db> for TdLiteralType<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.display_name(db)
  }
}

impl<'db> TdStaticType<'db> for TdLiteralType<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    match self.value(db) {
      LiteralValue::Str(s) => format!("\"{}\"", s),
      LiteralValue::Num(n) => n,
      LiteralValue::Bool(b) => b.to_string(),
    }
  }

  fn runtime_type(&self, db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some(self.underlying_type(db))
  }

  fn static_vtable(&self, db: &'db TypedownDatabase) -> HashMap<String, TdTypeEnum<'db>> {
    self.underlying_type(db).static_vtable(db)
  }

  fn get_fields(&self, db: &'db TypedownDatabase) -> HashMap<String, LazyType<'db>> {
    self.underlying_type(db).get_fields(db)
  }

  fn lookup_field_type(&self, db: &'db TypedownDatabase, name: &str) -> Option<TdTypeEnum<'db>> {
    self.underlying_type(db).lookup_field_type(db, name)
  }

  fn index_type(
    &self,
    db: &'db TypedownDatabase,
    key_type: &TdTypeEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    self.underlying_type(db).index_type(db, key_type)
  }

  fn call_type(
    &self,
    db: &'db TypedownDatabase,
    arg_types: Vec<TdTypeEnum<'db>>,
  ) -> Option<FuncSignature<'db>> {
    self.underlying_type(db).call_type(db, arg_types)
  }
}

impl<'db> TdLiteralType<'db> {
  pub fn underlying_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
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
