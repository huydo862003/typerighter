use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU32};
use std::sync::{Arc, OnceLock};

use dashmap::DashMap;

use super::Fingerprint;
use super::ingredient::{
  Dependency, DerivedFieldIngredient, DerivedQueryIngredient, FieldFactory, FieldInventory,
  Ingredient, InputFactory, InputIngredient, InputInventory, InternedFactory, InternedIngredient,
  InternedInventory, QueryFactory, QueryInventory,
};
use super::persist::serialized::SerializedQueryStorage;
use super::persist::serialized::dep_graph::{DepNode, DepNodeIndex};
use crate::{DepId, DeserializeContext, IngredientKind};

#[cfg(debug_assertions)]
pub struct IngredientStats {
  pub name: String,
  pub recompute_count: usize,
  pub entry_count: usize,
  pub no_hash: bool,
  pub kind: IngredientKind,
}

/// An entry in the query stack, used for cycle detection
pub struct QueryStackEntry {
  pub dep_id: DepId,
}

// Type-erased identity map that supports sweeping stale entries
pub trait IdentityMap: Any + Send + Sync {
  // Remove entries where predicate returns false, return the removed IDs
  fn retain(&self, predicate: &dyn Fn(u32) -> bool) -> HashSet<u32>;
}

impl<K: Eq + std::hash::Hash + Send + Sync + 'static> IdentityMap for DashMap<K, u32> {
  fn retain(&self, predicate: &dyn Fn(u32) -> bool) -> HashSet<u32> {
    let mut removed = HashSet::new();
    DashMap::retain(self, |_, id| {
      if predicate(*id) {
        true
      } else {
        removed.insert(*id);
        false
      }
    });
    removed
  }
}

// (entry_id, start_index) -> identity map
pub type IdentityMapTable = Arc<DashMap<(u32, u32), Arc<dyn IdentityMap>>>;

/// Context passed through derived query execution
pub struct ExecuteContext {
  pub query_stack: Vec<QueryStackEntry>,
  pub dependencies: Vec<Dependency>,
  pub disambiguator_map: HashMap<u64, u32>, // hash(ingredient_index, id_field_values) -> counter
  // (entry_id, start_index) -> identity map, from the creating query
  pub identity_maps: Option<IdentityMapTable>,
  pub created_ids: HashMap<u32, HashSet<u32>>, // start_index -> IDs created this execution
}

#[derive(Clone)]
pub struct QueryStorage {
  #[doc(hidden)]
  pub revision: Arc<AtomicU32>, // The current version of the query storage
  #[doc(hidden)]
  pub cancelled: Arc<AtomicBool>, // Set to true to cancel in-flight derived queries
  #[doc(hidden)]
  pub inputs: Arc<Vec<Box<dyn InputIngredient>>>,
  #[doc(hidden)]
  pub input_fingerprints: Arc<DashMap<(u32, u32), Fingerprint>>, // (start_index representing the input group, entry_id) -> cached input struct fingerprint
  #[doc(hidden)]
  pub interned: Arc<Vec<Box<dyn InternedIngredient>>>,
  #[doc(hidden)]
  pub queries: Arc<Vec<Box<dyn DerivedQueryIngredient>>>,
  #[doc(hidden)]
  pub fields: Arc<Vec<Box<dyn DerivedFieldIngredient>>>,
  #[doc(hidden)]
  pub derived_fingerprints: Arc<DashMap<(u32, u32), Fingerprint>>, // (start_index representing the derived group, entry_id) -> cached derived struct fingerprint
  #[doc(hidden)]
  pub deserialize_ctx: Arc<OnceLock<DeserializeContext>>, // Previous session's data for lazy deserialization
}

impl Default for QueryStorage {
  fn default() -> Self {
    Self::new(0)
  }
}

fn init_factories<T: ?Sized>(
  factories: &[fn(DepId) -> Box<T>],
  kind: IngredientKind,
) -> Arc<Vec<Box<T>>> {
  Arc::new(
    factories
      .iter()
      .enumerate()
      .map(|(idx, factory)| factory(DepId::ingredient(kind, idx as u32)))
      .collect(),
  )
}

impl QueryStorage {
  fn new(revision: u32) -> Self {
    let (inputs, interned, queries, fields) = registries();
    QueryStorage {
      revision: Arc::new(AtomicU32::new(revision)),
      cancelled: Arc::new(AtomicBool::new(false)),
      inputs: init_factories(inputs, IngredientKind::Input),
      interned: init_factories(interned, IngredientKind::Interned),
      queries: init_factories(queries, IngredientKind::Query),
      fields: init_factories(fields, IngredientKind::Field),
      deserialize_ctx: Arc::new(OnceLock::new()),
      input_fingerprints: Arc::new(DashMap::new()),
      derived_fingerprints: Arc::new(DashMap::new()),
    }
  }

  /// Create a QueryStorage from a previous session's serialized data
  pub fn from_serialized(serialized: SerializedQueryStorage) -> Arc<Self> {
    let revision = serialized.dep_graph.header.revision as u32;
    let storage = Arc::new(Self::new(revision));
    let _ = storage.deserialize_ctx.set(DeserializeContext::new(
      serialized,
      Arc::downgrade(&storage),
    ));
    storage.load_leaf_nodes();
    storage
  }

  /// Eagerly deserialize all input and interned nodes.
  // Must run before any derived query deserialization, because derived query blobs contain DepNodeIndex references to inputs/interned that need to be in the decoder's dep_id_table before decoding
  // WARNING: this also populates entry_id_map for input and interned types, which deserialize_return_value relies on for non-derived return types
  fn load_leaf_nodes(self: &Arc<Self>) {
    let Some(ctx) = self.deserialize_ctx.get() else {
      return;
    };
    for (i, node) in ctx.serialized.dep_graph.nodes.iter().enumerate() {
      let node_index = i as DepNodeIndex;
      match node {
        DepNode::InputField { .. } => {
          if ctx.decoder.get_dep_node_id(node_index).is_some() {
            continue;
          }
          let name = node.name();
          let node_field_index = node.field_index();
          for &idx in ctx.inputs_by_name(&name) {
            let input = &self.inputs[idx];
            if Some(input.field_index()) == node_field_index {
              input.deserialize(ctx, node_index);
              break;
            }
          }
        }
        DepNode::Interned { .. } => {
          if ctx.decoder.get_dep_node_id(node_index).is_some() {
            continue;
          }
          let name = node.name();
          if let Some(&idx) = ctx.interned_by_name(&name).first() {
            self.interned[idx].deserialize(ctx, node_index);
          }
        }
        _ => {}
      }
    }
  }

  pub fn reset_for_new_revision(&self) {
    // Only query ingredients have non-trivial reset (LRU eviction)
    for entry in self.queries.iter() {
      entry.reset_for_new_revision();
    }
  }

  /// Green check a dependency by dispatching to the correct ingredient array
  pub fn green_check_dep(&self, db: &dyn crate::QueryDatabase, dep: &Dependency) -> bool {
    let idx = dep.dep_id.ingredient_id() as usize;
    let entry_id = dep.dep_id.entry_id();
    match dep.dep_id.kind() {
      IngredientKind::Input => self.inputs[idx].green_check(entry_id, dep.changed_at),
      IngredientKind::Interned => true, // always green
      IngredientKind::Query => self.queries[idx].green_check(db, entry_id, dep.changed_at),
      IngredientKind::Field => self.fields[idx].green_check(entry_id, dep.changed_at),
    }
  }

  /// Re-execute a dependency
  pub fn re_execute_dep(&self, db: &dyn crate::QueryDatabase, dep: &Dependency) {
    let idx = dep.dep_id.ingredient_id() as usize;
    let entry_id = dep.dep_id.entry_id();
    match dep.dep_id.kind() {
      IngredientKind::Input => {}    // nothing to recompute
      IngredientKind::Interned => {} // nothing to recompute
      IngredientKind::Query => self.queries[idx].re_execute(db, entry_id),
      IngredientKind::Field => {} // fields are set by their parent query
    }
  }

  /// Remove field entries for a given field start_index
  pub fn remove_field_entries(&self, start_index: u32, removed: &HashSet<u32>) {
    let start = start_index as usize;
    for (i, field) in self.fields[start..].iter().enumerate() {
      if field.field_index() != i as u8 {
        break;
      }
      for &id in removed {
        field.remove_entry(id);
      }
    }
    for &id in removed {
      self.derived_fingerprints.remove(&(start_index, id));
    }
  }

  // Get the current session's fingerprint for a dependency, for cross-session validation
  // Look up the ingredient store that owns a given DepId
  pub fn get_ingredient_of_id(&self, dep_id: DepId) -> &dyn Ingredient {
    let idx = dep_id.ingredient_id() as usize;
    match dep_id.kind() {
      IngredientKind::Input => &*self.inputs[idx],
      IngredientKind::Interned => &*self.interned[idx],
      IngredientKind::Query => &*self.queries[idx],
      IngredientKind::Field => &*self.fields[idx],
    }
  }

  /// Total number of query function invocations across all derived ingredients
  #[cfg(debug_assertions)]
  pub fn total_recompute_count(&self) -> usize {
    let mut total = 0;
    for entry in self.inputs.iter() {
      total += entry.recompute_count();
    }
    for entry in self.interned.iter() {
      total += entry.recompute_count();
    }
    for entry in self.queries.iter() {
      total += entry.recompute_count();
    }
    for entry in self.fields.iter() {
      total += entry.recompute_count();
    }
    total
  }

  /// Stats for each ingredient
  #[cfg(debug_assertions)]
  pub fn ingredient_stats(&self) -> Vec<IngredientStats> {
    let mut stats = Vec::new();
    for entry in self.inputs.iter() {
      stats.push(IngredientStats {
        name: entry.readable_name(),
        recompute_count: entry.recompute_count(),
        entry_count: entry.entry_ids().count(),
        no_hash: false,
        kind: IngredientKind::Input,
      });
    }
    for entry in self.interned.iter() {
      stats.push(IngredientStats {
        name: entry.readable_name(),
        recompute_count: entry.recompute_count(),
        entry_count: entry.entry_ids().count(),
        no_hash: false,
        kind: IngredientKind::Interned,
      });
    }
    for entry in self.queries.iter() {
      stats.push(IngredientStats {
        name: entry.readable_name(),
        recompute_count: entry.recompute_count(),
        entry_count: entry.entry_ids().count(),
        no_hash: entry.no_hash(),
        kind: IngredientKind::Query,
      });
    }
    for entry in self.fields.iter() {
      stats.push(IngredientStats {
        name: entry.readable_name(),
        recompute_count: entry.recompute_count(),
        entry_count: entry.entry_ids().count(),
        no_hash: entry.no_hash(),
        kind: IngredientKind::Field,
      });
    }
    stats
  }

  #[cfg(debug_assertions)]
  #[doc(hidden)]
  pub const __TYPEDOWN_QUERY_STORAGE: () = ();

  /// Access the current thread's ExecuteContext
  #[doc(hidden)]
  pub fn with_context<R>(&self, f: impl FnOnce(&mut Option<ExecuteContext>) -> R) -> R {
    thread_local! {
      static CTX: RefCell<Option<ExecuteContext>> = const { RefCell::new(None) };
    }
    CTX.with(|c| f(&mut c.borrow_mut()))
  }

  // Whether the current thread is inside a query execution
  #[doc(hidden)]
  pub fn is_in_query(&self) -> bool {
    self.with_context(|ctx| ctx.is_some())
  }

  /// Get the next disambiguator for a given identity hash within the current query execution
  /// Returns 0 if not inside a query execution
  #[doc(hidden)]
  pub fn next_disambiguator(&self, identity_hash: u64) -> u32 {
    self.with_context(|ctx| {
      if let Some(ctx) = ctx {
        let counter = ctx.disambiguator_map.entry(identity_hash).or_insert(0);
        let value = *counter;
        *counter += 1;
        value
      } else {
        0
      }
    })
  }

  /// Get the current query's DepId from the top of the query stack
  #[doc(hidden)]
  pub fn current_query_dep_id(&self) -> Option<DepId> {
    self.with_context(|ctx| {
      ctx
        .as_ref()
        .and_then(|ctx| ctx.query_stack.last().map(|entry| entry.dep_id))
    })
  }
}

type Registries = (
  &'static Vec<InputFactory>,
  &'static Vec<InternedFactory>,
  &'static Vec<QueryFactory>,
  &'static Vec<FieldFactory>,
);

fn registries() -> Registries {
  static INPUT_REGISTRY: OnceLock<Vec<InputFactory>> = OnceLock::new();
  static INTERNED_REGISTRY: OnceLock<Vec<InternedFactory>> = OnceLock::new();
  static QUERY_REGISTRY: OnceLock<Vec<QueryFactory>> = OnceLock::new();
  static FIELD_REGISTRY: OnceLock<Vec<FieldFactory>> = OnceLock::new();

  let inputs = INPUT_REGISTRY.get_or_init(|| {
    let mut factories = Vec::new();
    for entry in inventory::iter::<InputInventory> {
      (entry.register)(&mut factories);
    }
    factories
  });
  let interned = INTERNED_REGISTRY.get_or_init(|| {
    let mut factories = Vec::new();
    for entry in inventory::iter::<InternedInventory> {
      (entry.register)(&mut factories);
    }
    factories
  });
  let queries = QUERY_REGISTRY.get_or_init(|| {
    let mut factories = Vec::new();
    for entry in inventory::iter::<QueryInventory> {
      (entry.register)(&mut factories);
    }
    factories
  });
  let fields = FIELD_REGISTRY.get_or_init(|| {
    let mut factories = Vec::new();
    for entry in inventory::iter::<FieldInventory> {
      (entry.register)(&mut factories);
    }
    factories
  });

  (inputs, interned, queries, fields)
}
