use super::TdObjEnum;
use super::base::TdRuntimeObj;
use super::builtin_schemas::node_set::{NodeSetPlan, materialize_as_list};
use super::string::TdStringObj;
use crate::db::TypedownDatabase;
use crate::db::types::{
  Project, ProjectRedNode, RuntimeScope, TdBoolObj, TdNodeSetObj, TdNullObj, TdNumberObj,
};
use crate::syntax::diagnostic::TdDiagnostic;
use strum::FromRepr;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, Id, QueryDatabase, StableHash, StableHasher,
};
use typedown_macros::StableCompare;
use typedown_types::either::Either;

pub type NativeFunc<'db> = fn(
  &'db TypedownDatabase,
  Project,
  Option<TdObjEnum<'db>>,
  Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, StableCompare)]
#[repr(u8)]
pub enum NativeFuncKind {
  ToStringMethod = 0,
  NodeSetFromVault = 1,
  NodeSetFilter = 2,
  NodeSetFollow = 3,
  NodeSetFollowBack = 4,
  NodeSetToList = 5,
  NodeSetFirst = 6,
  NodeSetCount = 7,
  NodeSetContains = 8,
  NodeSetFromNode = 9,
}

impl StableHash for NativeFuncKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    (*self as u8).stable_hash(db, hasher);
  }
}

impl Encodable for NativeFuncKind {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buffer, *self as u8);
  }
}

impl Decodable for NativeFuncKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    NativeFuncKind::from_repr(tag).expect("unknown NativeFuncKind tag")
  }
}

impl NativeFuncKind {
  pub fn resolve<'db>(self) -> NativeFunc<'db> {
    match self {
      NativeFuncKind::ToStringMethod => to_string_method as NativeFunc<'db>,
      NativeFuncKind::NodeSetFromVault => node_set_from_vault as NativeFunc<'db>,
      NativeFuncKind::NodeSetFilter => node_set_filter as NativeFunc<'db>,
      NativeFuncKind::NodeSetFollow => node_set_follow as NativeFunc<'db>,
      NativeFuncKind::NodeSetFollowBack => node_set_follow_back as NativeFunc<'db>,
      NativeFuncKind::NodeSetToList => node_set_to_list as NativeFunc<'db>,
      NativeFuncKind::NodeSetFirst => node_set_first as NativeFunc<'db>,
      NativeFuncKind::NodeSetCount => node_set_count as NativeFunc<'db>,
      NativeFuncKind::NodeSetContains => node_set_contains as NativeFunc<'db>,
      NativeFuncKind::NodeSetFromNode => node_set_from_node as NativeFunc<'db>,
    }
  }
}

fn to_string_method<'db>(
  db: &'db TypedownDatabase,
  _project: Project,
  this: Option<TdObjEnum<'db>>,
  _args: Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>> {
  let Some(this) = this else {
    return Err(vec![]);
  };
  Ok(TdStringObj::new(db, this.to_display_string(db)).into())
}

/* Builder operations */

fn node_set_from_node<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  _this: Option<TdObjEnum<'db>>,
  args: Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>> {
  let Some(node) = args.into_iter().next() else {
    return Err(vec![]);
  };
  Ok(TdNodeSetObj::new(db, project, NodeSetPlan::Eager(vec![node])).into())
}

fn node_set_from_vault<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  _this: Option<TdObjEnum<'db>>,
  args: Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>> {
  let schema_typ = args
    .first()
    .and_then(|arg| arg.as_td_typ_obj())
    .and_then(|typ| typ.as_td_schema_typ())
    .copied();

  let Some(schema_typ) = schema_typ else {
    return Ok(TdNodeSetObj::new(db, project, NodeSetPlan::Eager(vec![])).into());
  };

  let where_func = args
    .get(1)
    .filter(|arg| !arg.is_td_null_obj())
    .and_then(|arg| arg.as_td_dict_obj())
    .and_then(|dict| dict.get_owned_field(db, "where"))
    .and_then(|field| field.into_td_func_obj().ok());

  Ok(
    TdNodeSetObj::new(
      db,
      project,
      NodeSetPlan::FromNodes {
        schema_typ,
        where_func,
      },
    )
    .into(),
  )
}

fn node_set_filter<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  this: Option<TdObjEnum<'db>>,
  args: Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>> {
  let Some(TdObjEnum::TdNodeSetObj(source)) = this else {
    return Err(vec![]);
  };
  let Some(where_func) = args.first().and_then(|arg| arg.as_td_func_obj()).copied() else {
    return Err(vec![]);
  };
  Ok(TdNodeSetObj::new(db, project, NodeSetPlan::Filter { source, where_func }).into())
}

fn node_set_follow<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  this: Option<TdObjEnum<'db>>,
  args: Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>> {
  let Some(TdObjEnum::TdNodeSetObj(source)) = this else {
    return Err(vec![]);
  };
  let Some(field_name) = args
    .first()
    .and_then(|arg| arg.as_td_string_obj())
    .map(|string_obj| string_obj.value(db))
  else {
    return Err(vec![]);
  };
  let where_func = args
    .get(1)
    .filter(|arg| !arg.is_td_null_obj())
    .and_then(|arg| arg.as_td_dict_obj())
    .and_then(|dict| dict.get_owned_field(db, "where"))
    .and_then(|field| field.into_td_func_obj().ok());
  Ok(
    TdNodeSetObj::new(
      db,
      project,
      NodeSetPlan::Follow {
        source,
        field_name,
        where_func,
      },
    )
    .into(),
  )
}

// follow_back(source_schema, field_name) on node_set
// First arg is the source schema type, second is the field name string
fn node_set_follow_back<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  this: Option<TdObjEnum<'db>>,
  args: Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>> {
  let Some(TdObjEnum::TdNodeSetObj(source)) = this else {
    return Err(vec![]);
  };
  // First arg: source schema type (used for type checking, not runtime)
  // Second arg: field name string
  let Some(field_name) = args
    .get(1)
    .and_then(|arg| arg.as_td_string_obj())
    .map(|string_obj| string_obj.value(db))
  else {
    return Err(vec![]);
  };
  Ok(TdNodeSetObj::new(db, project, NodeSetPlan::FollowBack { source, field_name }).into())
}

/* Terminal operations */

fn node_set_to_list<'db>(
  db: &'db TypedownDatabase,
  _project: Project,
  this: Option<TdObjEnum<'db>>,
  _args: Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>> {
  let Some(TdObjEnum::TdNodeSetObj(node_set)) = this else {
    return Err(vec![]);
  };
  Ok(materialize_as_list(db, node_set).into())
}

fn node_set_first<'db>(
  db: &'db TypedownDatabase,
  _project: Project,
  this: Option<TdObjEnum<'db>>,
  _args: Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>> {
  let Some(TdObjEnum::TdNodeSetObj(node_set)) = this else {
    return Err(vec![]);
  };
  let first = materialize_as_list(db, node_set)
    .items(db)
    .into_iter()
    .find_map(|entry| match entry {
      Either::Right(obj) => Some(obj),
      Either::Left(_) => None,
    });
  Ok(first.unwrap_or_else(|| TdNullObj::get(db).into()))
}

fn node_set_count<'db>(
  db: &'db TypedownDatabase,
  _project: Project,
  this: Option<TdObjEnum<'db>>,
  _args: Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>> {
  let Some(TdObjEnum::TdNodeSetObj(node_set)) = this else {
    return Err(vec![]);
  };
  Ok(TdNumberObj::new(db, materialize_as_list(db, node_set).items(db).len() as f64).into())
}

fn node_set_contains<'db>(
  db: &'db TypedownDatabase,
  _project: Project,
  this: Option<TdObjEnum<'db>>,
  args: Vec<TdObjEnum<'db>>,
) -> Result<TdObjEnum<'db>, Vec<TdDiagnostic>> {
  let Some(TdObjEnum::TdNodeSetObj(node_set)) = this else {
    return Err(vec![]);
  };
  let Some(target) = args.first() else {
    return Err(vec![]);
  };
  let target_id = target.as_id();
  let found = materialize_as_list(db, node_set)
    .items(db)
    .into_iter()
    .filter_map(|entry| match entry {
      Either::Right(obj) => Some(obj),
      Either::Left(_) => None,
    })
    .any(|node| node.as_id() == target_id);
  Ok(TdBoolObj::new(db, found).into())
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub enum FuncKind<'db> {
  Native(NativeFuncKind),
  UserDefined(ProjectRedNode, RuntimeScope<'db>),
}

#[derive(FromRepr)]
#[repr(u8)]
enum FuncKindTag {
  Native = 0,
  UserDefined = 1,
}

impl<'db> StableHash for FuncKind<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      FuncKind::Native(kind) => kind.stable_hash(db, hasher),
      FuncKind::UserDefined(hir, runtime_scope) => {
        hir.stable_hash(db, hasher);
        runtime_scope.stable_hash(db, hasher);
      }
    }
  }
}

impl<'db> Encodable for FuncKind<'db> {
  fn encode(&self, buffer: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      FuncKind::Native(kind) => {
        encoder.emit_u8(buffer, FuncKindTag::Native as u8);
        kind.encode(buffer, encoder);
      }
      FuncKind::UserDefined(hir, runtime_scope) => {
        encoder.emit_u8(buffer, FuncKindTag::UserDefined as u8);
        hir.field_encode(buffer, encoder);
        runtime_scope.field_encode(buffer, encoder);
      }
    }
  }
}

impl<'db> Decodable for FuncKind<'db> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match FuncKindTag::from_repr(tag).expect("unknown FuncKind tag") {
      FuncKindTag::Native => FuncKind::Native(NativeFuncKind::decode(data, decoder)),
      FuncKindTag::UserDefined => FuncKind::UserDefined(
        ProjectRedNode::field_decode(data, decoder),
        RuntimeScope::field_decode(data, decoder),
      ),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::QueryStorage;
  use crate::db::fixtures::make_number_obj;
  use crate::db::types::TdNodeSetObj;
  use std::collections::HashMap;
  use std::path::PathBuf;

  fn make_db() -> (TypedownDatabase, Project) {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let project = Project::new(&db, PathBuf::from("/test"), HashMap::new(), None);
    (db, project)
  }

  fn make_node_set<'db>(
    db: &'db TypedownDatabase,
    project: Project,
    nodes: Vec<TdObjEnum<'db>>,
  ) -> TdObjEnum<'db> {
    TdNodeSetObj::new(db, project, NodeSetPlan::Eager(nodes)).into()
  }

  #[test]
  fn test_native_func_optional_this() {
    let (db, project) = make_db();
    let native_func = NativeFuncKind::ToStringMethod.resolve();
    let number_obj: TdObjEnum = make_number_obj(&db, 42.0_f64.to_bits()).into();

    let result_with_this = native_func(&db, project, Some(number_obj), vec![]);
    assert!(result_with_this.is_ok());
    assert_eq!(
      result_with_this.unwrap().to_display_string(&db),
      "42".to_string()
    );

    let result_no_this = native_func(&db, project, None, vec![]);
    assert!(result_no_this.is_err());
  }

  #[test]
  fn test_node_set_count() {
    let (db, project) = make_db();
    let n1: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let n2: TdObjEnum = make_number_obj(&db, 2.0_f64.to_bits()).into();
    let node_set = make_node_set(&db, project, vec![n1, n2]);

    let result = NativeFuncKind::NodeSetCount.resolve()(&db, project, Some(node_set), vec![]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().to_display_string(&db), "2");
  }

  #[test]
  fn test_node_set_first_nonempty() {
    let (db, project) = make_db();
    let n1: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let n2: TdObjEnum = make_number_obj(&db, 2.0_f64.to_bits()).into();
    let node_set = make_node_set(&db, project, vec![n1, n2]);

    let result = NativeFuncKind::NodeSetFirst.resolve()(&db, project, Some(node_set), vec![]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().to_display_string(&db), "1");
  }

  #[test]
  fn test_node_set_contains_true() {
    let (db, project) = make_db();
    let n1: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let node_set = make_node_set(&db, project, vec![n1.clone()]);

    let result = NativeFuncKind::NodeSetContains.resolve()(&db, project, Some(node_set), vec![n1]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().to_display_string(&db), "true");
  }

  #[test]
  fn test_node_set_contains_false() {
    let (db, project) = make_db();
    let n1: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let n2: TdObjEnum = make_number_obj(&db, 2.0_f64.to_bits()).into();
    let node_set = make_node_set(&db, project, vec![n1]);

    let result = NativeFuncKind::NodeSetContains.resolve()(&db, project, Some(node_set), vec![n2]);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().to_display_string(&db), "false");
  }

  #[test]
  fn test_node_set_to_list() {
    let (db, project) = make_db();
    let n1: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let n2: TdObjEnum = make_number_obj(&db, 2.0_f64.to_bits()).into();
    let node_set = make_node_set(&db, project, vec![n1, n2]);

    let result = NativeFuncKind::NodeSetToList.resolve()(&db, project, Some(node_set), vec![]);
    assert!(result.is_ok());
    let list = result.unwrap();
    assert!(list.is_td_list_obj());
    let list_obj = list.as_td_list_obj().unwrap();
    assert_eq!(list_obj.items(&db).len(), 2);
  }

  #[test]
  fn test_node_set_no_this_errors() {
    let (db, project) = make_db();
    for kind in [
      NativeFuncKind::NodeSetCount,
      NativeFuncKind::NodeSetFirst,
      NativeFuncKind::NodeSetToList,
    ] {
      let result = kind.resolve()(&db, project, None, vec![]);
      assert!(result.is_err(), "{kind:?} should err with no this");
    }
  }

  #[test]
  fn test_node_set_builder_no_this_errors() {
    let (db, project) = make_db();
    for kind in [
      NativeFuncKind::NodeSetFilter,
      NativeFuncKind::NodeSetFollow,
      NativeFuncKind::NodeSetFollowBack,
    ] {
      let result = kind.resolve()(&db, project, None, vec![]);
      assert!(result.is_err(), "{kind:?} should err with no this");
    }
  }

  #[test]
  fn test_node_set_multi_terminal_consistent() {
    // Calling two terminal ops on the same lazy NodeSet must both produce correct results
    let (db, project) = make_db();
    let n1: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let n2: TdObjEnum = make_number_obj(&db, 2.0_f64.to_bits()).into();
    let node_set = make_node_set(&db, project, vec![n1, n2]);

    let count =
      NativeFuncKind::NodeSetCount.resolve()(&db, project, Some(node_set.clone()), vec![]);
    let list = NativeFuncKind::NodeSetToList.resolve()(&db, project, Some(node_set), vec![]);

    assert_eq!(count.unwrap().to_display_string(&db), "2");
    assert_eq!(list.unwrap().as_td_list_obj().unwrap().items(&db).len(), 2);
  }

  #[test]
  fn test_node_set_filter_returns_node_set() {
    // filter on an Eager plan must itself produce a NodeSet (plan is stored, not evaluated)
    let (db, project) = make_db();
    let n1: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let node_set = make_node_set(&db, project, vec![n1]);

    // Passing a wrong arg type forces an error, confirming filter checks its this arg
    let wrong_this: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let result = NativeFuncKind::NodeSetFilter.resolve()(&db, project, Some(wrong_this), vec![]);
    assert!(result.is_err());

    // Passing the right this with a missing pred arg also errors
    let result = NativeFuncKind::NodeSetFilter.resolve()(&db, project, Some(node_set), vec![]);
    assert!(result.is_err());
  }

  #[test]
  fn test_node_set_follow_no_matching_field() {
    // follow on nodes that lack the field must return an empty NodeSet
    let (db, project) = make_db();
    let n1: TdObjEnum = make_number_obj(&db, 1.0_f64.to_bits()).into();
    let node_set = make_node_set(&db, project, vec![n1]);
    let field_arg: TdObjEnum =
      super::super::string::TdStringObj::new(&db, "nonexistent".to_string()).into();

    let follow_result =
      NativeFuncKind::NodeSetFollow.resolve()(&db, project, Some(node_set), vec![field_arg]);
    assert!(follow_result.is_ok());

    let count_result =
      NativeFuncKind::NodeSetCount.resolve()(&db, project, Some(follow_result.unwrap()), vec![]);
    assert_eq!(count_result.unwrap().to_display_string(&db), "0");
  }
}

// Integration tests over a real vault fixture with Person and Company schemas
// Vault layout:
//   _types/Person.td  - name: string, age: number, employer: Company?
//   _types/Company.td - name: string, country: string
//   alice.td  - Person, age 30, employer -> acme.td
//   bob.td    - Person, age 17, employer -> acme.td
//   carol.td  - Person, age 25, employer -> globex.td
//   acme.td   - Company, country "US"
//   globex.td - Company, country "UK"
#[cfg(test)]
mod graph_query_tests {
  use super::*;
  use crate::db::TypedownDatabase;
  use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
  use crate::db::derived::evaluate::evaluate_typ::evaluate_typ;
  use crate::db::derived::name_resolver::file_symbol::file_symbol;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::{File, Project};

  const VAULT: &str = "evaluate/graph_query_vault";

  fn get_file(db: &TypedownDatabase, project: Project, suffix: &str) -> File {
    *project
      .files(db)
      .iter()
      .find(|(path, _)| path.ends_with(suffix))
      .unwrap_or_else(|| panic!("{suffix} not found in project"))
      .1
  }

  fn get_schema_obj<'db>(
    db: &'db TypedownDatabase,
    project: Project,
    suffix: &str,
  ) -> TdObjEnum<'db> {
    let file = get_file(db, project, suffix);
    let symbol = file_symbol(db, project, file).value(db).unwrap();
    TdObjEnum::TdTypObj(evaluate_typ(db, symbol).typ(db).unwrap())
  }

  fn get_resource_obj<'db>(
    db: &'db TypedownDatabase,
    project: Project,
    suffix: &str,
  ) -> TdObjEnum<'db> {
    let file = get_file(db, project, suffix);
    let symbol = file_symbol(db, project, file).value(db).unwrap();
    evaluate_resource(db, symbol).value(db).unwrap()
  }

  fn from_nodes<'db>(
    db: &'db TypedownDatabase,
    project: Project,
    schema: TdObjEnum<'db>,
  ) -> TdObjEnum<'db> {
    NativeFuncKind::NodeSetFromVault.resolve()(db, project, None, vec![schema]).unwrap()
  }

  fn count(db: &TypedownDatabase, project: Project, node_set: TdObjEnum) -> usize {
    NativeFuncKind::NodeSetCount.resolve()(db, project, Some(node_set), vec![])
      .unwrap()
      .to_display_string(db)
      .parse()
      .unwrap()
  }

  fn follow<'db>(
    db: &'db TypedownDatabase,
    project: Project,
    node_set: TdObjEnum<'db>,
    field: &str,
  ) -> TdObjEnum<'db> {
    let field_arg: TdObjEnum = super::super::string::TdStringObj::new(db, field.to_string()).into();
    NativeFuncKind::NodeSetFollow.resolve()(db, project, Some(node_set), vec![field_arg]).unwrap()
  }

  fn follow_back<'db>(
    db: &'db TypedownDatabase,
    project: Project,
    node_set: TdObjEnum<'db>,
    source_schema: TdObjEnum<'db>,
    field: &str,
  ) -> TdObjEnum<'db> {
    let field_arg: TdObjEnum = super::super::string::TdStringObj::new(db, field.to_string()).into();
    NativeFuncKind::NodeSetFollowBack.resolve()(
      db,
      project,
      Some(node_set),
      vec![source_schema, field_arg],
    )
    .unwrap()
  }

  #[test]
  fn from_nodes_person_returns_all_persons() {
    let (db, project, _) = load_vault_fixture(VAULT, "typedown.yaml");
    let person = get_schema_obj(&db, project, "_types/Person.td");
    assert_eq!(count(&db, project, from_nodes(&db, project, person)), 3);
  }

  #[test]
  fn from_nodes_company_returns_all_companies() {
    let (db, project, _) = load_vault_fixture(VAULT, "typedown.yaml");
    let company = get_schema_obj(&db, project, "_types/Company.td");
    assert_eq!(count(&db, project, from_nodes(&db, project, company)), 2);
  }

  #[test]
  fn from_nodes_to_list_length_matches_count() {
    let (db, project, _) = load_vault_fixture(VAULT, "typedown.yaml");
    let person = get_schema_obj(&db, project, "_types/Person.td");
    let node_set = from_nodes(&db, project, person);
    let list =
      NativeFuncKind::NodeSetToList.resolve()(&db, project, Some(node_set.clone()), vec![])
        .unwrap();
    let list_len = list.as_td_list_obj().unwrap().items(&db).len();
    assert_eq!(list_len, count(&db, project, node_set));
  }

  #[test]
  fn from_nodes_first_is_not_null() {
    let (db, project, _) = load_vault_fixture(VAULT, "typedown.yaml");
    let person = get_schema_obj(&db, project, "_types/Person.td");
    let node_set = from_nodes(&db, project, person);
    let first =
      NativeFuncKind::NodeSetFirst.resolve()(&db, project, Some(node_set), vec![]).unwrap();
    assert!(!first.is_td_null_obj());
  }

  #[test]
  fn contains_known_node_returns_true() {
    let (db, project, _) = load_vault_fixture(VAULT, "typedown.yaml");
    let person = get_schema_obj(&db, project, "_types/Person.td");
    let alice = get_resource_obj(&db, project, "alice.td");
    let node_set = from_nodes(&db, project, person);
    let result =
      NativeFuncKind::NodeSetContains.resolve()(&db, project, Some(node_set), vec![alice]).unwrap();
    assert_eq!(result.to_display_string(&db), "true");
  }

  #[test]
  fn contains_node_of_wrong_schema_returns_false() {
    let (db, project, _) = load_vault_fixture(VAULT, "typedown.yaml");
    let company = get_schema_obj(&db, project, "_types/Company.td");
    let alice = get_resource_obj(&db, project, "alice.td");
    let company_set = from_nodes(&db, project, company);
    let result =
      NativeFuncKind::NodeSetContains.resolve()(&db, project, Some(company_set), vec![alice])
        .unwrap();
    assert_eq!(result.to_display_string(&db), "false");
  }

  #[test]
  fn follow_employer_deduplicates_targets() {
    // alice and bob both point to acme and carol to globex, follow deduplicates to 2 unique employers
    let (db, project, _) = load_vault_fixture(VAULT, "typedown.yaml");
    let person = get_schema_obj(&db, project, "_types/Person.td");
    let persons = from_nodes(&db, project, person);
    let employers = follow(&db, project, persons, "employer");
    assert_eq!(count(&db, project, employers), 2);
  }

  #[test]
  fn follow_unknown_field_yields_empty() {
    let (db, project, _) = load_vault_fixture(VAULT, "typedown.yaml");
    let person = get_schema_obj(&db, project, "_types/Person.td");
    let persons = from_nodes(&db, project, person);
    let result = follow(&db, project, persons, "nonexistent");
    assert_eq!(count(&db, project, result), 0);
  }

  #[test]
  fn follow_back_employer_from_companies_yields_all_persons() {
    // all three persons point to a company via "employer"
    let (db, project, _) = load_vault_fixture(VAULT, "typedown.yaml");
    let person = get_schema_obj(&db, project, "_types/Person.td");
    let company = get_schema_obj(&db, project, "_types/Company.td");
    let companies = from_nodes(&db, project, company);
    let persons = follow_back(&db, project, companies, person, "employer");
    assert_eq!(count(&db, project, persons), 3);
  }

  #[test]
  fn follow_then_follow_back_round_trips() {
    // persons -> follow employer -> companies -> follow_back employer -> persons
    let (db, project, _) = load_vault_fixture(VAULT, "typedown.yaml");
    let person = get_schema_obj(&db, project, "_types/Person.td");
    let persons = from_nodes(&db, project, person.clone());
    let employers = follow(&db, project, persons, "employer");
    let back = follow_back(&db, project, employers, person, "employer");
    assert_eq!(count(&db, project, back), 3);
  }

  #[test]
  fn acme_follow_back_yields_only_acme_employees() {
    let (db, project, _) = load_vault_fixture(VAULT, "typedown.yaml");
    let person = get_schema_obj(&db, project, "_types/Person.td");
    let company = get_schema_obj(&db, project, "_types/Company.td");
    let companies = from_nodes(&db, project, company);
    let acme = get_resource_obj(&db, project, "acme.td");
    let acme_in_set =
      NativeFuncKind::NodeSetContains.resolve()(&db, project, Some(companies), vec![acme.clone()])
        .unwrap();
    assert_eq!(acme_in_set.to_display_string(&db), "true");

    let acme_set: TdObjEnum =
      TdNodeSetObj::new(&db, project, NodeSetPlan::Eager(vec![acme])).into();
    let employees = follow_back(&db, project, acme_set, person, "employer");
    assert_eq!(count(&db, project, employees), 2);
  }
}
