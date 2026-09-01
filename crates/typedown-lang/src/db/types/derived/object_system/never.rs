use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_never_type;
use crate::db::types::FuncSignature;

#[query_derived]
pub struct TdNeverType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdNeverType<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::never".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdNeverType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "never".to_string()
  }

  fn lookup_field_type(&self, db: &'db TypedownDatabase, _name: &str) -> Option<TdTypeEnum<'db>> {
    Some(get_never_type(db).into())
  }

  fn index_type(
    &self,
    db: &'db TypedownDatabase,
    key_type: &TdTypeEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    Some(FuncSignature::new(
      db,
      vec![key_type.clone()],
      get_never_type(db).into(),
    ))
  }

  fn call_type(
    &self,
    db: &'db TypedownDatabase,
    arg_types: Vec<TdTypeEnum<'db>>,
  ) -> Option<FuncSignature<'db>> {
    Some(FuncSignature::new(db, arg_types, get_never_type(db).into()))
  }
}

impl<'db> TdNeverType<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdNeverType<'db> {
    get_never_type(db)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_types::get_num_type;
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn never_type_operations_always_return_never() {
    let db = make_db();
    let never_type = TdNeverType::get(&db);
    let never_enum: TdTypeEnum = never_type.into();

    // field lookup returns never
    assert_eq!(
      never_type.lookup_field_type(&db, "any_field"),
      Some(never_enum.clone())
    );

    // index type returns signature returning never
    let num_type: TdTypeEnum = get_num_type(&db).into();
    let idx_sig = never_type.index_type(&db, &num_type).unwrap();
    assert_eq!(idx_sig.ret(&db), never_enum);

    // call type returns signature returning never
    let call_sig = never_type.call_type(&db, vec![num_type]).unwrap();
    assert_eq!(call_sig.ret(&db), never_enum);
  }
}
