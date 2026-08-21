use std::collections::HashMap;
use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{FuncSignature, LazyType, TypeParams};

/// Existential type: `exists <T0 <: Bound, ...>. Body`
#[query_derived]
pub struct TdExistentialType {
  #[id]
  pub type_params: TypeParams,
  #[id]
  pub body: Option<LazyType>,
}

impl TdStaticType for TdExistentialType {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    let params = self.type_params(db);
    let params_str = params
      .params(db)
      .iter()
      .enumerate()
      .map(|(i, p)| {
        if let Some(b) = p.upper_bound(db).resolve(db) {
          format!("T{} <: {}", i, b.display_name(db))
        } else {
          format!("T{}", i)
        }
      })
      .collect::<Vec<_>>()
      .join(", ");

    let body_str = self
      .body(db)
      .and_then(|b| b.resolve(db))
      .map(|b| b.display_name(db))
      .unwrap_or_else(|| "Never".to_string());

    if params_str.is_empty() {
      format!("exists. {}", body_str)
    } else {
      format!("exists <{}>. {}", params_str, body_str)
    }
  }

  fn static_vtable(&self, db: &TypedownDatabase) -> HashMap<String, TdTypeEnum> {
    self
      .body(db)
      .and_then(|b| b.resolve(db))
      .map(|b| b.static_vtable(db))
      .unwrap_or_default()
  }

  fn get_fields(&self, db: &TypedownDatabase) -> HashMap<String, LazyType> {
    self
      .body(db)
      .and_then(|b| b.resolve(db))
      .map(|b| b.get_fields(db))
      .unwrap_or_default()
  }

  fn lookup_field_type(&self, db: &TypedownDatabase, name: &str) -> Option<TdTypeEnum> {
    self
      .body(db)
      .and_then(|b| b.resolve(db))?
      .lookup_field_type(db, name)
  }

  fn index_type(&self, db: &TypedownDatabase, key_type: &TdTypeEnum) -> Option<FuncSignature> {
    self
      .body(db)
      .and_then(|b| b.resolve(db))?
      .index_type(db, key_type)
  }

  fn call_type(&self, db: &TypedownDatabase, arg_types: Vec<TdTypeEnum>) -> Option<FuncSignature> {
    self
      .body(db)
      .and_then(|b| b.resolve(db))?
      .call_type(db, arg_types)
  }
}

impl TdRuntimeObject for TdExistentialType {
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
  use crate::db::derived::get_builtin_types::{
    get_func_type, get_list_type, get_num_type, get_str_type,
  };
  use crate::db::types::derived::object_system::TdStructuralType;
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn existential_type_delegates_static_operations_to_body() {
    let db = make_db();
    let str_type: TdTypeEnum = get_str_type(&db).into();

    let mut fields = HashMap::new();
    fields.insert("title".to_string(), LazyType::eager(str_type.clone()));
    let body_struct: TdTypeEnum = TdStructuralType::new(&db, fields).into();

    let ex_params = TypeParams::new(&db, vec![], vec![]);
    let ex_type = TdExistentialType::new(&db, ex_params, Some(LazyType::eager(body_struct)));

    // static operations delegate to body
    assert_eq!(
      ex_type.lookup_field_type(&db, "title"),
      Some(str_type.clone())
    );
    assert_eq!(ex_type.lookup_field_type(&db, "nonexistent"), None);

    let fields_map = ex_type.get_fields(&db);
    assert!(fields_map.contains_key("title"));
  }

  #[test]
  fn existential_type_delegates_index_type_to_body() {
    let db = make_db();
    let str_type: TdTypeEnum = get_str_type(&db).into();
    let num_type: TdTypeEnum = get_num_type(&db).into();

    let list_str = get_list_type(&db)
      .instantiate(&db, vec![LazyType::eager(str_type.clone())])
      .typ(&db);

    let ex_params = TypeParams::new(&db, vec![], vec![]);
    let ex_type = TdExistentialType::new(&db, ex_params, Some(LazyType::eager(list_str)));

    let idx_sig = ex_type.index_type(&db, &num_type).unwrap();
    assert_eq!(idx_sig.ret(&db), str_type);
  }

  #[test]
  fn existential_type_delegates_call_type_to_body() {
    let db = make_db();
    let str_type: TdTypeEnum = get_str_type(&db).into();
    let num_type: TdTypeEnum = get_num_type(&db).into();

    let sig = FuncSignature::new(&db, vec![str_type.clone()], num_type.clone());
    let func_type: TdTypeEnum = get_func_type(&db, sig).into();

    let ex_params = TypeParams::new(&db, vec![], vec![]);
    let ex_type = TdExistentialType::new(&db, ex_params, Some(LazyType::eager(func_type)));

    let call_sig = ex_type.call_type(&db, vec![str_type]).unwrap();
    assert_eq!(call_sig.ret(&db), num_type);
  }
}
