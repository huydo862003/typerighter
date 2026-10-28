use typedown_incremental::StableCompare;
use typedown_lang::db::types::{Scope, ScopeKind, Symbol};
use typedown_lang::db::utils::{is_content_file, is_typ_file};

use crate::lsp::service::utils::symbol::get_resource_label;
use lsp_types::{
  CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, CompletionTextEdit,
  InsertTextFormat, Range, TextEdit,
};
use ropey::Rope;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_typ::evaluate_typ;
use typedown_lang::db::derived::get_component_contract::get_component_contract;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::members::{all_visible_members, members};
use typedown_lang::db::derived::name_resolver::scope::scope;
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::db::derived::typechecker::actual_node_typ::actual_node_typ;
use typedown_lang::db::derived::typechecker::expected_node_typ::expected_node_typ;
use typedown_lang::db::derived::typechecker::get_symbol_typ::get_symbol_typ;
use typedown_lang::db::types::typecheck::{is_nullable, is_subtyp_of};
use typedown_lang::db::types::{
  File, LazyTyp, LitValue, Project, ProjectRedNode, SymbolKind, TdStaticTyp, TdTypEnum,
};
use typedown_lang::db::utils::{get_mapping_schema_name, kebab_to_pascal};
use typedown_lang::syntax::ast::{
  AstNode, BinaryExpr, Expr, MdContainerBlock, MdContainerShorthand, YamlOpKind,
};
use typedown_lang::syntax::red::RedNode;
use typedown_lang::syntax::syntax_kind::SyntaxKind;

use crate::core::analysis::Analysis;
use crate::core::utils::ast::{
  find_ancestor, get_nearest_expr_ancestor, is_in_mapping_value_position, node_at_offset,
};
use crate::core::utils::position::{lsp_position_to_text_offset, text_offset_to_lsp_position};
use crate::core::utils::uri::uri_to_path;

pub fn resolve_completion(
  analysis: &Analysis,
  params: CompletionParams,
) -> Option<CompletionResponse> {
  let db = &analysis.db;
  let project = analysis.project;

  let path = uri_to_path(&params.text_document_position.text_document.uri)?;
  let rope = analysis.file_rope(&path)?;
  let offset = lsp_position_to_text_offset(&rope, params.text_document_position.position)?;

  let file = *project.files(db).get(&path)?;
  let root = parse_file(db, project, file).ast(db).node.clone();
  // Use offset-1 so the cursor position (between characters) resolves to the token just typed
  let lookup = offset.saturating_sub(1);
  let node = node_at_offset(root, lookup)?;

  // No completions inside code/math tokens (each is a single self-contained token)
  if matches!(
    node.kind(),
    SyntaxKind::CodeBlock | SyntaxKind::InlineCode | SyntaxKind::InlineMath | SyntaxKind::MathBlock
  ) {
    return None;
  }

  // Cursor inside container prop block: suggest prop names from the component contract
  if let Some(items) = collect_container_prop_completions(db, project, &node) {
    return Some(CompletionResponse::Array(items));
  }

  // Cursor on a === slot separator: suggest slot names from the component contract
  if let Some(items) = collect_container_slot_completions(db, project, &node) {
    return Some(CompletionResponse::Array(items));
  }

  // Cursor in a _typ value: suggest schema names
  if is_typ_value_position(&node) {
    return Some(CompletionResponse::Array(collect_schema_completions(
      db, project,
    )));
  }

  // Cursor after "." in a dot expression: suggest members of the LHS type
  if let Some(items) = collect_dot_access_completions(db, project, file, &node) {
    return Some(CompletionResponse::Array(items));
  }

  // Cursor inside a fref() string argument: suggest file paths
  if is_fref_arg_position(&node) {
    return Some(CompletionResponse::Array(collect_fref_completions(
      db, project, file, &node, &rope, offset,
    )));
  }

  // Cursor inside ${} interpolation: suggest scope variables and fref("path") completions
  if is_interp_position(&node) {
    let mut items = collect_fref_wrapped_completions(db, project, None);
    let scope = Scope::new(db, ScopeKind::File(project, file));
    for (name, symbol) in all_visible_members(db, scope) {
      if name.starts_with('_') {
        continue;
      }
      let kind = match symbol.kind(db) {
        SymbolKind::BuiltinMacro(_) => CompletionItemKind::FUNCTION,
        SymbolKind::BuiltinGlobal(_) => CompletionItemKind::VARIABLE,
        SymbolKind::UserDefinedSchema(..) => CompletionItemKind::CLASS,
        _ => CompletionItemKind::VARIABLE,
      };
      items.push(CompletionItem {
        label: name,
        kind: Some(kind),
        ..Default::default()
      });
    }
    return Some(CompletionResponse::Array(items));
  }

  // Schema-typed field value (including empty values): suggest fref("path") completions
  if let Some(typ) = get_declared_field_typ_at_value(db, project, file, &node)
    && (typ.is_td_schema_typ() || has_nullable_member(db, &typ, TdTypEnum::is_td_schema_typ))
  {
    let items = collect_fref_wrapped_completions(db, project, Some(&typ));
    if !items.is_empty() {
      return Some(CompletionResponse::Array(items));
    }
  }

  // Cursor in a field value: suggest value completions (booleans, null for optional fields)
  if let Some(items) = collect_value_completions(db, project, file, &node) {
    return Some(CompletionResponse::Array(items));
  }

  // Cursor in a mapping key or blank line: suggest field names from the declared type
  if let Some((typ, mapping)) = find_enclosing_mapping_typ(db, project, file, &node) {
    let existing = collect_existing_keys(&mapping);
    return Some(CompletionResponse::Array(
      collect_field_completions_from_typ(db, &typ, &existing),
    ));
  }

  None
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

// Suggest members of the LHS type after a "." in a binary expression
// Returns Some (even if empty) when we're clearly in a dot position, to prevent fallthrough to interp/fref completions which would be irrelevant
fn collect_dot_access_completions(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<Vec<CompletionItem>> {
  let binary_node = find_ancestor(node, SyntaxKind::BinaryExpr)?;
  let binary_expr = BinaryExpr::cast(binary_node)?;

  if !matches!(binary_expr.op()?.kind()?, YamlOpKind::Dot) {
    return None;
  }

  // We're in a dot-access position, always return Some to prevent fallthrough
  let mut items = Vec::new();

  let Some(lhs) = binary_expr.left() else {
    return Some(items);
  };
  let project_node = ProjectRedNode::new(project, file, lhs.syntax().clone());
  let Some(lhs_typ) = actual_node_typ(db, project_node).typ(db) else {
    return Some(items);
  };

  for (name, _) in lhs_typ.get_fields(db) {
    // Skip internal fields
    if name.starts_with('_') {
      continue;
    }
    items.push(CompletionItem {
      label: name,
      kind: Some(CompletionItemKind::FIELD),
      ..Default::default()
    });
  }

  for (name, _) in lhs_typ.static_vtable(db) {
    items.push(CompletionItem {
      label: name,
      kind: Some(CompletionItemKind::METHOD),
      ..Default::default()
    });
  }

  Some(items)
}

// Returns true if the cursor is inside a ${} interpolation
fn is_interp_position(node: &RedNode) -> bool {
  find_ancestor(node, SyntaxKind::InterpFragment).is_some()
}

// Returns true if the cursor is inside the argument of a fref() call
fn is_fref_arg_position(node: &RedNode) -> bool {
  // Walk up to find a CallExpr, checking for StrLit or direct position
  let string_lit = find_ancestor(node, SyntaxKind::StrLit);
  let call = match string_lit {
    Some(ref lit) => find_ancestor(lit, SyntaxKind::CallExpr),
    None => find_ancestor(node, SyntaxKind::CallExpr),
  };
  let Some(call) = call else {
    return false;
  };
  call
    .children()
    .next()
    .is_some_and(|callee| callee.text().trim() == "fref")
}

// A resolved fref candidate with display info
struct FrefCandidate {
  // Vault-relative path (e.g. "people/alice.td")
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
      let relative_string = relative.to_string_lossy().into_owned();
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
  let expected_typ = get_declared_field(db, project, file, node);
  let candidates = collect_fref_candidates(db, project, expected_typ.as_ref());
  let inside_str = find_ancestor(node, SyntaxKind::StrLit).is_some();

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
    start: text_offset_to_lsp_position(rope, arg_start_offset),
    end: text_offset_to_lsp_position(rope, cursor_offset),
  })
}

// Resolve the declared field type at a value position or empty value after a colon
fn get_declared_field_typ_at_value<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypEnum<'db>> {
  if is_in_mapping_value_position(node) {
    return get_declared_field(db, project, file, node);
  }

  // Empty value: resolve from the entry key name
  let entry = find_ancestor(node, SyntaxKind::YamlMappingEntry)?;
  let key_text = get_entry_key_text(&entry)?;
  let mapping = entry
    .parent()
    .filter(|parent| parent.kind() == SyntaxKind::YamlMapping)?;
  resolve_field_typ_from_schema(db, project, &mapping, &key_text)
}

// Check if a nullable type (T?) has a member satisfying the predicate
fn has_nullable_member<'db>(
  db: &'db TypedownDatabase,
  typ: &TdTypEnum<'db>,
  predicate: fn(&TdTypEnum<'db>) -> bool,
) -> bool {
  typ.as_td_sum_typ().is_some_and(|sum| {
    sum
      .members(db)
      .iter()
      .any(|member| member.resolve(db).is_some_and(|typ| predicate(&typ)))
  })
}

// Suggest fref("path") completions, optionally filtered by an expected type
fn collect_fref_wrapped_completions(
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

fn find_enclosing_mapping_typ<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<(TdTypEnum<'db>, RedNode)> {
  if is_in_mapping_value_position(node) {
    return None;
  }
  let mapping = find_ancestor(node, SyntaxKind::YamlMapping)?;

  // Explicit _typ in this mapping
  if let Some(schema_name) = get_mapping_schema_name(&mapping) {
    let scope = Scope::new(db, ScopeKind::Project(project));
    let symbol = *members(db, scope).members(db).get(&schema_name)?;
    let typ = evaluate_typ(db, symbol).typ(db)?;
    return Some((typ, mapping));
  }

  // No explicit _typ, resolve via the parent field's declared type
  let mapping_expr = Expr::cast(mapping.clone())?;
  let project_node = ProjectRedNode::new(project, file, mapping_expr.syntax().clone());
  let typ = expected_node_typ(db, project_node).typ(db)?;
  if typ.is_td_product_typ() || typ.is_td_schema_typ() {
    return Some((typ, mapping));
  }
  None
}

// Suggest value completions based on the declared field type
fn collect_value_completions(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<Vec<CompletionItem>> {
  if !is_in_mapping_value_position(node) {
    return None;
  }

  let mut items = vec![make_keyword_item("true"), make_keyword_item("false")];

  // Variables in scope (icon, fref, vault, file names, imports, closure params)
  // Find the nearest Expr ancestor to resolve scope (captures enclosing closures)
  let current_scope = get_nearest_expr_ancestor(node)
    .map(|expr| {
      let project_node = ProjectRedNode::new(project, file, expr.syntax().clone());
      scope(db, project_node)
    })
    .unwrap_or_else(|| Scope::new(db, ScopeKind::File(project, file)));

  for (name, symbol) in all_visible_members(db, current_scope) {
    if name.starts_with('_') {
      continue;
    }
    // Assets are not valid expression values
    if matches!(symbol.kind(db), SymbolKind::Asset(..)) {
      continue;
    }
    let kind = match symbol.kind(db) {
      SymbolKind::BuiltinMacro(_) => CompletionItemKind::FUNCTION,
      SymbolKind::BuiltinGlobal(_) => CompletionItemKind::VARIABLE,
      SymbolKind::UserDefinedSchema(..) => CompletionItemKind::CLASS,
      _ => CompletionItemKind::VARIABLE,
    };
    // fref inserts a snippet with quotes and cursor inside
    if name == "fref" {
      items.push(CompletionItem {
        label: name,
        kind: Some(kind),
        insert_text: Some(r#"fref("$1")"#.to_string()),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
      });
    } else {
      items.push(CompletionItem {
        label: name,
        kind: Some(kind),
        ..Default::default()
      });
    }
  }

  // Type-specific completions from declared field type
  let Some(typ) = get_declared_field(db, project, file, node) else {
    return Some(items);
  };

  if is_nullable(db, &typ) {
    items.push(make_keyword_item("null"));
  }

  // When cursor is already inside a string literal, completions must not add surrounding quotes
  let inside_str = find_ancestor(node, SyntaxKind::StrLit).is_some();

  // Enum values from union of literals
  collect_enum_items(db, &typ, inside_str, &mut items);

  // Date placeholder
  if typ.is_td_date_typ() || has_nullable_member(db, &typ, TdTypEnum::is_td_date_typ) {
    let (label, insert) = if inside_str {
      ("YYYY-MM-DD", "$1")
    } else {
      (r#""YYYY-MM-DD""#, r#""$1""#)
    };
    items.push(CompletionItem {
      label: label.to_string(),
      insert_text: Some(insert.to_string()),
      insert_text_format: Some(InsertTextFormat::SNIPPET),
      detail: Some("ISO 8601 date".to_string()),
      kind: Some(CompletionItemKind::VALUE),
      ..Default::default()
    });
  }

  // List scaffold
  if typ.is_td_list_typ() || has_nullable_member(db, &typ, TdTypEnum::is_td_list_typ) {
    items.push(CompletionItem {
      label: "- ...".to_string(),
      insert_text: Some(
        r#"
  - $1"#
          .to_string(),
      ),
      insert_text_format: Some(InsertTextFormat::SNIPPET),
      detail: Some("List".to_string()),
      kind: Some(CompletionItemKind::SNIPPET),
      ..Default::default()
    });
  }

  Some(items)
}

// Collect literal values from a union type as completion items
// When inside_str is true, string values are inserted without surrounding quotes
fn collect_enum_items(
  db: &TypedownDatabase,
  typ: &TdTypEnum,
  inside_str: bool,
  items: &mut Vec<CompletionItem>,
) {
  let sum = if let Some(sum) = typ.as_td_sum_typ() {
    sum
  } else {
    return;
  };
  for member in sum.members(db) {
    let Some(resolved_member) = member.resolve(db) else {
      continue;
    };
    let Some(lit) = resolved_member.as_td_lit_typ() else {
      continue;
    };
    let (label, insert_text, detail) = match lit.value(db) {
      LitValue::String(string) => {
        let label = format!("\"{string}\"");
        let insert = if inside_str {
          string.clone()
        } else {
          label.clone()
        };
        (label, insert, "string".to_string())
      }
      LitValue::Number(number) => (number.clone(), number.clone(), "number".to_string()),
      LitValue::Bool(bool) => (bool.to_string(), bool.to_string(), "boolean".to_string()),
    };
    items.push(CompletionItem {
      label,
      insert_text: Some(insert_text),
      detail: Some(detail),
      kind: Some(CompletionItemKind::ENUM_MEMBER),
      ..Default::default()
    });
  }
}

// Resolve the declared type for the field whose value the cursor is in
fn get_declared_field<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
) -> Option<TdTypEnum<'db>> {
  let entry_value = find_ancestor(node, SyntaxKind::YamlMappingEntryValue)?;

  // Try the value expression first
  if let Some(value_expr) = entry_value.children().find_map(Expr::cast) {
    let project_node = ProjectRedNode::new(project, file, value_expr.syntax().clone());
    if let Some(typ) = expected_node_typ(db, project_node).typ(db) {
      return Some(typ);
    }
  }

  // Fall back to looking up the field name in the enclosing schema
  let entry = entry_value.parent()?;
  let key_text = get_entry_key_text(&entry)?;
  let mapping = find_ancestor(&entry, SyntaxKind::YamlMapping)?;
  resolve_field_typ_from_schema(db, project, &mapping, &key_text)
}

// Extract the key text from a YamlMappingEntry node
fn get_entry_key_text(entry: &RedNode) -> Option<String> {
  entry
    .children()
    .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)
    .map(|key| key.text().trim().to_string())
}

// Look up a field's declared type from the enclosing schema
fn resolve_field_typ_from_schema<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  mapping: &RedNode,
  key: &str,
) -> Option<TdTypEnum<'db>> {
  let schema_name = get_mapping_schema_name(mapping)?;
  let scope = Scope::new(db, ScopeKind::Project(project));
  let symbol = *members(db, scope).members(db).get(&schema_name)?;
  let typ = evaluate_typ(db, symbol).typ(db)?;
  let schema = typ.as_td_schema_typ()?;
  let prop = schema.fields(db).get(key)?.clone();
  prop.field_typ.resolve(db)
}

// Build a keyword completion item (true, false, null)
fn make_keyword_item(label: &str) -> CompletionItem {
  CompletionItem {
    label: label.to_string(),
    kind: Some(CompletionItemKind::KEYWORD),
    ..Default::default()
  }
}

// Suggest all user-defined schema names visible in the project scope
fn collect_schema_completions(db: &TypedownDatabase, project: Project) -> Vec<CompletionItem> {
  let scope = Scope::new(db, ScopeKind::Project(project));
  members(db, scope)
    .members(db)
    .iter()
    .filter(|(_, symbol)| matches!(symbol.kind(db), SymbolKind::UserDefinedSchema(..)))
    .map(|(name, symbol)| {
      let snippet = build_schema_snippet(db, name, symbol);

      CompletionItem {
        label: name.clone(),
        kind: Some(CompletionItemKind::CLASS),
        insert_text: Some(snippet),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
      }
    })
    .collect()
}

// Build a snippet with all schema fields as placeholders
fn build_schema_snippet(db: &TypedownDatabase, name: &str, symbol: &Symbol) -> String {
  let typ = evaluate_typ(db, *symbol).typ(db);
  let schema = typ.as_ref().and_then(|typ| typ.as_td_schema_typ());

  let Some(schema) = schema else {
    return name.to_string();
  };

  let fields = schema.fields(db);
  let mut snippet = name.to_string();
  for (tab_stop, (field_name, prop_descriptor)) in fields.iter().enumerate() {
    let placeholder = make_snippet_placeholder(db, &prop_descriptor.field_typ, 0);
    let index = tab_stop + 1;

    snippet.push_str(&format!("\n{field_name}: ${{{index}:{placeholder}}}"));
  }

  snippet
}

// Generate a placeholder string for a lazy type
fn make_snippet_placeholder(db: &TypedownDatabase, lazy: &LazyTyp, indent: usize) -> String {
  let Some(typ) = lazy.resolve(db) else {
    return "value".to_string();
  };
  match typ {
    TdTypEnum::TdSumTyp(sum) => {
      let mut members: Vec<_> = sum.members(db).into_iter().collect();
      members.sort_by(|a, b| a.stable_cmp(db, b));
      // Optional type: use non-null member's placeholder
      let non_null: Vec<_> = members
        .iter()
        .filter(|member| {
          member
            .resolve(db)
            .is_none_or(|typ| typ.as_td_null_typ().is_none())
        })
        .collect();
      if non_null.len() == 1 {
        return make_snippet_placeholder(db, non_null[0], indent);
      }
      // Enum: use first literal string option as default
      let first = members.iter().find_map(|member| {
        if let Some(TdTypEnum::TdLitTyp(lit)) = member.resolve(db)
          && let LitValue::String(string) = lit.value(db)
        {
          return Some(string);
        }
        None
      });
      first.unwrap_or_else(|| "value".to_string())
    }
    _ => make_simple_typ_placeholder(db, &typ, indent),
  }
}

fn make_simple_typ_placeholder(db: &TypedownDatabase, typ: &TdTypEnum, indent: usize) -> String {
  match typ {
    TdTypEnum::TdStringTyp(_) => "string".to_string(),
    TdTypEnum::TdNumberTyp(_) => "0".to_string(),
    TdTypEnum::TdBoolTyp(_) => "true".to_string(),
    TdTypEnum::TdDateTyp(_) => "date".to_string(),
    TdTypEnum::TdDateTimeTyp(_) => "datetime".to_string(),
    TdTypEnum::TdTimeTyp(_) => "time".to_string(),
    TdTypEnum::TdListTyp(list) => {
      let inner = list
        .element(db)
        .and_then(|element| element.resolve(db))
        .map(|element| make_simple_typ_placeholder(db, &element, indent + 1))
        .unwrap_or_else(|| "value".to_string());
      let pad = "  ".repeat(indent);

      format!("\n{pad}- {inner}")
    }
    TdTypEnum::TdSchemaTyp(schema) => {
      format!("fref(\"{}\")", schema.name(db))
    }
    TdTypEnum::TdProductTyp(product) => {
      let fields = product.get_fields(db);
      let pad = "  ".repeat(indent + 1);
      let mut nested = String::new();
      for (field_name, field_lazy) in &fields {
        let placeholder = make_snippet_placeholder(db, field_lazy, indent + 1);
        nested.push_str(&format!("\n{pad}{field_name}: {placeholder}"));
      }
      nested
    }
    _ => "value".to_string(),
  }
}

// Collect existing key names from a mapping node
fn collect_existing_keys(mapping: &RedNode) -> Vec<String> {
  mapping
    .children()
    .filter(|child| child.kind() == SyntaxKind::YamlMappingEntry)
    .filter_map(|entry| {
      entry
        .children()
        .find(|child| child.kind() == SyntaxKind::YamlMappingEntryKey)
        .map(|key| key.text().trim().to_string())
    })
    .collect()
}

// Suggest field names from a resolved type, excluding already-present keys
fn collect_field_completions_from_typ(
  db: &TypedownDatabase,
  typ: &TdTypEnum,
  existing: &[String],
) -> Vec<CompletionItem> {
  let fields = typ.get_fields(db);
  let mut items: Vec<CompletionItem> = fields
    .iter()
    .filter(|(field, _)| !existing.iter().any(|key| key == *field))
    .map(|(field, lazy)| {
      let optional = lazy.resolve(db).is_some_and(|typ| is_nullable(db, &typ));
      let detail = if optional { "optional" } else { "required" };

      CompletionItem {
        label: field.clone(),
        kind: Some(CompletionItemKind::FIELD),
        detail: Some(detail.to_string()),
        sort_text: Some(format!("{}{}", if optional { "1" } else { "0" }, field)),
        ..Default::default()
      }
    })
    .collect();

  // Add _meta snippet if not already present
  if !existing.iter().any(|key| key == "_meta") {
    items.push(CompletionItem {
      label: "_meta".to_string(),
      kind: Some(CompletionItemKind::SNIPPET),
      detail: Some("SEO overrides".to_string()),
      insert_text: Some(
        r#"_meta:
  title: "$1"
  description: "$2""#
          .to_string(),
      ),
      insert_text_format: Some(InsertTextFormat::SNIPPET),
      sort_text: Some("2_meta".to_string()),
      ..Default::default()
    });
  }

  items
}

// Suggest prop names when the cursor is inside a container's prop block ({...})
fn collect_container_prop_completions(
  db: &TypedownDatabase,
  project: Project,
  node: &RedNode,
) -> Option<Vec<CompletionItem>> {
  // Must be inside a MdContainerPropBlock
  let prop_block = find_ancestor(node, SyntaxKind::MdContainerPropBlock)?;
  // Walk up to the container block
  let container = prop_block.parent()?;
  if container.kind() != SyntaxKind::MdContainerBlock
    && container.kind() != SyntaxKind::MdContainerShorthand
  {
    return None;
  }

  let (component_name, existing) = if let Some(block) = MdContainerBlock::cast(container.clone()) {
    (block.label()?, block.prop_names())
  } else {
    let shorthand = MdContainerShorthand::cast(container)?;
    (shorthand.label()?, shorthand.prop_names())
  };
  let pascal = kebab_to_pascal(&component_name);

  let contract_file = project.custom_component_contract(db)?;
  let contract = get_component_contract(db, contract_file);
  let entries = contract.entries(db).clone();
  let entry = entries
    .iter()
    .find(|entry| entry.name == component_name || entry.name == pascal)?;

  let props = entry.props.as_ref()?;

  let items = props
    .iter()
    .filter(|prop| !existing.iter().any(|entry| entry == &prop.name))
    .map(|prop| {
      let insert = if prop.required {
        format!("{}=", prop.name)
      } else {
        prop.name.clone()
      };
      CompletionItem {
        label: prop.name.clone(),
        kind: Some(CompletionItemKind::PROPERTY),
        detail: Some(format!(
          "{}{}",
          prop.typ_name,
          if prop.required { "" } else { "?" }
        )),
        documentation: prop.description.as_ref().map(|documentation| {
          lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: documentation.clone(),
          })
        }),
        insert_text: Some(insert),
        ..Default::default()
      }
    })
    .collect();

  Some(items)
}

// Suggest slot names when the cursor is on a === separator line inside a container
fn collect_container_slot_completions(
  db: &TypedownDatabase,
  project: Project,
  node: &RedNode,
) -> Option<Vec<CompletionItem>> {
  // Must be inside or adjacent to a MdContainerSlotSeparator
  let separator = if node.kind() == SyntaxKind::MdContainerSlotSeparator {
    node.clone()
  } else {
    find_ancestor(node, SyntaxKind::MdContainerSlotSeparator)?
  };

  let container = separator.parent()?;
  let block = MdContainerBlock::cast(container)?;
  let component_name = block.label()?;
  let pascal = kebab_to_pascal(&component_name);

  let contract_file = project.custom_component_contract(db)?;
  let contract = get_component_contract(db, contract_file);
  let entries = contract.entries(db).clone();
  let entry = entries
    .iter()
    .find(|entry| entry.name == component_name || entry.name == pascal)?;

  if entry.slots.is_empty() {
    return None;
  }

  let existing = block.slot_names();
  let items = entry
    .slots
    .iter()
    .filter(|slot| !existing.iter().any(|entry| entry == *slot))
    .map(|slot| CompletionItem {
      label: slot.clone(),
      kind: Some(CompletionItemKind::ENUM_MEMBER),
      detail: Some("slot".to_string()),
      ..Default::default()
    })
    .collect();

  Some(items)
}

#[cfg(test)]
mod tests {
  use super::resolve_completion as completion;
  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;
  use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionParams, CompletionResponse, PartialResultParams,
    Position, TextDocumentIdentifier, TextDocumentPositionParams, Uri, WorkDoneProgressParams,
  };
  use ropey::Rope;
  use std::collections::HashMap;
  use std::path::PathBuf;
  use std::sync::{Arc, Condvar, Mutex};
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

  const VAULT_CONFIG: &str = r#"version: "1"
vault:
  root_dir: "."
"#;
  const SCHEMA_PERSON: &str = r#"---
_type: schema
properties:
  name:
    type: string
  age:
    type: number
  verified:
    type: boolean
  nickname:
    type: string?
---
"#;
  const SCHEMA_EVENT: &str = r#"---
_type: schema
properties:
  name:
    type: string
  date:
    type: date
---
"#;

  const SCHEMA_TASK: &str = r#"---
_type: schema
properties:
  title:
    type: string
  status:
    type: ['todo', 'in_progress', 'done']
  assignee:
    type: Person?
---
"#;

  // Schema with a nested inline object field (no named type reference)
  const SCHEMA_PERSON_WITH_ADDRESS: &str = r#"---
_type: schema
properties:
  name:
    type: string
  address:
    type:
      street:
        type: string
      city:
        type: string
---
"#;

  /// Strip the `|` cursor marker from content and return its char offset
  fn cursor(content: &str) -> (String, usize) {
    let offset = content
      .find('|')
      .expect("content must have a '|' cursor marker");
    (content.replacen('|', "", 1), offset)
  }

  /// Build CompletionParams from a URI, content string, and char offset
  fn make_params(uri: Uri, content: &str, offset: usize) -> CompletionParams {
    let rope = Rope::from(content);
    let line = rope.char_to_line(offset);
    let character = offset - rope.line_to_char(line);
    CompletionParams {
      text_document_position: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri },
        position: Position {
          line: line as u32,
          character: character as u32,
        },
      },
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
      context: None,
    }
  }

  /// Build an in-memory vault with Person and Event schemas, plus the given content file
  fn setup(content: &str) -> (Analysis, Uri) {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });

    let type_root = root.join("_types");

    let test_path = root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let config_file = File::new(
      &db,
      FileHandle::Content(
        root.join("typedown.yaml"),
        VAULT_CONFIG.to_string(),
        FileMetadata::default(),
      ),
    );
    let person_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Person.td"),
        SCHEMA_PERSON.to_string(),
        FileMetadata::default(),
      ),
    );
    let event_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Event.td"),
        SCHEMA_EVENT.to_string(),
        FileMetadata::default(),
      ),
    );
    let task_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Task.td"),
        SCHEMA_TASK.to_string(),
        FileMetadata::default(),
      ),
    );
    let person_with_address_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("PersonWithAddress.td"),
        SCHEMA_PERSON_WITH_ADDRESS.to_string(),
        FileMetadata::default(),
      ),
    );
    let test_file = File::new(
      &db,
      FileHandle::Content(
        test_path.clone(),
        content.to_string(),
        FileMetadata::default(),
      ),
    );

    let files = HashMap::from([
      (root.join("typedown.yaml"), config_file),
      (root.join("_types/Person.td"), person_file),
      (root.join("_types/Event.td"), event_file),
      (root.join("_types/Task.td"), task_file),
      (
        root.join("_types/PersonWithAddress.td"),
        person_with_address_file,
      ),
      (test_path, test_file),
    ]);

    let project = Project::new(&db, root, files, None);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    (analysis, uri)
  }

  #[test]
  fn schema_name_completion_in_typ_value() {
    let (content, offset) = cursor(
      r#"---
_type: |
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completion items");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"Person"), "should suggest Person schema");
    assert!(labels.contains(&"Event"), "should suggest Event schema");
    // Should NOT suggest non-schema items (field names, file paths, etc.)
    assert!(
      !labels.contains(&"name"),
      "should not suggest field names in _type position"
    );
    assert!(
      !labels.contains(&"age"),
      "should not suggest field names in _type position"
    );
    // All items should be class-kind (schemas)
    assert!(
      items
        .iter()
        .all(|item| item.kind == Some(CompletionItemKind::CLASS)),
      "all _type completions should be CLASS kind"
    );
  }

  #[test]
  fn schema_completion_generates_snippet_with_fields() {
    let (content, offset) = cursor(
      r#"---
_type: |
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completion items");
    };

    let person = items
      .iter()
      .find(|item| item.label == "Person")
      .expect("should have Person");
    let snippet = person
      .insert_text
      .as_ref()
      .expect("should have insert_text");

    assert!(
      snippet.starts_with("Person\n"),
      "snippet should start with schema name: {snippet}"
    );
    assert!(
      snippet.contains("name:"),
      "snippet should contain name field: {snippet}"
    );
    assert!(
      snippet.contains("age:"),
      "snippet should contain age field: {snippet}"
    );
    assert!(
      snippet.contains("string"),
      "string field should have string placeholder: {snippet}"
    );
    assert!(
      snippet.contains(":0}"),
      "number field should have 0 placeholder: {snippet}"
    );
    assert!(
      person.insert_text_format == Some(lsp_types::InsertTextFormat::SNIPPET),
      "should use snippet format"
    );
  }

  #[test]
  fn schema_snippet_includes_enum_and_relation() {
    let (content, offset) = cursor(
      r#"---
_type: |
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completion items");
    };

    let task = items
      .iter()
      .find(|item| item.label == "Task")
      .expect("should have Task");
    let snippet = task.insert_text.as_ref().expect("should have insert_text");

    assert!(
      snippet.contains("done") || snippet.contains("todo"),
      "enum field should have first option as placeholder: {snippet}"
    );
    assert!(
      snippet.contains(r#"fref("Person")"#),
      "relation field should have fref placeholder: {snippet}"
    );
  }

  #[test]
  fn schema_name_completion_while_partially_typd() {
    // Cursor in the middle of a partially typed schema name
    let (content, offset) = cursor(
      r#"---
_type: Per|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completion items");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"Person"), "should still suggest Person");
  }

  #[test]
  fn field_completion_based_on_declared_typ() {
    // Cursor after typing a partial key, _typ already set
    let (content, offset) = cursor(
      r#"---
_type: Person
na|:
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"name"), "should suggest 'name' field");
    assert!(labels.contains(&"age"), "should suggest 'age' field");
    // Should NOT suggest _type itself as a field completion
    assert!(
      !labels.contains(&"_type"),
      "'_type' should not be suggested as a field completion"
    );
  }

  #[test]
  fn field_completion_when_typ_declared_after_other_fields() {
    // _typ appears after the cursor position in the mapping
    let (content, offset) = cursor(
      r#"---
name: Alice
ag|:
_type: Person
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      !labels.contains(&"name"),
      "should not suggest 'name' (already exists)"
    );
    assert!(labels.contains(&"age"), "should suggest 'age' field");
    assert!(
      labels.contains(&"verified"),
      "should suggest 'verified' field"
    );
  }

  #[test]
  fn no_field_completion_without_typ() {
    // No _typ in mapping: no field completions expected
    let (content, offset) = cursor(
      r#"---
na|:
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let is_empty = response.is_none()
      || matches!(response, Some(CompletionResponse::Array(ref items)) if items.is_empty());
    assert!(is_empty, "should not suggest fields when _typ is absent");
  }

  #[test]
  fn field_completion_on_blank_line_between_fields() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
|
age: 30
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions on blank line");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"verified"),
      "should suggest 'verified' on blank line between fields"
    );
  }

  #[test]
  fn field_completion_on_last_line_of_mapping() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions on last line of mapping");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"age"),
      "should suggest 'age' on last blank line"
    );
  }

  #[test]
  fn field_completion_excludes_existing_keys() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
age: 30
|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      !labels.contains(&"name"),
      "should not suggest 'name' (already exists)"
    );
    assert!(
      !labels.contains(&"age"),
      "should not suggest 'age' (already exists)"
    );
    assert!(
      labels.contains(&"verified"),
      "should suggest 'verified' (not yet used)"
    );
    assert!(
      labels.contains(&"nickname"),
      "should suggest 'nickname' (not yet used)"
    );
  }

  #[test]
  fn field_completion_on_key_excludes_other_existing_keys() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
ver|:
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      !labels.contains(&"name"),
      "should not suggest 'name' (already exists)"
    );
    assert!(labels.contains(&"age"), "should suggest 'age'");
    assert!(labels.contains(&"verified"), "should suggest 'verified'");
  }

  #[test]
  fn no_completion_in_plain_markdown_body() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
---

Some bod|y text.
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let is_empty = response.is_none()
      || matches!(response, Some(CompletionResponse::Array(ref items)) if items.is_empty());
    assert!(
      is_empty,
      "should not suggest anything in plain markdown body"
    );
  }

  // Inside ${} interpolation, suggest fref("path") completions and scope variables
  #[test]
  fn fref_completion_in_interpolation() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
---

Reference: ${|}
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected fref completions inside interpolation");
    };

    assert!(
      !items.is_empty(),
      "should suggest fref completions inside ${{}}"
    );

    // Should have fref("path") as insert text
    let has_fref_insert = items.iter().any(|item| {
      item
        .insert_text
        .as_deref()
        .is_some_and(|text| text.starts_with(r#"fref(""#) && text.ends_with(r#"")"#))
    });
    assert!(
      has_fref_insert,
      "should wrap path in fref(): {:?}",
      items
        .iter()
        .map(|item| (&item.label, &item.insert_text))
        .collect::<Vec<_>>()
    );

    // Label should be the display name, not the path
    let alice_item = items
      .iter()
      .find(|item| item.label == "Alice Chen")
      .or_else(|| items.iter().find(|item| item.label.contains("alice")));
    assert!(
      alice_item.is_some(),
      "should have alice completion with human-readable label: {:?}",
      items.iter().map(|item| &item.label).collect::<Vec<_>>()
    );

    // Detail should be the path
    let alice = alice_item.unwrap();
    assert!(
      alice
        .detail
        .as_deref()
        .is_some_and(|detail| detail.contains("alice")),
      "detail should contain the file path: {:?}",
      alice.detail
    );

    // Should also have scope variables like `self`, `fref`, `icon`
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"self"),
      "should suggest 'self' variable inside ${{}}:\n{labels:?}"
    );
    assert!(
      labels.contains(&"fref"),
      "should suggest 'fref' builtin inside ${{}}:\n{labels:?}"
    );
  }

  #[test]
  fn boolean_keywords_suggested_in_any_value_position() {
    // true/false are keywords usable in any value position, not limited to boolean-typed fields
    let (content, offset) = cursor(
      r#"---
_type: Person
name: tru|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"true"),
      "should suggest 'true' in any value position"
    );
    assert!(
      labels.contains(&"false"),
      "should suggest 'false' in any value position"
    );
    assert!(
      !labels.contains(&"null"),
      "non-optional field should not suggest 'null'"
    );
  }

  #[test]
  fn null_completion_for_optional_field() {
    // Cursor in the value of a nullable field: suggest null
    let (content, offset) = cursor(
      r#"---
_type: Person
nickname: nu|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"null"),
      "optional field should suggest 'null'"
    );
  }

  // A schema with a field typed as another schema (Person)
  const SCHEMA_DIRECTORY: &str = r#"---
_type: schema
properties:
  featured:
    type: Person
---
"#;

  const SCHEMA_TASK_REF: &str = r#"---
_type: schema
properties:
  title:
    type: string
  assignee:
    type: Person?
---
"#;

  // Schema with list[string], list[number], dict[string,string], and list[Person] fields
  const SCHEMA_COLLECTION: &str = r#"---
_type: schema
properties:
  tags:
    type: list[string]
  scores:
    type: list[number]
  labels:
    type: dict[string, string]
  authors:
    type: list[Person]
---
"#;

  const CONTENT_ALICE: &str = r#"---
_type: Person
_label: "Alice Chen"
name: Alice
age: 30
---
"#;

  const CONTENT_BIRTHDAY: &str = r#"---
_type: Event
name: Birthday
date: 2024-01-01
---
"#;

  /// Build a vault with Person, Event, Directory schemas plus two content files
  fn setup_with_content(content: &str) -> (Analysis, Uri) {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });

    let type_root = root.join("_types");

    let test_path = root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let config_file = File::new(
      &db,
      FileHandle::Content(
        root.join("typedown.yaml"),
        VAULT_CONFIG.to_string(),
        FileMetadata::default(),
      ),
    );
    let person_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Person.td"),
        SCHEMA_PERSON.to_string(),
        FileMetadata::default(),
      ),
    );
    let event_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Event.td"),
        SCHEMA_EVENT.to_string(),
        FileMetadata::default(),
      ),
    );
    let directory_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Directory.td"),
        SCHEMA_DIRECTORY.to_string(),
        FileMetadata::default(),
      ),
    );
    let task_reference_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("TaskRef.td"),
        SCHEMA_TASK_REF.to_string(),
        FileMetadata::default(),
      ),
    );
    let collection_file = File::new(
      &db,
      FileHandle::Content(
        type_root.join("Collection.td"),
        SCHEMA_COLLECTION.to_string(),
        FileMetadata::default(),
      ),
    );
    let alice_file = File::new(
      &db,
      FileHandle::Content(
        root.join("alice.td"),
        CONTENT_ALICE.to_string(),
        FileMetadata::default(),
      ),
    );
    let birthday_file = File::new(
      &db,
      FileHandle::Content(
        root.join("birthday.td"),
        CONTENT_BIRTHDAY.to_string(),
        FileMetadata::default(),
      ),
    );
    let editing_file = File::new(
      &db,
      FileHandle::Content(
        test_path.clone(),
        content.to_string(),
        FileMetadata::default(),
      ),
    );

    let files = HashMap::from([
      (root.join("typedown.yaml"), config_file),
      (root.join("_types/Person.td"), person_file),
      (root.join("_types/Event.td"), event_file),
      (root.join("_types/Directory.td"), directory_file),
      (root.join("_types/TaskRef.td"), task_reference_file),
      (root.join("_types/Collection.td"), collection_file),
      (root.join("alice.td"), alice_file),
      (root.join("birthday.td"), birthday_file),
      (test_path, editing_file),
    ]);

    let project = Project::new(&db, root, files, None);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    (analysis, uri)
  }

  #[test]
  fn fref_completion_filters_by_declared_field_typ() {
    // The 'featured' field on Directory expects type Person
    // Only content/alice.td (_type: Person) should be suggested, not content/birthday.td (_type: Event)
    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: fref("|")
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected fref completions");
    };
    // Detail contains the path, label is the display name
    let details: Vec<String> = items
      .iter()
      .filter_map(|item| item.detail.clone())
      .collect();
    assert!(
      details.iter().any(|detail| detail.contains("alice")),
      "should suggest alice.td (Person type), got: {:?}",
      details
    );
    assert!(
      !details.iter().any(|detail| detail.contains("birthday")),
      "should not suggest birthday.td (Event type), got: {:?}",
      details
    );
  }

  // fref completions should use vault-relative paths, not project-relative paths
  // When root_dir is "vault", fref("alice.td") not fref("vault/alice.td")
  #[test]
  fn fref_completion_uses_vault_relative_paths() {
    let project_root = PathBuf::from(if cfg!(windows) {
      "C:\\project"
    } else {
      "/project"
    });
    let vault_root = project_root.join("vault");
    let typ_root = vault_root.join("_types");

    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: fref("|")
---
"#,
    );

    let test_path = vault_root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let nested_config = r#"version: "1"
vault:
  root_dir: "vault"
"#;

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let files = HashMap::from([
      (
        project_root.join("typedown.yaml"),
        File::new(
          &db,
          FileHandle::Content(
            project_root.join("typedown.yaml"),
            nested_config.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        typ_root.join("Person.td"),
        File::new(
          &db,
          FileHandle::Content(
            typ_root.join("Person.td"),
            SCHEMA_PERSON.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        typ_root.join("Directory.td"),
        File::new(
          &db,
          FileHandle::Content(
            typ_root.join("Directory.td"),
            SCHEMA_DIRECTORY.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        vault_root.join("alice.td"),
        File::new(
          &db,
          FileHandle::Content(
            vault_root.join("alice.td"),
            CONTENT_ALICE.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        test_path.clone(),
        File::new(
          &db,
          FileHandle::Content(test_path, content.clone(), FileMetadata::default()),
        ),
      ),
    ]);

    let project = Project::new(&db, project_root, files, None);
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    let params = make_params(uri, &content, offset);
    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected fref completions");
    };
    // Detail contains the path, should be vault-relative
    let details: Vec<String> = items
      .iter()
      .filter_map(|item| item.detail.clone())
      .collect();

    // Should suggest "alice.td", not "vault/alice.td"
    assert!(
      details.iter().any(|detail| detail == "alice.td"),
      "should suggest vault-relative path 'alice.td', got: {:?}",
      details
    );
    assert!(
      !details.iter().any(|detail| detail.contains("vault/")),
      "should not include vault dir prefix in path, got: {:?}",
      details
    );
  }

  // fref completions include _label as detail and basename in filterText
  #[test]
  fn fref_completion_includes_label_and_filter_text() {
    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: fref("|")
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected fref completions");
    };

    let alice_item = items
      .iter()
      .find(|item| item.label == "Alice Chen")
      .expect("should have alice completion with display name as label");

    // detail should contain the vault-relative path
    assert_eq!(
      alice_item.detail.as_deref(),
      Some("alice.td"),
      "detail should be the vault-relative path"
    );

    // filterText should include path, basename, and label for fuzzy matching
    let filter = alice_item.filter_text.as_deref().unwrap_or("");
    assert!(
      filter.contains("alice") && filter.contains("Alice Chen"),
      "filterText should contain basename and label: {filter}"
    );
  }

  // fref(|) without quotes wraps the path in quotes: insert "path" not bare path
  #[test]
  fn fref_without_quotes_wraps_path_in_quotes() {
    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: fref(|)
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected fref completions inside fref() without quotes");
    };

    let alice_item = items
      .iter()
      .find(|item| {
        item
          .detail
          .as_deref()
          .is_some_and(|detail| detail.contains("alice"))
      })
      .expect("should suggest alice.td");

    let insert = alice_item
      .insert_text
      .as_deref()
      .expect("should have insert_text");
    assert!(
      insert.starts_with('"') && insert.ends_with('"'),
      "insert_text should be quoted path, got: {insert}"
    );
    assert!(
      insert.contains("alice"),
      "insert_text should contain the path, got: {insert}"
    );
  }

  // Empty value on a schema-typed field suggests fref snippet
  #[test]
  fn schema_typd_empty_value_suggests_fref() {
    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: |
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions for empty schema-typed field");
    };

    // Should have specific file suggestions with fref("path") insert text
    let has_alice = items.iter().any(|item| {
      item
        .insert_text
        .as_deref()
        .is_some_and(|text| text.contains("alice"))
    });
    assert!(
      has_alice,
      "should suggest alice.td: {:?}",
      items.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
  }

  // Typing in a schema-typed field value auto-suggests fref completions
  #[test]
  fn schema_typd_field_suggests_fref() {
    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: a|
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions for schema-typed field");
    };

    // Should suggest alice with fref() wrapper
    let alice_item = items
      .iter()
      .find(|item| {
        item
          .insert_text
          .as_deref()
          .is_some_and(|text| text.contains("alice"))
      })
      .expect("should suggest alice for Person-typed field");

    assert!(
      alice_item
        .insert_text
        .as_deref()
        .is_some_and(|text| text.starts_with(r#"fref(""#) && text.ends_with(r#"")"#)),
      "insert_text should wrap path in fref(): {:?}",
      alice_item.insert_text
    );
  }

  // Nullable schema field (Person?) also auto-suggests fref on empty value
  #[test]
  fn nullable_schema_field_suggests_fref() {
    let (content, offset) = cursor(
      r#"---
_type: TaskRef
title: "Test"
assignee: |
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions for nullable schema field");
    };

    let has_fref = items.iter().any(|item| {
      item
        .insert_text
        .as_deref()
        .is_some_and(|text| text.contains("fref("))
    });
    assert!(
      has_fref,
      "nullable Person? field should suggest fref completions: {:?}",
      items.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
  }

  // Non-schema field (string) should NOT auto-suggest fref
  #[test]
  fn string_field_does_not_suggest_fref() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: a|
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };

    // Wrapped fref completions like fref("alice.td") should not appear for string fields
    // The bare fref function is fine since it's always in scope
    let has_wrapped_fref = items.iter().any(|item| {
      item.kind == Some(CompletionItemKind::REFERENCE)
        && item
          .insert_text
          .as_deref()
          .is_some_and(|text| text.contains("fref("))
    });
    assert!(
      !has_wrapped_fref,
      "string field should not suggest fref file completions: {:?}",
      items.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
  }

  // Resource files should never appear as bare completions (no insert_text wrapping)
  // They should only be suggested via fref() when the field type is a schema
  #[test]
  fn resource_files_not_suggested_as_bare_completions_in_string_field() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: a|
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions");
    };

    // No REFERENCE items (resource file completions) should appear
    let has_resource = items
      .iter()
      .any(|item| item.kind == Some(CompletionItemKind::REFERENCE));
    assert!(
      !has_resource,
      "resource files should not appear as bare completions: {:?}",
      items.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
  }

  // Resource files should not appear in value completions outside of fref context
  #[test]
  fn resource_files_not_suggested_outside_fref_context() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: |
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    if let Some(CompletionResponse::Array(items)) = response {
      let resource_labels: Vec<&str> = items
        .iter()
        .filter(|item| item.kind == Some(CompletionItemKind::REFERENCE))
        .map(|item| item.label.as_str())
        .collect();
      assert!(
        resource_labels.is_empty(),
        "resource files should not appear in plain string field: {:?}",
        resource_labels
      );
    }
  }

  #[test]
  fn resource_files_not_suggested_in_number_field() {
    let (content, offset) = cursor(
      r#"---
_type: Person
age: |
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    if let Some(CompletionResponse::Array(items)) = response {
      let has_resource = items
        .iter()
        .any(|item| item.kind == Some(CompletionItemKind::REFERENCE));
      assert!(
        !has_resource,
        "resource files should not appear in number field: {:?}",
        items.iter().map(|item| &item.label).collect::<Vec<_>>()
      );
    }
  }

  #[test]
  fn resource_files_not_suggested_in_object_type_field() {
    let (content, offset) = cursor(
      r#"---
_type: PersonWithAddress
address: |
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    if let Some(CompletionResponse::Array(items)) = response {
      let has_resource = items
        .iter()
        .any(|item| item.kind == Some(CompletionItemKind::REFERENCE));
      assert!(
        !has_resource,
        "resource files should not appear in object-typed field: {:?}",
        items.iter().map(|item| &item.label).collect::<Vec<_>>()
      );
    }
  }

  #[test]
  fn resource_files_not_suggested_inside_quoted_string() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: "ali|"
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    if let Some(CompletionResponse::Array(items)) = response {
      let has_resource = items
        .iter()
        .any(|item| item.kind == Some(CompletionItemKind::REFERENCE));
      assert!(
        !has_resource,
        "resource files should not appear inside a quoted string value: {:?}",
        items.iter().map(|item| &item.label).collect::<Vec<_>>()
      );
    }
  }

  fn assert_no_reference_items(items: &[CompletionItem], ctx: &str) {
    let found: Vec<&str> = items
      .iter()
      .filter(|item| item.kind == Some(CompletionItemKind::REFERENCE))
      .map(|item| item.label.as_str())
      .collect();
    assert!(
      found.is_empty(),
      "{ctx}: unexpected resource items: {found:?}"
    );
  }

  #[test]
  fn resource_files_not_suggested_in_list_of_string_field() {
    let (content, offset) = cursor(
      r#"---
_type: Collection
tags:
  - t|
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);
    if let Some(CompletionResponse::Array(items)) = completion(&analysis, params) {
      assert_no_reference_items(&items, "list[string] item");
    }
  }

  #[test]
  fn resource_files_not_suggested_in_list_of_number_field() {
    let (content, offset) = cursor(
      r#"---
_type: Collection
scores:
  - |
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);
    if let Some(CompletionResponse::Array(items)) = completion(&analysis, params) {
      assert_no_reference_items(&items, "list[number] item");
    }
  }

  #[test]
  fn resource_files_not_suggested_in_dict_value_field() {
    let (content, offset) = cursor(
      r#"---
_type: Collection
labels:
  key: v|
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);
    if let Some(CompletionResponse::Array(items)) = completion(&analysis, params) {
      assert_no_reference_items(&items, "dict[string,string] value");
    }
  }

  // list[Person] items SHOULD suggest fref-wrapped completions, not bare resource names
  #[test]
  fn list_of_schema_field_suggests_fref_not_bare_resource() {
    let (content, offset) = cursor(
      r#"---
_type: Collection
authors:
  - |
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);
    if let Some(CompletionResponse::Array(items)) = completion(&analysis, params) {
      // No bare REFERENCE items (insert_text without fref wrapper)
      let has_bare_resource = items.iter().any(|item| {
        item.kind == Some(CompletionItemKind::REFERENCE)
          && item
            .insert_text
            .as_deref()
            .is_none_or(|text| !text.starts_with("fref("))
      });
      assert!(
        !has_bare_resource,
        "list[Person] items should not appear as bare resource names: {:?}",
        items
          .iter()
          .map(|item| (&item.label, &item.insert_text))
          .collect::<Vec<_>>()
      );
    }
  }

  // Enum field suggests literal values
  #[test]
  fn enum_field_suggests_literal_values() {
    let (content, offset) = cursor(
      r#"---
_type: Task
status: t|
priority: "high"
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions for enum field");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&r#""todo""#),
      "should suggest 'todo': {:?}",
      labels
    );
    assert!(
      labels.contains(&r#""in_progress""#),
      "should suggest 'in_progress': {:?}",
      labels
    );
    assert!(
      labels.contains(&r#""done""#),
      "should suggest 'done': {:?}",
      labels
    );
  }

  // Date field suggests ISO 8601 placeholder
  #[test]
  fn date_field_suggests_date_placeholder() {
    let (content, offset) = cursor(
      r#"---
_type: Event
date: d|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions for date field");
    };
    let has_date = items.iter().any(|item| item.label.contains("YYYY-MM-DD"));
    assert!(
      has_date,
      "date field should suggest ISO format: {:?}",
      items.iter().map(|item| &item.label).collect::<Vec<_>>()
    );
  }

  // Cursor on a key inside a nested mapping whose type is inferred from the parent schema field
  #[test]
  fn field_completion_in_nested_mapping_without_typ() {
    let (content, offset) = cursor(
      r#"---
_type: PersonWithAddress
name: Alice
address:
  str|:
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions for nested address mapping");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"street"),
      "should suggest 'street' from nested address type, got: {:?}",
      labels
    );
    assert!(
      labels.contains(&"city"),
      "should suggest 'city' from nested address type, got: {:?}",
      labels
    );
  }

  // Expression and dot-access completion tests

  #[test]
  fn value_position_suggests_builtin_variables() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: i|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(labels.contains(&"icon"), "should suggest builtin 'icon'");
    assert!(labels.contains(&"fref"), "should suggest builtin 'fref'");
    assert!(labels.contains(&"vault"), "should suggest builtin 'vault'");
  }

  #[test]
  fn value_position_suggests_typ_names() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: s|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"string"),
      "should suggest builtin type 'string'"
    );
    assert!(
      labels.contains(&"number"),
      "should suggest builtin type 'number'"
    );
    assert!(
      labels.contains(&"boolean"),
      "should suggest builtin type 'boolean'"
    );
  }

  #[test]
  fn value_position_suggests_project_file_names() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: a|
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"alice"),
      "should suggest project file 'alice'"
    );
    assert!(
      labels.contains(&"birthday"),
      "should suggest project file 'birthday'"
    );
  }

  #[test]
  fn value_position_excludes_underscore_prefixed() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: x|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      !labels.iter().any(|label| label.starts_with('_')),
      "should not suggest underscore-prefixed items: {:?}",
      labels
    );
  }

  #[test]
  fn value_position_includes_keywords_and_variables() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: ic|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    // Keywords
    assert!(labels.contains(&"true"), "should include keyword 'true'");
    assert!(labels.contains(&"false"), "should include keyword 'false'");
    // Variables
    assert!(labels.contains(&"icon"), "should include variable 'icon'");
  }

  #[test]
  fn dot_access_on_icon_suggests_icon_names() {
    let (content, offset) = cursor(
      r#"---
_icon: icon.|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected dot-access completions for icon");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"book"),
      "should suggest 'book' icon: {:?}",
      labels
    );
    assert!(
      labels.contains(&"star"),
      "should suggest 'star' icon: {:?}",
      labels
    );
    // Should NOT include the "icon." prefix (just the member name)
    assert!(
      !labels.iter().any(|label| label.contains('.')),
      "labels should be bare names, not dotted: {:?}",
      labels
    );
  }

  #[test]
  fn dot_access_on_icon_with_partial_input() {
    let (content, offset) = cursor(
      r#"---
_icon: icon.bo|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected dot-access completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"book"),
      "should suggest 'book' with partial 'bo': {:?}",
      labels
    );
  }

  #[test]
  fn no_dot_access_before_dot_typd() {
    // Typing just "ic" in _icon should NOT trigger icon member completions
    let (content, offset) = cursor(
      r#"---
_icon: ic|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    if let Some(CompletionResponse::Array(items)) = response {
      let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
      // Should not have icon member names like "book" without the dot
      assert!(
        !labels.contains(&"book"),
        "should not suggest icon members without dot: {:?}",
        labels
      );
    }
  }

  #[test]
  fn dot_access_on_icon_in_regular_field() {
    // Dot-access should work in any value position, not just _icon
    let (content, offset) = cursor(
      r#"---
_type: Person
name: icon.|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected dot-access completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"book"),
      "dot-access should work in any field: {:?}",
      labels
    );
  }

  // After ${fref("...").| in markdown body, only schema field names should be suggested,
  // not file paths or scope variables from the interp fallthrough
  #[test]
  fn dot_access_on_fref_in_markdown_body_suggests_fields_not_file_paths() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
---

${fref("alice.td").|
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected dot-access completions in markdown body");
    };

    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();

    // Should suggest schema field names
    assert!(
      labels.contains(&"name"),
      "should suggest 'name' field from Person schema: {:?}",
      labels
    );
    assert!(
      labels.contains(&"age"),
      "should suggest 'age' field from Person schema: {:?}",
      labels
    );

    // Should NOT suggest file paths or fref-wrapped completions
    let has_fref_completion = items.iter().any(|item| {
      item
        .insert_text
        .as_deref()
        .is_some_and(|text| text.starts_with("fref("))
    });
    assert!(
      !has_fref_completion,
      "should not suggest fref() file completions after dot: {:?}",
      labels
    );

    // Should NOT suggest scope variables like 'self' or 'fref' (those are for interp, not dot access)
    assert!(
      !labels.contains(&"self"),
      "should not suggest 'self' in dot-access position: {:?}",
      labels
    );
  }

  // When cursor is inside fref() with unquoted partial path (e.g. fref(some|)),
  // completion should use text_edit to replace the existing unquoted content with a quoted path
  #[test]
  fn fref_unquoted_arg_completion_uses_text_edit_to_replace_and_quote() {
    let (content, offset) = cursor(
      r#"---
_type: Directory
featured: fref(some|)
---
"#,
    );
    let (analysis, uri) = setup_with_content(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected fref completions");
    };

    let alice_item = items
      .iter()
      .find(|item| {
        item
          .detail
          .as_deref()
          .is_some_and(|detail| detail.contains("alice"))
      })
      .expect("should suggest alice.td");

    // Should have a text_edit that replaces existing unquoted content with quoted path
    let text_edit = alice_item
      .text_edit
      .as_ref()
      .expect("should have text_edit to replace unquoted arg");

    let new_text = match text_edit {
      lsp_types::CompletionTextEdit::Edit(edit) => &edit.new_text,
      lsp_types::CompletionTextEdit::InsertAndReplace(entry) => &entry.new_text,
    };
    assert!(
      new_text.starts_with('"') && new_text.ends_with('"'),
      "text_edit new_text should be a quoted path, got: {new_text}"
    );
    assert!(
      new_text.contains("alice"),
      "text_edit new_text should contain 'alice', got: {new_text}"
    );
  }

  #[test]
  fn closure_param_suggested_in_body() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: (x) -> x|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions inside closure body");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"x"),
      "should suggest closure param 'x': {:?}",
      labels
    );
    assert!(
      labels.contains(&"icon"),
      "should also suggest builtins: {:?}",
      labels
    );
  }

  #[test]
  fn multi_param_closure_suggests_all_params() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: (a, b) -> a|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected completions inside closure body");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"a"),
      "should suggest param 'a': {:?}",
      labels
    );
    assert!(
      labels.contains(&"b"),
      "should suggest param 'b': {:?}",
      labels
    );
  }

  #[test]
  fn self_suggested_in_value_position() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: s|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected value completions");
    };
    let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
    assert!(
      labels.contains(&"self"),
      "should suggest 'self': {:?}",
      labels
    );
  }

  #[test]
  fn field_completions_show_required_and_optional() {
    let (content, offset) = cursor(
      r#"---
_type: Person
|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };

    let name_item = items
      .iter()
      .find(|item| item.label == "name")
      .expect("should suggest name");
    assert_eq!(name_item.detail.as_deref(), Some("required"));

    let nickname_item = items
      .iter()
      .find(|item| item.label == "nickname")
      .expect("should suggest nickname");
    assert_eq!(nickname_item.detail.as_deref(), Some("optional"));
  }

  #[test]
  fn required_fields_sorted_before_optional() {
    let (content, offset) = cursor(
      r#"---
_type: Person
|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };

    let field_items: Vec<&CompletionItem> = items
      .iter()
      .filter(|item| item.kind == Some(CompletionItemKind::FIELD))
      .collect();

    // All required fields should have sort_text starting with "0"
    // All optional fields should have sort_text starting with "1"
    for item in &field_items {
      let sort = item.sort_text.as_deref().unwrap_or("");
      let detail = item.detail.as_deref().unwrap_or("");
      if detail == "required" {
        assert!(
          sort.starts_with('0'),
          "required field {} should sort first",
          item.label
        );
      } else {
        assert!(
          sort.starts_with('1'),
          "optional field {} should sort after required",
          item.label
        );
      }
    }
  }

  #[test]
  fn meta_snippet_suggested_in_field_position() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: "Alice"
|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };

    let meta_item = items
      .iter()
      .find(|item| item.label == "_meta")
      .expect("should suggest _meta");
    assert_eq!(meta_item.kind, Some(CompletionItemKind::SNIPPET));
    assert_eq!(meta_item.detail.as_deref(), Some("SEO overrides"));
    assert!(
      meta_item.insert_text.as_ref().unwrap().contains("title"),
      "_meta snippet should contain title placeholder"
    );
  }

  #[test]
  fn meta_snippet_not_suggested_when_already_present() {
    let (content, offset) = cursor(
      r#"---
_type: Person
_meta:
  title: "Custom"
name: "Alice"
|
---
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);

    let response = completion(&analysis, params);
    let Some(CompletionResponse::Array(items)) = response else {
      panic!("expected field completions");
    };

    assert!(
      !items.iter().any(|item| item.label == "_meta"),
      "_meta should not be suggested when already present"
    );
  }

  // No completions inside code/math tokens

  #[test]
  fn no_completions_inside_code_block() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
---

```js
co|de here
```
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);
    let response = completion(&analysis, params);
    assert!(
      response.is_none(),
      "should not suggest completions inside a code block"
    );
  }

  // No completions when cursor is inside a quoted string in code block body
  #[test]
  fn no_completions_inside_code_block_string_content() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
---

```
'tex|t'
```
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);
    let response = completion(&analysis, params);
    assert!(
      response.is_none(),
      "should not suggest completions inside a string within a code block"
    );
  }

  // No completions when cursor is inside a quoted string in the code fence info string
  #[test]
  fn no_completions_inside_code_block_string_lang() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
---

```'j|s'
code here
```
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);
    let response = completion(&analysis, params);
    assert!(
      response.is_none(),
      "should not suggest completions inside a quoted lang in a code block"
    );
  }

  #[test]
  fn no_completions_inside_inline_code() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
---

Some text with `inl|ine code` here.
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);
    let response = completion(&analysis, params);
    assert!(
      response.is_none(),
      "should not suggest completions inside inline code"
    );
  }

  #[test]
  fn no_completions_inside_inline_math() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
---

Some text with $x^|2$ here.
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);
    let response = completion(&analysis, params);
    assert!(
      response.is_none(),
      "should not suggest completions inside inline math"
    );
  }

  #[test]
  fn no_completions_inside_math_block() {
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Alice
---

$$
x^|2
$$
"#,
    );
    let (analysis, uri) = setup(&content);
    let params = make_params(uri, &content, offset);
    let response = completion(&analysis, params);
    assert!(
      response.is_none(),
      "should not suggest completions inside a math block"
    );
  }

  fn setup_with_contract(content: &str, contract_json: &str) -> (Analysis, Uri) {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let test_path = root.join("file.td");
    let uri = path_to_uri(&test_path, "file");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let config_file = File::new(
      &db,
      FileHandle::Content(
        root.join("typedown.yaml"),
        VAULT_CONFIG.to_string(),
        FileMetadata::default(),
      ),
    );
    let contract_path = root.join(".typedown/components.json");
    let contract_file = File::new(
      &db,
      FileHandle::Content(
        contract_path.clone(),
        contract_json.to_string(),
        FileMetadata::default(),
      ),
    );
    let test_file = File::new(
      &db,
      FileHandle::Content(
        test_path.clone(),
        content.to_string(),
        FileMetadata::default(),
      ),
    );

    let files = HashMap::from([
      (root.join("typedown.yaml"), config_file),
      (contract_path, contract_file),
      (test_path, test_file),
    ]);

    let project = Project::new(&db, root, files, Some(contract_file));
    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      Arc::new((Mutex::new(1), Condvar::new())),
    );

    (analysis, uri)
  }

  #[test]
  fn prop_completion_includes_description_as_documentation() {
    let contract_json = r#"{
      "TdAlert": {
        "source": "./TdAlert.vue",
        "props": {
          "kind": {
            "schema": { "type": ["string", "null"] },
            "description": "The alert variant to display"
          }
        }
      }
    }"#;
    let (content, offset) = cursor(
      r#"---
---
:::td-alert{|}
:::
"#,
    );
    let (analysis, uri) = setup_with_contract(&content, contract_json);
    let params = make_params(uri, &content, offset);
    let Some(CompletionResponse::Array(items)) = completion(&analysis, params) else {
      panic!("expected completion items");
    };
    let kind_item = items
      .iter()
      .find(|item| item.label == "kind")
      .expect("should suggest 'kind' prop");
    let doc = kind_item
      .documentation
      .as_ref()
      .expect("'kind' should have documentation");
    let doc_text = match doc {
      lsp_types::Documentation::MarkupContent(m) => m.value.as_str(),
      _ => panic!("expected markup content"),
    };
    assert!(
      doc_text.contains("The alert variant to display"),
      "documentation should contain the prop description: {doc_text}"
    );
  }

  #[test]
  fn prop_completion_without_description_has_no_documentation() {
    let contract_json = r#"{
      "TdAlert": {
        "source": "./TdAlert.vue",
        "props": {
          "kind": {
            "schema": { "type": ["string", "null"] }
          }
        }
      }
    }"#;
    let (content, offset) = cursor(
      r#"---
---
:::td-alert{|}
:::
"#,
    );
    let (analysis, uri) = setup_with_contract(&content, contract_json);
    let params = make_params(uri, &content, offset);
    let Some(CompletionResponse::Array(items)) = completion(&analysis, params) else {
      panic!("expected completion items");
    };
    let kind_item = items
      .iter()
      .find(|item| item.label == "kind")
      .expect("should suggest 'kind' prop");
    assert!(
      kind_item.documentation.is_none(),
      "prop without description should have no documentation, got: {:?}",
      kind_item.documentation
    );
  }
}
