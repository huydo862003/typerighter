//! Cursor classification: the single place that decides where the cursor is
//!
//! `classify` returns exactly one `CursorPosition`; precedence is the order of the checks in it

use typedown_lang::syntax::ast::{AstNode, BinaryExpr, YamlOpKind};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::utils::ast::{containing_fref_expr, find_ancestor, is_in_mapping_value_position};
use crate::lsp::service::utils::cursor::CursorContext;

pub(super) enum CursorPosition {
  /// Opening fence line of a code block
  CodeBlockFence,
  /// `:::` or `[[` label; `builtin_callouts` is false for the `[[` shorthand
  ContainerLabel { builtin_callouts: bool },
  /// Inside a container's `{...}` props
  ContainerProps { container: RedNode },
  /// On a `===` slot separator line
  ContainerSlot { separator: RedNode },
  /// Value of `_type`
  TypeName,
  /// After the `.` of `lhs.|`
  MemberAccess { binary: BinaryExpr },
  /// Inside the argument of `fref(...)`
  FrefArg,
  /// String inside the `_imports` mapping
  ImportsValue,
  /// Inside `${...}`
  Interpolation,
  /// Any expression after `key:`, in a list item, dict value or call argument
  Value,
  /// Key position (or blank line) inside a mapping
  FieldKey,
}

/// `None` means there is nothing to complete here
pub(super) fn classify(ctx: &CursorContext) -> Option<CursorPosition> {
  let node = &ctx.node;

  if node.kind() == SyntaxKind::CodeBlock {
    return Some(CursorPosition::CodeBlockFence);
  }
  // No completions inside other code or math tokens
  if matches!(
    node.kind(),
    SyntaxKind::InlineCode | SyntaxKind::InlineMath | SyntaxKind::MathBlock
  ) {
    return None;
  }

  if let Some(position) = container_label(node)
    .or_else(|| container_props(node))
    .or_else(|| container_slot(node))
  {
    return Some(position);
  }
  if is_typ_value_position(node) {
    return Some(CursorPosition::TypeName);
  }
  if let Some(binary) = member_access(node) {
    return Some(CursorPosition::MemberAccess { binary });
  }
  if containing_fref_expr(node).is_some() {
    return Some(CursorPosition::FrefArg);
  }
  if is_imports_value_position(node) {
    return Some(CursorPosition::ImportsValue);
  }
  if is_interp_position(node) {
    return Some(CursorPosition::Interpolation);
  }
  if is_in_mapping_value_position(node) {
    return Some(CursorPosition::Value);
  }
  find_ancestor(node, SyntaxKind::YamlMapping).map(|_| CursorPosition::FieldKey)
}

fn container_label(node: &RedNode) -> Option<CursorPosition> {
  let parent_kind = node.parent().map(|parent| parent.kind());
  let is_block_label = node.kind() == SyntaxKind::MdSymbol
    && node.text() == ":::"
    && parent_kind == Some(SyntaxKind::MdContainerBlock);
  let is_shorthand_label = parent_kind == Some(SyntaxKind::MdContainerShorthand)
    && matches!(
      node.kind(),
      SyntaxKind::LBracket | SyntaxKind::RBracket | SyntaxKind::MdSymbol
    );
  let is_label_ident = node.kind() == SyntaxKind::Ident
    && matches!(
      parent_kind,
      Some(SyntaxKind::MdContainerBlock | SyntaxKind::MdContainerShorthand)
    );

  (is_block_label || is_shorthand_label || is_label_ident).then_some(
    CursorPosition::ContainerLabel {
      builtin_callouts: !is_shorthand_label,
    },
  )
}

fn container_props(node: &RedNode) -> Option<CursorPosition> {
  let prop_block = find_ancestor(node, SyntaxKind::MdContainerPropBlock)?;
  let container = prop_block.parent()?;
  matches!(
    container.kind(),
    SyntaxKind::MdContainerBlock | SyntaxKind::MdContainerShorthand
  )
  .then_some(CursorPosition::ContainerProps { container })
}

fn container_slot(node: &RedNode) -> Option<CursorPosition> {
  let separator = if node.kind() == SyntaxKind::MdContainerSlotSeparator {
    node.clone()
  } else {
    find_ancestor(node, SyntaxKind::MdContainerSlotSeparator)?
  };
  Some(CursorPosition::ContainerSlot { separator })
}

fn member_access(node: &RedNode) -> Option<BinaryExpr> {
  let binary = BinaryExpr::cast(find_ancestor(node, SyntaxKind::BinaryExpr)?)?;
  matches!(binary.op()?.kind()?, YamlOpKind::Dot).then_some(binary)
}

// Returns true if the cursor is inside the value of a _typ mapping entry
fn is_typ_value_position(node: &RedNode) -> bool {
  let Some(entry) = find_ancestor(node, SyntaxKind::YamlMappingEntry) else {
    return false;
  };
  let Some(key) = entry
    .children()
    .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)
  else {
    return false;
  };
  key.text().trim() == "_type"
}

// Returns true if the cursor is inside a ${} interpolation
fn is_interp_position(node: &RedNode) -> bool {
  find_ancestor(node, SyntaxKind::InterpFragment).is_some()
}

// Returns true if the cursor is inside a string value inside an _imports mapping
fn is_imports_value_position(node: &RedNode) -> bool {
  // Must be inside a StringLit or directly in a mapping entry value
  let string_lit = find_ancestor(node, SyntaxKind::StringLit);
  let search_start = string_lit.as_ref().unwrap_or(node);
  // The string is wrapped in a YamlMappingEntry
  let Some(entry) = find_ancestor(search_start, SyntaxKind::YamlMappingEntry) else {
    return false;
  };
  // The parent of that entry is a YamlMapping (the _imports block value)
  let Some(mapping) = entry.parent() else {
    return false;
  };
  if mapping.kind() != SyntaxKind::YamlMapping {
    return false;
  }
  // The mapping is the value of an outer entry with key _imports
  let Some(outer_entry) = mapping.parent().and_then(|value| value.parent()) else {
    return false;
  };
  if outer_entry.kind() != SyntaxKind::YamlMappingEntry {
    return false;
  }
  outer_entry
    .children()
    .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)
    .map(|key| key.text().trim() == "_imports")
    .unwrap_or(false)
}
