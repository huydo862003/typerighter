use std::collections::HashMap;

use typedown_macros::query_derived;

use super::base::{BUILTIN_TO_STRING, TdRuntimeObject, TdStaticType, TdTypeType};
use super::func::TdFuncObj;
use super::native_fn::{FnKind, NativeFnKind};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_func_type, get_str_type};
use crate::db::types::FuncSignature;

/// Top type: `Object` (the universal supertype of all value types)
#[query_derived]
pub struct TdObjectType<'db> {}

impl<'db> TdStaticType<'db> for TdObjectType<'db> {
  fn display_name(&self, _db: &TypedownDatabase) -> String {
    "Object".to_string()
  }

  fn parent_type(&self, _db: &TypedownDatabase) -> Option<TdTypeEnum> {
    None
  }

  fn runtime_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    let mut result = HashMap::new();
    let sig = FuncSignature::new(db, vec![], get_str_type(db).into());
    let to_string_fn = TdFuncObj::new(
      db,
      BUILTIN_TO_STRING.to_string(),
      sig,
      FnKind::Native(NativeFnKind::ToStringMethod),
    );
    result.insert(BUILTIN_TO_STRING.to_string(), to_string_fn);
    result
  }

  fn static_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdTypeEnum> {
    let mut result = HashMap::new();
    let sig = FuncSignature::new(db, vec![], get_str_type(db).into());
    let func_type = get_func_type(db, sig).into();
    result.insert(BUILTIN_TO_STRING.to_string(), func_type);
    result
  }
}

impl<'db> TdRuntimeObject for TdObjectType<'db> {
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_types::{get_func_type, get_object_type, get_str_type};
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn object_type_to_string_lookup_returns_func_type() {
    let db = make_db();
    let obj_type = get_object_type(&db);

    let to_string_type = obj_type.lookup_field_type(&db, "to_string").unwrap();
    let expected_sig = FuncSignature::new(&db, vec![], get_str_type(&db).into());
    let expected_func_type: TdTypeEnum = get_func_type(&db, expected_sig).into();

    assert_eq!(to_string_type, expected_func_type);
  }
}
