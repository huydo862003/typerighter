use std::collections::HashMap;
use typedown_macros::query_derived;

use super::base::{
  BUILTIN_TO_STRING, PROTOCOL_CALL, PROTOCOL_INDEX, TdRuntimeObject, TdStaticType, TdTypeType,
};
use super::func::TdFuncObj;
use super::null::TdNullObj;
use super::product::{PropertyDescriptor, TdProductType};
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_types::{
  get_dict_type, get_func_type, get_object_type, get_schema_meta_type, get_str_type, get_sum_type,
};
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::derived::schema_property::get_schema_property_type;
use crate::db::types::{FnKind, FuncSignature, HirValue, LazyType, NativeFnKind, Project, Symbol};
use crate::syntax::diagnostic::Diagnostic;
use typedown_types::either::Either;

// The metatype of all schema types
// schema is to TdSchemaType as type is to TdTypeType
#[query_derived]
pub struct TdSchemaMetaType<'db> {}

impl<'db> TdRuntimeObject<'db> for TdSchemaMetaType<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    TdTypeType::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "schema".to_string()
  }
}

impl<'db> TdStaticType<'db> for TdSchemaMetaType<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "schema".to_string()
  }
  fn parent_type(&self, db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some(TdTypeType::get(db).into())
  }
  fn get_fields(&self, db: &'db TypedownDatabase) -> HashMap<String, LazyType<'db>> {
    let properties_type = get_dict_type(db)
      .instantiate(
        db,
        vec![
          LazyType::eager(get_str_type(db).into()),
          LazyType::eager(get_schema_property_type(db).into()),
        ],
      )
      .typ(db);
    HashMap::from([("properties".to_string(), LazyType::eager(properties_type))])
  }
  fn is_type(&self, _db: &'db TypedownDatabase) -> bool {
    true
  }
  fn runtime_type(&self, _db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some((*self).into())
  }
}

// Named opaque type with methods, construction, and nominal subtyping
// Analogous to a class in JS
#[query_derived]
pub struct TdSchemaType<'db> {
  pub name: String,
  pub fields: HashMap<String, PropertyDescriptor<'db>>,
  pub vtable: HashMap<String, TdFuncObj<'db>>,
  pub parent: Option<TdTypeEnum<'db>>,
}

impl<'db> TdRuntimeObject<'db> for TdSchemaType<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    get_schema_meta_type(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjectEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.display_name(db)
  }
}

impl<'db> TdStaticType<'db> for TdSchemaType<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    self.name(db)
  }
  fn runtime_type(&self, _db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    Some((*self).into())
  }
  fn parent_type(&self, db: &'db TypedownDatabase) -> Option<TdTypeEnum<'db>> {
    self.parent(db).or_else(|| Some(get_object_type(db).into()))
  }
  // Construct a schema instance from a product object
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    project: crate::db::types::Project,
    args: Vec<TdObjectEnum<'db>>,
  ) -> Option<TdObjectEnum<'db>> {
    let arg = args.into_iter().next()?;
    let product = arg.as_td_product_obj()?;
    let fields = product.fields(db);
    Some(TdSchemaObj::new(db, (*self).into(), project, None, fields).into())
  }
  fn runtime_vtable(&self, db: &'db TypedownDatabase) -> HashMap<String, TdFuncObj<'db>> {
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
  fn static_vtable(&self, db: &'db TypedownDatabase) -> HashMap<String, TdTypeEnum<'db>> {
    let mut result = self
      .parent_type(db)
      .map(|p| p.static_vtable(db))
      .unwrap_or_default();
    let sig = FuncSignature::new(db, vec![], get_str_type(db).into());
    let func_type = get_func_type(db, sig).into();
    result
      .entry(BUILTIN_TO_STRING.to_string())
      .or_insert(func_type);
    for (name, func_obj) in self.vtable(db) {
      result.insert(name, get_func_type(db, func_obj.signature(db)).into());
    }
    result
  }
  fn get_fields(&self, db: &'db TypedownDatabase) -> HashMap<String, LazyType<'db>> {
    // Include inherited fields from parent schema
    let mut result = self
      .parent(db)
      .map(|p| p.get_fields(db))
      .unwrap_or_default();
    result.extend(
      self
        .fields(db)
        .into_iter()
        .map(|(name, desc)| (name, desc.field_type)),
    );
    result
  }
  fn is_type(&self, _db: &'db TypedownDatabase) -> bool {
    true
  }
}

impl<'db> TdSchemaType<'db> {
  // The argument type for construct: product | self
  pub fn construct_arg_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    let product: TdTypeEnum = TdProductType::new(db, None, self.get_fields(db)).into();
    let schema: TdTypeEnum = (*self).into();
    get_sum_type(db, vec![LazyType::eager(product), LazyType::eager(schema)]).into()
  }
}

// Runtime instance of a schema type, with computed fields, defaults, and methods
#[query_derived]
pub struct TdSchemaObj<'db> {
  pub schema: TdTypeEnum<'db>,
  pub project: Project,
  pub file_symbol: Option<Symbol<'db>>,
  pub fields: HashMap<String, Either<HirValue<'db>, TdObjectEnum<'db>>>,
}

impl<'db> TdRuntimeObject<'db> for TdSchemaObj<'db> {
  fn get_type(&self, db: &'db TypedownDatabase) -> TdTypeEnum<'db> {
    self.schema(db)
  }
  fn get_owned_field(&self, db: &'db TypedownDatabase, key: &str) -> Option<TdObjectEnum<'db>> {
    match self.fields(db).get(key).cloned() {
      Some(Either::Left(hir)) => {
        let file_scope = get_file_runtime_scope(db, hir.project(db), hir.file(db));
        evaluate_node(db, hir, file_scope).value(db)
      }
      Some(Either::Right(obj)) => Some(obj),
      // Missing fields: check schema for computed/default, then null
      None => {
        if let Some(schema_type) = self.schema(db).as_td_schema_type()
          && let Some(prop_desc) = schema_type.fields(db).get(key)
        {
          if let Some(ref computed_enum) = prop_desc.computed_fn
            && let Some(func_obj) = computed_enum.as_td_func_obj()
            && let Ok(res_val) = func_obj.call(db, self.project(db), None, vec![(*self).into()])
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
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_type(db).source_path(db)
  }
  fn index(&self, db: &'db TypedownDatabase, key: &TdObjectEnum<'db>) -> Option<TdObjectEnum<'db>> {
    let this: TdObjectEnum = (*self).into();
    self
      .lookup_method(db, PROTOCOL_INDEX)?
      .call(db, self.project(db), Some(this), vec![key.clone()])
      .ok()
  }
  fn call(
    &self,
    db: &'db TypedownDatabase,
    project: Project,
    this: Option<TdObjectEnum<'db>>,
    args: Vec<TdObjectEnum<'db>>,
  ) -> Result<TdObjectEnum<'db>, Vec<Diagnostic>> {
    let Some(func) = self.lookup_method(db, PROTOCOL_CALL) else {
      return Err(vec![]);
    };
    func.call(db, project, this, args)
  }
}
