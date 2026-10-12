use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::get_never_typ;
use crate::db::types::FuncSignature;

#[query_derived]
pub struct TdNeverTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdNeverTyp<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdTypTyp::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::never".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdNeverTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "never".to_string()
  }

  fn lookup_field_typ(&self, db: &'db TypedownDatabase, _name: &str) -> Option<TdTypEnum<'db>> {
    Some(get_never_typ(db).into())
  }

  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    key_typ: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    Some(FuncSignature::new(
      db,
      vec![key_typ.clone()],
      get_never_typ(db).into(),
    ))
  }

  fn call_typ(
    &self,
    db: &'db TypedownDatabase,
    arg_typs: Vec<TdTypEnum<'db>>,
  ) -> Option<FuncSignature<'db>> {
    Some(FuncSignature::new(db, arg_typs, get_never_typ(db).into()))
  }
}

impl<'db> TdNeverTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdNeverTyp<'db> {
    get_never_typ(db)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_typs::get_number_typ;
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn never_type_operations_always_return_never() {
    let db = make_db();
    let never_typ = TdNeverTyp::get(&db);
    let never_enum: TdTypEnum = never_typ.into();

    // field lookup returns never
    assert_eq!(
      never_typ.lookup_field_typ(&db, "any_field"),
      Some(never_enum.clone())
    );

    // index type returns signature returning never
    let num_typ: TdTypEnum = get_number_typ(&db).into();
    let index_signature = never_typ.index_typ(&db, &num_typ).unwrap();
    assert_eq!(index_signature.ret(&db), never_enum);

    // call type returns signature returning never
    let call_signature = never_typ.call_typ(&db, vec![num_typ]).unwrap();
    assert_eq!(call_signature.ret(&db), never_enum);
  }
}
