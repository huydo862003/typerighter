use std::collections::BTreeMap;

use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::func::TdFuncObj;
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{FuncSignature, LazyTyp, LitValue};

/// Type-level function: given a metatype T, represents the type of its instances
/// Inner must be a metatype (schema_meta, type, lit<schema>, etc.), not a concrete type
/// e.g. metatype_instance<lit<Person>> is equivalent to Person
#[query_derived]
pub struct TdMetatypeInstanceTyp<'db> {
  pub inner: TdTypEnum<'db>,
}

impl<'db> TdRuntimeObj<'db> for TdMetatypeInstanceTyp<'db> {
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

impl<'db> TdMetatypeInstanceTyp<'db> {
  // Resolve inner to get the concrete type this represents
  pub fn resolve_to_typ(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    match &self.inner(db) {
      TdTypEnum::TdLitTyp(lit) => match lit.value(db) {
        LitValue::Typ(typ) => Some(typ),
        _ => None,
      },
      _ => None,
    }
  }
}

impl<'db> TdStaticTyp<'db> for TdMetatypeInstanceTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    match self.resolve_to_typ(db) {
      Some(resolved) => resolved.display_name(db),
      None => format!("metatype_instance<{}>", self.inner(db).display_name(db)),
    }
  }

  fn lookup_field_typ(&self, db: &'db TypedownDatabase, name: &str) -> Option<TdTypEnum<'db>> {
    self.resolve_to_typ(db)?.lookup_field_typ(db, name)
  }

  fn get_fields(&self, db: &'db TypedownDatabase) -> BTreeMap<String, LazyTyp<'db>> {
    self
      .resolve_to_typ(db)
      .map(|typ| typ.get_fields(db))
      .unwrap_or_default()
  }

  fn runtime_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdFuncObj<'db>> {
    self
      .resolve_to_typ(db)
      .map(|typ| typ.runtime_vtable(db))
      .unwrap_or_default()
  }

  fn static_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdTypEnum<'db>> {
    self
      .resolve_to_typ(db)
      .map(|typ| typ.static_vtable(db))
      .unwrap_or_default()
  }

  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    key_typ: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    self.resolve_to_typ(db)?.index_typ(db, key_typ)
  }

  fn runtime_typ(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    self.resolve_to_typ(db)
  }
}
