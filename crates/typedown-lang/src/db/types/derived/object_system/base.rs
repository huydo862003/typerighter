//! Type system core traits
//! - TdStaticType: static properties for the typechecker (display_name, arity, etc)
//! - TdRuntimeObject: runtime object protocol for the evaluator (field access, method dispatch)

use std::collections::HashMap;

use ambassador::delegatable_trait;

use super::func::TdFuncObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_object_type, get_type_type};
use crate::db::typecheck::utils::validate_type_params;
use crate::db::types::{FuncSignature, InstResult, LazyType, TypeParams};
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
pub trait TdStaticType<'x0> {
  fn display_name(&self, db: &'x0 TypedownDatabase) -> String;

  fn arity(&self, _db: &'x0 TypedownDatabase) -> usize {
    0
  }

  fn get_type_args(&self, _db: &'x0 TypedownDatabase) -> Vec<TdTypeEnum<'x0>> {
    vec![]
  }

  fn runtime_type(&self, _db: &'x0 TypedownDatabase) -> Option<TdTypeEnum<'x0>> {
    None
  }

  fn construct(
    &self,
    _db: &'x0 TypedownDatabase,
    _project: ::typedown_lang::db::types::Project,
    _args: Vec<TdObjectEnum<'x0>>,
  ) -> Option<TdObjectEnum<'x0>> {
    None
  }

  fn to_type_enum(&self, db: &'x0 TypedownDatabase) -> TdTypeEnum<'x0> {
    self
      .runtime_type(db)
      .expect("to_type_enum must be implemented for types without runtime_type")
  }

  fn instantiate(&self, db: &'x0 TypedownDatabase, args: Vec<LazyType<'x0>>) -> InstResult<'x0> {
    let self_type = self.to_type_enum(db);
    let diagnostics = validate_type_params(db, self.type_params(db).as_ref(), &args);
    InstResult::new(db, self_type, diagnostics)
  }

  fn type_params(&self, _db: &'x0 TypedownDatabase) -> Option<TypeParams<'x0>> {
    None
  }

  fn parent_type(&self, db: &'x0 TypedownDatabase) -> Option<TdTypeEnum<'x0>> {
    Some(get_object_type(db).into())
  }

  fn runtime_vtable(&self, db: &'x0 TypedownDatabase) -> HashMap<String, TdFuncObj<'x0>> {
    self
      .parent_type(db)
      .map(|p| p.runtime_vtable(db))
      .unwrap_or_default()
  }

  fn static_vtable(&self, db: &'x0 TypedownDatabase) -> HashMap<String, TdTypeEnum<'x0>> {
    self
      .parent_type(db)
      .map(|p| p.static_vtable(db))
      .unwrap_or_default()
  }

  fn get_fields(&self, _db: &'x0 TypedownDatabase) -> HashMap<String, LazyType<'x0>> {
    HashMap::new()
  }

  fn get_owned_field_type(&self, db: &'x0 TypedownDatabase, name: &str) -> Option<TdTypeEnum<'x0>> {
    self.get_fields(db).get(name)?.resolve(db)
  }

  fn lookup_field_type(&self, db: &'x0 TypedownDatabase, name: &str) -> Option<TdTypeEnum<'x0>> {
    if let Some(field) = self.get_owned_field_type(db, name) {
      return Some(field);
    }
    self.static_vtable(db).get(name).cloned()
  }

  fn is_type(&self, _db: &'x0 TypedownDatabase) -> bool {
    false
  }

  fn index_type(
    &self,
    db: &'x0 TypedownDatabase,
    _key_type: &TdTypeEnum<'x0>,
  ) -> Option<FuncSignature<'x0>> {
    if let Some(TdTypeEnum::TdFuncType(func)) = self.lookup_field_type(db, PROTOCOL_INDEX) {
      return Some(func.signature(db));
    }
    None
  }

  fn call_type(
    &self,
    db: &'x0 TypedownDatabase,
    _arg_types: Vec<TdTypeEnum<'x0>>,
  ) -> Option<FuncSignature<'x0>> {
    if let Some(TdTypeEnum::TdFuncType(func)) = self.lookup_field_type(db, PROTOCOL_CALL) {
      return Some(func.signature(db));
    }
    None
  }
}

// Runtime object protocol for the evaluator
#[delegatable_trait]
pub trait TdRuntimeObject<'x0>: Id {
  fn get_type(&self, db: &'x0 TypedownDatabase) -> TdTypeEnum<'x0>;

  fn lookup_method(&self, db: &'x0 TypedownDatabase, key: &str) -> Option<TdFuncObj<'x0>> {
    let mut current = Some(self.get_type(db));
    while let Some(typ) = current {
      if let Some(func) = typ.runtime_vtable(db).get(key) {
        return Some(*func);
      }
      current = typ.parent_type(db);
    }
    None
  }

  fn lookup_field(&self, db: &'x0 TypedownDatabase, key: &str) -> Option<TdObjectEnum<'x0>> {
    if let Some(field) = self.get_owned_field(db, key) {
      return Some(field);
    }
    self.lookup_method(db, key).map(TdObjectEnum::from)
  }

  fn get_owned_field(&self, db: &'x0 TypedownDatabase, key: &str) -> Option<TdObjectEnum<'x0>>;

  // Access builtin fields (_icon, _label, etc.)
  // Schema instances fall back to schema type builtins
  fn get_builtin_field(&self, db: &'x0 TypedownDatabase, key: &str) -> Option<TdObjectEnum<'x0>>;

  fn source_path(&self, db: &'x0 TypedownDatabase) -> String;

  fn eq(&self, _db: &'x0 TypedownDatabase, other: &TdObjectEnum<'x0>) -> bool {
    self.as_id() == other.as_id()
  }

  fn lt(&self, _db: &'x0 TypedownDatabase, other: &TdObjectEnum<'x0>) -> bool {
    self.as_id() < other.as_id()
  }

  fn gt(&self, _db: &'x0 TypedownDatabase, other: &TdObjectEnum<'x0>) -> bool {
    self.as_id() > other.as_id()
  }

  fn le(&self, _db: &'x0 TypedownDatabase, other: &TdObjectEnum<'x0>) -> bool {
    self.as_id() <= other.as_id()
  }

  fn ge(&self, _db: &'x0 TypedownDatabase, other: &TdObjectEnum<'x0>) -> bool {
    self.as_id() >= other.as_id()
  }

  fn call(
    &self,
    _db: &'x0 TypedownDatabase,
    _project: ::typedown_lang::db::types::Project,
    _this: Option<TdObjectEnum<'x0>>,
    _args: Vec<TdObjectEnum<'x0>>,
  ) -> Result<TdObjectEnum<'x0>, Vec<Diagnostic>> {
    Err(vec![])
  }

  fn index(
    &self,
    _db: &'x0 TypedownDatabase,
    _key: &TdObjectEnum<'x0>,
  ) -> Option<TdObjectEnum<'x0>> {
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
pub struct TdTypeType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdTypeType<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::type".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdTypeType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "type".to_string()
  }
  fn is_type(&self, _db: &'db TypedownDatabase) -> bool {
    true
  }
  fn runtime_type(&self, _db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some((*self).into())
  }
}

impl<'db> TdTypeType<'db> {
  pub fn get(db: &'db ::typedown_lang::db::TypedownDatabase) -> TdTypeType<'db> {
    get_type_type(db)
  }
}
