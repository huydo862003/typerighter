use typedown_macros::query_derived;
use typedown_types::path::normalize_path;

use crate::db::TypedownDatabase;
use crate::db::types::{AssetKind, File, Project, Symbol, SymbolKind};
use crate::db::utils::is_type_file;
use typedown_incremental::QueryDatabase;

#[query_derived]
pub struct MaybeSymbol<'db> {
  pub value: Option<Symbol>,
}

#[query_derived]
pub fn file_symbol<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
) -> MaybeSymbol<'db> {
  let path = file.handle(db).path().cloned().unwrap_or_default();
  let is_schema_file = is_type_file(&path);

  let name = path
    .file_stem()
    .and_then(|s| s.to_str())
    .unwrap_or_default()
    .to_string();

  let ext = path
    .extension()
    .and_then(|e| e.to_str())
    .unwrap_or_default();

  let kind = if let Some(asset_kind) = AssetKind::from_extension(ext) {
    SymbolKind::Asset(asset_kind, project, file)
  } else if is_schema_file {
    SymbolKind::UserDefinedSchema(project, file)
  } else {
    SymbolKind::UserDefinedResource(project, file)
  };

  let root = project.root_dir(db);
  let relative = path.strip_prefix(&root).unwrap_or(&path);
  let def_id = format!("@vault::{}", normalize_path(relative));

  MaybeSymbol::new(db, Some(Symbol::new(db, kind, name, def_id)))
}
