use std::collections::HashMap;
use typedown_macros::query_derived;
use typedown_types::either::Either;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike, TdTypeType};
use super::func::TdFuncObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::get_dict_type;
use crate::db::types::{HirValue, InstResult, LazyType, MemberType, TypeMember};
use crate::db::utils::typecheck::member_types_compatible;
use typedown_incremental::Id;

#[query_derived]
pub struct TdDictType {
  pub key: Option<LazyType>,
  pub value: Option<LazyType>,
}

impl TdObjectLike for TdDictType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    match (
      self.key(db).and_then(|l| l.resolve(db)),
      self.value(db).and_then(|l| l.resolve(db)),
    ) {
      (Some(key), Some(value)) => format!(
        "@builtin::dict[{}, {}]",
        key.source_path(db),
        value.source_path(db)
      ),
      _ => "@builtin::dict".to_string(),
    }
  }
}

impl TdTypeLike for TdDictType {
  fn arity(&self, db: &TypedownDatabase) -> usize {
    [
      self.key(db).and_then(|l| l.resolve(db)).is_none(),
      self.value(db).and_then(|l| l.resolve(db)).is_none(),
    ]
    .iter()
    .filter(|&&absent| absent)
    .count()
  }
  fn get_supertype(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdObjectType::get(db).into()
  }
  fn get_vtable(&self, _db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    HashMap::new()
  }
  fn get_owned_field_type_member(&self, _db: &TypedownDatabase, _name: &str) -> Option<TypeMember> {
    None
  }
  fn instantiate(&self, db: &TypedownDatabase, args: Vec<LazyType>) -> InstResult {
    assert_eq!(args.len(), self.arity(db), "arity mismatch");
    let mut iter = args.into_iter();
    let key = iter.next().unwrap();
    let value = iter.next().unwrap();
    InstResult::new(
      db,
      TdDictType::new(db, Some(key), Some(value)).into(),
      vec![],
    )
  }
  fn is_compatible_with(&self, db: &TypedownDatabase, actual: &TdTypeEnum) -> bool {
    if let TdTypeEnum::TdProductType(product) = actual {
      let value_type = match self.value(db).and_then(|l| l.resolve(db)) {
        Some(vt) => vt,
        None => return true,
      };
      return product.fields(db).values().all(|member| {
        let value_member = MemberType::Simple(LazyType::eager(value_type.clone()));
        member_types_compatible(db, &value_member, &member.typ(db))
      });
    }

    if self.as_id().0 != actual.as_id().0 {
      return false;
    }
    let self_args = self.get_type_args(db);
    if self_args.is_empty() {
      return true;
    }
    let actual_args = actual.get_type_args(db);
    if actual_args.is_empty() {
      return false;
    }
    self_args
      .iter()
      .zip(actual_args.iter())
      .all(|(s, a)| s.is_compatible_with(db, a))
  }
  fn get_type_args(&self, db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    match (
      self.key(db).and_then(|l| l.resolve(db)),
      self.value(db).and_then(|l| l.resolve(db)),
    ) {
      (Some(key), Some(value)) => vec![key, value],
      _ => vec![],
    }
  }
  fn construct(&self, db: &TypedownDatabase, args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    let mut entries = HashMap::new();
    for arg in args {
      let pair = arg.as_td_list_obj()?;
      if pair.len(db) != 2 {
        return None;
      }
      let key_obj = pair.get(db, 0)?;
      let key_str = key_obj.as_td_str_obj()?.value(db);
      let val = pair.get(db, 1)?;
      entries.insert(key_str, Either::Right(val));
    }
    Some(TdDictObj::new(db, entries).into())
  }
  fn display_name(&self, db: &TypedownDatabase) -> String {
    match (
      self.key(db).and_then(|l| l.resolve(db)),
      self.value(db).and_then(|l| l.resolve(db)),
    ) {
      (Some(key), Some(value)) => {
        format!("dict[{}, {}]", key.display_name(db), value.display_name(db))
      }
      _ => "dict".to_string(),
    }
  }
}

impl TdDictType {
  pub fn get(db: &TypedownDatabase) -> TdDictType {
    get_dict_type(db)
  }
}

#[query_derived]
pub struct TdDictObj {
  pub entries: HashMap<String, Either<HirValue, TdObjectEnum>>,
}

impl TdObjectLike for TdDictObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdDictType::get(db).into()
  }
  fn get_owned_field(&self, db: &TypedownDatabase, key: &str) -> Option<TdObjectEnum> {
    match self.entries(db).get(key).cloned()? {
      Either::Left(hir) => evaluate_node(db, hir).value(db),
      Either::Right(obj) => Some(obj),
    }
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
}
