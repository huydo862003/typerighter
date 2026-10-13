//! Type system core traits
//! - TdStaticType: static properties for the typechecker (display_name, arity, etc)
//! - TdRuntimeObject: runtime object protocol for the evaluator (field access, method dispatch)

use std::collections::BTreeMap;

use ambassador::delegatable_trait;

use super::func::TdFuncObj;
use super::typecheck::validate_typ_params;
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_obj_typ, get_typ_typ};
use crate::db::types::{FuncSignature, InstantiateResult, LazyTyp, TypParams};
use typedown_incremental::Id;
use typedown_macros::query_derived;

use crate::syntax::diagnostic::Diagnostic;

/// Special protocol method names (bracketed)
pub const PROTOCOL_INDEX: &str = "[[index]]";

pub const PROTOCOL_CALL: &str = "[[call]]";

/// Built-in method names
pub const BUILTIN_TO_STRING: &str = "to_string";

// Static type properties for the typechecker
// Each type defines its own display name, arity, and type arguments
#[delegatable_trait]
pub trait TdStaticTyp<'x0> {
  fn display_name(&self, db: &'x0 TypedownDatabase) -> String;

  fn arity(&self, _db: &'x0 TypedownDatabase) -> usize {
    0
  }

  fn get_typ_args(&self, _db: &'x0 TypedownDatabase) -> Vec<TdTypEnum<'x0>> {
    vec![]
  }

  fn runtime_typ(&self, _db: &'x0 TypedownDatabase) -> Option<TdTypEnum<'x0>> {
    None
  }

  fn construct(
    &self,
    _db: &'x0 TypedownDatabase,
    _project: ::typedown_lang::db::types::Project,
    _args: Vec<TdObjEnum<'x0>>,
  ) -> Option<TdObjEnum<'x0>> {
    None
  }

  fn to_typ_enum(&self, db: &'x0 TypedownDatabase) -> TdTypEnum<'x0> {
    self
      .runtime_typ(db)
      .expect("to_type_enum must be implemented for types without runtime_type")
  }

  fn instantiate(
    &self,
    db: &'x0 TypedownDatabase,
    args: Vec<LazyTyp<'x0>>,
  ) -> InstantiateResult<'x0> {
    let self_type = self.to_typ_enum(db);
    let diagnostics = validate_typ_params(db, self.typ_params(db).as_ref(), &args);
    InstantiateResult::new(db, self_type, diagnostics)
  }

  fn typ_params(&self, _db: &'x0 TypedownDatabase) -> Option<TypParams<'x0>> {
    None
  }

  fn parent_typ(&self, db: &'x0 TypedownDatabase) -> Option<TdTypEnum<'x0>> {
    Some(get_obj_typ(db).into())
  }

  fn runtime_vtable(&self, db: &'x0 TypedownDatabase) -> BTreeMap<String, TdFuncObj<'x0>> {
    self
      .parent_typ(db)
      .map(|p| p.runtime_vtable(db))
      .unwrap_or_default()
  }

  fn static_vtable(&self, db: &'x0 TypedownDatabase) -> BTreeMap<String, TdTypEnum<'x0>> {
    self
      .parent_typ(db)
      .map(|p| p.static_vtable(db))
      .unwrap_or_default()
  }

  fn get_fields(&self, _db: &'x0 TypedownDatabase) -> BTreeMap<String, LazyTyp<'x0>> {
    BTreeMap::new()
  }

  fn get_owned_field_typ(&self, db: &'x0 TypedownDatabase, name: &str) -> Option<TdTypEnum<'x0>> {
    self.get_fields(db).get(name)?.resolve(db)
  }

  fn lookup_field_typ(&self, db: &'x0 TypedownDatabase, name: &str) -> Option<TdTypEnum<'x0>> {
    if let Some(field) = self.get_owned_field_typ(db, name) {
      return Some(field);
    }
    self.static_vtable(db).get(name).cloned()
  }

  fn is_typ(&self, _db: &'x0 TypedownDatabase) -> bool {
    false
  }

  fn index_typ(
    &self,
    db: &'x0 TypedownDatabase,
    _key_type: &TdTypEnum<'x0>,
  ) -> Option<FuncSignature<'x0>> {
    if let Some(TdTypEnum::TdFuncTyp(func)) = self.lookup_field_typ(db, PROTOCOL_INDEX) {
      return Some(func.signature(db));
    }
    None
  }

  fn call_typ(
    &self,
    db: &'x0 TypedownDatabase,
    _arg_types: Vec<TdTypEnum<'x0>>,
  ) -> Option<FuncSignature<'x0>> {
    if let Some(TdTypEnum::TdFuncTyp(func)) = self.lookup_field_typ(db, PROTOCOL_CALL) {
      return Some(func.signature(db));
    }
    None
  }
}

// Runtime object protocol for the evaluator
#[delegatable_trait]
pub trait TdRuntimeObj<'x0>: Id {
  fn get_typ(&self, db: &'x0 TypedownDatabase) -> TdTypEnum<'x0>;

  fn lookup_method(&self, db: &'x0 TypedownDatabase, key: &str) -> Option<TdFuncObj<'x0>> {
    let mut current = Some(self.get_typ(db));
    while let Some(typ) = current {
      if let Some(func) = typ.runtime_vtable(db).get(key) {
        return Some(*func);
      }
      current = typ.parent_typ(db);
    }
    None
  }

  fn lookup_field(&self, db: &'x0 TypedownDatabase, key: &str) -> Option<TdObjEnum<'x0>> {
    if let Some(field) = self.get_owned_field(db, key) {
      return Some(field);
    }
    self.lookup_method(db, key).map(TdObjEnum::from)
  }

  fn get_owned_field(&self, db: &'x0 TypedownDatabase, key: &str) -> Option<TdObjEnum<'x0>>;

  // Access builtin fields (_icon, _label, etc.)
  // Schema instances fall back to schema type builtins
  fn get_builtin_field(&self, db: &'x0 TypedownDatabase, key: &str) -> Option<TdObjEnum<'x0>>;

  fn source_path(&self, db: &'x0 TypedownDatabase) -> String;

  fn eq(&self, _db: &'x0 TypedownDatabase, other: &TdObjEnum<'x0>) -> bool {
    self.as_id() == other.as_id()
  }

  fn lt(&self, _db: &'x0 TypedownDatabase, other: &TdObjEnum<'x0>) -> bool {
    self.as_id() < other.as_id()
  }

  fn gt(&self, _db: &'x0 TypedownDatabase, other: &TdObjEnum<'x0>) -> bool {
    self.as_id() > other.as_id()
  }

  fn le(&self, _db: &'x0 TypedownDatabase, other: &TdObjEnum<'x0>) -> bool {
    self.as_id() <= other.as_id()
  }

  fn ge(&self, _db: &'x0 TypedownDatabase, other: &TdObjEnum<'x0>) -> bool {
    self.as_id() >= other.as_id()
  }

  fn call(
    &self,
    _db: &'x0 TypedownDatabase,
    _project: ::typedown_lang::db::types::Project,
    _this: Option<TdObjEnum<'x0>>,
    _args: Vec<TdObjEnum<'x0>>,
  ) -> Result<TdObjEnum<'x0>, Vec<Diagnostic>> {
    Err(vec![])
  }

  fn index(&self, _db: &'x0 TypedownDatabase, _key: &TdObjEnum<'x0>) -> Option<TdObjEnum<'x0>> {
    None
  }

  fn len(&self, _db: &'x0 TypedownDatabase) -> Option<usize> {
    None
  }

  fn to_display_string(&self, db: &'x0 TypedownDatabase) -> String {
    self.source_path(db)
  }
}

// The metatype is the type of all types
// It is an instance of itself and the type of every type
#[query_derived]
pub struct TdTypTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdTypTyp<'db> {
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
    "@builtin::type".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdTypTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "type".to_string()
  }
  fn is_typ(&self, _db: &'db TypedownDatabase) -> bool {
    true
  }
  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some((*self).into())
  }
}

impl<'db> TdTypTyp<'db> {
  pub fn get(db: &'db ::typedown_lang::db::TypedownDatabase) -> TdTypTyp<'db> {
    get_typ_typ(db)
  }
}
