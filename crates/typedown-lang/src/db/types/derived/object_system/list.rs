use std::collections::HashMap;
use typedown_macros::query_derived;
use typedown_types::either::Either;

use super::base::{TdObjectLike, TdObjectType, TdTypeLike, TdTypeType};
use super::func::TdFuncObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::get_list_type;
use crate::db::types::{HirValue, InstResult, LazyType, TypeMember};
use typedown_incremental::Id;

#[query_derived]
pub struct TdListType {
  pub elem: Option<LazyType>,
}

impl TdObjectLike for TdListType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    match self.elem(db).and_then(|e| e.resolve(db)) {
      Some(elem) => format!("@builtin::list[{}]", elem.source_path(db)),
      None => "@builtin::list".to_string(),
    }
  }
}

impl TdTypeLike for TdListType {
  fn arity(&self, db: &TypedownDatabase) -> usize {
    if self.elem(db).is_none() { 1 } else { 0 }
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
    InstResult::new(
      db,
      TdListType::new(db, Some(iter.next().unwrap())).into(),
      vec![],
    )
  }
  fn accepts(&self, db: &TypedownDatabase, actual: &TdTypeEnum) -> bool {
    if self.as_id().0 != actual.as_id().0 {
      return false;
    }
    let self_elem = match self.elem(db).and_then(|e| e.resolve(db)) {
      Some(t) => t,
      None => return true,
    };
    let actual_elem = match actual
      .as_td_list_type()
      .and_then(|l| l.elem(db))
      .and_then(|e| e.resolve(db))
    {
      Some(t) => t,
      None => return false,
    };
    self_elem.accepts(db, &actual_elem)
  }
  fn get_type_args(&self, db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    self
      .elem(db)
      .and_then(|e| e.resolve(db))
      .into_iter()
      .collect()
  }
  fn construct(&self, db: &TypedownDatabase, args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    let items = args.into_iter().map(Either::Right).collect();
    Some(TdListObj::new(db, items).into())
  }
  fn display_name(&self, db: &TypedownDatabase) -> String {
    match self.elem(db).and_then(|e| e.resolve(db)) {
      Some(elem) => format!("list[{}]", elem.display_name(db)),
      None => "list".to_string(),
    }
  }
}

impl TdListType {
  pub fn get(db: &TypedownDatabase) -> TdListType {
    get_list_type(db)
  }
}

#[query_derived]
pub struct TdListObj {
  pub items: Vec<Either<HirValue, TdObjectEnum>>,
}

impl TdObjectLike for TdListObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdListType::get(db).into()
  }
  fn get_owned_field(&self, db: &TypedownDatabase, key: &str) -> Option<TdObjectEnum> {
    let idx: usize = key.parse().ok()?;
    self.get(db, idx)
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
}

impl TdListObj {
  pub fn len(&self, db: &TypedownDatabase) -> usize {
    self.items(db).len()
  }

  pub fn get(&self, db: &TypedownDatabase, idx: usize) -> Option<TdObjectEnum> {
    match self.items(db).into_iter().nth(idx)? {
      Either::Left(hir) => evaluate_node(db, hir).value(db),
      Either::Right(obj) => Some(obj),
    }
  }
}
