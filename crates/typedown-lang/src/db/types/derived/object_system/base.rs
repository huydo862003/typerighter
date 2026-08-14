//! We follow a simple system for supertypes
//! - Only owned fields can be accessed via the object
//! - Methods can be inheritted via supertypes

use std::collections::HashMap;

use ambassador::delegatable_trait;

use super::func::TdFuncObj;
use super::native_fn::NativeFnKind;
use super::str::TdStrType;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::derived::get_builtin_types::{get_object_type, get_type_type};
use crate::db::types::{
  FuncSignature, InstResult, LazyType, MemberType, TypeMember, TypeMemberDescriptors,
};
use typedown_incremental::Id;
use typedown_macros::query_derived;

// Everything is an object
// This need not be object-safe
// We access via the enum, not via dyn trait
#[delegatable_trait]
pub trait TdObjectLike: Id {
  fn get_type(&self, db: &::typedown_lang::db::TypedownDatabase) -> TdTypeEnum;

  fn lookup_method(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    key: &str,
  ) -> Option<TdFuncObj> {
    let mut current = self.get_type(db);
    loop {
      if let Some(func_obj) = current.get_vtable(db).remove(key) {
        return Some(func_obj);
      }
      let supertype = current.get_supertype(db);
      if supertype.as_id() == current.as_id() {
        return None;
      }
      current = supertype;
    }
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
}

// This need not be object-safe
// We access via the enum, not via dyn trait
#[delegatable_trait]
pub trait TdTypeLike: TdObjectLike {
  fn arity(&self, db: &::typedown_lang::db::TypedownDatabase) -> usize;
  fn get_supertype(&self, db: &::typedown_lang::db::TypedownDatabase) -> TdTypeEnum;
  fn get_vtable(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
  ) -> std::collections::HashMap<String, TdFuncObj>;
  fn get_owned_field_type_member(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    name: &str,
  ) -> Option<::typedown_lang::db::types::TypeMember>;
  fn lookup_field_type_member(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    name: &str,
  ) -> Option<::typedown_lang::db::types::TypeMember> {
    self.get_owned_field_type_member(db, name).or_else(|| {
      Some(TypeMember::new(
        db,
        MemberType::Simple(LazyType::eager(self.lookup_method(db, name)?.get_type(db))),
        TypeMemberDescriptors::empty(),
      ))
    })
  }

  fn instantiate(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    args: Vec<LazyType>,
  ) -> InstResult;

  fn is_compatible_with(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    actual: &TdTypeEnum,
  ) -> bool;

  fn get_type_args(&self, db: &::typedown_lang::db::TypedownDatabase) -> Vec<TdTypeEnum>;

  fn display_name(&self, db: &::typedown_lang::db::TypedownDatabase) -> String;

  fn construct(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    args: Vec<TdObjectEnum>,
  ) -> Option<TdObjectEnum>;

  fn lookup_instance_method(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    key: &str,
  ) -> Option<TdFuncObj> {
    if let Some(func_obj) = self.get_vtable(db).get(key) {
      return Some(*func_obj);
    }
    let supertype = self.get_supertype(db);
    if supertype.as_id() == self.as_id() {
      return None;
    }
    supertype.lookup_instance_method(db, key)
  }
}

/// The metatype is the type of all types.
/// It's an instance of itself and the type of every type.
#[query_derived]
pub struct TdTypeType {}

impl TdObjectLike for TdTypeType {
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

impl TdTypeLike for TdTypeType {
  fn arity(&self, _db: &::typedown_lang::db::TypedownDatabase) -> usize {
    0
  }
  fn get_supertype(&self, db: &::typedown_lang::db::TypedownDatabase) -> TdTypeEnum {
    TdObjectType::get(db).into()
  }
  fn get_vtable(
    &self,
    _db: &::typedown_lang::db::TypedownDatabase,
  ) -> std::collections::HashMap<String, TdFuncObj> {
    HashMap::new()
  }
  fn get_owned_field_type_member(
    &self,
    _db: &::typedown_lang::db::TypedownDatabase,
    _name: &str,
  ) -> Option<TypeMember> {
    None
  }
  fn instantiate(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    _args: Vec<LazyType>,
  ) -> InstResult {
    InstResult::new(db, (*self).into(), vec![])
  }
  fn get_type_args(&self, _db: &::typedown_lang::db::TypedownDatabase) -> Vec<TdTypeEnum> {
    vec![]
  }
  // Any type is assignable to the metatype
  fn is_compatible_with(
    &self,
    _db: &::typedown_lang::db::TypedownDatabase,
    _actual: &TdTypeEnum,
  ) -> bool {
    true
  }
  fn construct(
    &self,
    _db: &::typedown_lang::db::TypedownDatabase,
    _args: Vec<TdObjectEnum>,
  ) -> Option<TdObjectEnum> {
    None
  }
  fn display_name(&self, _db: &::typedown_lang::db::TypedownDatabase) -> String {
    "type".to_string()
  }
}

impl TdTypeType {
  pub fn get(db: &::typedown_lang::db::TypedownDatabase) -> TdTypeType {
    get_type_type(db)
  }
}

/// The base type for all objects in Typedown
#[query_derived]
pub struct TdObjectType {}

impl TdObjectLike for TdObjectType {
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
    "@builtin::object".to_string()
  }
}

impl TdTypeLike for TdObjectType {
  fn arity(&self, _db: &::typedown_lang::db::TypedownDatabase) -> usize {
    0
  }
  fn get_supertype(&self, db: &::typedown_lang::db::TypedownDatabase) -> TdTypeEnum {
    TdObjectType::get(db).into()
  }
  fn get_vtable(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
  ) -> std::collections::HashMap<String, TdFuncObj> {
    let sig = FuncSignature::new(db, vec![], TdStrType::get(db).into());
    let func_obj = TdFuncObj::new(
      db,
      "to_string".to_string(),
      get_object_type(db).into(),
      sig,
      NativeFnKind::ObjectToString,
    );
    HashMap::from([("to_string".to_string(), func_obj)])
  }
  fn get_owned_field_type_member(
    &self,
    _db: &::typedown_lang::db::TypedownDatabase,
    _name: &str,
  ) -> Option<TypeMember> {
    None
  }
  fn instantiate(
    &self,
    db: &::typedown_lang::db::TypedownDatabase,
    _args: Vec<LazyType>,
  ) -> InstResult {
    InstResult::new(db, (*self).into(), vec![])
  }
  fn get_type_args(&self, _db: &::typedown_lang::db::TypedownDatabase) -> Vec<TdTypeEnum> {
    vec![]
  }
  fn is_compatible_with(
    &self,
    _db: &::typedown_lang::db::TypedownDatabase,
    actual: &TdTypeEnum,
  ) -> bool {
    self.as_id() == actual.as_id()
  }
  fn construct(
    &self,
    _db: &::typedown_lang::db::TypedownDatabase,
    args: Vec<TdObjectEnum>,
  ) -> Option<TdObjectEnum> {
    args.into_iter().next()
  }
  fn display_name(&self, _db: &::typedown_lang::db::TypedownDatabase) -> String {
    "object".to_string()
  }
}

impl TdObjectType {
  pub fn get(db: &::typedown_lang::db::TypedownDatabase) -> TdObjectType {
    get_object_type(db)
  }
}
