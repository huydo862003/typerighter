use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{LazyType, TypeParams};

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
