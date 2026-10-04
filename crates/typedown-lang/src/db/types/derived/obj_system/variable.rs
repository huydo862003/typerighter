use std::collections::BTreeMap;
use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{FuncSignature, LazyTyp, TypVariable};

/// A type variable reference within a type expression
#[query_derived]
pub struct TdVariableTyp<'db> {
  #[id]
  pub index: usize,
  #[id]
  pub variable: TypVariable<'db>,
}

impl<'db> TdStaticTyp<'db> for TdVariableTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    let var = Self::variable(*self, db);
    if let Some(bound) = var.upper_bound(db).resolve(db) {
      if bound.as_td_obj_typ().is_some() {
        format!("T{}", Self::index(*self, db))
      } else {
        format!("T{} <: {}", Self::index(*self, db), bound.display_name(db))
      }
    } else {
      format!("T{}", Self::index(*self, db))
    }
  }

  fn static_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdTypEnum<'db>> {
    self
      .variable(db)
      .upper_bound(db)
      .resolve(db)
      .map(|upper| upper.static_vtable(db))
      .unwrap_or_default()
  }

  fn get_fields(&self, db: &'db TypedownDatabase) -> BTreeMap<String, LazyTyp<'db>> {
    self
      .variable(db)
      .upper_bound(db)
      .resolve(db)
      .map(|upper| upper.get_fields(db))
      .unwrap_or_default()
  }

  fn lookup_field_typ(&self, db: &'db TypedownDatabase, name: &str) -> Option<TdTypEnum<'db>> {
    self
      .variable(db)
      .upper_bound(db)
      .resolve(db)?
      .lookup_field_typ(db, name)
  }

  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    key_typ: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    self
      .variable(db)
      .upper_bound(db)
      .resolve(db)?
      .index_typ(db, key_typ)
  }

  fn call_typ(
    &self,
    db: &'db TypedownDatabase,
    arg_typs: Vec<TdTypEnum<'db>>,
  ) -> Option<FuncSignature<'db>> {
    self
      .variable(db)
      .upper_bound(db)
      .resolve(db)?
      .call_typ(db, arg_typs)
  }
}

impl<'db> TdRuntimeObj<'db> for TdVariableTyp<'db> {
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_typs::get_string_typ;
  use crate::db::types::derived::obj_system::TdProductTyp;
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn variable_typ_delegates_to_upper_bound() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();

    let mut fields = BTreeMap::new();
    fields.insert("name".to_string(), LazyTyp::eager(string_typ.clone()));
    let struct_typ: TdTypEnum = TdProductTyp::new(&db, None, fields).into();

    let variable = TypVariable::get(&db, Some(LazyTyp::eager(struct_typ)));
    let variable_typ = TdVariableTyp::new(&db, 0, variable);

    // field lookup delegates to Upper
    assert_eq!(variable_typ.lookup_field_typ(&db, "name"), Some(string_typ));
    assert_eq!(variable_typ.lookup_field_typ(&db, "nonexistent"), None);
  }
}
