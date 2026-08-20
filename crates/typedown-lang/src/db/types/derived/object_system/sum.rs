use std::collections::HashSet;
use typedown_macros::query_derived;

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::types::LazyType;
use typedown_incremental::StableCompare;

// A union type: accepts any of its member types
#[query_derived]
pub struct TdSumType {
  pub members: HashSet<LazyType>,
}

impl TdStaticType for TdSumType {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    let mut members: Vec<_> = self.members(db).into_iter().collect();
    members.sort_by(|a, b| a.stable_cmp(db, b));
    let parts: Vec<String> = members
      .iter()
      .filter_map(|m| m.resolve(db).map(|t| t.display_name(db)))
      .collect();
    parts.join(" | ")
  }
}

impl TdRuntimeObject for TdSumType {
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
