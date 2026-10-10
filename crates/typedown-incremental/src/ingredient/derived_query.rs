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

use crate::{
  Cancelled, DerivedIdentity, EntryId, ExecuteContext, IdentityMapTable, QueryStackEntry,
  QueryStorage, Revision,
};
use crate::{
  Decodable, DepId, Encodable, Fingerprint, Id, LazyFingerprint, QueryDatabase, StableHash,
  StableHasher,
};
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
pub struct StampedDerivedQuery<K, V: Id + From<u32> + Into<u32>> {
  pub key: K,                        // The original key, for re-execution
  pub value: V,                      // The derived struct ID
  pub changed_at: Revision,          // Revision when the value last actually changed
  pub verified_at: AtomicU32,        // Revision when last confirmed valid
  pub dependencies: Vec<Dependency>, // What this query read during execution
  pub key_fingerprint: LazyFingerprint,
  pub value_fingerprint: LazyFingerprint,
  // (name_fingerprint, identity_hash, disambiguator, entry_id) for derived structs created in this execution
  pub derived_query_identities: Vec<DerivedIdentity>,
}

/// The state of a query entry in the cache
pub enum QueryState<K, V: Id + From<u32> + Into<u32>> {
  /// The query is currently being computed
  Computing,
  /// The query has a cached result
  Computed(StampedDerivedQuery<K, V>),
}

/// Ingredient for a derived query function: maps key tuple to memoized result
#[derive(Clone)]
#[doc(hidden)]
pub struct DerivedQueryIngredientStore<DB, K, V: Id + From<u32> + Into<u32>> {
  pub(crate) ingredient_id: DepId, // A DepId with entry_id = 0, identifies this ingredient
  pub(crate) name_fingerprint: Fingerprint,
  pub(crate) ret_typ_fingerprint: Fingerprint, // Fingerprint of the return type name (e.g. "FibResult")
  pub(crate) next_entry_id: Arc<AtomicU32>,
  query_func: fn(&DB, K) -> V,
  pub(crate) intern_map: Arc<DashMap<K, EntryId>>, // Key -> stable entry_id
  #[doc(hidden)]
  pub data: Arc<IdDashMap<QueryState<K, V>>>, // Entry ID -> state
  pub(crate) identity_maps: IdentityMapTable,
  pub no_hash_flag: bool,
  #[cfg(debug_assertions)]
  recompute_count: Arc<AtomicU32>,
  pub(crate) readable_name: &'static str,
}

impl<DB, K, V: Id + From<u32> + Into<u32>> std::fmt::Debug
  for DerivedQueryIngredientStore<DB, K, V>
{
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct(self.readable_name).finish_non_exhaustive()
  }
}

impl<
  DB: QueryDatabase + Send + Sync + 'static,
  K: StableHash + std::fmt::Debug + Encodable + Decodable + Eq + Hash + Clone + Send + Sync + 'static,
  V: StableHash
    + std::fmt::Debug
    + Encodable
    + Decodable
    + Id
    + From<u32>
    + Into<u32>
    + Clone
    + PartialEq
    + Send
    + Sync
    + 'static,
> DerivedQueryIngredientStore<DB, K, V>
{
  pub fn new(
    ingredient_id: DepId,
    name_fingerprint: &'static str,
    ret_typ_name: &'static str,
    query_func: fn(&DB, K) -> V,
  ) -> Self {
    Self {
      ingredient_id,
      name_fingerprint: Fingerprint::from_name(name_fingerprint),
      ret_typ_fingerprint: Fingerprint::from_name(ret_typ_name),
      next_entry_id: Arc::new(AtomicU32::new(0)),
      query_func,
      intern_map: Arc::new(DashMap::new()),
      data: Arc::new(IdDashMap::default()),
      identity_maps: Arc::new(DashMap::new()),
      no_hash_flag: false,
      #[cfg(debug_assertions)]
      recompute_count: Arc::new(AtomicU32::new(0)),
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

  /// Get or create a stable entry ID for a key
  pub(crate) fn get_or_create_entry_id(&self, arg: &K) -> EntryId {
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

  /// Cache lookup with three-tier fallback:
  /// 1. Already verified this revision -> return immediately
  /// 2. Stale memo in memory -> green_check (walk deps, backdate if all green, else recompute)
  /// 3. Not in memory -> lazy-load from previous session cache (cross-session fingerprint check)
  /// 4. Cache miss or fingerprint mismatch -> recompute via query_func
  fn execute_query_inner(
    &self,
    db: &DB,
    storage: &QueryStorage,
    current_revision: Revision,
    query_entry_id: EntryId,
    arg: K,
  ) -> (V, Revision) {
    // In-memory cache
    if let Some(entry) = self.data.get(&query_entry_id) {
      match &*entry {
        // Hot path: already verified this revision
        QueryState::Computed(memo)
          if memo.verified_at.load(Ordering::Acquire) >= current_revision =>
        {
          return (memo.value.clone(), memo.changed_at);
        }
        QueryState::Computing => {
          // Cycle detection: Check if this entry is in our call stack
          let dep_id = self.ingredient_id.with_entry(query_entry_id);
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
        // Stale memo: green check walks deps and backdates if all still match
        QueryState::Computed(memo) => {
          let changed_at = memo.changed_at;
          // Release the read lock before walking deps
          drop(entry);

          if DerivedQueryIngredient::green_check(self, db, query_entry_id, changed_at)
            && let Some(entry) = self.data.get(&query_entry_id)
            && let QueryState::Computed(memo) = &*entry
          {
            return (memo.value.clone(), memo.changed_at);
          }
        }
      }
    }

    // Previous session cache: deserialize deps on demand, validate fingerprints
    // On success, memo is in self.data with the stored changed_at from the previous session
    // Also seeds self.identity_maps as a side effect (on both green and stale paths)
    if self.try_load_cached_query_memo(db, storage, &arg)
      && let Some(entry) = self.data.get(&query_entry_id)
      && let QueryState::Computed(memo) = &*entry
    {
      return (memo.value.clone(), memo.changed_at);
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
      .entry(query_entry_id)
      .and_modify(|state| {
        if let QueryState::Computed(memo) = state {
          if memo.verified_at.load(Ordering::Acquire) >= current_revision {
            cached = Some((memo.value.clone(), memo.changed_at));
            return;
          }
          // Save old value for backdating
          old_memo = Some((memo.value.clone(), memo.changed_at));
        }
        *state = QueryState::Computing;
      })
      .or_insert(QueryState::Computing);

    if let Some((value, changed_at)) = cached {
      return (value, changed_at);
    }

    // Save parent context and push to query stack
    let dep_id = self.ingredient_id.with_entry(query_entry_id);
    let (
      parent_deps,
      parent_disambiguators,
      parent_identity_maps,
      parent_derived_identities,
      parent_created_ids,
    ) = storage.with_context(|ctx| {
      let ctx = ctx.get_or_insert_with(|| ExecuteContext {
        query_stack: Vec::new(),
        dependencies: Vec::new(),
        disambiguator_map: HashMap::new(),
        identity_maps: None,
        derived_identities: Vec::new(),
        created_ids: HashMap::new(),
      });
      ctx.query_stack.push(QueryStackEntry { dep_id });
      let parent_store = ctx.identity_maps.replace(self.identity_maps.clone());
      (
        std::mem::take(&mut ctx.dependencies),
        std::mem::take(&mut ctx.disambiguator_map),
        parent_store,
        std::mem::take(&mut ctx.derived_identities),
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
      (self.query_func)(db, arg)
    }));

    // Collect recorded dependencies, restore parent state, and pop stack
    let (dependencies, derived_identities, created_ids) = storage.with_context(|ctx| {
      let ctx = ctx
        .as_mut()
        .expect("context disappeared during query execution");
      let dependencies = std::mem::replace(&mut ctx.dependencies, parent_deps);
      ctx.disambiguator_map = parent_disambiguators;
      ctx.identity_maps = parent_identity_maps;
      let derived_identities =
        std::mem::replace(&mut ctx.derived_identities, parent_derived_identities);
      let created_ids = std::mem::replace(&mut ctx.created_ids, parent_created_ids);
      ctx.query_stack.pop();
      (dependencies, derived_identities, created_ids)
    });

    let value = match execute_result {
      Ok(value) => value,
      Err(payload) => {
        // Remove stale Computing state so other threads don't see it
        self.data.remove(&query_entry_id);
        resume_unwind(payload);
      }
    };

    // Remove identity map entries for structs not recreated
    self.cleanup_identity_maps(storage, query_entry_id, &created_ids);

    // Backdating: if the new value equals the old, keep the old changed_at
    // This prevents unnecessary invalidation of downstream queries
    let changed_at = match old_memo {
      Some((old_value, old_changed_at)) if old_value == value => old_changed_at,
      _ => current_revision,
    };

    // Store the result
    self.data.insert(
      query_entry_id,
      QueryState::Computed(StampedDerivedQuery {
        key,
        value: value.clone(),
        changed_at,
        verified_at: AtomicU32::new(current_revision),
        dependencies,
        key_fingerprint: LazyFingerprint::new(),
        value_fingerprint: if self.no_hash_flag {
          LazyFingerprint::from_stored(Fingerprint::SKIPPED)
        } else {
          LazyFingerprint::new()
        },
        derived_query_identities: derived_identities,
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

      let mut removed = HashSet::new();
      map.retain(|_, id| {
        if active_ids.contains(id) {
          true
        } else {
          removed.insert(*id);
          false
        }
      });

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
    + Id
    + From<u32>
    + Into<u32>
    + Clone
    + PartialEq
    + Send
    + Sync
    + 'static,
> Ingredient for DerivedQueryIngredientStore<DB, K, V>
{
  fn readable_name(&self) -> String {
    self.readable_name.to_string()
  }

  fn name_fingerprint(&self) -> Fingerprint {
    self.name_fingerprint
  }

  fn entry_ids(&self) -> Box<dyn Iterator<Item = EntryId> + '_> {
    Box::new(self.data.iter().map(|entry| *entry.key()))
  }

  fn value_fingerprint(&self, db: &dyn QueryDatabase, entry_id: EntryId) -> Option<Fingerprint> {
    if let Some(entry) = self.data.get(&entry_id)
      && let QueryState::Computed(memo) = &*entry
    {
      return Some(memo.value_fingerprint.get_or_compute(db, &memo.value));
    }
    None
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
    + Id
    + From<u32>
    + Into<u32>
    + Clone
    + PartialEq
    + Send
    + Sync
    + 'static,
> DerivedQueryIngredient for DerivedQueryIngredientStore<DB, K, V>
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
  ///
  /// 1. Walk each dep: re-execute any red dep so it can backdate its own verified_at
  /// 2. If all deps end up green, bump this memo's verified_at and return true
  /// 3. If any dep is still red, this returns false, the caller is responsible for recomputing
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
          // Already verified this revision: compare timestamps only
          if memo.verified_at.load(Ordering::Acquire) >= current_revision {
            return memo.changed_at <= last_changed_at;
          }
          // Stale memo: walk deps & re-execute any that appear red so they can backdate
          let deps = memo.dependencies.clone();
          drop(entry);

          for dep in &deps {
            if !storage.green_check_dep(db, dep) {
              // Dep reports changed, force it to re-execute so it can backdate itself
              storage.re_execute_dep(db, dep);
            }
          }

          // Re-check all deps after re-execution
          let all_green = deps.iter().all(|dep| storage.green_check_dep(db, dep));

          if all_green {
            // All deps green: bump verified_at (backdate) and report green
            if let Some(entry) = self.data.get(&entry_id)
              && let QueryState::Computed(memo) = &*entry
            {
              memo.verified_at.store(current_revision, Ordering::Release);
              return memo.changed_at <= last_changed_at;
            }
          }
          // At least one dep still changed: report red, caller recomputes
          false
        }
        QueryState::Computing => false, // Conservatively assume changed
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
}
