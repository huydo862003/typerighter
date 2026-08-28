//! Tracked query to parse a file into an AST

use typedown_macros::query_derived;

use crate::syntax::{
  green::cache::green_cache,
  parse::ctx::{ParseCtx, ParseResult},
  red::RedNode,
};

use crate::db::TypedownDatabase;
use crate::db::types::{File, FileAstResult, Project};
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn parse_file<'db>(db: &'db TypedownDatabase, project: Project, file: File) -> FileAstResult<'db> {
  let handle = file.handle(db);
  let stream = handle.open().expect("failed to open file");

  let cache = green_cache();
  let mut ctx = ParseCtx::new(stream, cache);
  let ParseResult { diagnostics, ast } = ctx.parse();

  let root = RedNode::new_root(ast.as_node().expect("AST root must be a node").clone());
  FileAstResult::new(
    db,
    file.handle(db),
    project,
    file,
    root,
    diagnostics.to_vec(),
  )
}

#[cfg(test)]
mod tests {
  use std::{collections::HashMap, path::PathBuf};

  use crate::syntax::ast::{AstNode, SourceFile};

  use crate::db::{
    QueryStorage, TypedownDatabase,
    fixtures::load_fixtures,
    types::{File, FileHandle, FileMetadata, Project},
  };

  use super::parse_file;

  #[test]
  fn parse_file_with_content_handle() {
    let fixtures = load_fixtures("parse_file");
    let fixture = fixtures.get("valid.td").expect("missing valid.td fixture");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let project = Project::new(&db, PathBuf::from("/"), HashMap::new());

    let file = File::new(
      &db,
      FileHandle::Content(
        PathBuf::from("test.td"),
        fixture.contents.clone(),
        FileMetadata::default(),
      ),
    );
    let result = parse_file(&db, project, file);

    assert!(
      SourceFile::cast(result.ast(&db)).is_some(),
      "AST root should be a SourceFile"
    );

    let diagnostics = result.diagnostics(&db);
    assert!(
      diagnostics.is_empty(),
      "Expected no diagnostics, got: {:?}",
      diagnostics
    );
  }

  #[test]
  fn parse_file_with_path_handle() {
    let fixtures = load_fixtures("parse_file");
    let fixture = fixtures.get("valid.td").expect("missing valid.td fixture");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let project = Project::new(&db, PathBuf::from("/"), HashMap::new());
    let file = File::new(
      &db,
      FileHandle::Path(fixture.path.clone(), FileMetadata::default()),
    );
    let result = parse_file(&db, project, file);

    assert!(
      SourceFile::cast(result.ast(&db)).is_some(),
      "AST root should be a SourceFile"
    );

    let diagnostics = result.diagnostics(&db);
    assert!(
      diagnostics.is_empty(),
      "Expected no diagnostics, got: {:?}",
      diagnostics
    );
  }

  #[test]
  fn parse_file_without_frontmatter_content_handle() {
    let fixtures = load_fixtures("parse_file");
    let fixture = fixtures
      .get("no_frontmatter.td")
      .expect("missing no_frontmatter.td fixture");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let project = Project::new(&db, PathBuf::from("/"), HashMap::new());
    let file = File::new(
      &db,
      FileHandle::Content(
        PathBuf::from("test.td"),
        fixture.contents.clone(),
        FileMetadata::default(),
      ),
    );
    let result = parse_file(&db, project, file);

    assert!(
      SourceFile::cast(result.ast(&db)).is_some(),
      "AST should be a SourceFile for file without frontmatter"
    );

    let diagnostics = result.diagnostics(&db);
    assert!(
      diagnostics.is_empty(),
      "No diagnostics expected for file without frontmatter: {:?}",
      diagnostics
    );
  }

  #[test]
  fn parse_file_without_frontmatter_path_handle() {
    let fixtures = load_fixtures("parse_file");
    let fixture = fixtures
      .get("no_frontmatter.td")
      .expect("missing no_frontmatter.td fixture");

    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let project = Project::new(&db, PathBuf::from("/"), HashMap::new());
    let file = File::new(
      &db,
      FileHandle::Path(fixture.path.clone(), FileMetadata::default()),
    );
    let result = parse_file(&db, project, file);

    assert!(
      SourceFile::cast(result.ast(&db)).is_some(),
      "AST should be a SourceFile for file without frontmatter"
    );

    let diagnostics = result.diagnostics(&db);
    assert!(
      diagnostics.is_empty(),
      "No diagnostics expected for file without frontmatter: {:?}",
      diagnostics
    );
  }
}
