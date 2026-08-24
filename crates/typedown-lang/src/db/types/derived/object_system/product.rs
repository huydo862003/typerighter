use std::collections::HashMap;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, Id, QueryDatabase, StableHash, StableHasher,
};
use typedown_macros::{StableCompare, query_derived};

use super::base::{TdRuntimeObject, TdStaticType};
use super::func::TdFuncObj;
use super::null::TdNullObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::{get_func_type, get_str_type, get_type_type};
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::types::derived::object_system::base::{
  BUILTIN_TO_STRING, PROTOCOL_CALL, PROTOCOL_INDEX,
};
use crate::db::types::{FnKind, FuncSignature, HirValue, LazyType, NativeFnKind, Symbol};
use crate::db::utils::static_type::format_field_map;
use crate::syntax::diagnostic::Diagnostic;
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

#[query_derived]
pub struct TdProductType {
  pub name: Option<String>,
  pub metatype: TdTypeEnum,
  pub fields: HashMap<String, PropertyDescriptor>,
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
    format_field_map(db, &self.get_fields(db))
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
    self
      .fields(db)
      .into_iter()
      .map(|(name, desc)| (name, desc.field_type))
      .collect()
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
      Some(Either::Left(hir)) => {
        let file_scope = get_file_runtime_scope(db, hir.project(db), hir.file(db));
        evaluate_node(db, hir, file_scope).value(db)
      }
      Some(Either::Right(obj)) => Some(obj),
      // Missing fields evaluate to computed, default from PropertyDescriptor if present, or null
      None => {
        if let Some(product_type) = self.schema(db).as_td_product_type()
          && let Some(prop_desc) = product_type.fields(db).get(key)
        {
          if let Some(ref computed_enum) = prop_desc.computed_fn
            && let Some(func_obj) = computed_enum.as_td_func_obj()
            && let Ok(res_val) = func_obj.call(db, None, vec![(*self).into()])
          {
            return Some(res_val);
          }
          if let Some(ref def_obj) = prop_desc.default_value {
            return Some(def_obj.clone());
          }
        }
        Some(TdNullObj::get(db).into())
      }
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

#[cfg(test)]
mod tests {
  use std::collections::HashMap;

  use super::*;
  use crate::db::derived::get_builtin_types::{get_str_type, get_type_type};
  use crate::db::fixtures::load_vault_fixture;

  #[test]
  fn test_product_type_get_fields_and_get_owned_field_type_include_default_and_computed() {
    let (db, _, _) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let str_type: TdTypeEnum = get_str_type(&db).into();
    let schema_metatype: TdTypeEnum = get_type_type(&db).into();

    let mut fields = HashMap::new();
    fields.insert(
      "plain".to_string(),
      PropertyDescriptor {
        field_type: LazyType::eager(str_type.clone()),
        default_value: None,
        computed_fn: None,
      },
    );
    fields.insert(
      "with_default".to_string(),
      PropertyDescriptor {
        field_type: LazyType::eager(str_type.clone()),
        default_value: Some(crate::db::types::TdStrObj::new(&db, "default_val".to_string()).into()),
        computed_fn: None,
      },
    );
    fields.insert(
      "with_computed".to_string(),
      PropertyDescriptor {
        field_type: LazyType::eager(str_type.clone()),
        default_value: None,
        computed_fn: Some(crate::db::types::TdStrObj::new(&db, "dummy_fn".to_string()).into()),
      },
    );

    let product_type = TdProductType::new(
      &db,
      Some("TestProduct".to_string()),
      schema_metatype,
      fields,
      HashMap::new(),
    );

    let all_fields = product_type.get_fields(&db);
    assert_eq!(all_fields.len(), 3);
    assert!(all_fields.contains_key("plain"));
    assert!(all_fields.contains_key("with_default"));
    assert!(all_fields.contains_key("with_computed"));

    assert_eq!(
      product_type.get_owned_field_type(&db, "plain"),
      Some(str_type.clone())
    );
    assert_eq!(
      product_type.get_owned_field_type(&db, "with_default"),
      Some(str_type.clone())
    );
    assert_eq!(
      product_type.get_owned_field_type(&db, "with_computed"),
      Some(str_type.clone())
    );
  }
}
