use std::collections::BTreeMap;

use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_bool_typ, get_number_typ, get_string_typ};
use crate::db::types::{FuncSignature, LazyTyp, LitValue};

#[query_derived]
pub struct TdLitTyp<'db> {
  pub value: LitValue,
}

impl<'db> TdRuntimeObj<'db> for TdLitTyp<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdTypTyp::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.display_name(db)
  }
}

impl<'db> TdStaticTyp<'db> for TdLitTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    match self.value(db) {
      LitValue::String(s) => format!("\"{}\"", s),
      LitValue::Number(n) => n,
      LitValue::Bool(b) => b.to_string(),
    }
  }

  fn runtime_typ(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some(self.underlying_typ(db))
  }

  fn static_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdTypEnum<'db>> {
    self.underlying_typ(db).static_vtable(db)
  }

  fn get_fields(&self, db: &'db TypedownDatabase) -> BTreeMap<String, LazyTyp<'db>> {
    self.underlying_typ(db).get_fields(db)
  }

  fn lookup_field_typ(&self, db: &'db TypedownDatabase, name: &str) -> Option<TdTypEnum<'db>> {
    self.underlying_typ(db).lookup_field_typ(db, name)
  }

  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    key_type: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    self.underlying_typ(db).index_typ(db, key_type)
  }

  fn call_typ(
    &self,
    db: &'db TypedownDatabase,
    arg_types: Vec<TdTypEnum<'db>>,
  ) -> Option<FuncSignature<'db>> {
    self.underlying_typ(db).call_typ(db, arg_types)
  }
}

impl<'db> TdLitTyp<'db> {
  pub fn underlying_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    match self.value(db) {
      LitValue::String(_) => get_string_typ(db).into(),
      LitValue::Number(_) => get_number_typ(db).into(),
      LitValue::Bool(_) => get_bool_typ(db).into(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_typs::{get_lit_typ, get_obj_typ};
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn lit_typ_delegates_static_operations_to_underlying_type() {
    let db = make_db();
    let lit_string = get_lit_typ(&db, LitValue::String("hello".to_string()));
    let string_typ: TdTypEnum = get_string_typ(&db).into();

    // parent_type returns default Object for static types
    assert_eq!(lit_string.parent_typ(&db), Some(get_obj_typ(&db).into()));

    // static operations delegate to underlying type
    assert_eq!(lit_string.static_vtable(&db), string_typ.static_vtable(&db));
    assert_eq!(
      lit_string.lookup_field_typ(&db, "nonexistent"),
      string_typ.lookup_field_typ(&db, "nonexistent")
    );
  }
}
