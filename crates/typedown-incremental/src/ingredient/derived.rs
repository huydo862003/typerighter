use std::{
  any::Any,
  collections::HashMap,
  hash::Hash,
  panic::panic_any,
  sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
  },
};

use indexmap::IndexSet;

use crate::persist::serialized::dep_graph::{DepNode, DepNodeIndex};
use crate::{Cancelled, ExecuteContext, IdentityMapTable, QueryStackEntry, QueryStorage};
use crate::{
  Decodable, DepId, DeserializeContext, Encodable, Fingerprint, StableHash, StableHasher,
};
use crate::{DerivedId, QueryDatabase, SerializeContext, UnresolvedDepNode};
use dashmap::DashMap;

use super::Ingredient;

pub const LRU_CAPACITY: usize = 1024;

// LRU tracker for derived query memos
#[derive(Default)]
pub struct Lru {
  access_order: Mutex<IndexSet<usize>>, // oldest first
}

impl Lru {
  // Record access and returns arg_ids to evict
  pub fn touch(&self, arg_id: usize) -> Vec<usize> {
    let mut order = self.access_order.lock().unwrap();
    order.shift_remove(&arg_id);
    order.insert(arg_id);
    let mut evicted = Vec::new();
    while order.len() > LRU_CAPACITY {
      if let Some(id) = order.shift_remove_index(0) {
        evicted.push(id);
      }
    }
    evicted
  }
}

/// A dependency recorded during a derived query execution
#[derive(Clone)]
pub struct Dependency {
  pub ingredient_index: usize, // Which ingredient
  pub arg_id: usize,           // Which entry in that ingredient
  pub changed_at: usize,       // The revision it had when we read it
}

/// A memoized derived query result
pub struct StampedDerivedQuery<K, V: DerivedId> {
  pub key: K,                        // The original key, for re-execution
  pub value: V,                      // The derived struct ID
  pub changed_at: usize,             // Revision when the value last actually changed
  pub verified_at: usize,            // Revision when last confirmed valid
  pub dependencies: Vec<Dependency>, // What this query read during execution
}

/// The state of a query entry in the cache
pub enum QueryState<K, V: DerivedId> {
  /// The query is currently being computed
  Computing,
  /// The query has a cached result
  Computed(StampedDerivedQuery<K, V>),
}

/// Ingredient for a derived query function: maps key tuple to memoized result
#[derive(Clone)]
#[doc(hidden)]
pub struct DerivedQueryIngredient<DB, K, V: DerivedId> {
  ingredient_index: usize,
  name_fingerprint: Fingerprint,
  value_fingerprint: Fingerprint, // name of the return type struct (e.g. "FibResult")
  next_arg_id: Arc<AtomicUsize>,
  value_id_counter: &'static AtomicUsize,
  query_fn: fn(&DB, K) -> V,
  intern_map: Arc<DashMap<K, usize>>, // key -> stable arg_id
  #[doc(hidden)]
  pub data: Arc<DashMap<usize, QueryState<K, V>>>, // arg_id -> state
  identity_maps: IdentityMapTable,
  lru: Arc<Lru>, // For stale entry eviction
  #[cfg(debug_assertions)]
  recompute_count: Arc<AtomicUsize>,
  #[cfg(debug_assertions)]
  readable_name: &'static str,
}

impl<DB, K, V: DerivedId> std::fmt::Debug for DerivedQueryIngredient<DB, K, V> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let name = {
      #[cfg(debug_assertions)]
      {
        self.readable_name
      }
      #[cfg(not(debug_assertions))]
      {
        "DerivedQueryIngredient"
      }
    };
    f.debug_struct(name).finish_non_exhaustive()
  }
}

impl<
  DB: QueryDatabase + Send + Sync + 'static,
  K: StableHash + std::fmt::Debug + Encodable + Decodable + Eq + Hash + Clone + Send + Sync + 'static,
  V: StableHash
    + std::fmt::Debug
    + Encodable
    + Decodable
    + DerivedId
    + Clone
    + PartialEq
    + Send
    + Sync
    + 'static,
> DerivedQueryIngredient<DB, K, V>
{
  /// Deserialize all sibling DerivedField nodes for a value struct.
  fn deserialize_field_group(&self, ctx: &DeserializeContext, serialized_entry_id: u64) {
    let group_key = (self.value_fingerprint, serialized_entry_id);
    if let Some(field_group) = ctx.derived_groups.get(&group_key) {
      for &(_, field_node_index) in &field_group.fields {
        ctx.decoder.get_or_deserialize_dep_node_id(field_node_index);
      }
    }
  }

  /// Deserialize edge DepNodeIndices into session-local Dependencies
  /// Edges that fail to deserialize are silently dropped
  fn deserialize_deps(edges: &[u32], ctx: &DeserializeContext) -> Vec<Dependency> {
    edges
      .iter()
      .filter_map(|&edge_idx| {
        let dep_id = ctx.decoder.get_or_deserialize_dep_node_id(edge_idx)?;
        let edge_node = &ctx.serialized.dep_graph.nodes[edge_idx as usize];
        Some(Dependency {
          ingredient_index: dep_id.0,
          arg_id: dep_id.1,
          changed_at: edge_node.changed_at() as usize,
        })
      })
      .collect()
  }

  pub fn new(
    ingredient_index: usize,
    name_fingerprint: &'static str,
    value_fingerprint: &'static str,
    value_id_counter: &'static AtomicUsize,
    query_fn: fn(&DB, K) -> V,
  ) -> Self {
    Self {
      ingredient_index,
      name_fingerprint: Fingerprint::from_name(name_fingerprint),
      value_fingerprint: Fingerprint::from_name(value_fingerprint),
      next_arg_id: Arc::new(AtomicUsize::new(0)),
      value_id_counter,
      query_fn,
      intern_map: Arc::new(DashMap::new()),
      data: Arc::new(DashMap::new()),
      identity_maps: Arc::new(DashMap::new()),
      lru: Arc::new(Lru::default()),
      #[cfg(debug_assertions)]
      recompute_count: Arc::new(AtomicUsize::new(0)),
      #[cfg(debug_assertions)]
      readable_name: name_fingerprint,
    }
  }

  pub fn key_fingerprint(&self, db: &dyn QueryDatabase, arg_id: usize) -> Option<Fingerprint>
  where
    K: StableHash,
  {
    let db = (db as &dyn Any)
      .downcast_ref::<DB>()
      .expect("database type mismatch in key_fingerprint");
    if let Some(entry) = self.data.get(&arg_id)
      && let QueryState::Computed(memo) = &*entry
    {
      let mut hasher: StableHasher = StableHasher::new();
      memo.key.stable_hash(db, &mut hasher);
      return Some(Fingerprint::from_hasher(hasher));
    }
    None
  }

  pub fn value_fingerprint(&self, db: &dyn QueryDatabase, arg_id: usize) -> Option<Fingerprint>
  where
    V: StableHash,
  {
    let db = (db as &dyn Any)
      .downcast_ref::<DB>()
      .expect("database type mismatch in value_fingerprint");
    if let Some(entry) = self.data.get(&arg_id)
      && let QueryState::Computed(memo) = &*entry
    {
      let mut hasher: StableHasher = StableHasher::new();
      memo.value.stable_hash(db, &mut hasher);
      return Some(Fingerprint::from_hasher(hasher));
    }
    None
  }

  /// Try to load a cached result from the serialized cache.
  fn try_load_from_serialized(
    &self,
    db: &DB,
    storage: &QueryStorage,
    arg: &K,
  ) -> Option<(V, usize)> {
    let ctx = storage.deserialize_ctx.get()?;

    // Compute key fingerprint to find the matching node
    let mut hasher = StableHasher::new();
    arg.stable_hash(db, &mut hasher);
    let key_fp = Fingerprint::from_hasher(hasher);

    let (node_index, node) = ctx.find_derived_query(self.name_fingerprint, key_fp)?;

    let DepNode::DerivedQuery {
      value_entry_id: serialized_value_entry_id,
      changed_at,
      edges,
      ..
    } = node
    else {
      return None;
    };

    // Ensure all edge deps are deserialized before green checking
    let decoder = &ctx.decoder;
    for &edge_idx in edges {
      decoder.get_or_deserialize_dep_node_id(edge_idx);
    }

    // Green check: compare multisets of (ingredient_name, value_fingerprint)
    // Both the serialized edges and current entries must have matching counts
    let mut expected: HashMap<(Fingerprint, Fingerprint), usize> = HashMap::new();
    for edge_idx in edges {
      let edge_node = &ctx.serialized.dep_graph.nodes[*edge_idx as usize];
      *expected
        .entry((edge_node.name(), edge_node.value_fingerprint()))
        .or_default() += 1;
    }

    let mut actual: HashMap<(Fingerprint, Fingerprint), usize> = HashMap::new();
    for (name, _) in expected.keys() {
      for &idx in ctx.ingredients_by_name(name) {
        let entry = &storage.ingredients[idx];
        for eid in entry.ingredient.entry_ids() {
          if let Some(fp) = entry.ingredient.value_fingerprint(db, eid) {
            *actual.entry((*name, fp)).or_default() += 1;
          }
        }
      }
    }

    for (key, &needed) in &expected {
      let available = actual.get(key).copied().unwrap_or(0);
      if available < needed {
        return None;
      }
    }

    // Deserialize field data
    self.deserialize_field_group(ctx, *serialized_value_entry_id);
    let value_entry_id = *ctx
      .entry_id_map
      .entry((self.value_fingerprint, *serialized_value_entry_id))
      .or_insert_with(|| self.value_id_counter.fetch_add(1, Ordering::Relaxed));

    // Decode key
    let blob = ctx.serialized.query_cache.get(node_index)?;
    let mut data: &[u8] = blob;
    let key = K::decode(&mut data, decoder);
    let arg_id = self.get_or_intern_arg(&key);
    let value = V::from(value_entry_id);
    let changed_at = *changed_at as usize;
    let dependencies = Self::deserialize_deps(edges, ctx);
    let current_revision = storage.revision.load(Ordering::Acquire);
    self.data.insert(
      arg_id,
      QueryState::Computed(StampedDerivedQuery {
        key,
        value: value.clone(),
        changed_at,
        verified_at: current_revision,
        dependencies,
      }),
    );

    Some((value, changed_at))
  }

  /// Get or create a stable entry ID for a key
  fn get_or_intern_arg(&self, arg: &K) -> usize {
    if let Some(entry) = self.intern_map.get(arg) {
      return *entry.value();
    }
    let arg_id = self.next_arg_id.fetch_add(1, Ordering::Relaxed);
    *self.intern_map.entry(arg.clone()).or_insert(arg_id).value()
  }

  /// Execute a derived query: returns cached result if valid, otherwise runs the query function
  pub fn execute_query(&self, db: &DB, arg: K) -> V {
    let storage = unsafe { db.storage() };
    let current_revision = storage.revision.load(Ordering::Acquire);
    let ingredient_index = self.ingredient_index;
    let arg_id = self.get_or_intern_arg(&arg);

    let (value, changed_at) =
      self.execute_query_inner(db, storage, current_revision, ingredient_index, arg_id, arg);

    // Record dependency for the caller
    storage.with_context(|ctx| {
      if let Some(ctx) = ctx {
        ctx.dependencies.push(Dependency {
          ingredient_index,
          arg_id,
          changed_at,
        });
      }
    });

    for evicted_arg_id in self.lru.touch(arg_id) {
      self.evict_memo(evicted_arg_id);
    }

    value
  }

  // Evict a memo, forcing recomputation on next access
  fn evict_memo(&self, arg_id: usize) {
    self.data.remove(&arg_id);
  }

  /// Inner implementation that returns (value, changed_at)
  fn execute_query_inner(
    &self,
    db: &DB,
    storage: &QueryStorage,
    current_revision: usize,
    ingredient_index: usize,
    arg_id: usize,
    arg: K,
  ) -> (V, usize) {
    // Check cache
    if let Some(entry) = self.data.get(&arg_id) {
      match &*entry {
        QueryState::Computed(memo) if memo.verified_at >= current_revision => {
          return (memo.value.clone(), memo.changed_at);
        }
        QueryState::Computing => {
          // Cycle detection: Check if this entry is in our call stack
          let is_cycle = storage.with_context(|ctx| {
            ctx.as_ref().is_some_and(|ctx| {
              ctx
                .query_stack
                .iter()
                .any(|e| e.ingredient_index == ingredient_index && e.arg_id == arg_id)
            })
          });
          if is_cycle {
            panic!("cycle detected in derived query");
          }
          // Not in our stack: another thread is computing this, compute anyway, which should be negligible
          // Don't wait here, else you risk deadlock
        }
        QueryState::Computed(memo) => {
          // Stale compared to current revision (not sure if real stale)
          // Run green check
          let changed_at = memo.changed_at;
          drop(entry); // Release the read lock

          if self.green_check(db, arg_id, changed_at) {
            // The green check has verified or recomputed + backdated so the entry must now be fresh
            if let Some(entry) = self.data.get(&arg_id)
              && let QueryState::Computed(memo) = &*entry
            {
              return (memo.value.clone(), memo.changed_at);
            }
          }
          // green_check returned false, need to recompute
        }
      }
    }

    // Try loading from previous session before recomputing
    if let Some((value, changed_at)) = self.try_load_from_serialized(db, storage, &arg) {
      return (value, changed_at);
    }

    #[allow(unused_labels)]
    'Time_A: {}

    #[allow(unused_labels)]
    'Time_B: {}

    // Mark as computing
    // This can override a fresh computed value between 'Time_A and 'Time_B
    // But it should not matter except for a little redundant work:
    // - Everything is immutable, so recomputation is fine
    // - If a thread computes the value then see a stale value again, it would just trigger recompute (redundant work), but it doesn't cause any cycle
    // - The thread that computes the value still return the fresh value

    // EDIT: The current optimization (?) is to use a shard lock provided by DashMap to check if the value is overrided with a fresh value already to skip unnecessary computation
    // However, this introduces a lock, so I don't really know
    let mut cached = None;
    let mut old_memo = None;
    self
      .data
      .entry(arg_id)
      .and_modify(|state| {
        if let QueryState::Computed(memo) = state {
          if memo.verified_at >= current_revision {
            cached = Some((memo.value.clone(), memo.changed_at));
            return;
          }
          // Save old value and changed_at for backdating after recompute
          old_memo = Some((memo.value.clone(), memo.changed_at));
        }
        *state = QueryState::Computing;
      })
      .or_insert(QueryState::Computing);

    if let Some((value, changed_at)) = cached {
      return (value, changed_at);
    }

    // Save parent context and push to query stack
    let (parent_deps, parent_disambiguators, parent_store, parent_created_ids) = storage
      .with_context(|ctx| {
        let ctx = ctx.get_or_insert_with(|| ExecuteContext {
          query_stack: Vec::new(),
          dependencies: Vec::new(),
          disambiguator_map: HashMap::new(),
          identity_maps: None,
          created_ids: HashMap::new(),
        });
        ctx.query_stack.push(QueryStackEntry {
          ingredient_index,
          arg_id,
        });
        let parent_store = ctx.identity_maps.replace(self.identity_maps.clone());
        (
          std::mem::take(&mut ctx.dependencies),
          std::mem::take(&mut ctx.disambiguator_map),
          parent_store,
          std::mem::take(&mut ctx.created_ids),
        )
      });

    // Check for cancellation before recomputing
    let storage = unsafe { db.storage() };
    if storage.cancelled.load(Ordering::Relaxed) {
      panic_any(Cancelled);
    }

    // Recompute
    #[cfg(debug_assertions)]
    self.recompute_count.fetch_add(1, Ordering::Relaxed);
    let key = arg.clone();
    let value = (self.query_fn)(db, arg);

    // Collect recorded dependencies, restore parent state, and pop stack
    let (dependencies, created_ids) = storage.with_context(|ctx| {
      let ctx = ctx
        .as_mut()
        .expect("context disappeared during query execution");
      let dependencies = std::mem::replace(&mut ctx.dependencies, parent_deps);
      ctx.disambiguator_map = parent_disambiguators;
      ctx.identity_maps = parent_store;
      let created_ids = std::mem::replace(&mut ctx.created_ids, parent_created_ids);
      ctx.query_stack.pop();
      (dependencies, created_ids)
    });

    // Remove identity map entries for structs not recreated in this execution
    for (start_index, active_ids) in &created_ids {
      if let Some(map) = self.identity_maps.get(&(arg_id, *start_index)) {
        map.retain_ids(active_ids);
      }
    }

    // Backdating: if the new value equals the old, keep the old changed_at
    // This prevents unnecessary invalidation of downstream queries
    let changed_at = match old_memo {
      Some((old_value, old_changed_at)) if old_value == value => old_changed_at,
      _ => current_revision,
    };

    // Store the result
    self.data.insert(
      arg_id,
      QueryState::Computed(StampedDerivedQuery {
        key,
        value: value.clone(),
        changed_at,
        verified_at: current_revision,
        dependencies,
      }),
    );

    (value, changed_at)
  }
}

impl<
  DB: QueryDatabase + Send + Sync + 'static,
  K: StableHash + std::fmt::Debug + Encodable + Decodable + Eq + Hash + Clone + Send + Sync + 'static,
  V: StableHash
    + std::fmt::Debug
    + Encodable
    + Decodable
    + DerivedId
    + Clone
    + PartialEq
    + Send
    + Sync
    + 'static,
> Ingredient for DerivedQueryIngredient<DB, K, V>
{
  #[cfg(debug_assertions)]
  fn readable_name(&self) -> String {
    self.readable_name.to_string()
  }

  fn name_fingerprint(&self) -> Fingerprint {
    self.name_fingerprint
  }

  /// Check the red-green algo here: https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html#improving-accuracy-the-red-green-algorithm
  /// We're similar in idea
  fn green_check(&self, db: &dyn QueryDatabase, arg_id: usize, last_changed_at: usize) -> bool {
    let storage = unsafe { db.storage() };
    let current_revision = storage.revision.load(Ordering::Acquire);

    match self.data.get(&arg_id) {
      Some(entry) => match &*entry {
        QueryState::Computed(memo) => {
          if memo.verified_at >= current_revision {
            return memo.changed_at <= last_changed_at;
          }
          // Stale: re-execute deps so they can backdate, then re-check
          let deps = memo.dependencies.clone();
          drop(entry);

          for dep in &deps {
            let ingredient = &storage.ingredients[dep.ingredient_index].ingredient;
            if !ingredient.green_check(db, dep.arg_id, dep.changed_at) {
              // Dep reports changed, force it to re-execute
              ingredient.re_execute(db, dep.arg_id);
            }
          }

          // Re-check whether are all deps green
          let all_green = deps.iter().all(|dep| {
            storage.ingredients[dep.ingredient_index]
              .ingredient
              .green_check(db, dep.arg_id, dep.changed_at)
          });

          if all_green {
            // Bump verified_at
            if let Some(mut entry) = self.data.get_mut(&arg_id)
              && let QueryState::Computed(memo) = &mut *entry
            {
              memo.verified_at = current_revision;
              return memo.changed_at <= last_changed_at;
            }
          }
          false
        }
        QueryState::Computing => false, // conservatively assume changed
      },
      None => false,
    }
  }

  fn re_execute(&self, db: &dyn QueryDatabase, arg_id: usize) {
    let db: &DB = (db as &dyn Any)
      .downcast_ref::<DB>()
      .expect("database type mismatch in re_execute");
    let key = match self.data.get(&arg_id) {
      Some(entry) => match &*entry {
        QueryState::Computed(memo) => Some(memo.key.clone()),
        QueryState::Computing => None,
      },
      None => None,
    };
    if let Some(key) = key {
      self.execute_query(db, key);
    }
  }

  fn entry_ids(&self) -> Box<dyn Iterator<Item = usize> + '_> {
    Box::new(self.data.iter().map(|entry| *entry.key()))
  }

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: usize) -> Option<Fingerprint> {
    DerivedQueryIngredient::value_fingerprint(self, db, entry_id)
  }

  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId> {
    if let Some(dep_id) = ctx.decoder.get_dep_node_id(node_index) {
      return Some(dep_id);
    }
    let node = &ctx.serialized.dep_graph.nodes[node_index as usize];
    let DepNode::DerivedQuery {
      value_entry_id: serialized_value_entry_id,
      changed_at,
      verified_at,
      edges,
      ..
    } = node
    else {
      return None;
    };
    // Register dep_id BEFORE decoding to prevent recursion via get_or_deserialize_dep_node_id (idempotency guard)
    let arg_id = self.next_arg_id.fetch_add(1, Ordering::Relaxed);
    let dep_id = (self.ingredient_index, arg_id);
    ctx.decoder.set_dep_node_id(node_index, dep_id);

    // Deserialize all sibling DerivedField nodes, which populates field data
    self.deserialize_field_group(ctx, *serialized_value_entry_id);

    // Get the session-local entry_id allocated by field deserialization
    let value_entry_id = *ctx
      .entry_id_map
      .entry((self.value_fingerprint, *serialized_value_entry_id))
      .or_insert_with(|| self.value_id_counter.fetch_add(1, Ordering::Relaxed));

    // Decode key
    let blob = ctx.serialized.query_cache.get(node_index)?;
    let mut data = blob;
    let key = K::decode(&mut data, &ctx.decoder);
    let value = V::from(value_entry_id);

    // FIXME: This can be optimized
    // We should only lazily load the dependencies
    // If we do, must perform cache promotion
    let dependencies = Self::deserialize_deps(edges, ctx);

    self.intern_map.entry(key.clone()).or_insert(arg_id);
    self.data.insert(
      arg_id,
      QueryState::Computed(StampedDerivedQuery {
        key,
        value,
        changed_at: *changed_at as usize,
        verified_at: *verified_at as usize,
        dependencies,
      }),
    );

    Some(dep_id)
  }

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: usize) {
    let Some(entry) = self.data.get(&entry_id) else {
      return;
    };
    let QueryState::Computed(memo) = &*entry else {
      return;
    };

    // Collect dependency edges as DepIds
    let edges = memo
      .dependencies
      .iter()
      .map(|dep| (dep.ingredient_index, dep.arg_id))
      .collect();

    let dep_id = (self.ingredient_index, entry_id);
    let node_index = ctx.encoder.add_dep_id(dep_id);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::DerivedQuery {
        name: self.name_fingerprint,
        key: self
          .key_fingerprint(ctx.db(), entry_id)
          .expect("Computed entry must have a key fingerprint"),
        value: self
          .value_fingerprint(ctx.db(), entry_id)
          .expect("Computed entry must have a value fingerprint"),
        entry_id: entry_id as u64,
        value_entry_id: memo.value.clone().into() as u64,
        changed_at: memo.changed_at as u64,
        verified_at: memo.verified_at as u64,
        edges,
      },
    );

    // Encode key into the query cache
    let mut buf = vec![];
    memo.key.encode(&mut buf, &mut ctx.encoder);
    ctx.query_cache.set(node_index, &buf);
  }

  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize {
    self.recompute_count.load(Ordering::Relaxed)
  }
}

/// A stamped field value for a derived struct
pub struct StampedDerivedField<T> {
  pub value: T,
  pub changed_at: usize, // The last revision number this one changed
}

/// Ingredient for a derived struct field: maps entry id to stamped value
#[derive(Clone)]
#[doc(hidden)]
pub struct DerivedFieldIngredient<T> {
  ingredient_index: usize,
  field_index: u8,
  name: &'static str,
  pub id_counter: &'static AtomicUsize,
  #[doc(hidden)]
  pub data: Arc<DashMap<usize, StampedDerivedField<T>>>,
}

impl<T> std::fmt::Debug for DerivedFieldIngredient<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DerivedFieldIngredient")
      .field("name", &self.name)
      .finish_non_exhaustive()
  }
}

impl<T> DerivedFieldIngredient<T> {
  #[cfg(debug_assertions)]
  #[doc(hidden)]
  pub const __TYPEDOWN_DERIVED_FIELD_INGREDIENT: () = ();

  pub fn new(
    ingredient_index: usize,
    name: &'static str,
    field_index: u8,
    id_counter: &'static AtomicUsize,
  ) -> Self {
    Self {
      ingredient_index,
      field_index,
      name,
      id_counter,
      data: Arc::new(DashMap::new()),
    }
  }
}

impl<T: StableHash + std::fmt::Debug + Encodable + Decodable + Send + Sync + 'static> Ingredient
  for DerivedFieldIngredient<T>
{
  #[cfg(debug_assertions)]
  fn readable_name(&self) -> String {
    self.name.to_string()
  }

  fn name_fingerprint(&self) -> Fingerprint {
    Fingerprint::from_name(self.name)
  }

  fn green_check(&self, _db: &dyn QueryDatabase, arg_id: usize, last_changed_at: usize) -> bool {
    self
      .data
      .get(&arg_id)
      .map(|entry| entry.changed_at <= last_changed_at)
      .unwrap_or(false)
  }

  fn re_execute(&self, _db: &dyn QueryDatabase, _arg_id: usize) {
    // Derived fields are set by the query, nothing to recompute
  }

  fn entry_ids(&self) -> Box<dyn Iterator<Item = usize> + '_> {
    Box::new(self.data.iter().map(|entry| *entry.key()))
  }

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: usize) -> Option<Fingerprint> {
    DerivedFieldIngredient::value_fingerprint(self, db, entry_id)
  }

  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId> {
    if let Some(dep_id) = ctx.decoder.get_dep_node_id(node_index) {
      return Some(dep_id);
    }
    let node = &ctx.serialized.dep_graph.nodes[node_index as usize];
    let DepNode::DerivedField {
      name,
      entry_id: serialized_entry_id,
      changed_at,
      ..
    } = node
    else {
      return None;
    };

    // Allocate a session-local entry_id, shared across sibling fields
    let entry_id = *ctx
      .entry_id_map
      .entry((*name, *serialized_entry_id))
      .or_insert_with(|| self.id_counter.fetch_add(1, Ordering::Relaxed));

    let blob = ctx.serialized.query_cache.get(node_index)?;
    let mut data = blob;
    let value = T::decode(&mut data, &ctx.decoder);
    self.data.insert(
      entry_id,
      StampedDerivedField {
        value,
        changed_at: *changed_at as usize,
      },
    );

    let dep_id = (self.ingredient_index, entry_id);
    ctx.decoder.set_dep_node_id(node_index, dep_id);

    // Trigger deserialization of sibling fields so the whole struct is populated
    let group_key = (*name, *serialized_entry_id);
    if let Some(field_group) = ctx.derived_groups.get(&group_key) {
      for &(_, sibling_node_index) in &field_group.fields {
        ctx
          .decoder
          .get_or_deserialize_dep_node_id(sibling_node_index);
      }
    }

    Some(dep_id)
  }

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: usize) {
    let Some(entry) = self.data.get(&entry_id) else {
      return;
    };

    let dep_id = (self.ingredient_index, entry_id);
    let node_index = ctx.encoder.add_dep_id(dep_id);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::DerivedField {
        name: self.name_fingerprint(),
        field_index: self.field_index,
        entry_id: entry_id as u64,
        value: self
          .value_fingerprint(ctx.db(), entry_id)
          .expect("Entry is available so there must be a fingerprint"),
        changed_at: entry.changed_at as u64,
      },
    );

    // Write field value blob
    let mut buf = vec![];
    entry.value.encode(&mut buf, &mut ctx.encoder);
    ctx.query_cache.set(node_index, &buf);
  }

  // Derived fields are set by their parent query, not independently recomputed
  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize {
    0
  }
}

impl<T: StableHash + Send + Sync + 'static> DerivedFieldIngredient<T> {
  pub fn value_fingerprint(&self, db: &dyn QueryDatabase, arg_id: usize) -> Option<Fingerprint> {
    self.data.get(&arg_id).map(|entry| {
      let mut hasher: StableHasher = StableHasher::new();
      entry.value.stable_hash(db, &mut hasher);
      Fingerprint::from_hasher(hasher)
    })
  }
}
