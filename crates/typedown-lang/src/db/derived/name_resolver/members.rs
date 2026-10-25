use std::collections::BTreeMap;

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::builtin_scope::builtin_scope;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::scope::parent_scope;
use crate::db::derived::parse_file::parse_file;
use crate::db::types::{File, MembersResult, Project, Scope, ScopeKind, Symbol, SymbolKind};
use crate::db::utils::{is_content_file, is_typ_file};
use crate::syntax::ast::{AstNode, ClosureExpr, StrLit};
use typedown_incremental::QueryDatabase;
use typedown_types::either::Either;

// Schema-only members for server-side schema lookup
#[query_derived]
pub fn schema_members<'db>(db: &'db TypedownDatabase, project: Project) -> MembersResult<'db> {
  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let project_files = project.files(db);

  let mut members = BTreeMap::new();
  for (path, file) in &*project_files {
    if !path.starts_with(&root_dir) || !is_typ_file(path) {
      continue;
    }
    if let Some(symbol) = file_symbol(db, project, *file).value(db) {
      let name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or_default()
        .to_string();
      members.insert(name, symbol);
    }
  }

  MembersResult::new(db, members)
}

#[query_derived]
pub fn members<'db>(db: &'db TypedownDatabase, scope: Scope) -> MembersResult<'db> {
  match scope.kind(db) {
    ScopeKind::Builtin(_) => MembersResult::new(db, builtin_scope(db).members(db)),
    ScopeKind::File(project, file) => {
      let mut members = BTreeMap::new();

      if let Some(symbol) = file_symbol(db, project, file).value(db) {
        let name = file
          .handle(db)
          .path()
          .and_then(|path| path.file_stem())
          .and_then(|string| string.to_str())
          .unwrap_or_default()
          .to_string();

        if !name.is_empty() {
          members.insert(name, symbol);
        }

        // self resolves to the file's own resource symbol
        members.insert("self".to_string(), symbol);
      }

      // Resolve _imports aliases into the file scope
      resolve_import_members(db, project, file, &mut members);

      MembersResult::new(db, members)
    }
    ScopeKind::Project(project) => {
      let project_files = project.files(db);

      let mut members = BTreeMap::new();

      for (path, file) in &*project_files {
        if !is_content_file(path) {
          continue;
        }
        if let Some(symbol) = file_symbol(db, project, *file).value(db) {
          let name = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default()
            .to_string();
          members.insert(name, symbol);
        }
      }

      MembersResult::new(db, members)
    }
    ScopeKind::Func(project, file, value) => {
      let func = value.node.clone();
      let closure = ClosureExpr::cast(func).expect("expected ClosureExpr");
      let mut members = BTreeMap::new();

      if let Some(params) = closure.params() {
        let param_idents: Vec<_> = match params {
          Either::Left(param_list) => param_list.params().collect(),
          Either::Right(ident) => vec![ident],
        };

        for ident in param_idents {
          if let Some(name) = ident.value()
            && name != "self"
          {
            let symbol = Symbol::new(
              db,
              SymbolKind::FuncParam(project, file, value.clone()),
              name.clone(),
              format!("@param::{}", name),
            );
            members.insert(name, symbol);
          }
        }
      }

      MembersResult::new(db, members)
    }
  }
}

// Collect all members visible at a scope by walking up the scope chain
// Inner scopes shadow outer scopes
pub fn all_visible_members(db: &TypedownDatabase, start: Scope) -> BTreeMap<String, Symbol> {
  let mut result = BTreeMap::new();
  let mut current = Some(start);

  // Walk from outermost to innermost so inner scopes shadow outer
  let mut chain = Vec::new();
  while let Some(scope) = current {
    chain.push(scope);
    current = parent_scope(db, scope).value(db);
  }

  for scope in chain.into_iter().rev() {
    for (name, symbol) in members(db, scope).members(db).iter() {
      result.insert(name.clone(), *symbol);
    }
  }

  result
}

// Extract _imports from a file's frontmatter and register each alias as a member
fn resolve_import_members(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  members: &mut BTreeMap<String, Symbol>,
) {
  let root_node = parse_file(db, project, file).ast(db);
  let entries = root_node.mapping_entries();
  let Some(entries) = entries else { return };
  let Some((_, imports_node)) = entries.iter().find(|(key, _)| key == "_imports") else {
    return;
  };
  let Some(import_entries) = imports_node.mapping_entries() else {
    return;
  };

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);

  for (alias, path_node) in import_entries {
    let Some(path) = StrLit::cast(path_node.node.clone()).and_then(|string| string.plain_value())
    else {
      continue;
    };
    let target_path = root_dir.join(&path);
    let Some(&target_file) = project.files(db).get(&target_path) else {
      continue;
    };
    let symbol = file_symbol(db, project, target_file);
    if let Some(symbol) = symbol.value(db) {
      members.insert(alias, symbol);
    }
  }
}
