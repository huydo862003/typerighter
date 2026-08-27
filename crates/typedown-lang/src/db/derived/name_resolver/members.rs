use std::collections::HashMap;

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::builtin_scope::builtin_scope;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::types::{
  File, HirValueKind, MembersResult, Project, Scope, ScopeKind, Symbol, SymbolKind,
};
use crate::db::utils::{is_content_file, is_type_file, lower_file};
use crate::syntax::ast::{AstNode, ClosureExpr};
use typedown_incremental::QueryDatabase;
use typedown_types::either::Either;

/// Schema-only members (fast path for _type resolution)
#[query_derived]
pub fn schema_members(db: &TypedownDatabase, project: Project) -> MembersResult {
  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let proj_files = project.files(db);

  let mut members = HashMap::new();
  for (path, file) in &proj_files {
    if !path.starts_with(&root_dir) || !is_type_file(path) {
      continue;
    }
    if let Some(sym) = file_symbol(db, project, *file).value(db) {
      let name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_string();
      members.insert(name, sym);
    }
  }

  MembersResult::new(db, members)
}

#[query_derived]
pub fn members(db: &TypedownDatabase, scope: Scope) -> MembersResult {
  match scope.kind(db) {
    ScopeKind::Builtin(_) => MembersResult::new(db, builtin_scope(db).members(db)),
    ScopeKind::File(project, file) => {
      let mut members = HashMap::new();

      if let Some(sym) = file_symbol(db, project, file).value(db) {
        let name = file
          .handle(db)
          .path()
          .and_then(|p| p.file_stem())
          .and_then(|s| s.to_str())
          .unwrap_or_default()
          .to_string();

        if !name.is_empty() {
          members.insert(name, sym);
        }

        // self resolves to the file's own resource symbol
        members.insert("self".to_string(), sym);
      }

      // Resolve _imports aliases into the file scope
      resolve_import_members(db, project, file, &mut members);

      MembersResult::new(db, members)
    }
    ScopeKind::Project(project) => {
      let proj_files = project.files(db);

      let mut members = HashMap::new();

      for (path, file) in &proj_files {
        if !is_content_file(path) {
          continue;
        }
        if let Some(sym) = file_symbol(db, project, *file).value(db) {
          let name = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
          members.insert(name, sym);
        }
      }

      MembersResult::new(db, members)
    }
    ScopeKind::Fn(project, file, value) => {
      let func = value.node(db);
      let closure = ClosureExpr::cast(func).expect("expected ClosureExpr");
      let mut members = HashMap::new();

      if let Some(params) = closure.params() {
        let param_idents: Vec<_> = match params {
          Either::Left(param_list) => param_list.params().collect(),
          Either::Right(ident) => vec![ident],
        };

        for ident in param_idents {
          if let Some(name) = ident.value()
            && name != "self"
          {
            let sym = Symbol::new(
              db,
              SymbolKind::FnParam(project, file, value),
              name.clone(),
              format!("@param::{}", name),
            );
            members.insert(name, sym);
          }
        }
      }

      MembersResult::new(db, members)
    }
  }
}

// Extract _imports from a file's frontmatter and register each alias as a member
fn resolve_import_members(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  members: &mut HashMap<String, Symbol>,
) {
  let (hir, _) = lower_file(db, project, file);
  let Some(hir) = hir else { return };
  let HirValueKind::Mapping(entries) = hir.kind(db) else {
    return;
  };
  let Some((_, imports_hir)) = entries.iter().find(|(k, _)| k == "_imports") else {
    return;
  };
  let HirValueKind::Mapping(import_entries) = imports_hir.kind(db) else {
    return;
  };

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);

  for (alias, path_hir) in import_entries {
    let HirValueKind::Str(path) = path_hir.kind(db) else {
      continue;
    };
    let target_path = root_dir.join(&path);
    let Some(&target_file) = project.files(db).get(&target_path) else {
      continue;
    };
    if let Some(sym) = file_symbol(db, project, target_file).value(db) {
      members.insert(alias, sym);
    }
  }
}
