//! Parse and evaluate the config file (typedown.yaml)

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node_with_expected;
use crate::db::derived::get_builtin_typs::get_config_typ;
use crate::db::derived::name_resolver::scope::get_builtin_runtime_scope;
use crate::db::types::{Project, ProjectRedNode, ResourceResult, TdTypEnum};
use crate::syntax::diagnostic::*;
use crate::syntax::green::cache::green_cache;
use crate::syntax::parse::ctx::{ParseCtx, ParseResult};
use crate::syntax::red::RedNode;
use typedown_incremental::QueryDatabase;

#[query_derived(no_hash)]
pub fn evaluate_config<'db>(db: &'db TypedownDatabase, project: Project) -> ResourceResult<'db> {
  let root_dir = project.root_dir(db);
  let files = project.files(db);

  let yaml_path = root_dir.join("typedown.yaml");
  let yml_path = root_dir.join("typedown.yml");

  let config_file = if let Some(file) = files.get(&yaml_path) {
    *file
  } else if let Some(file) = files.get(&yml_path) {
    *file
  } else {
    return ResourceResult::new(
      db,
      None,
      vec![
        TdDiagnosticKind::MissingVaultConfig {
          root_dir: root_dir.display().to_string(),
        }
        .into_error(0, 0),
      ],
    );
  };

  let handle = config_file.handle(db);
  let stream = match handle.open() {
    Ok(stream) => stream,
    Err(err) => {
      let path = handle
        .path()
        .map_or_else(|| "typedown.yaml".to_string(), |p| p.display().to_string());

      return ResourceResult::new(
        db,
        None,
        vec![
          TdDiagnosticKind::VaultConfigReadError {
            path,
            message: err.to_string(),
          }
          .into_error(0, 0),
        ],
      );
    }
  };

  // Parse
  let cache = green_cache();
  let ctx = ParseCtx::new(stream, cache);
  let ParseResult {
    mut diagnostics,
    ast,
  } = ctx.parse_yaml_document();
  let root = RedNode::new_root(ast.as_node().expect("AST root must be a node").clone());

  // Typecheck against built-in config type and evaluate
  let project_node = ProjectRedNode::new(project, config_file, root);
  let config_type: TdTypEnum = get_config_typ(db).into();
  let scope = get_builtin_runtime_scope(db, project);
  let result = evaluate_node_with_expected(db, project_node, &config_type, scope);
  diagnostics.extend(result.diagnostics(db).iter().cloned());

  ResourceResult::new(db, result.value(db), diagnostics)
}
