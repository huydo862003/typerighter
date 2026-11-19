// Shared utilities for schema property descriptor detection

use crate::syntax::ast::{AstNode, YamlMappingEntry};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

// Carries the descriptor mapping node for sibling field lookups (default, computed)
pub(super) enum DescriptorField {
  Type,
  Default(RedNode),
  Computed(RedNode),
}

// Check if a mapping entry is inside a property descriptor inside "properties"
fn is_descriptor_entry(entry: &RedNode) -> bool {
  // 1. Walk up to the properties entry
  let descriptor_mapping = match entry.parent() {
    Some(parent) if parent.kind() == SyntaxKind::YamlMapping => parent,
    _ => return false,
  };
  let prop_entry_value = match descriptor_mapping.parent() {
    Some(parent) if parent.kind() == SyntaxKind::YamlMappingEntryValue => parent,
    _ => return false,
  };
  let prop_entry = match prop_entry_value.parent() {
    Some(parent) if parent.kind() == SyntaxKind::YamlMappingEntry => parent,
    _ => return false,
  };
  let properties_mapping = match prop_entry.parent() {
    Some(parent) if parent.kind() == SyntaxKind::YamlMapping => parent,
    _ => return false,
  };
  let properties_entry_value = match properties_mapping.parent() {
    Some(parent) if parent.kind() == SyntaxKind::YamlMappingEntryValue => parent,
    _ => return false,
  };
  let properties_entry = match properties_entry_value.parent() {
    Some(parent) if parent.kind() == SyntaxKind::YamlMappingEntry => parent,
    _ => return false,
  };

  // 2. Verify the outer entry has key="properties"
  YamlMappingEntry::cast(properties_entry)
    .and_then(|entry| entry.key())
    .is_some_and(|key| key.trim() == "properties")
}

// Detect which descriptor field a node is the direct value of
pub(super) fn get_descriptor_context(node: &RedNode) -> Option<DescriptorField> {
  // 1. Walk up from node to the mapping entry
  let entry_value = node.parent()?;
  if entry_value.kind() != SyntaxKind::YamlMappingEntryValue {
    return None;
  }
  let entry = entry_value.parent()?;
  if entry.kind() != SyntaxKind::YamlMappingEntry {
    return None;
  }

  // 2. Verify we're inside a descriptor inside "properties"
  if !is_descriptor_entry(&entry) {
    return None;
  }

  // 3. Match the field name
  let field_name = YamlMappingEntry::cast(entry.clone())?.key()?;
  let descriptor_mapping = entry.parent()?;

  match field_name.trim() {
    "type" => Some(DescriptorField::Type),
    "default" => Some(DescriptorField::Default(descriptor_mapping)),
    "computed" => Some(DescriptorField::Computed(descriptor_mapping)),
    _ => None,
  }
}

// Check if a node is anywhere inside a descriptor `type` field
pub(super) fn is_inside_descriptor_type_field(node: &RedNode) -> bool {
  let mut current = node.clone();
  loop {
    // 1. Walk up to the nearest entry value
    let entry_value = match current.parent() {
      Some(parent) if parent.kind() == SyntaxKind::YamlMappingEntryValue => parent,
      Some(parent) => {
        current = parent;
        continue;
      }
      None => return false,
    };
    let entry = match entry_value.parent() {
      Some(parent) if parent.kind() == SyntaxKind::YamlMappingEntry => parent,
      _ => return false,
    };

    // 2. Check if this entry has key="type"
    let is_type_key = YamlMappingEntry::cast(entry.clone())
      .and_then(|typed_entry| typed_entry.key())
      .is_some_and(|key| key.trim() == "type");
    if !is_type_key {
      current = entry;
      continue;
    }

    // 3. Verify it's inside a descriptor inside "properties"
    return is_descriptor_entry(&entry);
  }
}
