use std::collections::HashSet;
use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::get_sum_typ;
use crate::db::types::{FuncSignature, LazyTyp};
use typedown_incremental::StableCompare;

// A union type: accepts any of its member types
#[query_derived]
pub struct TdSumTyp<'db> {
  pub members: HashSet<LazyTyp<'db>>,
}

impl<'db> TdStaticTyp<'db> for TdSumTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    let mut members: Vec<_> = self.members(db).into_iter().collect();
    members.sort_by(|a, b| a.stable_cmp(db, b));
    let parts: Vec<String> = members
      .iter()
      .filter_map(|m| m.resolve(db).map(|t| t.display_name(db)))
      .collect();
    parts.join(" | ")
  }

  fn lookup_field_typ(&self, db: &'db TypedownDatabase, name: &str) -> Option<TdTypEnum<'db>> {
    let mut field_types = vec![];
    for member in self.members(db) {
      let resolved = member.resolve(db)?;
      let field_type = resolved.lookup_field_typ(db, name)?;
      field_types.push(LazyTyp::eager(field_type));
    }
    if field_types.is_empty() {
      return None;
    }
    Some(get_sum_typ(db, field_types).into())
  }

  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    key_typ: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    let mut ret_typs = vec![];
    for member in self.members(db) {
      let resolved = member.resolve(db)?;
      let signature = resolved.index_typ(db, key_typ)?;
      ret_typs.push(LazyTyp::eager(signature.ret(db)));
    }
    if ret_typs.is_empty() {
      return None;
    }
    let union_ret = get_sum_typ(db, ret_typs).into();
    Some(FuncSignature::new(db, vec![key_typ.clone()], union_ret))
  }

  fn call_typ(
    &self,
    db: &'db TypedownDatabase,
    arg_typs: Vec<TdTypEnum<'db>>,
  ) -> Option<FuncSignature<'db>> {
    let mut ret_typs = vec![];
    for member in self.members(db) {
      let resolved = member.resolve(db)?;
      let signature = resolved.call_typ(db, arg_typs.clone())?;
      ret_typs.push(LazyTyp::eager(signature.ret(db)));
    }
    if ret_typs.is_empty() {
      return None;
    }
    let union_ret = get_sum_typ(db, ret_typs).into();
    Some(FuncSignature::new(db, arg_typs, union_ret))
  }
}

impl<'db> TdRuntimeObj<'db> for TdSumTyp<'db> {
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
  use crate::db::derived::get_builtin_typs::{get_number_typ, get_string_typ};
  use crate::db::types::derived::obj_system::TdProductTyp;
  use crate::db::{QueryStorage, TypedownDatabase};
  use std::collections::BTreeMap;

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn sum_type_lookup_field_type_unions_field_types_when_present_on_all_members() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_type: TdTypEnum = get_number_typ(&db).into();

    let mut fields1 = BTreeMap::new();
    fields1.insert("val".to_string(), LazyTyp::eager(string_typ.clone()));
    let struct1: TdTypEnum = TdProductTyp::new(&db, None, fields1).into();

    let mut fields2 = BTreeMap::new();
    fields2.insert("val".to_string(), LazyTyp::eager(number_type.clone()));
    let struct2: TdTypEnum = TdProductTyp::new(&db, None, fields2).into();

    let sum_typ = get_sum_typ(&db, vec![LazyTyp::eager(struct1), LazyTyp::eager(struct2)]);

    let value_typ = sum_typ.lookup_field_typ(&db, "val").unwrap();
    assert_eq!(
      value_typ,
      get_sum_typ(
        &db,
        vec![LazyTyp::eager(string_typ), LazyTyp::eager(number_type)]
      )
      .into()
    );
  }

  #[test]
  fn sum_type_lookup_field_type_returns_none_if_missing_on_any_member() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();

    let mut fields1 = BTreeMap::new();
    fields1.insert("val".to_string(), LazyTyp::eager(string_typ));
    let struct1: TdTypEnum = TdProductTyp::new(&db, None, fields1).into();

    let mut fields2 = BTreeMap::new();
    fields2.insert("other".to_string(), LazyTyp::eager(number_typ));
    let struct2: TdTypEnum = TdProductTyp::new(&db, None, fields2).into();

    let sum_typ = get_sum_typ(&db, vec![LazyTyp::eager(struct1), LazyTyp::eager(struct2)]);

    assert_eq!(sum_typ.lookup_field_typ(&db, "val"), None);
  }
}
