use std::{
  any::Any,
  collections::{HashMap, HashSet},
  hash::Hash,
  panic::{AssertUnwindSafe, catch_unwind, panic_any, resume_unwind},
  sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
  },
};

use crate::persist::serialized::dep_graph::{DepNode, DepNodeIndex};
use crate::{
  Cancelled, EntryId, ExecuteContext, IdentityMapTable, QueryStackEntry, QueryStorage, Revision,
};
use crate::{
  Decodable, DepId, DeserializeContext, Encodable, Fingerprint, StableHash, StableHasher,
};
use crate::{DerivedId, QueryDatabase, SerializeContext, UnresolvedDepNode};
use dashmap::DashMap;

use super::{DerivedQueryIngredient, IdDashMap, Ingredient};

pub const LRU_CAPACITY: usize = 1024;

/// A dependency recorded during a derived query execution
#[derive(Clone)]
pub struct Dependency {
  pub dep_id: DepId,
  pub changed_at: Revision,
}

/// A memoized derived query result
// TIL: verified_at is AtomicU32 so green_check can bump it via get() instead of get_mut()
pub struct StampedDerivedQuery<K, V: DerivedId> {
  pub key: K,                        // The original key, for re-execution
  pub value: V,                      // The derived struct ID
  pub changed_at: Revision,          // Revision when the value last actually changed
  pub verified_at: AtomicU32,        // Revision when last confirmed valid
  pub dependencies: Vec<Dependency>, // What this query read during execution
}

/// The state of a query entry in the cache
pub enum QueryState<K, V: DerivedId> {
  /// The query is currently being computed
  Computing,
  /// The query has a cached result
  Computed(StampedDerivedQuery<K, V>),
}

/// A stamped field value for a derived struct
pub struct StampedDerivedField<T> {
  pub value: T,
  pub changed_at: Revision,
}

/// Ingredient for a derived query function: maps key tuple to memoized result
#[derive(Clone)]
#[doc(hidden)]
pub struct DerivedQueryIngredientStore<DB, K, V: DerivedId> {
  ingredient_id: DepId, // DepId with entry_id=0, identifies this ingredient
  name_fingerprint: Fingerprint,
  return_type_fingerprint: Fingerprint, // fingerprint of the return type name (e.g. "FibResult")
  next_entry_id: Arc<AtomicU32>,
  value_id_counter: &'static AtomicU32,
  query_fn: fn(&DB, K) -> V,
  intern_map: Arc<DashMap<K, EntryId>>, // key -> stable entry_id
  #[doc(hidden)]
  pub data: Arc<IdDashMap<QueryState<K, V>>>, // entry_id -> state
  identity_maps: IdentityMapTable,
  pub no_hash_flag: bool,
  #[cfg(debug_assertions)]
  recompute_count: Arc<AtomicU32>,
  #[cfg(debug_assertions)]
  readable_name: &'static str,
}

impl<DB, K, V: DerivedId> std::fmt::Debug for DerivedQueryIngredientStore<DB, K, V> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    let name = {
      #[cfg(debug_assertions)]
      {
        self.readable_name
      }
      #[cfg(not(debug_assertions))]
      {
        "DerivedQueryIngredientStore"
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
> DerivedQueryIngredientStore<DB, K, V>
{
  /// Deserialize all sibling DerivedField nodes for a value struct.
  fn deserialize_field_group(&self, ctx: &DeserializeContext, serialized_entry_id: u64) {
    let group_key = (self.return_type_fingerprint, serialized_entry_id);
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
          dep_id,
          changed_at: edge_node.changed_at() as u32,
        })
      })
      .collect()
  }

  pub fn new(
    ingredient_id: DepId,
    name_fingerprint: &'static str,
    return_type_name: &'static str,
    value_id_counter: &'static AtomicU32,
    query_fn: fn(&DB, K) -> V,
  ) -> Self {
    Self {
      ingredient_id,
      name_fingerprint: Fingerprint::from_name(name_fingerprint),
      return_type_fingerprint: Fingerprint::from_name(return_type_name),
      next_entry_id: Arc::new(AtomicU32::new(0)),
      value_id_counter,
      query_fn,
      intern_map: Arc::new(DashMap::new()),
      data: Arc::new(IdDashMap::default()),
      identity_maps: Arc::new(DashMap::new()),
      no_hash_flag: false,
      #[cfg(debug_assertions)]
      recompute_count: Arc::new(AtomicU32::new(0)),
      #[cfg(debug_assertions)]
      readable_name: name_fingerprint,
    }
  }

  pub fn key_fingerprint(&self, db: &dyn QueryDatabase, entry_id: EntryId) -> Option<Fingerprint>
  where
    K: StableHash,
  {
    let db = (db as &dyn Any)
      .downcast_ref::<DB>()
      .expect("database type mismatch in key_fingerprint");
    if let Some(entry) = self.data.get(&entry_id)
      && let QueryState::Computed(memo) = &*entry
    {
      let mut hasher: StableHasher = StableHasher::new();
      memo.key.stable_hash(db, &mut hasher);
      return Some(Fingerprint::from_hasher(hasher));
    }
    None
  }

  // Try to load a serialized derived query node from the previous session's cache
  // Validates that all dependencies still have matching fingerprints (cross-session green check)
  fn try_load_serialized_derived_query_node(
    &self,
    db: &DB,
    storage: &QueryStorage,
    arg: &K,
  ) -> Option<(V, Revision)> {
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

    // Load all dep nodes first then perform
    // cross-session green check: verify each dependency's fingerprint still matches
    let decoder = &ctx.decoder;
    for &edge_idx in edges {
      let edge_node = &ctx.serialized.dep_graph.nodes[edge_idx as usize];
      if matches!(edge_node, DepNode::Evicted) {
        return None;
      }
      let Some(dep_id) = decoder.get_or_deserialize_dep_node_id(edge_idx) else {
        return None;
      };
      // no_hash deps: skip fingerprint check, handled by runtime green_check
      let expected_fp = edge_node.value_fingerprint();
      if expected_fp == Fingerprint::SKIPPED {
        continue;
      }
      let actual_fp = storage.value_fingerprint_by_dep_id(db, dep_id);
      if actual_fp != Some(expected_fp) {
        return None;
      }
    }

    // Load returned value
    self.deserialize_field_group(ctx, *serialized_value_entry_id);
    let value_entry_id = *ctx
      .entry_id_map
      .entry((self.return_type_fingerprint, *serialized_value_entry_id))
      .or_insert_with(|| self.value_id_counter.fetch_add(1, Ordering::Relaxed));

    // Decode key
    let blob = ctx.serialized.query_cache.get(node_index)?;
    let mut data: &[u8] = blob;
    let key = K::decode(&mut data, decoder);
    let entry_id = self.get_or_create_entry_id(&key);
    let value = V::from(value_entry_id);
    let changed_at = *changed_at as u32;
    let dependencies = Self::deserialize_deps(edges, ctx);
    let current_revision = storage.revision.load(Ordering::Acquire);
    self.data.insert(
      entry_id,
      QueryState::Computed(StampedDerivedQuery {
        key,
        value: value.clone(),
        changed_at,
        verified_at: AtomicU32::new(current_revision),
        dependencies,
      }),
    );

    Some((value, changed_at))
  }

  /// Get or create a stable entry ID for a key
  fn get_or_create_entry_id(&self, arg: &K) -> EntryId {
    if let Some(entry) = self.intern_map.get(arg) {
      return *entry.value();
    }
    let entry_id = self.next_entry_id.fetch_add(1, Ordering::Relaxed);
    *self
      .intern_map
      .entry(arg.clone())
      .or_insert(entry_id)
      .value()
  }

  /// Execute a derived query: returns cached result if valid, otherwise runs the query function
  pub fn execute_query(&self, db: &DB, arg: K) -> V {
    let storage = unsafe { db.storage() };
    let current_revision = storage.revision.load(Ordering::Acquire);
    let entry_id = self.get_or_create_entry_id(&arg);

    let (value, changed_at) =
      self.execute_query_inner(db, storage, current_revision, entry_id, arg);

    // Record dependency for the caller
    let dep_id = self.ingredient_id.with_entry(entry_id);
    storage.with_context(|ctx| {
      if let Some(ctx) = ctx {
        ctx.dependencies.push(Dependency { dep_id, changed_at });
      }
    });

    value
  }

  /// Inner implementation that returns (value, changed_at)
  fn execute_query_inner(
    &self,
    db: &DB,
    storage: &QueryStorage,
    current_revision: Revision,
    entry_id: EntryId,
    arg: K,
  ) -> (V, Revision) {
    // Check cache
    if let Some(entry) = self.data.get(&entry_id) {
      match &*entry {
        QueryState::Computed(memo)
          if memo.verified_at.load(Ordering::Acquire) >= current_revision =>
        {
          return (memo.value.clone(), memo.changed_at);
        }
        QueryState::Computing => {
          // Cycle detection: Check if this entry is in our call stack
          let dep_id = self.ingredient_id.with_entry(entry_id);
          let is_cycle = storage.with_context(|ctx| {
            ctx
              .as_ref()
              .is_some_and(|ctx| ctx.query_stack.iter().any(|e| e.dep_id == dep_id))
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

          if DerivedQueryIngredient::green_check(self, db, entry_id, changed_at) {
            // The green check has verified or recomputed + backdated so the entry must now be fresh
            if let Some(entry) = self.data.get(&entry_id)
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
    if let Some((value, changed_at)) =
      self.try_load_serialized_derived_query_node(db, storage, &arg)
    {
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
      .entry(entry_id)
      .and_modify(|state| {
        if let QueryState::Computed(memo) = state {
          if memo.verified_at.load(Ordering::Acquire) >= current_revision {
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
    let dep_id = self.ingredient_id.with_entry(entry_id);
    let (parent_deps, parent_disambiguators, parent_identity_maps, parent_created_ids) = storage
      .with_context(|ctx| {
        let ctx = ctx.get_or_insert_with(|| ExecuteContext {
          query_stack: Vec::new(),
          dependencies: Vec::new(),
          disambiguator_map: HashMap::new(),
          identity_maps: None,
          created_ids: HashMap::new(),
        });
        ctx.query_stack.push(QueryStackEntry { dep_id });
        let parent_store = ctx.identity_maps.replace(self.identity_maps.clone());
        (
          std::mem::take(&mut ctx.dependencies),
          std::mem::take(&mut ctx.disambiguator_map),
          parent_store,
          std::mem::take(&mut ctx.created_ids),
        )
      });

    let storage = unsafe { db.storage() };

    // Recompute
    #[cfg(debug_assertions)]
    self.recompute_count.fetch_add(1, Ordering::Relaxed);
    let key = arg.clone();
    let execute_result = catch_unwind(AssertUnwindSafe(|| {
      // Check for cancellation before recomputing
      if storage.cancelled.load(Ordering::Relaxed) {
        panic_any(Cancelled);
      }
      (self.query_fn)(db, arg)
    }));

    // Collect recorded dependencies, restore parent state, and pop stack
    let (dependencies, created_ids) = storage.with_context(|ctx| {
      let ctx = ctx
        .as_mut()
        .expect("context disappeared during query execution");
      let dependencies = std::mem::replace(&mut ctx.dependencies, parent_deps);
      ctx.disambiguator_map = parent_disambiguators;
      ctx.identity_maps = parent_identity_maps;
      let created_ids = std::mem::replace(&mut ctx.created_ids, parent_created_ids);
      ctx.query_stack.pop();
      (dependencies, created_ids)
    });

    let value = match execute_result {
      Ok(v) => v,
      Err(payload) => {
        // Remove stale Computing state so other threads don't see it
        self.data.remove(&entry_id);
        resume_unwind(payload);
      }
    };

    // Remove identity map entries for structs not recreated
    self.cleanup_identity_maps(storage, entry_id, &created_ids);

    // Backdating: if the new value equals the old, keep the old changed_at
    // This prevents unnecessary invalidation of downstream queries
    let changed_at = match old_memo {
      Some((old_value, old_changed_at)) if old_value == value => old_changed_at,
      _ => current_revision,
    };

    // Store the result
    self.data.insert(
      entry_id,
      QueryState::Computed(StampedDerivedQuery {
        key,
        value: value.clone(),
        changed_at,
        verified_at: AtomicU32::new(current_revision),
        dependencies,
      }),
    );

    (value, changed_at)
  }

  /// Remove identity map entries for structs not recreated during recomputation
  fn cleanup_identity_maps(
    &self,
    storage: &QueryStorage,
    entry_id: EntryId,
    created_ids: &HashMap<EntryId, HashSet<EntryId>>,
  ) {
    for (start_index, active_ids) in created_ids {
      let Some(map) = self.identity_maps.get(&(entry_id, *start_index)) else {
        continue;
      };
      let removed = map.retain(&|id| active_ids.contains(&id));
      if removed.is_empty() {
        continue;
      }
      // Remove field data for sibling field ingredients
      storage.remove_field_entries(*start_index, &removed);
    }
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
> Ingredient for DerivedQueryIngredientStore<DB, K, V>
{
  #[cfg(debug_assertions)]
  fn readable_name(&self) -> String {
    self.readable_name.to_string()
  }

  fn name_fingerprint(&self) -> Fingerprint {
    self.name_fingerprint
  }

  fn entry_ids(&self) -> Box<dyn Iterator<Item = EntryId> + '_> {
    Box::new(self.data.iter().map(|entry| *entry.key()))
  }

  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize {
    self.recompute_count.load(Ordering::Relaxed) as usize
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
> super::DerivedQueryIngredient for DerivedQueryIngredientStore<DB, K, V>
{
  fn reset_for_new_revision(&self) {
    // Evict entries not verified in recent revisions
    if self.data.len() <= LRU_CAPACITY {
      return;
    }
    let mut entries: Vec<(u32, u32)> = self
      .data
      .iter()
      .filter_map(|entry| {
        if let QueryState::Computed(memo) = &*entry {
          Some((*entry.key(), memo.verified_at.load(Ordering::Relaxed)))
        } else {
          None
        }
      })
      .collect();
    if entries.len() <= LRU_CAPACITY {
      return;
    }
    // Sort by verified_at ascending (oldest first)
    entries.sort_unstable_by_key(|&(_, rev)| rev);
    let evict_count = entries.len() - LRU_CAPACITY;
    for &(entry_id, _) in entries.iter().take(evict_count) {
      self.data.remove(&entry_id);
    }
  }

  /// Red-green algorithm: https://rustc-dev-guide.rust-lang.org/queries/incremental-compilation-in-detail.html#improving-accuracy-the-red-green-algorithm
  fn green_check(
    &self,
    db: &dyn QueryDatabase,
    entry_id: EntryId,
    last_changed_at: Revision,
  ) -> bool {
    let storage = unsafe { db.storage() };
    let current_revision = storage.revision.load(Ordering::Acquire);

    match self.data.get(&entry_id) {
      Some(entry) => match &*entry {
        QueryState::Computed(memo) => {
          if memo.verified_at.load(Ordering::Acquire) >= current_revision {
            return memo.changed_at <= last_changed_at;
          }
          // Stale: re-execute deps so they can backdate, then re-check
          let deps = memo.dependencies.clone();
          drop(entry);

          for dep in &deps {
            if !storage.green_check_dep(db, dep) {
              // Dep reports changed, force it to re-execute
              storage.re_execute_dep(db, dep);
            }
          }

          // Re-check whether all deps are green
          let all_green = deps.iter().all(|dep| storage.green_check_dep(db, dep));

          if all_green {
            // Bump verified_at
            if let Some(entry) = self.data.get(&entry_id)
              && let QueryState::Computed(memo) = &*entry
            {
              memo.verified_at.store(current_revision, Ordering::Release);
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

  fn re_execute(&self, db: &dyn QueryDatabase, entry_id: EntryId) {
    let db: &DB = (db as &dyn Any)
      .downcast_ref::<DB>()
      .expect("database type mismatch in re_execute");
    let key = match self.data.get(&entry_id) {
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

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: EntryId) -> Option<Fingerprint> {
    let db = (db as &dyn Any)
      .downcast_ref::<DB>()
      .expect("database type mismatch in value_fingerprint");
    if let Some(entry) = self.data.get(&entry_id)
      && let QueryState::Computed(memo) = &*entry
    {
      let mut hasher: StableHasher = StableHasher::new();
      memo.value.stable_hash(db, &mut hasher);
      return Some(Fingerprint::from_hasher(hasher));
    }
    None
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
    let entry_id = self.next_entry_id.fetch_add(1, Ordering::Relaxed);
    let dep_id = self.ingredient_id.with_entry(entry_id);
    ctx.decoder.set_dep_node_id(node_index, dep_id);

    // Deserialize all sibling DerivedField nodes, which populates field data
    self.deserialize_field_group(ctx, *serialized_value_entry_id);

    // Get the session-local entry_id allocated by field deserialization
    let value_entry_id = *ctx
      .entry_id_map
      .entry((self.return_type_fingerprint, *serialized_value_entry_id))
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

    self.intern_map.entry(key.clone()).or_insert(entry_id);
    self.data.insert(
      entry_id,
      QueryState::Computed(StampedDerivedQuery {
        key,
        value,
        changed_at: *changed_at as u32,
        verified_at: AtomicU32::new(*verified_at as u32),
        dependencies,
      }),
    );

    Some(dep_id)
  }

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: EntryId) {
    let Some(entry) = self.data.get(&entry_id) else {
      return;
    };
    let QueryState::Computed(memo) = &*entry else {
      return;
    };

    // Collect dependency edges as DepIds
    let edges = memo.dependencies.iter().map(|dep| dep.dep_id).collect();

    let dep_id = self.ingredient_id.with_entry(entry_id);
    let node_index = ctx.encoder.add_dep_id(dep_id);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::DerivedQuery {
        name: self.name_fingerprint,
        key: self
          .key_fingerprint(ctx.db(), entry_id)
          .expect("Computed entry must have a key fingerprint"),
        value: if self.no_hash_flag {
          Fingerprint::SKIPPED
        } else {
          self
            .value_fingerprint(ctx.db(), entry_id)
            .expect("Computed entry must have a value fingerprint")
        },
        entry_id: entry_id as u64,
        value_entry_id: <V as Into<u32>>::into(memo.value.clone()) as u64,
        changed_at: memo.changed_at as u64,
        verified_at: memo.verified_at.load(Ordering::Relaxed) as u64,
        edges,
      },
    );

    // Encode key into the query cache
    let mut buf = vec![];
    memo.key.encode(&mut buf, &mut ctx.encoder);
    ctx.query_cache.set(node_index, &buf);
  }

  fn no_hash(&self) -> bool {
    self.no_hash_flag
  }
}

/// Ingredient for a derived struct field: maps entry id to stamped value
#[derive(Clone)]
#[doc(hidden)]
pub struct DerivedFieldIngredientStore<T> {
  ingredient_id: DepId,
  field_index: u8,
  // Parent struct name, shared across sibling fields so they map to the same entry ID
  struct_name: &'static str,
  pub id_counter: &'static AtomicU32,
  #[doc(hidden)]
  pub data: Arc<IdDashMap<StampedDerivedField<T>>>,
  pub no_hash_flag: bool,
}

impl<T> std::fmt::Debug for DerivedFieldIngredientStore<T> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("DerivedFieldIngredientStore")
      .field("struct_name", &self.struct_name)
      .finish_non_exhaustive()
  }
}

impl<T> DerivedFieldIngredientStore<T> {
  #[cfg(debug_assertions)]
  #[doc(hidden)]
  pub const __TYPEDOWN_DERIVED_FIELD_INGREDIENT: () = ();

  pub fn new(
    ingredient_id: DepId,
    struct_name: &'static str,
    field_index: u8,
    id_counter: &'static AtomicU32,
  ) -> Self {
    Self {
      ingredient_id,
      field_index,
      struct_name,
      id_counter,
      data: Arc::new(IdDashMap::default()),
      no_hash_flag: false,
    }
  }
}

impl<T: StableHash + std::fmt::Debug + Encodable + Decodable + Send + Sync + 'static> Ingredient
  for DerivedFieldIngredientStore<T>
{
  #[cfg(debug_assertions)]
  fn readable_name(&self) -> String {
    self.struct_name.to_string()
  }

  fn name_fingerprint(&self) -> Fingerprint {
    Fingerprint::from_name(self.struct_name)
  }

  fn entry_ids(&self) -> Box<dyn Iterator<Item = EntryId> + '_> {
    Box::new(self.data.iter().map(|entry| *entry.key()))
  }

  // Derived fields are set by their parent query, not independently recomputed
  #[cfg(debug_assertions)]
  fn recompute_count(&self) -> usize {
    0
  }
}

impl<T: StableHash + std::fmt::Debug + Encodable + Decodable + Send + Sync + 'static>
  super::DerivedFieldIngredient for DerivedFieldIngredientStore<T>
{
  fn remove_entry(&self, entry_id: EntryId) {
    self.data.remove(&entry_id);
  }

  fn green_check(&self, entry_id: EntryId, last_changed_at: Revision) -> bool {
    self
      .data
      .get(&entry_id)
      .map(|entry| entry.changed_at <= last_changed_at)
      .unwrap_or(false)
  }

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: EntryId) -> Option<Fingerprint> {
    self.data.get(&entry_id).map(|entry| {
      let mut hasher: StableHasher = StableHasher::new();
      entry.value.stable_hash(db, &mut hasher);
      Fingerprint::from_hasher(hasher)
    })
  }

  fn field_index(&self) -> u8 {
    self.field_index
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
        changed_at: *changed_at as u32,
      },
    );

    let dep_id = self.ingredient_id.with_entry(entry_id);
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

  fn serialize(&self, ctx: &mut SerializeContext, entry_id: EntryId) {
    let Some(entry) = self.data.get(&entry_id) else {
      return;
    };

    let dep_id = self.ingredient_id.with_entry(entry_id);
    let node_index = ctx.encoder.add_dep_id(dep_id);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::DerivedField {
        name: self.name_fingerprint(),
        field_index: self.field_index,
        entry_id: entry_id as u64,
        value: if self.no_hash_flag {
          Fingerprint::SKIPPED
        } else {
          self
            .value_fingerprint(ctx.db(), entry_id)
            .expect("Entry is available so there must be a fingerprint")
        },
        changed_at: entry.changed_at as u64,
      },
    );

    // Write field value blob
    let mut buf = vec![];
    entry.value.encode(&mut buf, &mut ctx.encoder);
    ctx.query_cache.set(node_index, &buf);
  }

  fn no_hash(&self) -> bool {
    self.no_hash_flag
  }
}
