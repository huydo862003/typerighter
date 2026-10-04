//! Derived queries for constructing builtin type singletons

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use std::collections::{BTreeMap, HashSet};

use crate::db::types::typecheck::make_nullable;
use crate::db::types::{
  BuiltinSchemaKind, FuncSignature, LazyTyp, LitValue, Symbol, SymbolKind, TdBlobTyp, TdBoolObj,
  TdBoolTyp, TdDateTimeTyp, TdDateTyp, TdDictTyp, TdFuncTyp, TdIconTyp, TdListTyp, TdLitTyp,
  TdMathTyp, TdNeverTyp, TdNullObj, TdNullTyp, TdNumberTyp, TdObjTyp, TdProductTyp,
  TdSchemaMetaTyp, TdStringTyp, TdSumTyp, TdTimeTyp, TdTypEnum, TdTypTyp,
};
use typedown_incremental::{QueryDatabase, StableCompare};

#[query_derived]
pub fn get_typ_typ<'db>(db: &'db TypedownDatabase) -> TdTypTyp<'db> {
  TdTypTyp::new(db)
}

#[query_derived]
pub fn get_obj_typ<'db>(db: &'db TypedownDatabase) -> TdObjTyp<'db> {
  TdObjTyp::new(db)
}

#[query_derived]
pub fn get_bool_typ<'db>(db: &'db TypedownDatabase) -> TdBoolTyp<'db> {
  TdBoolTyp::new(db)
}

#[query_derived]
pub fn get_string_typ<'db>(db: &'db TypedownDatabase) -> TdStringTyp<'db> {
  TdStringTyp::new(db)
}

#[query_derived]
pub fn get_number_typ<'db>(db: &'db TypedownDatabase) -> TdNumberTyp<'db> {
  TdNumberTyp::new(db)
}

#[query_derived]
pub fn get_list_typ<'db>(db: &'db TypedownDatabase) -> TdListTyp<'db> {
  TdListTyp::new(db, None)
}

#[query_derived]
pub fn get_dict_typ<'db>(db: &'db TypedownDatabase) -> TdDictTyp<'db> {
  TdDictTyp::new(db, None, None)
}

#[query_derived]
pub fn get_math_typ<'db>(db: &'db TypedownDatabase) -> TdMathTyp<'db> {
  TdMathTyp::new(db)
}

#[query_derived]
pub fn get_datetime_typ<'db>(db: &'db TypedownDatabase) -> TdDateTimeTyp<'db> {
  TdDateTimeTyp::new(db)
}

#[query_derived]
pub fn get_date_typ<'db>(db: &'db TypedownDatabase) -> TdDateTyp<'db> {
  TdDateTyp::new(db)
}

#[query_derived]
pub fn get_time_typ<'db>(db: &'db TypedownDatabase) -> TdTimeTyp<'db> {
  TdTimeTyp::new(db)
}

#[query_derived]
pub fn get_true<'db>(db: &'db TypedownDatabase) -> TdBoolObj<'db> {
  TdBoolObj::new(db, true)
}

#[query_derived]
pub fn get_false<'db>(db: &'db TypedownDatabase) -> TdBoolObj<'db> {
  TdBoolObj::new(db, false)
}

// Schema metatype: the type of all schema types
#[query_derived]
pub fn get_schema_meta_typ<'db>(db: &'db TypedownDatabase) -> TdSchemaMetaTyp<'db> {
  TdSchemaMetaTyp::new(db)
}

pub fn get_typ_typ_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::TypTyp),
    "type".to_string(),
    "@builtin::type".to_string(),
  )
}

pub fn get_obj_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::TypTyp),
    "Object".to_string(),
    "@builtin::Object".to_string(),
  )
}

pub fn get_schema_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Schema),
    "schema".to_string(),
    "@builtin::schema".to_string(),
  )
}

pub fn get_string_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::String),
    "string".to_string(),
    "@builtin::string".to_string(),
  )
}

pub fn get_number_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Number),
    "number".to_string(),
    "@builtin::number".to_string(),
  )
}

pub fn get_bool_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Bool),
    "boolean".to_string(),
    "@builtin::boolean".to_string(),
  )
}

pub fn get_date_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Date),
    "date".to_string(),
    "@builtin::date".to_string(),
  )
}

pub fn get_datetime_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::DateTime),
    "datetime".to_string(),
    "@builtin::datetime".to_string(),
  )
}

pub fn get_time_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Time),
    "time".to_string(),
    "@builtin::time".to_string(),
  )
}

pub fn get_math_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Math),
    "math".to_string(),
    "@builtin::math".to_string(),
  )
}

pub fn get_list_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::List),
    "list".to_string(),
    "@builtin::list".to_string(),
  )
}

pub fn get_dict_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::Dict),
    "dict".to_string(),
    "@builtin::dict".to_string(),
  )
}

#[query_derived]
pub fn get_icon_typ<'db>(db: &'db TypedownDatabase) -> TdIconTyp<'db> {
  TdIconTyp::new(db)
}

#[query_derived]
pub fn get_blob_typ<'db>(db: &'db TypedownDatabase) -> TdBlobTyp<'db> {
  TdBlobTyp::new(db)
}

#[query_derived]
pub fn get_null_typ<'db>(db: &'db TypedownDatabase) -> TdNullTyp<'db> {
  TdNullTyp::new(db)
}

#[query_derived]
pub fn get_never_typ<'db>(db: &'db TypedownDatabase) -> TdNeverTyp<'db> {
  TdNeverTyp::new(db)
}

#[query_derived]
pub fn get_lit_typ<'db>(db: &'db TypedownDatabase, value: LitValue) -> TdLitTyp<'db> {
  TdLitTyp::new(db, value)
}

#[query_derived]
pub fn get_null_obj<'db>(db: &'db TypedownDatabase) -> TdNullObj<'db> {
  TdNullObj::new(db)
}

#[query_derived]
pub fn get_func_typ<'db>(
  db: &'db TypedownDatabase,
  signature: FuncSignature<'db>,
) -> TdFuncTyp<'db> {
  TdFuncTyp::new(db, signature)
}

#[query_derived]
pub fn get_sum_typ<'db>(db: &'db TypedownDatabase, members: Vec<LazyTyp<'db>>) -> TdSumTyp<'db> {
  fn flatten_sum_members<'db>(
    db: &'db TypedownDatabase,
    members: &[LazyTyp<'db>],
  ) -> Vec<LazyTyp<'db>> {
    fn recurse<'db>(
      db: &'db TypedownDatabase,
      members: &[LazyTyp<'db>],
      visited: &mut HashSet<TdSumTyp<'db>>,
      out: &mut Vec<LazyTyp<'db>>,
    ) {
      for member in members {
        if let Some(TdTypEnum::TdSumTyp(sum)) = member.as_eager() {
          if visited.insert(sum) {
            let sum_members: Vec<_> = sum.members(db).into_iter().collect();
            recurse(db, &sum_members, visited, out);
          }
        } else {
          out.push(member.clone());
        }
      }
    }

    let mut out = Vec::new();
    let mut visited = HashSet::new();
    recurse(db, members, &mut visited, &mut out);
    out
  }

  let flat_members = {
    let mut members = flatten_sum_members(db, &members);
    members.sort_by(|a, b| a.stable_cmp(db, b));
    members.dedup();
    members
  };

  if flat_members != members {
    get_sum_typ(db, flat_members)
  } else {
    let members_set: HashSet<LazyTyp> = flat_members.into_iter().collect();
    TdSumTyp::new(db, members_set)
  }
}

// Built-in config type for typedown.yaml structural validation
#[query_derived]
pub fn get_config_typ<'db>(db: &'db TypedownDatabase) -> TdProductTyp<'db> {
  let string_typ: TdTypEnum = get_string_typ(db).into();
  let optional_string = make_nullable(db, string_typ.clone());

  let vault_fields = BTreeMap::from([(
    "root_dir".to_string(),
    LazyTyp::eager(optional_string.clone()),
  )]);
  let vault_typ: TdTypEnum = TdProductTyp::new(db, Some("vault".to_string()), vault_fields).into();

  let icon_typ: TdTypEnum = get_icon_typ(db).into();
  let optional_icon = make_nullable(db, icon_typ);

  let nav_item_fields = BTreeMap::from([
    ("title".to_string(), LazyTyp::eager(string_typ.clone())),
    ("link".to_string(), LazyTyp::eager(string_typ.clone())),
    ("icon".to_string(), LazyTyp::eager(optional_icon)),
  ]);
  let nav_item_typ: TdTypEnum = TdProductTyp::new(db, None, nav_item_fields).into();
  let nav_list_typ: TdTypEnum = TdListTyp::new(db, Some(LazyTyp::eager(nav_item_typ))).into();
  let optional_nav = make_nullable(db, nav_list_typ);

  let site_fields = BTreeMap::from([
    ("title".to_string(), LazyTyp::eager(optional_string.clone())),
    (
      "description".to_string(),
      LazyTyp::eager(optional_string.clone()),
    ),
    (
      "origin".to_string(),
      LazyTyp::eager(optional_string.clone()),
    ),
    ("lang".to_string(), LazyTyp::eager(optional_string.clone())),
    (
      "base_path".to_string(),
      LazyTyp::eager(optional_string.clone()),
    ),
    (
      "author".to_string(),
      LazyTyp::eager(optional_string.clone()),
    ),
    (
      "license".to_string(),
      LazyTyp::eager(optional_string.clone()),
    ),
    (
      "public_dir".to_string(),
      LazyTyp::eager(optional_string.clone()),
    ),
    ("nav".to_string(), LazyTyp::eager(optional_nav)),
  ]);
  let site_typ: TdTypEnum = TdProductTyp::new(db, Some("site".to_string()), site_fields).into();

  let config_fields = BTreeMap::from([
    ("version".to_string(), LazyTyp::eager(string_typ)),
    (
      "vault".to_string(),
      LazyTyp::eager(make_nullable(db, vault_typ)),
    ),
    ("repo".to_string(), LazyTyp::eager(optional_string)),
    (
      "site".to_string(),
      LazyTyp::eager(make_nullable(db, site_typ)),
    ),
  ]);

  TdProductTyp::new(db, Some("config".to_string()), config_fields)
}

#[cfg(test)]
mod tests {
  use super::{get_bool_typ, get_sum_typ};
  use crate::db::types::derived::obj_system::TdStaticTyp;
  use crate::db::types::typecheck::is_nullable;
  use crate::db::types::typecheck::validate_typ_params;
  use crate::db::types::{LazyTyp, TdTypEnum, TypParams, TypVariable};
  use crate::syntax::diagnostic::Diagnostic;

  use crate::db::{
    QueryStorage, TypedownDatabase,
    derived::get_builtin_typs::{
      get_config_typ, get_dict_typ, get_list_typ, get_number_typ, get_string_typ,
    },
  };

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn instantiate_list_with_correct_arity() {
    let db = make_db();
    let list_typ = TdTypEnum::from(get_list_typ(&db));
    let string_typ = TdTypEnum::from(get_string_typ(&db));

    let result_typ = list_typ.instantiate(&db, vec![LazyTyp::eager(string_typ.clone())]);

    assert!(
      result_typ.diagnostics(&db).is_empty(),
      "expected no diagnostics"
    );
    let _expected_typ = TdTypEnum::from(get_string_typ(&db));
    let instantiated = result_typ.typ(&db);
    // The result should be a TdListType with elem = str
    assert!(
      instantiated.arity(&db) == 0,
      "instantiated list should have arity 0"
    );
  }

  #[test]
  fn instantiate_record_with_correct_arity() {
    let db = make_db();
    let record_typ = TdTypEnum::from(get_dict_typ(&db));
    let string_typ = TdTypEnum::from(get_string_typ(&db));
    let number_typ = TdTypEnum::from(get_number_typ(&db));

    let result_typ = record_typ.instantiate(
      &db,
      vec![LazyTyp::eager(string_typ), LazyTyp::eager(number_typ)],
    );

    assert!(
      result_typ.diagnostics(&db).is_empty(),
      "expected no diagnostics"
    );
    assert!(
      result_typ.typ(&db).arity(&db) == 0,
      "instantiated record should have arity 0"
    );
  }

  #[test]
  fn instantiate_list_wrong_arity_produces_diagnostic() {
    let db = make_db();
    let list_typ = TdTypEnum::from(get_list_typ(&db));

    let result_typ = list_typ.instantiate(&db, vec![]);

    let diagnostics = result_typ.diagnostics(&db);
    assert_eq!(diagnostics.len(), 1);
    assert!(
      matches!(
        diagnostics[0],
        Diagnostic::WrongTypArgCount {
          expected: 1,
          got: 0
        }
      ),
      "expected WrongTypeArgCount diagnostic"
    );
  }

  #[test]
  fn instantiate_record_wrong_arity_produces_diagnostic() {
    let db = make_db();
    let record_typ = TdTypEnum::from(get_dict_typ(&db));
    let string_typ = TdTypEnum::from(get_string_typ(&db));

    // Only 1 arg, record needs 2
    let result_typ = record_typ.instantiate(&db, vec![LazyTyp::eager(string_typ)]);

    let diagnostics = result_typ.diagnostics(&db);
    assert_eq!(diagnostics.len(), 1);
    assert!(
      matches!(
        diagnostics[0],
        Diagnostic::WrongTypArgCount {
          expected: 2,
          got: 1
        }
      ),
      "expected WrongTypeArgCount diagnostic"
    );
  }

  #[test]
  fn instantiate_arity0_typ_with_no_args() {
    let db = make_db();
    let string_typ = TdTypEnum::from(get_string_typ(&db));
    let expected_typ = string_typ.clone();

    let result = string_typ.instantiate(&db, vec![]);

    assert!(
      result.diagnostics(&db).is_empty(),
      "expected no diagnostics"
    );
    assert!(
      result.typ(&db) == expected_typ,
      "arity-0 type instantiated with no args should return itself"
    );
  }

  #[test]
  fn instantiate_arity0_typ_with_extra_args_produces_diagnostic() {
    let db = make_db();
    let string_typ = TdTypEnum::from(get_string_typ(&db));
    let number_typ = TdTypEnum::from(get_number_typ(&db));

    let result = string_typ.instantiate(&db, vec![LazyTyp::eager(number_typ)]);

    let diagnostics = result.diagnostics(&db);
    assert_eq!(diagnostics.len(), 1);
    assert!(
      matches!(
        diagnostics[0],
        Diagnostic::WrongTypArgCount {
          expected: 0,
          got: 1
        }
      ),
      "expected WrongTypeArgCount diagnostic"
    );
  }

  #[test]
  fn instantiate_bounded_type_violating_bound_produces_diagnostic() {
    let db = make_db();
    let number_typ = TdTypEnum::from(get_number_typ(&db));
    let string_typ = TdTypEnum::from(get_string_typ(&db));

    let params = TypParams::new(
      &db,
      vec![TypVariable::get(&db, Some(LazyTyp::eager(number_typ)))],
      vec![],
    );
    let diagnostics = validate_typ_params(&db, Some(&params), &[LazyTyp::eager(string_typ)]);
    assert_eq!(diagnostics.len(), 1);
    assert!(
      matches!(
        diagnostics[0],
        Diagnostic::TypArgBoundViolation { index: 0, .. }
      ),
      "expected TypeArgBoundViolation diagnostic"
    );
  }

  #[test]
  fn sum_typ_flattening() {
    let db = make_db();
    let string_typ = LazyTyp::eager(get_string_typ(&db).into());
    let number_typ = LazyTyp::eager(get_number_typ(&db).into());
    let bool_typ = LazyTyp::eager(get_bool_typ(&db).into());

    let inner_sum_typ = get_sum_typ(&db, vec![number_typ.clone(), bool_typ.clone()]);
    let outer_sum_typ = get_sum_typ(
      &db,
      vec![string_typ.clone(), LazyTyp::eager(inner_sum_typ.into())],
    );

    let members = outer_sum_typ.members(&db);
    assert_eq!(members.len(), 3);
    assert!(members.contains(&string_typ));
    assert!(members.contains(&number_typ));
    assert!(members.contains(&bool_typ));
  }

  #[test]
  fn config_typ_has_expected_fields() {
    let db = make_db();
    let config = get_config_typ(&db);
    let fields = config.fields(&db);

    assert!(fields.contains_key("version"), "missing version");
    assert!(fields.contains_key("vault"), "missing vault");
    assert!(fields.contains_key("repo"), "missing repo");
    assert!(fields.contains_key("site"), "missing site");
    assert_eq!(fields.len(), 4, "unexpected field count");
  }

  #[test]
  fn config_version_is_required() {
    let db = make_db();
    let config_typ = get_config_typ(&db);
    let fields = config_typ.fields(&db);

    let version_typ = fields["version"].resolve(&db).unwrap();
    assert!(
      !is_nullable(&db, &version_typ),
      "version should not be nullable"
    );
  }

  #[test]
  fn config_site_has_expected_fields() {
    let db = make_db();
    let config = get_config_typ(&db);
    let fields = config.fields(&db);

    let site_typ = fields["site"].resolve(&db).unwrap();
    // site is nullable, unwrap the sum to get the product
    let site_product = if let TdTypEnum::TdSumTyp(sum) = &site_typ {
      sum
        .members(&db)
        .iter()
        .find_map(|member| {
          let resolved = member.resolve(&db)?;
          resolved.as_td_product_typ().cloned()
        })
        .expect("site sum should contain a product type")
    } else {
      panic!("site should be a sum type (nullable)")
    };

    let site_fields = site_product.fields(&db);
    assert!(site_fields.contains_key("title"), "missing title");
    assert!(
      site_fields.contains_key("description"),
      "missing description"
    );
    assert!(site_fields.contains_key("base_path"), "missing base_path");
    assert!(site_fields.contains_key("author"), "missing author");
    assert!(site_fields.contains_key("license"), "missing license");
    assert!(site_fields.contains_key("public_dir"), "missing public_dir");
    assert!(site_fields.contains_key("nav"), "missing nav");
  }
}
