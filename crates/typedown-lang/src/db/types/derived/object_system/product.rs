use std::collections::HashMap;
use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType};
use super::func::TdFuncObj;
use super::null::TdNullObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::utils::static_type::format_field_map;
use typedown_incremental::Id;
use typedown_types::either::Either;

use crate::db::derived::get_builtin_types::{get_func_type, get_str_type, get_type_type};
use crate::db::types::derived::object_system::base::{
  BUILTIN_TO_STRING, PROTOCOL_CALL, PROTOCOL_INDEX,
};
use crate::db::types::{
  FnKind, FuncSignature, HirValue, LazyType, NativeFnKind, RuntimeScope, Symbol,
};
use crate::syntax::diagnostic::Diagnostic;

#[query_derived]
pub struct TdProductType {
  pub name: Option<String>,
  pub metatype: TdTypeEnum,
  pub fields: HashMap<String, LazyType>,
  pub vtable: HashMap<String, TdFuncObj>,
}

impl TdRuntimeObject for TdProductType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    self.metatype(db)
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }

  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.display_name(db)
  }
}

impl TdStaticType for TdProductType {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    if let Some(name) = self.name(db) {
      return name;
    }
    format_field_map(db, &self.fields(db))
  }
  fn runtime_type(&self, _db: &TypedownDatabase) -> Option<TdTypeEnum> {
    Some((*self).into())
  }
  fn construct(&self, db: &TypedownDatabase, args: Vec<TdObjectEnum>) -> Option<TdObjectEnum> {
    let arg = args.into_iter().next()?;
    let dict = arg.as_td_dict_obj()?;
    let fields = dict.entries(db);
    Some(TdProductObj::new(db, (*self).into(), None, fields).into())
  }
  fn runtime_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    let mut result = self
      .parent_type(db)
      .map(|p| p.runtime_vtable(db))
      .unwrap_or_default();
    let sig = FuncSignature::new(db, vec![], get_str_type(db).into());
    let to_string_fn = TdFuncObj::new(
      db,
      BUILTIN_TO_STRING.to_string(),
      sig,
      FnKind::Native(NativeFnKind::ToStringMethod),
    );
    result
      .entry(BUILTIN_TO_STRING.to_string())
      .or_insert(to_string_fn);
    result.extend(self.vtable(db));
    result
  }
  fn static_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdTypeEnum> {
    let mut result = HashMap::new();
    let sig = FuncSignature::new(db, vec![], get_str_type(db).into());
    let func_type = get_func_type(db, sig).into();
    result.insert(BUILTIN_TO_STRING.to_string(), func_type);
    for (name, func_obj) in self.vtable(db) {
      result.insert(name, get_func_type(db, func_obj.signature(db)).into());
    }
    result
  }
  fn get_fields(&self, db: &TypedownDatabase) -> HashMap<String, LazyType> {
    self.fields(db)
  }
  fn is_type(&self, db: &TypedownDatabase) -> bool {
    let type_type = get_type_type(db);
    self.metatype(db).as_id() == TdTypeEnum::from(type_type).as_id()
  }
}

#[query_derived]
pub struct TdProductObj {
  pub schema: TdTypeEnum,
  pub file_symbol: Option<Symbol>,
  pub fields: HashMap<String, Either<HirValue, TdObjectEnum>>,
}

impl TdRuntimeObject for TdProductObj {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    self.schema(db)
  }
  fn get_owned_field(&self, db: &TypedownDatabase, key: &str) -> Option<TdObjectEnum> {
    match self.fields(db).get(key).cloned() {
      Some(Either::Left(hir)) => evaluate_node(db, hir, RuntimeScope::empty(db)).value(db),
      Some(Either::Right(obj)) => Some(obj),
      // Missing fields evaluate to null
      None => Some(TdNullObj::get(db).into()),
    }
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn index(&self, db: &TypedownDatabase, key: &TdObjectEnum) -> Option<TdObjectEnum> {
    let this: TdObjectEnum = (*self).into();
    self
      .lookup_method(db, PROTOCOL_INDEX)?
      .call(db, Some(this), vec![key.clone()])
      .ok()
  }
  fn call(
    &self,
    db: &TypedownDatabase,
    this: Option<TdObjectEnum>,
    args: Vec<TdObjectEnum>,
  ) -> Result<TdObjectEnum, Vec<Diagnostic>> {
    let Some(func) = self.lookup_method(db, PROTOCOL_CALL) else {
      return Err(vec![]);
    };
    func.call(db, this, args)
  }
}
