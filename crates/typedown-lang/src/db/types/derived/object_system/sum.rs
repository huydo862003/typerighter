use std::collections::HashSet;
use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_sum_type;
use crate::db::types::{FuncSignature, LazyType};
use typedown_incremental::StableCompare;

// A union type: accepts any of its member types
#[query_derived]
pub struct TdSumType<'db> {
  pub members: HashSet<LazyType>,
}

impl TdStaticType for TdSumType<'_> {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    let mut members: Vec<_> = self.members(db).into_iter().collect();
    members.sort_by(|a, b| a.stable_cmp(db, b));
    let parts: Vec<String> = members
      .iter()
      .filter_map(|m| m.resolve(db).map(|t| t.display_name(db)))
      .collect();
    parts.join(" | ")
  }

  fn lookup_field_type(&self, db: &TypedownDatabase, name: &str) -> Option<TdTypeEnum> {
    let mut field_types = vec![];
    for member in self.members(db) {
      let resolved = member.resolve(db)?;
      let field_type = resolved.lookup_field_type(db, name)?;
      field_types.push(LazyType::eager(field_type));
    }
    if field_types.is_empty() {
      return None;
    }
    Some(get_sum_type(db, field_types).into())
  }

  fn index_type(&self, db: &TypedownDatabase, key_type: &TdTypeEnum) -> Option<FuncSignature> {
    let mut ret_types = vec![];
    for member in self.members(db) {
      let resolved = member.resolve(db)?;
      let sig = resolved.index_type(db, key_type)?;
      ret_types.push(LazyType::eager(sig.ret(db)));
    }
    if ret_types.is_empty() {
      return None;
    }
    let union_ret = get_sum_type(db, ret_types).into();
    Some(FuncSignature::new(db, vec![key_type.clone()], union_ret))
  }

  fn call_type(&self, db: &TypedownDatabase, arg_types: Vec<TdTypeEnum>) -> Option<FuncSignature> {
    let mut ret_types = vec![];
    for member in self.members(db) {
      let resolved = member.resolve(db)?;
      let sig = resolved.call_type(db, arg_types.clone())?;
      ret_types.push(LazyType::eager(sig.ret(db)));
    }
    if ret_types.is_empty() {
      return None;
    }
    let union_ret = get_sum_type(db, ret_types).into();
    Some(FuncSignature::new(db, arg_types, union_ret))
  }
}

impl TdRuntimeObject for TdSumType<'_> {
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
  use crate::db::derived::get_builtin_types::{get_num_type, get_str_type};
  use crate::db::types::derived::object_system::TdProductType;
  use crate::db::{QueryStorage, TypedownDatabase};
  use std::collections::HashMap;

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn sum_type_lookup_field_type_unions_field_types_when_present_on_all_members() {
    let db = make_db();
    let str_type: TdTypeEnum = get_str_type(&db).into();
    let num_type: TdTypeEnum = get_num_type(&db).into();

    let mut fields1 = HashMap::new();
    fields1.insert("val".to_string(), LazyType::eager(str_type.clone()));
    let struct1: TdTypeEnum = TdProductType::new(&db, None, fields1).into();

    let mut fields2 = HashMap::new();
    fields2.insert("val".to_string(), LazyType::eager(num_type.clone()));
    let struct2: TdTypeEnum = TdProductType::new(&db, None, fields2).into();

    let sum = get_sum_type(
      &db,
      vec![LazyType::eager(struct1), LazyType::eager(struct2)],
    );

    let val_type = sum.lookup_field_type(&db, "val").unwrap();
    assert_eq!(
      val_type,
      get_sum_type(
        &db,
        vec![LazyType::eager(str_type), LazyType::eager(num_type)]
      )
      .into()
    );
  }

  #[test]
  fn sum_type_lookup_field_type_returns_none_if_missing_on_any_member() {
    let db = make_db();
    let str_type: TdTypeEnum = get_str_type(&db).into();
    let num_type: TdTypeEnum = get_num_type(&db).into();

    let mut fields1 = HashMap::new();
    fields1.insert("val".to_string(), LazyType::eager(str_type));
    let struct1: TdTypeEnum = TdProductType::new(&db, None, fields1).into();

    let mut fields2 = HashMap::new();
    fields2.insert("other".to_string(), LazyType::eager(num_type));
    let struct2: TdTypeEnum = TdProductType::new(&db, None, fields2).into();

    let sum = get_sum_type(
      &db,
      vec![LazyType::eager(struct1), LazyType::eager(struct2)],
    );

    assert_eq!(sum.lookup_field_type(&db, "val"), None);
  }
}
