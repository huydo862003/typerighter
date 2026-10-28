use typedown_macros::{query_derived, query_interned};
use typedown_types::path::normalize_path;

use crate::db::TypedownDatabase;
use crate::db::types::{AssetKind, File, Project, Symbol, SymbolKind};
use crate::db::utils::is_typ_file;
use typedown_incremental::QueryDatabase;

#[query_interned]
pub struct MaybeSymbol {
  pub value: Option<Symbol>,
}

#[query_derived]
pub fn file_symbol<'db>(db: &'db TypedownDatabase, project: Project, file: File) -> MaybeSymbol {
  let path = file.handle(db).path().cloned().unwrap_or_default();
  let is_schema_file = is_typ_file(&path);

  let name = path
    .file_stem()
    .and_then(|stem| stem.to_str())
    .unwrap_or_default()
    .to_string();

  let extension = path
    .extension()
    .and_then(|extension| extension.to_str())
    .unwrap_or_default();

  let kind = if let Some(asset_kind) = AssetKind::from_extension(extension) {
    SymbolKind::Asset(asset_kind, project, file)
  } else if is_schema_file {
    SymbolKind::UserDefinedSchema(project, file)
  } else {
    SymbolKind::UserDefinedResource(project, file)
  };

  let root = project.root_dir(db);
  let relative = path.strip_prefix(&*root).unwrap_or(&path);
  let def_id = format!("@vault::{}", normalize_path(relative));

  MaybeSymbol::new(db, Some(Symbol::new(db, kind, name, def_id)))
}
