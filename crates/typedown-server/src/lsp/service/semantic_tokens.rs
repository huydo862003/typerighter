use lsp_types::{
  SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens, SemanticTokensParams,
  SemanticTokensResult,
};
use ropey::Rope;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::analysis::Analysis;
use crate::core::utils::ast::ident_is_type_ref;
use crate::core::utils::position::text_offset_to_lsp_position;
use crate::core::utils::uri::uri_to_path;

#[cfg(test)]
use typedown_lang::db::{
  QueryStorage, TypedownDatabase,
  types::{File, FileHandle, Project},
};

pub fn token_types() -> Vec<SemanticTokenType> {
  vec![SemanticTokenType::TYPE]
}

fn token_type_index(token_type: &SemanticTokenType) -> u32 {
  token_types()
    .iter()
    .position(|t| t == token_type)
    .expect("token type not in legend") as u32
}

// TODO: recompute incrementally in the future
pub fn semantic_tokens_full(
  analysis: &Analysis,
  params: SemanticTokensParams,
) -> Option<SemanticTokensResult> {
  // Resolve file
  let path = uri_to_path(&params.text_document.uri)?;
  let rope = analysis.file_rope(&path)?;
  let db = &analysis.db;
  let project = analysis.project;
  let file = *project.files(db).get(&path)?;

  // Collect tokens from the AST
  let root = parse_file(db, project, file).ast(db);
  let mut raw: Vec<(usize, usize, SemanticTokenType, u32)> = Vec::new();
  collect_tokens(root, &mut raw);

  // Delta-encode and return
  let data = delta_encode(raw, &rope);
  Some(SemanticTokensResult::Tokens(SemanticTokens {
    result_id: None,
    data,
  }))
}

// Push a span only if it has non-zero length.
fn emit(
  node: &RedNode,
  token_type: SemanticTokenType,
  modifiers: u32,
  out: &mut Vec<(usize, usize, SemanticTokenType, u32)>,
) {
  let len = node.text_len();
  if len > 0 {
    out.push((node.offset(), len, token_type, modifiers));
  }
}

// Walk the AST and collect (offset, length, type, modifiers) spans for all highlighted regions.
fn collect_tokens(node: RedNode, out: &mut Vec<(usize, usize, SemanticTokenType, u32)>) {
  if !node.is_token() {
    for child in node.children() {
      collect_tokens(child, out);
    }
    return;
  }
  if let Some(token_type) = classify_token(&node) {
    emit(&node, token_type, 0, out);
  }
}

pub fn token_modifiers() -> Vec<SemanticTokenModifier> {
  vec![]
}

fn classify_token(node: &RedNode) -> Option<SemanticTokenType> {
  match node.kind() {
    SyntaxKind::Ident if ident_is_type_ref(node) => Some(SemanticTokenType::TYPE),
    _ => None,
  }
}

// LSP tokens are encoded as deltas: each token's line and column are relative to the previous
// token, not absolute
fn delta_encode(
  raw: Vec<(usize, usize, SemanticTokenType, u32)>,
  rope: &Rope,
) -> Vec<SemanticToken> {
  let mut result = Vec::with_capacity(raw.len());
  let mut prev_line = 0u32;
  let mut prev_start = 0u32;

  for (offset, length, token_type, modifiers) in raw {
    // Convert char offset to line/character position
    let pos = text_offset_to_lsp_position(rope, offset);
    let line = pos.line;
    let start = pos.character;

    // Compute deltas relative to the previous token
    let delta_line = line - prev_line;
    let delta_start = if delta_line == 0 {
      start - prev_start
    } else {
      start
    };

    result.push(SemanticToken {
      delta_line,
      delta_start,
      length: length as u32,
      token_type: token_type_index(&token_type),
      token_modifiers_bitset: modifiers,
    });

    prev_line = line;
    prev_start = start;
  }

  result
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::PathBuf;

  use super::*;
  use typedown_lang::db::types::FileMetadata;

  // Token -> Semantic token
  fn parse_tokens(content: &str) -> Vec<SemanticTokenType> {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let path = PathBuf::from("/test.td");
    let file = File::new(
      &db,
      FileHandle::Content(path.clone(), content.to_string(), FileMetadata::default()),
    );
    let project = Project::new(
      &db,
      PathBuf::from("/"),
      HashMap::from([(path.clone(), file)]),
    );
    let ast = parse_file(&db, project, file).ast(&db);
    let mut raw = Vec::new();
    collect_tokens(ast, &mut raw);
    raw
      .into_iter()
      .map(|(_, _, token_type, _)| token_type)
      .collect()
  }

  #[test]
  fn type_ref_in_type_field() {
    let types = parse_tokens(
      r#"---
_type: Person
name: "Alice"
---
"#,
    );
    assert!(types.contains(&SemanticTokenType::TYPE));
    assert_eq!(types.len(), 1, "only Person should be a TYPE token");
  }

  #[test]
  fn schema_property_type_ref_highlighted_as_type() {
    let types = parse_tokens(
      r#"---
_type: schema
properties:
  name:
    type: string
  age:
    type: number
---
"#,
    );
    let type_count = types
      .iter()
      .filter(|t| **t == SemanticTokenType::TYPE)
      .count();
    assert_eq!(
      type_count, 3,
      "expected 3 TYPE tokens (schema, string, number), got: {type_count}"
    );
  }

  // string? should highlight "string" as a TYPE token via the postfix expression
  #[test]
  fn schema_property_postfix_optional_type_highlighted() {
    let types = parse_tokens(
      r#"---
_type: schema
properties:
  email:
    type: string?
  due:
    type: date?
---
"#,
    );
    let type_count = types
      .iter()
      .filter(|t| **t == SemanticTokenType::TYPE)
      .count();
    assert_eq!(
      type_count, 3,
      "expected 3 TYPE tokens (schema, string, date), got: {type_count}"
    );
  }

  #[test]
  fn non_type_identifiers_not_emitted() {
    let types = parse_tokens(
      r#"---
active: true
value: self.name
---
"#,
    );
    assert!(
      types.is_empty(),
      "no semantic tokens for non-type identifiers"
    );
  }
}
