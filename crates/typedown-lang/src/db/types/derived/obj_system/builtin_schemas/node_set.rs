use std::hash::Hasher;

use hashbrown::HashSet;
use typedown_incremental::{Id, QueryDatabase, StableHash, StableHasher};
use typedown_macros::{Decodable, Encodable, StableCompare, StableHash, query_derived};

use typedown_types::either::Either;

use crate::db::TypedownDatabase;
use crate::db::derived::builtin_schemas::node_set::get_node_set_typ;
use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::types::TdRuntimeObj;
use crate::db::types::{Project, SymbolKind};
use crate::db::types::{TdFuncObj, TdListObj, TdObjEnum, TdSchemaTyp, TdTypEnum};

/* NodeSetPlan */

#[derive(Clone, Debug, PartialEq, StableCompare, Encodable, Decodable, StableHash)]
pub enum NodeSetPlan<'db> {
  Eager(Vec<TdObjEnum<'db>>),
  FromNodes {
    schema_typ: TdSchemaTyp<'db>,
    where_func: Option<TdFuncObj<'db>>,
  },
  Filter {
    source: TdNodeSetObj<'db>,
    where_func: TdFuncObj<'db>,
  },
  Follow {
    source: TdNodeSetObj<'db>,
    field_name: String,
    where_func: Option<TdFuncObj<'db>>,
  },
  FollowBack {
    source: TdNodeSetObj<'db>,
    field_name: String,
  },
}

/* TdNodeSetObj */

/// A lazy collection of vault nodes returned by graph queries
#[query_derived(custom_hash)]
pub struct TdNodeSetObj<'db> {
  pub project: Project,
  pub plan: NodeSetPlan<'db>,
}

impl<'db> StableHash for TdNodeSetObj<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    let storage = unsafe { db.storage() };
    let cache_key = (Self::ingredient_start_index(), self.0);
    if let Some(cached) = storage.derived_fingerprints.get(&cache_key) {
      Hasher::write(hasher, &cached.0);
      return;
    }
    let mut inner_hasher = StableHasher::new();
    if let Some(plan) = Self::try_plan(*self, db) {
      plan.stable_hash(db, &mut inner_hasher);
    }
    let fingerprint = typedown_incremental::Fingerprint::from_hasher(inner_hasher);
    storage.derived_fingerprints.insert(cache_key, fingerprint);
    Hasher::write(hasher, &fingerprint.0);
  }
}

impl<'db> TdRuntimeObj<'db> for TdNodeSetObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    get_node_set_typ(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::node_set".to_string()
  }
}

/* Materialization */

fn vault_resources<'db>(db: &'db TypedownDatabase, project: Project) -> Vec<TdObjEnum<'db>> {
  project
    .files(db)
    .values()
    .filter_map(|file| {
      let symbol = file_symbol(db, project, *file).value(db)?;
      if !matches!(symbol.kind(db), SymbolKind::UserDefinedResource(_, _)) {
        return None;
      }
      evaluate_resource(db, symbol).value(db)
    })
    .collect()
}

#[query_derived]
pub(crate) fn materialize_as_list<'db>(
  db: &'db TypedownDatabase,
  node_set: TdNodeSetObj<'db>,
) -> TdListObj<'db> {
  let project = node_set.project(db);
  let nodes = match node_set.plan(db) {
    NodeSetPlan::Eager(nodes) => nodes,
    NodeSetPlan::FromNodes {
      schema_typ,
      where_func,
    } => vault_resources(db, project)
      .into_iter()
      .filter(|obj| {
        obj
          .as_td_schema_obj()
          .map(|schema_obj| schema_obj.schema(db).as_td_schema_typ().copied() == Some(schema_typ))
          .unwrap_or(false)
      })
      .filter(|obj| {
        let Some(ref func) = where_func else {
          return true;
        };
        func
          .call(db, project, None, vec![obj.clone()])
          .ok()
          .and_then(|result| result.as_td_bool_obj().map(|bool| bool.value(db)))
          .unwrap_or(false)
      })
      .collect(),
    NodeSetPlan::Filter { source, where_func } => materialize_as_list(db, source)
      .items(db)
      .into_iter()
      .filter_map(|entry| match entry {
        Either::Right(obj) => Some(obj),
        Either::Left(_) => None,
      })
      .filter(|node| {
        where_func
          .call(db, project, None, vec![node.clone()])
          .ok()
          .and_then(|result| result.as_td_bool_obj().map(|bool| bool.value(db)))
          .unwrap_or(false)
      })
      .collect(),
    NodeSetPlan::Follow {
      source,
      field_name,
      where_func,
    } => {
      let mut seen = HashSet::new();
      let mut targets = Vec::new();
      for src in materialize_as_list(db, source)
        .items(db)
        .into_iter()
        .filter_map(|entry| match entry {
          Either::Right(obj) => Some(obj),
          Either::Left(_) => None,
        })
      {
        let Some(field_value) = src.get_owned_field(db, &field_name) else {
          continue;
        };
        if field_value.is_td_null_obj() {
          continue;
        }
        if let Some(ref func) = where_func {
          match func.call(db, project, None, vec![field_value.clone()]) {
            Ok(result) => {
              if !result.as_td_bool_obj().is_some_and(|bool| bool.value(db)) {
                continue;
              }
            }
            Err(_) => continue,
          }
        }
        // deduplicate targets by ID
        if seen.insert(field_value.as_id()) {
          targets.push(field_value);
        }
      }
      targets
    }
    NodeSetPlan::FollowBack { source, field_name } => {
      let target_ids: HashSet<_> = materialize_as_list(db, source)
        .items(db)
        .into_iter()
        .filter_map(|entry| match entry {
          Either::Right(obj) => Some(obj.as_id()),
          Either::Left(_) => None,
        })
        .collect();
      let mut seen = HashSet::new();
      vault_resources(db, project)
        .into_iter()
        .filter(|obj| {
          let Some(field_value) = obj.get_owned_field(db, &field_name) else {
            return false;
          };
          target_ids.contains(&field_value.as_id()) && seen.insert(obj.as_id())
        })
        .collect()
    }
  };
  TdListObj::new(db, nodes.into_iter().map(Either::Right).collect())
}
