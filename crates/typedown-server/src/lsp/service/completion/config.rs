use lsp_types::{CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse};

use crate::core::analysis::Analysis;
use crate::core::utils::ast::node_at_offset;
use crate::core::utils::position::lsp_position_to_text_offset;
use crate::core::utils::uri::uri_to_path;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

pub fn completion(analysis: &Analysis, params: CompletionParams) -> Option<CompletionResponse> {
  let path = uri_to_path(&params.text_document_position.text_document.uri)?;
  let rope = analysis.file_rope(&path)?;
  let offset = lsp_position_to_text_offset(&rope, params.text_document_position.position)?;

  let db = &analysis.db;
  let project = analysis.project;
  let file = *project.files(db).get(&path)?;

  let root = parse_file(db, project, file).ast(db);
  let lookup = offset.saturating_sub(1);
  let node = node_at_offset(root, lookup)?;

  let items = if is_under_vault(&node) {
    vault_field_completions()
  } else {
    top_level_completions()
  };

  Some(CompletionResponse::Array(items))
}

/// Returns true if the cursor is nested inside a `vault` mapping entry value.
fn is_under_vault(node: &RedNode) -> bool {
  let mut current = node.parent();
  while let Some(ref cur) = current {
    if cur.kind() == SyntaxKind::YamlMappingEntry {
      let key = cur
        .children()
        .find(|c| c.kind() == SyntaxKind::YamlMappingEntryKey);
      if key.is_some_and(|k| k.text().trim() == "vault") {
        return true;
      }
    }
    current = cur.parent();
  }
  false
}

fn top_level_completions() -> Vec<CompletionItem> {
  vec![field_item("version"), field_item("vault")]
}

fn vault_field_completions() -> Vec<CompletionItem> {
  vec![field_item("root_dir")]
}

fn field_item(label: &str) -> CompletionItem {
  CompletionItem {
    label: label.to_string(),
    kind: Some(CompletionItemKind::FIELD),
    ..Default::default()
  }
}
