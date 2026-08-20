use std::collections::HashMap;
use typedown_macros::query_derived;
use typedown_types::either::Either;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::{get_dict_type, get_str_type};
use crate::db::types::{FuncSignature, HirValue, LazyType, RuntimeScope};

#[query_derived]
pub struct TdDictType {
  pub key: Option<LazyType>,
  pub value: Option<LazyType>,
}

impl TdRuntimeObject for TdDictType {
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

impl TdStaticType for TdDictType {
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

  fn runtime_type(&self, db: &TypedownDatabase) -> Option<TdTypeEnum> {
    if self.key(db).is_none() || self.value(db).is_none() {
      return None;
    }
    Some((*self).into())
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

  fn arity(&self, db: &TypedownDatabase) -> usize {
    if self.key(db).is_some() { 0 } else { 2 }
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

  fn instantiate(&self, db: &TypedownDatabase, args: Vec<LazyType>) -> Option<TdTypeEnum> {
    if args.len() != 2 {
      return None;
    }
    let mut iter = args.into_iter();
    let key = iter.next().unwrap();
    let value = iter.next().unwrap();
    Some(TdDictType::new(db, Some(key), Some(value)).into())
  }

  fn index_type(&self, db: &TypedownDatabase, _key_type: &TdTypeEnum) -> Option<FuncSignature> {
    let value = self.value(db).and_then(|v| v.resolve(db))?;
    let key = self
      .key(db)
      .and_then(|k| k.resolve(db))
      .unwrap_or_else(|| get_str_type(db).into());
    Some(FuncSignature::new(db, vec![key], value))
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

impl TdRuntimeObject for TdDictObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdDictType::get(db).into()
  }
  fn get_owned_field(&self, db: &TypedownDatabase, key: &str) -> Option<TdObjectEnum> {
    match self.entries(db).get(key).cloned()? {
      Either::Left(hir) => evaluate_node(db, hir, RuntimeScope::empty(db)).value(db),
      Either::Right(obj) => Some(obj),
    }
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn index(&self, db: &TypedownDatabase, key: &TdObjectEnum) -> Option<TdObjectEnum> {
    let str_key = key.as_td_str_obj()?;
    self.get_owned_field(db, &str_key.value(db))
  }
  fn len(&self, db: &TypedownDatabase) -> Option<usize> {
    Some(self.entries(db).len())
  }
}
