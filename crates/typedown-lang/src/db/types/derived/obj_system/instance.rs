use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;

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

impl<'db> TdStaticTyp<'db> for TdMetatypeInstanceTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    format!("metatype_instance<{}>", self.inner(db).display_name(db))
  }
}
