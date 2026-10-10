use std::collections::HashMap;
use std::sync::{Arc, Weak};

use dashmap::DashMap;

use crate::serial::format::binary_files::SerializedQueryStorage;
use crate::serial::format::binary_files::dep_graph::{DepNode, DepNodeIndex};
use crate::{Decoder, DerivedIdentity, Fingerprint, QueryStorage};

/// A group of field dep nodes that belong to the same struct entry
pub struct FieldGroup {
  pub fields: Vec<(u8 /* field index within the struct */, DepNodeIndex)>,
}

/// All state needed for lazy deserialization from a previous session
pub struct DeserializeContext {
  pub serialized: SerializedQueryStorage,
  pub decoder: Decoder,
  /// Ingredient name fingerprint -> list of dep node indices (all node kinds)
  pub fingerprint_reverse_map: HashMap<Fingerprint, Vec<DepNodeIndex>>,
  /// DerivedField nodes grouped by (struct_name, serialized entry_id) for atomic deserialization
  pub derived_groups: HashMap<(Fingerprint, u32), FieldGroup>,
  /// (struct_name, serialized entry_id) -> current session entry_id
  pub serialized_entry_id_to_session_local_map: DashMap<(Fingerprint, u32), u32>,
  /// Per-kind name -> ingredient indices
  inputs_by_name: HashMap<Fingerprint, Vec<usize>>,
  interned_by_name: HashMap<Fingerprint, Vec<usize>>,
  queries_by_name: HashMap<Fingerprint, Vec<usize>>,
  fields_by_name: HashMap<Fingerprint, Vec<usize>>,
}

fn build_name_index(
  fingerprints: impl Iterator<Item = Fingerprint>,
) -> HashMap<Fingerprint, Vec<usize>> {
  let mut map = HashMap::new();
  for (index, name) in fingerprints.enumerate() {
    map.entry(name).or_insert_with(Vec::new).push(index);
  }
  map
}

impl DeserializeContext {
  pub fn new(serialized: SerializedQueryStorage, storage: Weak<QueryStorage>) -> Self {
    let storage = storage
      .upgrade()
      .expect("QueryStorage must be alive during DeserializeContext creation");
    let intern_blobs = Arc::new(serialized.interned_blobs.records.clone());

    let mut fingerprint_reverse_map: HashMap<Fingerprint, Vec<DepNodeIndex>> = HashMap::new();
    let mut derived_groups: HashMap<(Fingerprint, u32), FieldGroup> = HashMap::new();
    for (index, node) in serialized.dep_graph.nodes.iter().enumerate() {
      fingerprint_reverse_map
        .entry(node.name())
        .or_default()
        .push(index as DepNodeIndex);
      if let DepNode::DerivedField {
        name,
        field_index,
        entry_id,
        ..
      } = node
      {
        derived_groups
          .entry((*name, *entry_id))
          .or_insert_with(|| FieldGroup { fields: Vec::new() })
          .fields
          .push((*field_index, index as DepNodeIndex));
      }
    }

    let inputs_by_name =
      build_name_index(storage.inputs.iter().map(|index| index.name_fingerprint()));
    let interned_by_name = build_name_index(
      storage
        .interned
        .iter()
        .map(|index| index.name_fingerprint()),
    );
    let queries_by_name =
      build_name_index(storage.queries.iter().map(|index| index.name_fingerprint()));
    let fields_by_name =
      build_name_index(storage.fields.iter().map(|index| index.name_fingerprint()));

    Self {
      decoder: Decoder::new(storage, intern_blobs),
      serialized,
      fingerprint_reverse_map,
      derived_groups,
      serialized_entry_id_to_session_local_map: DashMap::new(),
      inputs_by_name,
      interned_by_name,
      queries_by_name,
      fields_by_name,
    }
  }

  pub fn inputs_by_name(&self, name: &Fingerprint) -> &[usize] {
    self
      .inputs_by_name
      .get(name)
      .map(|value| value.as_slice())
      .unwrap_or(&[])
  }

  pub fn interned_by_name(&self, name: &Fingerprint) -> &[usize] {
    self
      .interned_by_name
      .get(name)
      .map(|value| value.as_slice())
      .unwrap_or(&[])
  }

  pub fn queries_by_name(&self, name: &Fingerprint) -> &[usize] {
    self
      .queries_by_name
      .get(name)
      .map(|value| value.as_slice())
      .unwrap_or(&[])
  }

  pub fn fields_by_name(&self, name: &Fingerprint) -> &[usize] {
    self
      .fields_by_name
      .get(name)
      .map(|value| value.as_slice())
      .unwrap_or(&[])
  }

  // Deserialize a single derived identity, remapping the entry_id to session-local
  // Lazily triggers field deserialization if not yet loaded
  pub fn deserialize_derived_identity(
    &self,
    identity: &DerivedIdentity,
  ) -> Option<DerivedIdentity> {
    let (name_fingerprint, identity_hash, disambiguator, old_entry_id) = *identity;
    let group_key = (name_fingerprint, old_entry_id);
    if !self
      .serialized_entry_id_to_session_local_map
      .contains_key(&group_key)
      && let Some(field_group) = self.derived_groups.get(&group_key)
    {
      for &(_, field_node_index) in &field_group.fields {
        self
          .decoder
          .get_or_deserialize_dep_node_id(field_node_index);
      }
    }
    let new_entry_id = *self
      .serialized_entry_id_to_session_local_map
      .get(&group_key)?
      .value();
    Some((name_fingerprint, identity_hash, disambiguator, new_entry_id))
  }

  /// Find a DerivedQuery node by name + key fingerprint
  pub fn find_derived_query(
    &self,
    name: Fingerprint,
    key: Fingerprint,
  ) -> Option<(DepNodeIndex, &DepNode)> {
    let indices = self.fingerprint_reverse_map.get(&name)?;
    indices.iter().find_map(|&idx| {
      let node = &self.serialized.dep_graph.nodes[idx as usize];
      if let DepNode::DerivedQuery { key: k, .. } = node
        && k == &key
      {
        return Some((idx, node));
      }
      None
    })
  }
}
