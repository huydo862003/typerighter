use std::{
  cmp::Reverse,
  hash::Hash,
  sync::{Arc, atomic::Ordering},
};

use dashmap::DashMap;

use crate::serial::format::binary_files::dep_graph::{DepNode, DepNodeIndex};
use crate::serial::ingredient::serde::{DeserializeContext, SerializeContext, UnresolvedDepNode};
use crate::{
  Decodable, DepId, Encodable, Fingerprint, Id, LazyFingerprint, QueryDatabase, QueryStorage,
  StableHash, StableHasher,
};
use crate::{
  Dependency, DerivedIdentity, DerivedQueryIngredientStore, LRU_CAPACITY, QueryState,
  StampedDerivedQuery,
};

use crate::serial::Codec;

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
  // Resolve the return value's session-local entry ID from the serialized cache
  // - For derived return types, triggers field group deserialization
  // - For input and interned return types, serialized_entry_id_to_session_local_map is pre-populated by load_leaf_nodes
  fn deserialize_return_value(
    &self,
    ctx: &DeserializeContext,
    serialized_entry_id: u32,
  ) -> Option<u32> {
    let group_key = (self.ret_typ_fingerprint, serialized_entry_id);

    if let Some(field_group) = ctx.derived_groups.get(&group_key) {
      for &(_, field_node_index) in &field_group.fields {
        ctx.decoder.get_or_deserialize_dep_node_id(field_node_index);
      }
    }
    ctx
      .serialized_entry_id_to_session_local_map
      .get(&group_key)
      .map(|entry| *entry.value())
  }

  fn load_cached_identity_map(
    &self,
    ctx: &DeserializeContext,
    entry_id: u32,
    identities: &[DerivedIdentity],
  ) {
    for &(name_fp, identity_hash, disambiguator, struct_entry_id) in identities {
      let Some(&field_idx) = ctx.fields_by_name(&name_fp).first() else {
        continue;
      };
      let map = self
        .identity_maps
        .entry((entry_id, field_idx as u32))
        .or_insert_with(|| Arc::new(DashMap::new()))
        .clone();
      map.insert((identity_hash, disambiguator), struct_entry_id);
    }
  }

  // Load a serialized DerivedQuery node into self.data as a cached memo
  fn load_cached_query_memo(
    &self,
    ctx: &DeserializeContext,
    entry_id: u32,
    key: K,
    node: &DepNode,
    verified_at: u32,
  ) -> Option<V> {
    let DepNode::DerivedQuery {
      key: key_fp,
      value: value_fp,
      value_entry_id: serialized_value_entry_id,
      changed_at,
      edges,
      derived_identities,
      ..
    } = node
    else {
      return None;
    };
    let value_entry_id = self.deserialize_return_value(ctx, *serialized_value_entry_id)?;
    let value = V::from(value_entry_id);

    // Deserialize edge DepNodeIndices into session-local Dependencies
    // Returns None if any edge fails to deserialize, forcing recomputation
    let dependencies: Option<Vec<Dependency>> = edges
      .iter()
      .map(|&edge_idx| {
        let dep_id = ctx.decoder.get_or_deserialize_dep_node_id(edge_idx)?;
        let edge_node = &ctx.serialized.dep_graph.nodes[edge_idx as usize];
        Some(Dependency {
          dep_id,
          changed_at: edge_node.changed_at(),
        })
      })
      .collect();
    let dependencies = dependencies?;

    let deserialized_identities: Vec<_> = derived_identities
      .iter()
      .filter_map(|id| ctx.deserialize_derived_identity(id))
      .collect();

    // Seed identity maps so re-execution (if this memo later becomes stale) reuses old entry IDs
    self.load_cached_identity_map(ctx, entry_id, &deserialized_identities);
    self.data.insert(
      entry_id,
      QueryState::Computed(StampedDerivedQuery {
        key,
        value: value.clone(),
        changed_at: *changed_at,
        verified_at: std::sync::atomic::AtomicU32::new(verified_at),
        dependencies,
        key_fingerprint: LazyFingerprint::from_stored(*key_fp),
        value_fingerprint: LazyFingerprint::from_stored(*value_fp),
        derived_query_identities: deserialized_identities,
      }),
    );
    Some(value)
  }

  // Try to load a serialized derived query node from the previous session's cache
  // WARNING: On dep mismatch, seeds self.identity_maps before returning false so that re-execution reuses the same derived struct entry IDs the previous session produced
  pub(crate) fn try_load_cached_query_memo(
    &self,
    db: &DB,
    storage: &QueryStorage,
    arg: &K,
  ) -> bool {
    let Some(ctx) = storage.deserialize_ctx.get() else {
      return false;
    };

    // Compute key fingerprint to find the matching node
    let mut hasher = StableHasher::new();
    arg.stable_hash(db, &mut hasher);
    let key_fingerprint = Fingerprint::from_hasher(hasher);

    let Some((node_index, node)) = ctx.find_derived_query(self.name_fingerprint, key_fingerprint)
    else {
      return false;
    };

    // Decode key and get entry_id before the dep check so both the stale and green paths have a stable entry_id available for identity map seeding
    let Some(blob) = ctx.serialized.query_cache.get(node_index) else {
      return false;
    };
    let mut data: &[u8] = blob;
    let key = K::decode(&mut data, &ctx.decoder);
    let entry_id = self.get_or_create_entry_id(&key);

    let DepNode::DerivedQuery { edges, .. } = node else {
      return false;
    };

    // Cross-session green check: deserialize and validate each dependency's fingerprint
    let decoder = &ctx.decoder;
    let mut all_deps_green = true;

    for &edge_idx in edges {
      let edge_node = &ctx.serialized.dep_graph.nodes[edge_idx as usize];
      // A dep can be missing (Evicted) for two reasons:
      //   - it was LRU-evicted from self.data at dump time so its serialize() returned early
      //   - it was never accessed this session and got no dep node slot
      // In either case we cannot verify the dep, so the whole query must recompute
      if matches!(edge_node, DepNode::Evicted) {
        all_deps_green = false;
        break;
      }
      // Deserialize the dep lazily: registers it into the current session and returns its DepId
      let Some(dep_id) = decoder.get_or_deserialize_dep_node_id(edge_idx) else {
        all_deps_green = false;
        break;
      };
      let expected_fingerprint = edge_node.value_fingerprint();
      if expected_fingerprint == Fingerprint::SKIPPED {
        // no_hash deps have no fingerprint, re-execute and check via backdating
        let dep = Dependency {
          dep_id,
          changed_at: edge_node.changed_at(),
        };
        storage.re_execute_dep(db, &dep);
        if !storage.green_check_dep(db, &dep) {
          all_deps_green = false;
          break;
        }
        continue;
      }
      let ingredient = storage.get_ingredient_of_id(dep_id);
      let actual_fingerprint = ingredient.value_fingerprint(db, dep_id.entry_id());
      if actual_fingerprint != Some(expected_fingerprint) {
        all_deps_green = false;
        break;
      }
    }

    if !all_deps_green {
      // Deps are stale: seed identity maps so re-execution reuses previous session entry IDs
      // This prevents ID mismatches when cached downstream queries are kept via backdating
      if let DepNode::DerivedQuery {
        derived_identities, ..
      } = node
      {
        let remapped: Vec<_> = derived_identities
          .iter()
          .filter_map(|id| ctx.deserialize_derived_identity(id))
          .collect();
        self.load_cached_identity_map(ctx, entry_id, &remapped);
      }
      return false;
    }

    // All deps green: load full memo into self.data with stored changed_at
    let current_revision = storage.revision.load(Ordering::Acquire);
    self
      .load_cached_query_memo(ctx, entry_id, key, node, current_revision)
      .is_some()
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
> Codec for DerivedQueryIngredientStore<DB, K, V>
{
  fn serialize(&self, ctx: &mut SerializeContext, entry_id: crate::EntryId) {
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
    let db = ctx.encoder.db();
    let key_fp = memo.key_fingerprint.get_or_compute(db, &memo.key);
    let value_fp = memo.value_fingerprint.get_or_compute(db, &memo.value);
    ctx.dep_graph.set(
      node_index,
      UnresolvedDepNode::DerivedQuery {
        name: self.name_fingerprint,
        key: key_fp,
        value: value_fp,
        value_entry_id: <V as Into<u32>>::into(memo.value.clone()),
        changed_at: memo.changed_at,
        verified_at: memo.verified_at.load(Ordering::Relaxed),
        edges,
        derived_identities: memo.derived_query_identities.clone(),
      },
    );

    // Encode key into the query cache
    let mut buffer = vec![];
    memo.key.encode(&mut buffer, &mut ctx.encoder);
    ctx.query_cache.set(node_index, &buffer);
  }

  fn deserialize(&self, ctx: &DeserializeContext, node_index: DepNodeIndex) -> Option<DepId> {
    if let Some(dep_id) = ctx.decoder.get_dep_node_id(node_index) {
      return Some(dep_id);
    }
    let node = &ctx.serialized.dep_graph.nodes[node_index as usize];
    let DepNode::DerivedQuery { verified_at, .. } = node else {
      return None;
    };
    // Register dep_id BEFORE decoding to prevent recursion via get_or_deserialize_dep_node_id
    let entry_id = self.next_entry_id.fetch_add(1, Ordering::Relaxed);
    let dep_id = self.ingredient_id.with_entry(entry_id);
    ctx.decoder.set_dep_node_id(node_index, dep_id);

    let blob = ctx.serialized.query_cache.get(node_index)?;
    let mut data = blob;
    let key = K::decode(&mut data, &ctx.decoder);
    self.intern_map.entry(key.clone()).or_insert(entry_id);
    self.load_cached_query_memo(ctx, entry_id, key, node, *verified_at)?;

    Some(dep_id)
  }

  // Deserialize unaccessed query memos so identity maps can be seeded on re-execution
  fn promote_cached(&self, ctx: &DeserializeContext) {
    let Some(indices) = ctx.fingerprint_reverse_map.get(&self.name_fingerprint) else {
      return;
    };
    let slots = LRU_CAPACITY.saturating_sub(self.data.len());
    if slots == 0 {
      return;
    }
    let mut unloaded: Vec<DepNodeIndex> = indices
      .iter()
      .copied()
      .filter(|&node_index| ctx.decoder.get_dep_node_id(node_index).is_none())
      .collect();
    if unloaded.len() > slots {
      // O(N) partial sort: top `slots` elements end up in [0..slots], no full sort needed
      // Nodes with no verified_at get revision 0 (promoted last, not dropped)
      unloaded.select_nth_unstable_by_key(slots - 1, |&node_index| {
        Reverse(
          ctx.serialized.dep_graph.nodes[node_index as usize]
            .verified_at()
            .unwrap_or(0),
        )
      });
      unloaded.truncate(slots);
    }
    for node_index in unloaded {
      self.deserialize(ctx, node_index);
    }
  }

  fn no_hash(&self) -> bool {
    self.no_hash_flag
  }
}
