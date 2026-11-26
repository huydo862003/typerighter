use typedown_lang::syntax::ast::{AstNode, CallExpr, Expr, YamlMappingEntry};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

/// Find the innermost red node whose source span contains `offset`
pub fn node_at_offset(root: RedNode, offset: usize) -> Option<RedNode> {
  let start = root.start_offset();
  let end = start + root.text_len();

  if offset < start || offset >= end {
    return None;
  }

  // Descend into whichever child contains the offset
  for child in root.children() {
    if let Some(found) = node_at_offset(child, offset) {
      return Some(found);
    }
  }

  Some(root)
}

/// Returns true if the cursor is in a mapping value position rather than a key position
pub fn is_in_mapping_value_position(node: &RedNode) -> bool {
  let mut _current = Some(node.clone());
  while let Some(ref current) = _current {
    match current.kind() {
      SyntaxKind::YamlMappingEntryValue => {
        // When a value has no non-trivia tokens (e.g. `address:\n  |`) or ends with a trailing newline, a cursor on a subsequent line is outside the inline value
        if !node.text().contains('\n') {
          let value_text = current.text();
          if let Some(first_newline) = value_text.find('\n') {
            let before_newline = &value_text[..first_newline];
            if before_newline.trim().is_empty()
              && node.start_offset() > current.start_offset() + first_newline
            {
              return false;
            }
          }
        }
        return true;
      }
      SyntaxKind::YamlSequenceItem => return true,
      SyntaxKind::YamlMappingEntryKey => return false,
      SyntaxKind::YamlMappingEntry => {
        // Direct token inside YamlMappingEntry (e.g. `:`, or trailing whitespace on `key: ` before newline) is in value position if it starts at or after the `:` token and stays on the same line
        let colon = current
          .children()
          .find(|child| child.kind() == SyntaxKind::Colon);
        let first_newline = current
          .children()
          .find(|child| child.kind() == SyntaxKind::Newline);
        if let Some(colon) = colon
          && node.start_offset() >= colon.end_offset()
          && node.kind() != SyntaxKind::Newline
          && first_newline.is_none_or(|newline| node.start_offset() < newline.start_offset())
        {
          return true;
        }
        return false;
      }
      SyntaxKind::YamlMapping => {
        // Trailing whitespace after an entry with an incomplete inline expression (e.g. `name: a - |`) can be attached to YamlMapping right before the closing newline
        if node.kind() == SyntaxKind::Whitespace
          && let Some(previous_entry) = current
            .children()
            .filter(|child| child.kind() == SyntaxKind::YamlMappingEntry)
            .take_while(|child| child.end_offset() <= node.start_offset())
            .last()
          && !previous_entry.text().ends_with('\n')
        {
          return true;
        }
        _current = current.parent();
      }
      _ => _current = current.parent(),
    }
  }
  false
}

/// Returns true if this `Ident` token is directly inside a `YamlMappingEntryKey`
pub fn ident_is_mapping_key(node: &RedNode) -> bool {
  node
    .parent()
    .is_some_and(|parent| parent.kind() == SyntaxKind::YamlMappingEntryKey)
}

/// Returns true if this `Ident` token is used as a type reference (inside an `IdentLit` that is the value of a `_type` mapping entry, or inside a `CallExpr`/`IndexExpr` type position)
pub fn ident_is_typ_reference(node: &RedNode) -> bool {
  let Some(parent) = node.parent() else {
    return false;
  };
  if parent.kind() != SyntaxKind::IdentLit {
    return false;
  }
  // Walk up through wrapping expression nodes (e.g. PostfixExpr for string?)
  let mut ancestor = parent.parent();
  while let Some(ref node) = ancestor {
    if node.kind() == SyntaxKind::YamlMappingEntryValue {
      break;
    }
    if node.kind() == SyntaxKind::PostfixExpr {
      ancestor = node.parent();
      continue;
    }
    return false;
  }
  let Some(entry_value) = ancestor else {
    return false;
  };
  let Some(entry) = entry_value.parent() else {
    return false;
  };
  // Find the sibling key node
  let key_text = entry
    .children()
    .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)
    .map(|key| key.text().trim().to_string());
  match key_text.as_deref() {
    Some("_type") => true,
    // `type: string` inside a schema property descriptor
    Some("type") => {
      YamlMappingEntry::cast(entry.clone()).is_some_and(|entry| entry.is_inside_schema_properties())
    }
    _ => false,
  }
}

/// Walk up to find the nearest ancestor with the given syntax kind
pub fn find_ancestor(node: &RedNode, kind: SyntaxKind) -> Option<RedNode> {
  let mut current = node.parent()?;
  loop {
    if current.kind() == kind {
      return Some(current);
    }
    current = current.parent()?;
  }
}

/// Walk up to find the nearest ancestor that can be cast to an Expr
pub fn get_nearest_expr_ancestor(node: &RedNode) -> Option<Expr> {
  let mut current = node.clone();
  loop {
    if let Some(expr) = Expr::cast(current.clone()) {
      return Some(expr);
    }
    current = current.parent()?;
  }
}

/// Check whether we're in the fref reference
/// fref("...")
///       ^^^
pub fn containing_fref_expr(node: &RedNode) -> Option<CallExpr> {
  let string_lit = find_ancestor(node, SyntaxKind::StringLit);

  let call = match string_lit {
    Some(ref lit) => find_ancestor(lit, SyntaxKind::CallExpr),
    None => find_ancestor(node, SyntaxKind::CallExpr),
  }?;

  let callee = call.children().next()?;

  if callee.text().trim() != "fref" {
    return None;
  }

  CallExpr::cast(call)
}
