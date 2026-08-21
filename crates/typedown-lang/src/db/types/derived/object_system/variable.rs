use std::collections::HashMap;
use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{FuncSignature, LazyType, TypeVariable};

/// A type variable reference within a type expression
#[query_derived]
pub struct TdVariableType {
  #[id]
  pub index: usize,
  #[id]
  pub variable: TypeVariable,
}

impl TdStaticType for TdVariableType {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    let var = self.variable(db);
    if let Some(b) = var.upper_bound(db).resolve(db) {
      if b.as_td_object_type().is_some() {
        format!("T{}", self.index(db))
      } else {
        format!("T{} <: {}", self.index(db), b.display_name(db))
      }
    } else {
      format!("T{}", self.index(db))
    }
  }

  fn static_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdTypeEnum> {
    self
      .variable(db)
      .upper_bound(db)
      .resolve(db)
      .map(|upper| upper.static_vtable(db))
      .unwrap_or_default()
  }

  fn get_fields(&self, db: &TypedownDatabase) -> HashMap<String, LazyType> {
    self
      .variable(db)
      .upper_bound(db)
      .resolve(db)
      .map(|upper| upper.get_fields(db))
      .unwrap_or_default()
  }

  fn lookup_field_type(&self, db: &TypedownDatabase, name: &str) -> Option<TdTypeEnum> {
    self
      .variable(db)
      .upper_bound(db)
      .resolve(db)?
      .lookup_field_type(db, name)
  }

  fn index_type(&self, db: &TypedownDatabase, key_type: &TdTypeEnum) -> Option<FuncSignature> {
    self
      .variable(db)
      .upper_bound(db)
      .resolve(db)?
      .index_type(db, key_type)
  }

  fn call_type(&self, db: &TypedownDatabase, arg_types: Vec<TdTypeEnum>) -> Option<FuncSignature> {
    self
      .variable(db)
      .upper_bound(db)
      .resolve(db)?
      .call_type(db, arg_types)
  }
}

impl TdRuntimeObject for TdVariableType {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &TypedownDatabase, _key: &str) -> Option<TdObjectEnum> {
    None
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.display_name(db)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_types::get_str_type;
  use crate::db::types::derived::object_system::TdStructuralType;
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn variable_type_delegates_to_upper_bound() {
    let db = make_db();
    let str_type: TdTypeEnum = get_str_type(&db).into();

    let mut fields = HashMap::new();
    fields.insert("name".to_string(), LazyType::eager(str_type.clone()));
    let struct_type: TdTypeEnum = TdStructuralType::new(&db, fields).into();

    let var = TypeVariable::get(&db, Some(LazyType::eager(struct_type)));
    let var_type = TdVariableType::new(&db, 0, var);

    // field lookup delegates to Upper
    assert_eq!(var_type.lookup_field_type(&db, "name"), Some(str_type));
    assert_eq!(var_type.lookup_field_type(&db, "nonexistent"), None);
  }
}
