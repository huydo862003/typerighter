use typedown_macros::query_derived;
use typedown_types::either::Either;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::{get_list_type, get_num_type};
use crate::db::types::{FuncSignature, HirValue, LazyType, RuntimeScope};

#[query_derived]
pub struct TdListType {
  pub elem: Option<LazyType>,
}

impl TdRuntimeObject for TdListType {
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
  // Type instantiation: list[string] at runtime
  fn index(&self, db: &TypedownDatabase, key: &TdObjectEnum) -> Option<TdObjectEnum> {
    let arg_type = key.as_td_type_obj()?.clone();
    let result = self.instantiate(db, vec![LazyType::eager(arg_type)])?;
    Some(TdObjectEnum::from(result))
  }
}

impl TdStaticType for TdListType {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    match self.elem(db).and_then(|e| e.resolve(db)) {
      Some(elem) => format!("list[{}]", elem.display_name(db)),
      None => "list".to_string(),
    }
  }

  fn runtime_type(&self, db: &TypedownDatabase) -> Option<TdTypeEnum> {
    self.elem(db)?;
    Some((*self).into())
  }
  fn construct(&self, db: &TypedownDatabase, args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    let items = args.into_iter().map(Either::Right).collect();
    Some(TdListObj::new(db, items).into())
  }

  fn arity(&self, db: &TypedownDatabase) -> usize {
    if self.elem(db).is_some() { 0 } else { 1 }
  }

  fn get_type_args(&self, db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    match self.elem(db).and_then(|e| e.resolve(db)) {
      Some(elem) => vec![elem],
      None => vec![],
    }
  }

  fn instantiate(&self, db: &TypedownDatabase, args: Vec<LazyType>) -> Option<TdTypeEnum> {
    if args.len() != 1 {
      return None;
    }
    Some(TdListType::new(db, Some(args.into_iter().next().unwrap())).into())
  }

  fn index_type(&self, db: &TypedownDatabase, _key_type: &TdTypeEnum) -> Option<FuncSignature> {
    let elem = self.elem(db).and_then(|e| e.resolve(db))?;
    let key_type: TdTypeEnum = get_num_type(db).into();
    Some(FuncSignature::new(db, vec![key_type], elem))
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

impl TdRuntimeObject for TdListObj {
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
  fn index(&self, db: &TypedownDatabase, key: &TdObjectEnum) -> Option<TdObjectEnum> {
    let num = key.as_td_num_obj()?;
    let idx = num.value(db) as usize;
    self.get(db, idx)
  }
  fn len(&self, db: &TypedownDatabase) -> Option<usize> {
    Some(self.items(db).len())
  }
}

impl TdListObj {
  pub fn len(&self, db: &TypedownDatabase) -> usize {
    self.items(db).len()
  }

  pub fn get(&self, db: &TypedownDatabase, idx: usize) -> Option<TdObjectEnum> {
    match self.items(db).into_iter().nth(idx)? {
      Either::Left(hir) => evaluate_node(db, hir, RuntimeScope::empty(db)).value(db),
      Either::Right(obj) => Some(obj),
    }
  }
}
