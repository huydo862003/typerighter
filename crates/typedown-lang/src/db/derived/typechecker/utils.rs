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

// Detect which descriptor field a node is the direct value of
pub(super) fn get_descriptor_context(node: &RedNode) -> Option<DescriptorField> {
  // 1. Walk up from node to the mapping entry
  let entry_value = node.parent()?;
  if entry_value.kind() != SyntaxKind::YamlMappingEntryValue {
    return None;
  }
  let entry = entry_value.parent()?;
  let typed_entry = YamlMappingEntry::cast(entry.clone())?;

  // 2. Verify we're inside a descriptor inside "properties"
  if !typed_entry.is_inside_schema_properties() {
    return None;
  }

  // 3. Match the field name
  let field_name = typed_entry.key()?;
  let descriptor_mapping = entry.parent()?;

  match field_name.trim() {
    "type" => Some(DescriptorField::Type),
    "default" => Some(DescriptorField::Default(descriptor_mapping)),
    "computed" => Some(DescriptorField::Computed(descriptor_mapping)),
    _ => None,
  }
}

// Check if a node is anywhere inside a descriptor `type` field
// Walks up the AST to handle sub-expressions like `string` in `string?`
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

    // 2. Check if this entry has key="type" and is inside schema properties
    let Some(typed_entry) = YamlMappingEntry::cast(entry.clone()) else {
      return false;
    };
    let is_type_key = typed_entry.key().is_some_and(|key| key.trim() == "type");
    if !is_type_key {
      current = entry;
      continue;
    }

    return typed_entry.is_inside_schema_properties();
  }
}
