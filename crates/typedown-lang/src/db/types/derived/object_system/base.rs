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
  fn display_name(&self, db: &TypedownDatabase) -> String;

  fn arity(&self, _db: &TypedownDatabase) -> usize {
    0
  }

  fn get_type_args(&self, _db: &TypedownDatabase) -> Vec<TdTypeEnum> {
    vec![]
  }

  // The runtime-constructible type equivalent
  // Most types are their own runtime equivalent
  fn runtime_type(&self, _db: &TypedownDatabase) -> Option<TdTypeEnum> {
    None
  }

  // Construct a runtime instance of this type from args
  fn construct(
    &self,
    _db: &TypedownDatabase,
    _project: ::typedown_lang::db::types::Project,
    _args: Vec<TdObjectEnum>,
  ) -> Option<TdObjectEnum> {
    None
  }

  fn to_type_enum(&self, db: &TypedownDatabase) -> TdTypeEnum {
    self
      .runtime_type(db)
      .expect("to_type_enum must be implemented for types without runtime_type")
  }

  // Instantiate a generic type with type arguments (list[string], dict[string, number])
  fn instantiate(&self, db: &TypedownDatabase, args: Vec<LazyType>) -> InstResult {
    let self_type = self.to_type_enum(db);
    let diagnostics = validate_type_params(db, self.type_params(db).as_ref(), &args);
    InstResult::new(db, self_type, diagnostics)
  }

  /// Type parameters declared on this generic type
  fn type_params(&self, _db: &TypedownDatabase) -> Option<TypeParams> {
    None
  }

  /// Parent type for prototype chain method lookup and type hierarchy
  fn parent_type (&self, db: &'x0 TypedownDatabase) -> Option<TdTypeEnum> {
    Some(get_object_type(db).into())
  }

  /// Runtime vtable mapping method names to TdFuncObj instances
  fn runtime_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdFuncObj> {
    self
      .parent_type(db)
      .map(|p| p.runtime_vtable(db))
      .unwrap_or_default()
  }

  // Return types of methods available on instances of this static type
  fn static_vtable(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
  ) -> HashMap<String, TdTypeEnum> {
    self
      .parent_type(db)
      .map(|p| p.static_vtable(db))
      .unwrap_or_default()
  }

  // Get all declared fields from a static type
  fn get_fields(
    &self,
    _db: &::typedown_lang::db::TypedownDatabase,
  ) -> HashMap<String, ::typedown_lang::db::types::LazyType> {
    HashMap::new()
  }

  // Get the type of a named field on this type
  fn get_owned_field_type(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    name: &str,
  ) -> Option<TdTypeEnum> {
    self.get_fields(db).get(name)?.resolve(db)
  }

  // Look up the type of a field or method on this type
  fn lookup_field_type(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    name: &str,
  ) -> Option<TdTypeEnum> {
    if let Some(field) = self.get_owned_field_type(db, name) {
      return Some(field);
    }
    self.static_vtable(db).get(name).cloned()
  }

  // Check if this is a metatype (a type whose instances are types)
  fn is_type(&self, _db: &::typedown_lang::db::TypedownDatabase) -> bool {
    false
  }

  // Signature when indexing an instance of this static type with a key type
  fn index_type(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    _key_type: &TdTypeEnum,
  ) -> Option<FuncSignature> {
    if let Some(TdTypeEnum::TdFuncType(func)) = self.lookup_field_type(db, PROTOCOL_INDEX) {
      return Some(func.signature(db));
    }
    None
  }

  // Signature when calling an instance of this static type as a function
  fn call_type(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    _arg_types: Vec<TdTypeEnum>,
  ) -> Option<FuncSignature> {
    if let Some(TdTypeEnum::TdFuncType(func)) = self.lookup_field_type(db, PROTOCOL_CALL) {
      return Some(func.signature(db));
    }
    None
  }
}

// Runtime object protocol for the evaluator
#[delegatable_trait]
pub trait TdRuntimeObject: Id {
  fn get_type(&self, db: &::typedown_lang::db::TypedownDatabase) -> TdTypeEnum;

  fn lookup_method(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    key: &str,
  ) -> Option<TdFuncObj> {
    let mut current = Some(self.get_type(db));
    while let Some(typ) = current {
      if let Some(func) = typ.runtime_vtable(db).get(key) {
        return Some(*func);
      }
      current = typ.parent_type(db);
    }
    None
  }

  fn lookup_field(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    key: &str,
  ) -> Option<TdObjectEnum> {
    if let Some(field) = self.get_owned_field(db, key) {
      return Some(field);
    }
    self.lookup_method(db, key).map(TdObjectEnum::from)
  }

  fn get_owned_field(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    key: &str,
  ) -> Option<TdObjectEnum>;

  fn source_path(&self, db: &::typedown_lang::db::TypedownDatabase) -> String;

  fn eq(&self, _db: &::typedown_lang::db::TypedownDatabase, other: &TdObjectEnum) -> bool {
    self.as_id() == other.as_id()
  }

  fn lt(&self, _db: &::typedown_lang::db::TypedownDatabase, other: &TdObjectEnum) -> bool {
    self.as_id() < other.as_id()
  }

  fn gt(&self, _db: &::typedown_lang::db::TypedownDatabase, other: &TdObjectEnum) -> bool {
    self.as_id() > other.as_id()
  }

  fn le(&self, _db: &::typedown_lang::db::TypedownDatabase, other: &TdObjectEnum) -> bool {
    self.as_id() <= other.as_id()
  }

  fn ge(&self, _db: &::typedown_lang::db::TypedownDatabase, other: &TdObjectEnum) -> bool {
    self.as_id() >= other.as_id()
  }

  fn call(
    &self,
    _db: &::typedown_lang::db::TypedownDatabase,
    _project: ::typedown_lang::db::types::Project,
    _this: Option<TdObjectEnum>,
    _args: Vec<TdObjectEnum>,
  ) -> Result<TdObjectEnum, Vec<Diagnostic>> {
    Err(vec![])
  }

  fn index(
    &self,
    _db: &::typedown_lang::db::TypedownDatabase,
    _key: &TdObjectEnum,
  ) -> Option<TdObjectEnum> {
    None
  }

  fn len(&self, _db: &::typedown_lang::db::TypedownDatabase) -> Option<usize> {
    None
  }

  fn to_display_string(&self, db: &::typedown_lang::db::TypedownDatabase) -> String {
    self.source_path(db)
  }
}

// The metatype is the type of all types
// It is an instance of itself and the type of every type
#[query_derived]
pub struct TdTypeType<'db> {}

impl TdRuntimeObject for TdTypeType<'_> {
  fn get_type(&self, db: &::typedown_lang::db::TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(
    &self,
    _db: &::typedown_lang::db::TypedownDatabase,
    _key: &str,
  ) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, _db: &::typedown_lang::db::TypedownDatabase) -> String {
    "@builtin::type".to_string()
  }
}

impl TdStaticType for TdTypeType<'_> {
  fn display_name(&self, _db: &::typedown_lang::db::TypedownDatabase) -> String {
    "type".to_string()
  }
  fn is_type(&self, _db: &::typedown_lang::db::TypedownDatabase) -> bool {
    true
  }
  fn runtime_type(&self, _db: &::typedown_lang::db::TypedownDatabase) -> Option<TdTypeEnum> {
    Some((*self).into())
  }
}

impl TdTypeType<'_> {
  pub fn get(db: &::typedown_lang::db::TypedownDatabase) -> TdTypeType {
    get_type_type(db)
  }
}
