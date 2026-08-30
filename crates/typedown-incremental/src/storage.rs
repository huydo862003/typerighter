use std::any::Any;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::{Arc, OnceLock};

use dashmap::DashMap;

use super::ingredient::{Dependency, IngredientEntry, IngredientFactory, Inventory};
use super::persist::serialized::SerializedQueryStorage;
use super::persist::serialized::dep_graph::{DepNode, DepNodeIndex};
use crate::{DeserializeContext, Fingerprint};

/// A registry of ingredient factories
/// This is used in QueryStorage::default() to initialize the internal ingredient vector
/// TIL: By storing the callbacks (IngredientFactory is a function pointer type), instead of the empty dyn Ingredients (used as templates so default can clone), this avoid requiring the Ingredient to be cloneable... but Ingredient is a supertrait of Any, which is not clonable so cannot be used with dyn!
static INGREDIENT_REGISTRY: OnceLock<Vec<IngredientFactory>> = OnceLock::new();

/// An entry in the query stack, used for cycle detection
pub struct QueryStackEntry {
  pub ingredient_index: usize,
  pub arg_id: usize,
}

// Type-erased identity map that supports sweeping stale entries
pub trait IdentityMap: Any + Send + Sync {
  // Remove entries where predicate returns false, return the removed IDs
  fn retain(&self, predicate: &dyn Fn(usize) -> bool) -> HashSet<usize>;
}

impl<K: Eq + std::hash::Hash + Send + Sync + 'static> IdentityMap for DashMap<K, usize> {
  fn retain(&self, predicate: &dyn Fn(usize) -> bool) -> HashSet<usize> {
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

// (arg_id, start_index) -> identity map
pub type IdentityMapTable = Arc<DashMap<(usize, usize), Arc<dyn IdentityMap>>>;

/// Context passed through derived query execution
pub struct ExecuteContext {
  pub query_stack: Vec<QueryStackEntry>,
  pub dependencies: Vec<Dependency>,
  pub disambiguator_map: HashMap<u64, usize>, // map hash(ingredient_index, id_field_values) to counter
  // (arg_id, start_index) -> identity map, from the creating query
  pub identity_maps: Option<IdentityMapTable>,
  pub created_ids: HashMap<usize, HashSet<usize>>, // start_index -> IDs created this execution
}

#[derive(Clone)]
pub struct QueryStorage {
  #[doc(hidden)]
  pub revision: Arc<AtomicUsize>, // The current version of the query storage
  #[doc(hidden)]
  pub cancelled: Arc<AtomicBool>, // Set to true to cancel in-flight derived queries
  #[doc(hidden)]
  pub ingredients: Arc<Vec<IngredientEntry>>, // All ingredients
  #[doc(hidden)]
  pub deserialize_ctx: Arc<OnceLock<DeserializeContext>>, // Previous session's data for lazy deserialization
}

impl Default for QueryStorage {
  fn default() -> Self {
    QueryStorage {
      revision: Arc::new(AtomicUsize::new(0)),
      cancelled: Arc::new(AtomicBool::new(false)),
      ingredients: Arc::new(
        registry()
          .iter()
          .enumerate()
          .map(|(idx, factory)| factory(idx))
          .collect(),
      ),
      deserialize_ctx: Arc::new(OnceLock::new()),
    }
  }
}

impl QueryStorage {
  /// Create a QueryStorage from a previous session's serialized data.
  pub fn from_serialized(serialized: SerializedQueryStorage) -> Arc<Self> {
    let revision = serialized.dep_graph.header.revision as usize;
    let storage = Arc::new(QueryStorage {
      revision: Arc::new(AtomicUsize::new(revision)),
      cancelled: Arc::new(AtomicBool::new(false)),
      ingredients: Arc::new(
        registry()
          .iter()
          .enumerate()
          .map(|(idx, factory)| factory(idx))
          .collect(),
      ),
      deserialize_ctx: Arc::new(OnceLock::new()),
    });
    let _ = storage.deserialize_ctx.set(DeserializeContext::new(
      serialized,
      Arc::downgrade(&storage),
    ));
    storage.load_leaf_nodes();
    storage
  }

  /// Eagerly deserialize all input and interned nodes.
  /// Must run before any derived query deserialization, because derived query
  /// blobs contain DepNodeIndex references to inputs/interned that need to be
  /// in the decoder's dep_id_table before decoding.
  fn load_leaf_nodes(self: &Arc<Self>) {
    let Some(ctx) = self.deserialize_ctx.get() else {
      return;
    };
    // Collect node indices to avoid borrowing ctx during iteration
    let leaf_nodes: Vec<(DepNodeIndex, Fingerprint)> = ctx
      .serialized
      .dep_graph
      .nodes
      .iter()
      .enumerate()
      .filter_map(|(i, node)| match node {
        DepNode::InputField { .. } | DepNode::Interned { .. } => {
          Some((i as DepNodeIndex, node.name()))
        }
        _ => None,
      })
      .collect();

    for (node_index, name) in &leaf_nodes {
      if ctx.decoder.get_dep_node_id(*node_index).is_some() {
        continue;
      }
      let node = &ctx.serialized.dep_graph.nodes[*node_index as usize];
      let node_field_index = node.field_index();
      for &idx in ctx.ingredients_by_name(name) {
        if self.ingredients[idx].field_index == node_field_index {
          self.ingredients[idx]
            .ingredient
            .deserialize(ctx, *node_index);
          break;
        }
      }
    }
  }

  /// Total number of query function invocations across all derived ingredients.
  #[cfg(debug_assertions)]
  pub fn total_recompute_count(&self) -> usize {
    self
      .ingredients
      .iter()
      .map(|e| e.ingredient.recompute_count())
      .sum()
  }

  /// Marker used by the `query_db` macro to verify the storage field type at compile time.
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

  /// Get the next disambiguator for a given identity hash within the current query execution
  /// Returns 0 if not inside a query execution
  #[doc(hidden)]
  pub fn next_disambiguator(&self, identity_hash: u64) -> usize {
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

  /// Get the current query's identity (ingredient_index, arg_id) from the top of the query stack.
  /// Returns (0, 0) if not inside a query execution.
  #[doc(hidden)]
  pub fn current_query_identity(&self) -> (usize, usize) {
    self.with_context(|ctx| {
      if let Some(ctx) = ctx {
        ctx
          .query_stack
          .last()
          .map(|entry| (entry.ingredient_index, entry.arg_id))
          .unwrap_or((0, 0))
      } else {
        (0, 0)
      }
    })
  }
}

pub(crate) fn registry() -> &'static Vec<IngredientFactory> {
  INGREDIENT_REGISTRY.get_or_init(|| {
    let mut factories = Vec::new();

    for entry in inventory::iter::<Inventory> {
      (entry.register)(&mut factories);
    }

    factories
  })
}
