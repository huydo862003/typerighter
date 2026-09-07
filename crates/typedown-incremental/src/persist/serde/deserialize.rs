use std::collections::HashMap;
use std::sync::{Arc, OnceLock, Weak};

use dashmap::DashMap;

use crate::persist::serialized::SerializedQueryStorage;
use crate::persist::serialized::dep_graph::{DepNode, DepNodeIndex};
use crate::{Decoder, Fingerprint, QueryStorage};

/// A group of field dep nodes that belong to the same struct entry
pub struct FieldGroup {
  pub fields: Vec<(u8, DepNodeIndex)>,
}

/// All state needed for lazy deserialization from a previous session
pub struct DeserializeContext {
  pub serialized: SerializedQueryStorage,
  pub decoder: Decoder,
  fingerprint_map: OnceLock<HashMap<Fingerprint, Vec<DepNodeIndex>>>,
  /// DerivedField nodes grouped by (name, serialized entry_id) for atomic deserialization
  pub derived_groups: HashMap<(Fingerprint, u64), FieldGroup>,
  /// (name, serialized entry_id) -> current session entry_id
  pub entry_id_map: DashMap<(Fingerprint, u64), u32>,
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
  for (idx, name) in fingerprints.enumerate() {
    map.entry(name).or_insert_with(Vec::new).push(idx);
  }
  map
}

impl DeserializeContext {
  pub fn new(serialized: SerializedQueryStorage, storage: Weak<QueryStorage>) -> Self {
    let storage = storage
      .upgrade()
      .expect("QueryStorage must be alive during DeserializeContext creation");
    let intern_blobs = Arc::new(serialized.interned_blobs.records.clone());

    let mut derived_groups: HashMap<(Fingerprint, u64), FieldGroup> = HashMap::new();
    for (i, node) in serialized.dep_graph.nodes.iter().enumerate() {
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
          .push((*field_index, i as DepNodeIndex));
      }
    }

    let inputs_by_name = build_name_index(storage.inputs.iter().map(|i| i.name_fingerprint()));
    let interned_by_name = build_name_index(storage.interned.iter().map(|i| i.name_fingerprint()));
    let queries_by_name = build_name_index(storage.queries.iter().map(|i| i.name_fingerprint()));
    let fields_by_name = build_name_index(storage.fields.iter().map(|i| i.name_fingerprint()));

    Self {
      decoder: Decoder::new(storage, intern_blobs, serialized.dep_graph.nodes.len()),
      serialized,
      fingerprint_map: OnceLock::new(),
      derived_groups,
      entry_id_map: DashMap::new(),
      inputs_by_name,
      interned_by_name,
      queries_by_name,
      fields_by_name,
    }
  }

  /// Lazily-built index: ingredient name fingerprint -> list of dep node indices
  pub fn fingerprint_map(&self) -> &HashMap<Fingerprint, Vec<DepNodeIndex>> {
    self.fingerprint_map.get_or_init(|| {
      let mut map: HashMap<Fingerprint, Vec<DepNodeIndex>> = HashMap::new();
      for (i, node) in self.serialized.dep_graph.nodes.iter().enumerate() {
        map.entry(node.name()).or_default().push(i as DepNodeIndex);
      }
      map
    })
  }

  pub fn inputs_by_name(&self, name: &Fingerprint) -> &[usize] {
    self
      .inputs_by_name
      .get(name)
      .map(|v| v.as_slice())
      .unwrap_or(&[])
  }

  pub fn interned_by_name(&self, name: &Fingerprint) -> &[usize] {
    self
      .interned_by_name
      .get(name)
      .map(|v| v.as_slice())
      .unwrap_or(&[])
  }

  pub fn queries_by_name(&self, name: &Fingerprint) -> &[usize] {
    self
      .queries_by_name
      .get(name)
      .map(|v| v.as_slice())
      .unwrap_or(&[])
  }

  pub fn fields_by_name(&self, name: &Fingerprint) -> &[usize] {
    self
      .fields_by_name
      .get(name)
      .map(|v| v.as_slice())
      .unwrap_or(&[])
  }

  /// Find a DerivedQuery node by name + key fingerprint
  pub fn find_derived_query(
    &self,
    name: Fingerprint,
    key: Fingerprint,
  ) -> Option<(DepNodeIndex, &DepNode)> {
    let indices = self.fingerprint_map().get(&name)?;
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
