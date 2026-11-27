//! `fref(...)` file references: the argument itself, and `fref("path")` suggestions for other positions

use lsp_types::{CompletionItem, CompletionItemKind, CompletionTextEdit, Range, TextEdit};
use ropey::Rope;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::typechecker::expected_node_typ::expected_node_typ;
use typedown_lang::db::derived::typechecker::get_symbol_typ::get_symbol_typ;
use typedown_lang::db::types::typecheck::is_subtyp_of;
use typedown_lang::db::types::{File, Project, ProjectRedNode, TdStaticTyp, TdTypEnum};
use typedown_lang::db::utils::{is_content_file, is_typ_file};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::utils::ast::find_ancestor;
use crate::core::utils::position::byte_offset_to_lsp_position;
use crate::lsp::service::utils::cursor::CursorContext;
use crate::lsp::service::utils::symbol::get_resource_label;

/// Paths inside an `fref(...)` argument, filtered by the type the call is expected to produce
pub(super) fn complete_arg(ctx: &CursorContext) -> Vec<CompletionItem> {
  collect_fref_completions(
    ctx.db,
    ctx.project,
    ctx.file,
    &ctx.node,
    &ctx.rope,
    ctx.offset,
  )
}

// A resolved fref candidate with display info
struct FrefCandidate {
  // Vault-root-relative path with / prefix (e.g. "/people/alice.td")
  path: String,
  // Human-readable label from _label field
  label: Option<String>,
  // Schema type name
  schema: Option<String>,
  // File basename without extension
  basename: String,
}

// Collect fref candidates compatible with an optional expected type
fn collect_fref_candidates(
  db: &TypedownDatabase,
  project: Project,
  expected_typ: Option<&TdTypEnum>,
) -> Vec<FrefCandidate> {
  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  project
    .files(db)
    .iter()
    .filter(|(path, _)| path.starts_with(&root_dir) && is_content_file(path) && !is_typ_file(path))
    .filter_map(|(path, target_file)| {
      let symbol = file_symbol(db, project, *target_file).value(db)?;

      // Filter by expected type if provided
      if let Some(expected_typ) = expected_typ {
        let file_typ = get_symbol_typ(db, symbol).typ(db)?;
        if !is_subtyp_of(db, &file_typ, expected_typ) {
          return None;
        }
      }

      let relative = path.strip_prefix(&root_dir).ok()?;
      let relative_string = format!("/{}", relative.to_string_lossy());
      let label = get_resource_label(db, symbol);
      let schema = get_symbol_typ(db, symbol)
        .typ(db)
        .map(|typ| typ.display_name(db));
      let basename = relative
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string();

      Some(FrefCandidate {
        path: relative_string,
        label,
        schema,
        basename,
      })
    })
    .collect()
}

// Build a CompletionItem from a candidate, wrapping the path with the given template
fn build_candidate_completion(
  candidate: &FrefCandidate,
  insert_template: impl Fn(&str) -> String,
) -> CompletionItem {
  let display = candidate.label.as_deref().unwrap_or(&candidate.basename);

  // filterText includes path, basename, and label for fuzzy matching
  let mut filter_parts = vec![candidate.path.clone(), candidate.basename.clone()];
  if let Some(ref label) = candidate.label {
    filter_parts.push(label.clone());
  }

  CompletionItem {
    label: display.to_string(),
    insert_text: Some(insert_template(&candidate.path)),
    detail: Some(candidate.path.clone()),
    label_details: candidate
      .schema
      .as_ref()
      .map(|schema| lsp_types::CompletionItemLabelDetails {
        detail: Some(schema.clone()),
        description: None,
      }),
    filter_text: Some(filter_parts.join(" ")),
    kind: Some(CompletionItemKind::REFERENCE),
    ..Default::default()
  }
}

// Suggest file paths inside a fref() argument
// When cursor is already inside a string literal, insert bare path (inside existing quotes)
// When cursor is inside fref() without quotes, use a text_edit to replace any existing unquoted content (e.g. `fref(./subfolder/|)`) with the properly-quoted path
fn collect_fref_completions(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
  rope: &Rope,
  cursor_offset: usize,
) -> Vec<CompletionItem> {
  let string_lit = find_ancestor(node, SyntaxKind::StringLit);
  let call_node = match string_lit {
    Some(ref lit) => find_ancestor(lit, SyntaxKind::CallExpr),
    None => find_ancestor(node, SyntaxKind::CallExpr),
  };
  let expected_typ = call_node.and_then(|call| {
    let project_node = ProjectRedNode::new(project, file, call);
    expected_node_typ(db, project_node).typ(db)
  });
  let candidates = collect_fref_candidates(db, project, expected_typ.as_ref());
  let inside_str = string_lit.is_some();

  if inside_str {
    candidates
      .iter()
      .map(|candidate| build_candidate_completion(candidate, |path| path.to_string()))
      .collect()
  } else {
    // Compute the range covering any existing unquoted arg content to replace
    let arg_replace_range = compute_fref_unquoted_arg_range(node, rope, cursor_offset);
    candidates
      .iter()
      .map(|candidate| {
        let mut item = build_candidate_completion(candidate, |path| format!("\"{path}\""));
        if let Some(range) = arg_replace_range {
          item.text_edit = Some(CompletionTextEdit::Edit(TextEdit {
            range,
            new_text: format!("\"{}\"", candidate.path),
          }));
          item.insert_text = None;
        }
        item
      })
      .collect()
  }
}

// Compute the LSP range covering unquoted arg content from after the `(` to the cursor
// Returns None when the cursor is already at the start (nothing to replace)
fn compute_fref_unquoted_arg_range(
  node: &RedNode,
  rope: &Rope,
  cursor_offset: usize,
) -> Option<Range> {
  let call_node = find_ancestor(node, SyntaxKind::CallExpr)?;

  // Find the `(` token and get the offset right after it (start of arg content)
  let arg_start_offset = call_node
    .children()
    .find(|child| child.text() == "(")
    .map(|paren| paren.end_offset())?;

  // Only build a replace range if there's existing content before the cursor
  if cursor_offset <= arg_start_offset {
    return None;
  }

  Some(Range {
    start: byte_offset_to_lsp_position(rope, arg_start_offset),
    end: byte_offset_to_lsp_position(rope, cursor_offset),
  })
}

// Suggest fref("path") completions, optionally filtered by an expected type
pub(super) fn collect_fref_wrapped_completions(
  db: &TypedownDatabase,
  project: Project,
  expected_typ: Option<&TdTypEnum>,
) -> Vec<CompletionItem> {
  let candidates = collect_fref_candidates(db, project, expected_typ);
  candidates
    .iter()
    .map(|candidate| build_candidate_completion(candidate, |path| format!("fref(\"{path}\")")))
    .collect()
}
