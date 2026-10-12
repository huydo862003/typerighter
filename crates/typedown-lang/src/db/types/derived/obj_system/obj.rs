use std::collections::BTreeMap;

use typedown_macros::query_derived;

use super::base::{BUILTIN_TO_STRING, TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::func::TdFuncObj;
use super::native_func::{FuncKind, NativeFuncKind};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_func_typ, get_string_typ};
use crate::db::types::FuncSignature;

/// Top type: `Object` (the universal supertype of all value types)
#[query_derived]
pub struct TdObjTyp<'db> {}

impl<'db> TdStaticTyp<'db> for TdObjTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "Object".to_string()
  }

  fn parent_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    None
  }

  fn runtime_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdFuncObj<'db>> {
    let mut result = BTreeMap::new();
    let signature = FuncSignature::new(db, vec![], get_string_typ(db).into());
    let to_string_fn = TdFuncObj::new(
      db,
      BUILTIN_TO_STRING.to_string(),
      signature,
      FuncKind::Native(NativeFuncKind::ToStringMethod),
    );
    result.insert(BUILTIN_TO_STRING.to_string(), to_string_fn);
    result
  }

  fn static_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdTypEnum<'db>> {
    let mut result = BTreeMap::new();
    let signature = FuncSignature::new(db, vec![], get_string_typ(db).into());
    let func_type = get_func_typ(db, signature).into();
    result.insert(BUILTIN_TO_STRING.to_string(), func_type);
    result
  }
}

impl<'db> TdRuntimeObj<'db> for TdObjTyp<'db> {
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
  use crate::db::derived::get_builtin_typs::{get_func_typ, get_obj_typ, get_string_typ};
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn object_type_to_string_lookup_returns_func_type() {
    let db = make_db();
    let obj_typ = get_obj_typ(&db);

    let to_string_typ = obj_typ.lookup_field_typ(&db, "to_string").unwrap();
    let expected_signature = FuncSignature::new(&db, vec![], get_string_typ(&db).into());
    let expected_func_typ: TdTypEnum = get_func_typ(&db, expected_signature).into();

    assert_eq!(to_string_typ, expected_func_typ);
  }
}
