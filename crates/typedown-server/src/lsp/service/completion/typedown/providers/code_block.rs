//! Prop names inside the `{...}` on a code block's opening fence

use lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};

use crate::lsp::service::utils::cursor::CursorContext;

pub(super) fn complete(ctx: &CursorContext) -> Option<Vec<CompletionItem>> {
  let text = ctx.node.text();
  let (node_offset, _) = ctx.node.trimmed_range();

  // The opening fence is the first line of the code block
  let relative_offset = ctx.offset.saturating_sub(node_offset);
  let first_line = text.lines().next().unwrap_or("");

  // Only inside {...} on that line
  let brace_open = first_line.find('{')?;
  let brace_close = first_line.find('}').unwrap_or(first_line.len());
  if relative_offset <= brace_open || relative_offset > brace_close {
    return None;
  }

  // Skip props that are already present
  let inner = &first_line[brace_open + 1..brace_close];
  let mut items = Vec::new();

  if !inner.contains("title") {
    items.push(CompletionItem {
      label: "title".to_string(),
      kind: Some(CompletionItemKind::PROPERTY),
      insert_text: Some("title=\"$1\"".to_string()),
      insert_text_format: Some(InsertTextFormat::SNIPPET),
      detail: Some("Code block title/filename".to_string()),
      ..Default::default()
    });
  }

  if !inner.contains("line-numbers") {
    items.push(CompletionItem {
      label: "line-numbers".to_string(),
      kind: Some(CompletionItemKind::PROPERTY),
      detail: Some("Show line numbers".to_string()),
      ..Default::default()
    });
  }

  if items.is_empty() {
    return None;
  }
  Some(items)
}
