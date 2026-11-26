//! File paths for the strings in the `_imports` mapping

use lsp_types::{CompletionItem, CompletionItemKind};
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::typechecker::get_symbol_typ::get_symbol_typ;
use typedown_lang::db::types::{Project, TdStaticTyp};
use typedown_lang::db::utils::{is_content_file, is_typ_file};

use crate::lsp::service::utils::cursor::CursorContext;
use crate::lsp::service::utils::symbol::get_resource_label;

pub(super) fn complete(ctx: &CursorContext) -> Vec<CompletionItem> {
  collect_imports_completions(ctx.db, ctx.project)
}

// Suggest file paths as bare vault-root-relative strings for _imports values
fn collect_imports_completions(db: &TypedownDatabase, project: Project) -> Vec<CompletionItem> {
  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  project
    .files(db)
    .iter()
    .filter(|(path, _)| path.starts_with(&root_dir) && is_content_file(path) && !is_typ_file(path))
    .filter_map(|(path, target_file)| {
      let symbol = file_symbol(db, project, *target_file).value(db)?;
      let relative = path.strip_prefix(&root_dir).ok()?;
      let vault_path = format!("/{}", relative.to_string_lossy());
      let label = get_resource_label(db, symbol);
      let display = label.clone().unwrap_or_else(|| {
        relative
          .file_stem()
          .and_then(|stem| stem.to_str())
          .unwrap_or("")
          .to_string()
      });
      let basename = relative
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string();
      let schema = get_symbol_typ(db, symbol)
        .typ(db)
        .map(|typ| typ.display_name(db));
      let mut filter_parts = vec![vault_path.clone(), basename];
      if let Some(ref label) = label {
        filter_parts.push(label.clone());
      }
      Some(CompletionItem {
        label: display,
        insert_text: Some(vault_path.clone()),
        detail: Some(vault_path),
        label_details: schema.map(|schema| lsp_types::CompletionItemLabelDetails {
          detail: Some(schema),
          description: None,
        }),
        filter_text: Some(filter_parts.join(" ")),
        kind: Some(CompletionItemKind::REFERENCE),
        ..Default::default()
      })
    })
    .collect()
}
