use std::collections::HashMap;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};
use typedown_macros::{StableCompare, query_derived};

use super::base::{TdRuntimeObject, TdStaticType, TdTypeType};
use super::null::TdNullObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::get_object_type;
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::typecheck::utils::{is_nullable, is_subtype_of};
use crate::db::types::{HirValue, LazyType, Symbol};
use crate::db::utils::static_type::format_field_map;
use typedown_types::either::Either;

#[derive(Debug, Clone, PartialEq, Eq, StableCompare)]
pub struct PropertyDescriptor {
  pub field_type: LazyType,
  pub default_value: Option<TdObjectEnum>,
  pub computed_fn: Option<TdObjectEnum>,
}

impl StableHash for PropertyDescriptor {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.field_type.stable_hash(db, hasher);
    self.default_value.stable_hash(db, hasher);
    self.computed_fn.stable_hash(db, hasher);
  }
}

impl Encodable for PropertyDescriptor {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.field_type.encode(buf, encoder);
    self.default_value.encode(buf, encoder);
    self.computed_fn.encode(buf, encoder);
  }
}

impl Decodable for PropertyDescriptor {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let field_type = LazyType::decode(data, decoder);
    let default_value = Option::<TdObjectEnum>::decode(data, decoder);
    let computed_fn = Option::<TdObjectEnum>::decode(data, decoder);
    PropertyDescriptor {
      field_type,
      default_value,
      computed_fn,
    }
  }
}

// Structural data bag with optional display name
#[query_derived]
pub struct TdProductType {
  pub name: Option<String>,
  pub fields: HashMap<String, LazyType>,
}

impl TdRuntimeObject for TdProductType<'_> {
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

impl TdStaticType for TdProductType<'_> {
  fn display_name(&self, db: &TypedownDatabase) -> String {
    if let Some(name) = self.name(db) {
      return name;
    }
    format_field_map(db, &self.fields(db))
  }
  fn runtime_type(&self, _db: &TypedownDatabase) -> Option<TdTypeEnum> {
    None
  }
  fn parent_type(&self, db: &TypedownDatabase) -> Option<TdTypeEnum> {
    Some(get_object_type(db).into())
  }
  fn get_fields(&self, db: &TypedownDatabase) -> HashMap<String, LazyType> {
    self.fields(db)
  }
}

// Runtime instance of a product type, plain data bag
#[query_derived]
pub struct TdProductObj {
  pub product_type: TdTypeEnum,
  pub file_symbol: Option<Symbol>,
  pub fields: HashMap<String, Either<HirValue, TdObjectEnum>>,
}

impl TdRuntimeObject for TdProductObj<'_> {
  fn get_type(&self, db: &TypedownDatabase) -> TdTypeEnum {
    self.product_type(db)
  }
  fn get_owned_field(&self, db: &TypedownDatabase, key: &str) -> Option<TdObjectEnum> {
    match self.fields(db).get(key).cloned() {
      Some(Either::Left(hir)) => {
        let file_scope = get_file_runtime_scope(db, hir.project(db), hir.file(db));
        evaluate_node(db, hir, file_scope).value(db)
      }
      Some(Either::Right(obj)) => Some(obj),
      None => Some(TdNullObj::get(db).into()),
    }
  }
  fn source_path(&self, db: &TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
}

// Check if expected fields are compatible with actual fields
pub fn fields_compatible(
  db: &TypedownDatabase,
  expected_fields: &HashMap<String, LazyType>,
  actual_fields: &HashMap<String, LazyType>,
) -> bool {
  expected_fields.iter().all(|(name, expected_lazy)| {
    let optional = expected_lazy
      .resolve(db)
      .is_some_and(|t| is_nullable(db, &t));
    match actual_fields.get(name) {
      Some(actual_lazy) => {
        let Some(expected_type) = expected_lazy.resolve(db) else {
          return false;
        };
        let Some(actual_type) = actual_lazy.resolve(db) else {
          return false;
        };
        is_subtype_of(db, &actual_type, &expected_type)
      }
      None => optional,
    }
  })
}

pub fn make_property_descriptors(
  _db: &TypedownDatabase,
  fields: HashMap<String, LazyType>,
) -> HashMap<String, PropertyDescriptor> {
  fields
    .into_iter()
    .map(|(k, v)| {
      (
        k,
        PropertyDescriptor {
          field_type: v,
          default_value: None,
          computed_fn: None,
        },
      )
    })
    .collect()
}
