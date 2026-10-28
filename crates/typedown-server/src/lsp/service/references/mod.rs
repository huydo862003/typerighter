use crate::core::analysis::Analysis;
use crate::core::utils::position::{lsp_position_to_text_offset, node_trimmed_range};
use crate::core::utils::uri::{path_to_uri, uri_to_path};
use crate::lsp::service::utils::symbol::{
  CursorSymbol, find_symbol_at_cursor, get_string_content_node, symbol_file_path,
};
use lsp_types::{Location, Range, ReferenceParams};
use ropey::Rope;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::name_resolver::referee::referee;
use typedown_lang::db::derived::name_resolver::resolution_index::{ReferenceKind, references};
use typedown_lang::db::types::{File, Project, ProjectRedNode, Symbol, SymbolKind};
use typedown_lang::syntax::ast::{AstNode, CallExpr};
use typedown_lang::syntax::syntax_kind::SyntaxKind;

pub fn find_references(analysis: &Analysis, params: ReferenceParams) -> Option<Vec<Location>> {
  let db = &analysis.db;
  let project = analysis.project;
  let include_declaration = params.context.include_declaration;

  let uri = &params.text_document_position.text_document.uri;
  let path = uri_to_path(uri)?;
  let rope = analysis.file_rope(&path)?;

  let offset = lsp_position_to_text_offset(&rope, params.text_document_position.position)?;
  let file = *project.files(db).get(&path)?;

  // resolve the symbol the cursor is pointing at, either fref or ident
  let symbol = resolve_symbol(db, project, file, offset).filter(|symbol| {
    // builtin symbols (schema, fref macro) have no user references, treat as unresolved
    !matches!(
      symbol.kind(db),
      SymbolKind::BuiltinSchema(_) | SymbolKind::BuiltinMacro(_)
    )
  })?;

  // collect all project-wide references to the symbol
  let references = references(db, project, symbol);
  let mut locations: Vec<Location> = references
    .into_iter()
    .filter_map(|reference| {
      let reference_path = reference.node.file.handle(db).path()?.clone();
      let reference_rope = analysis.file_rope(&reference_path)?;

      let range = match reference.kind {
        ReferenceKind::Ident => node_trimmed_range(&reference_rope, &reference.node.node),
        ReferenceKind::Fref => fref_path_range(db, &reference_rope, &reference.node)
          .unwrap_or_else(|| node_trimmed_range(&reference_rope, &reference.node.node)),
      };
      let scheme = analysis
        .scheme_map
        .get(&reference_path)
        .map(String::as_str)
        .unwrap_or("file");
      Some(Location {
        uri: path_to_uri(&reference_path, scheme),
        range,
      })
    })
    .collect();

  if include_declaration && let Some(declaration_location) = declaration_location(analysis, db, symbol) {
    locations.push(declaration_location);
  }

  if locations.is_empty() {
    return None;
  }
  locations.sort_by(|first_location, second_location| {
    first_location
      .uri
      .as_str()
      .cmp(second_location.uri.as_str())
      .then_with(|| {
        first_location
          .range
          .start
          .line
          .cmp(&second_location.range.start.line)
      })
      .then_with(|| {
        first_location
          .range
          .start
          .character
          .cmp(&second_location.range.start.character)
      })
  });

  Some(locations)
}

/// Resolve the symbol at the given offset, handling fref and ident reference sites
fn resolve_symbol(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  offset: usize,
) -> Option<Symbol> {
  let rename_symbol = find_symbol_at_cursor(db, project, file, offset)?;
  let syntax = match &rename_symbol {
    CursorSymbol::Fref { call_node } => call_node.syntax().clone(),
    CursorSymbol::Identifier { ident_node } => ident_node.syntax().clone(),
  };
  referee(db, ProjectRedNode::new(project, file, syntax)).value(db)
}

/// Build the declaration Location for a symbol, pointing to the top of its backing file
fn declaration_location(
  analysis: &Analysis,
  db: &TypedownDatabase,
  symbol: Symbol,
) -> Option<Location> {
  let declaration_path = symbol_file_path(db, symbol)?;
  let scheme = analysis
    .scheme_map
    .get(&declaration_path)
    .map(String::as_str)
    .unwrap_or("file");
  Some(Location {
    uri: path_to_uri(&declaration_path, scheme),
    range: Range::default(),
  })
}

/// For a fref call node, return the range of the path string content
fn fref_path_range(
  _db: &TypedownDatabase,
  rope: &Rope,
  project_node: &ProjectRedNode,
) -> Option<Range> {
  let call = CallExpr::cast(project_node.node.clone())?;
  let first_arg = call.args().into_iter().next()?;
  let arg_node = first_arg.syntax().clone();
  // interpolated fref arguments have no single text node, fall back to full call
  if arg_node
    .children()
    .any(|child| child.kind() == SyntaxKind::InterpFragment)
  {
    return None;
  }
  let content = get_string_content_node(&arg_node)?;
  Some(node_trimmed_range(rope, &content))
}
#[cfg(test)]
mod tests {
  use super::find_references;
  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;
  use lsp_types::{
    PartialResultParams, Position, ReferenceContext, ReferenceParams, TextDocumentIdentifier,
    TextDocumentPositionParams, WorkDoneProgressParams,
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
---
"#;
  const CONTENT_ALICE: &str = r#"---
_type: Person
name: Alice
age: 30
---
"#;
  const CONTENT_BOB: &str = r#"---
_type: Person
name: Bob
age: 25
---
"#;
  fn cursor(content: &str) -> (String, usize) {
    let offset = content
      .find('|')
      .expect("content must have a cursor marker");
    (content.replacen('|', "", 1), offset)
  }
  fn make_params(
    uri: lsp_types::Uri,
    content: &str,
    offset: usize,
    include_declaration: bool,
  ) -> ReferenceParams {
    let rope = Rope::from(content);
    let line = rope.char_to_line(offset);
    let character = offset - rope.line_to_char(line);
    ReferenceParams {
      text_document_position: TextDocumentPositionParams {
        text_document: TextDocumentIdentifier { uri },
        position: Position {
          line: line as u32,
          character: character as u32,
        },
      },
      work_done_progress_params: WorkDoneProgressParams::default(),
      partial_result_params: PartialResultParams::default(),
      context: ReferenceContext {
        include_declaration,
      },
    }
  }
  fn setup_with_files(
    test_path: PathBuf,
    extra_files: Vec<(PathBuf, String)>,
  ) -> (Analysis, lsp_types::Uri) {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let uri = path_to_uri(&test_path, "file");
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };
    let mut files = HashMap::from([
      (
        root.join("typedown.yaml"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("typedown.yaml"),
            VAULT_CONFIG.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        root.join("_types/Person.td"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("_types/Person.td"),
            SCHEMA_PERSON.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        root.join("alice.td"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("alice.td"),
            CONTENT_ALICE.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
      (
        root.join("bob.td"),
        File::new(
          &db,
          FileHandle::Content(
            root.join("bob.td"),
            CONTENT_BOB.to_string(),
            FileMetadata::default(),
          ),
        ),
      ),
    ]);
    for (path, content) in extra_files {
      files.insert(
        path.clone(),
        File::new(
          &db,
          FileHandle::Content(path, content, FileMetadata::default()),
        ),
      );
    }
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
  fn setup(editing_path: PathBuf, editing_content: &str) -> (Analysis, lsp_types::Uri) {
    setup_with_files(
      editing_path.clone(),
      vec![(editing_path, editing_content.to_string())],
    )
  }
  // Cursor on _type: Person in a content file finds all usages
  #[test]
  fn references_on_type_ident_finds_all_usages() {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let test_path = root.join("file.td");
    let content_raw = r#"---
_type: Per|son
name: Alice
---
"#;
    let (content, offset) = cursor(content_raw);
    let (analysis, uri) = setup(test_path, &content);
    let params = make_params(uri, &content, offset, false);
    let locations = find_references(&analysis, params).expect("should find references");
    // alice.td, bob.td, file.td all have _type: Person, sorted by URI
    assert_eq!(
      locations.len(),
      3,
      "should find exactly 3 references, got: {:?}",
      locations
        .iter()
        .map(|location| location.uri.as_str())
        .collect::<Vec<_>>()
    );
    assert!(
      locations[0].uri.as_str().contains("alice"),
      "first should be alice.td"
    );
    assert!(
      locations[1].uri.as_str().contains("bob"),
      "second should be bob.td"
    );
    assert!(
      locations[2].uri.as_str().contains("file"),
      "third should be file.td"
    );
    // All point to "Person" on line 1, col 7-13
    for loc in &locations {
      assert_eq!(
        loc.range.start,
        Position {
          line: 1,
          character: 7
        }
      );
      assert_eq!(
        loc.range.end,
        Position {
          line: 1,
          character: 13
        }
      );
    }
    // declaration is not included when include_declaration is false
    assert!(
      !locations
        .iter()
        .any(|location| location.uri.as_str().contains("Person.td"))
    );
  }
  // include_declaration appends the schema definition file, pointing to its _type value
  #[test]
  fn references_with_include_declaration_adds_definition() {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let test_path = root.join("file.td");
    let content_raw = r#"---
_type: Per|son
name: Alice
---
"#;
    let (content, offset) = cursor(content_raw);
    let (analysis, uri) = setup(test_path, &content);
    let params = make_params(uri, &content, offset, true);
    let locations = find_references(&analysis, params).expect("should find references");
    let declaration = locations
      .iter()
      .find(|location| location.uri.as_str().contains("Person.td"))
      .expect("should include Person.td declaration");
    // Declaration always points to top of file
    assert_eq!(declaration.range, lsp_types::Range::default());
  }
  // Cursor on a builtin ident returns None, builtins have no user references
  #[test]
  fn references_on_builtin_ident_returns_none() {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let schema_path = root.join("_types/Person.td");
    let schema_raw = r#"---
_type: sc|hema
properties:
  name:
    type: string
---
"#;
    let (schema_content, offset) = cursor(schema_raw);
    let (analysis, uri) = setup(schema_path, &schema_content);
    let params = make_params(uri, &schema_content, offset, false);
    let result = find_references(&analysis, params);
    assert!(
      result.is_none(),
      "builtin schema ident should have no references"
    );
  }
  // Cursor on a fref argument returns a location whose range covers just the path string
  #[test]
  fn references_on_fref_range_covers_path_string() {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let test_path = root.join("file.td");
    let content_with_fref = r#"---
_type: Person
name: fref("ali|ce.td")
---
"#;
    let (content, offset) = cursor(content_with_fref);
    let (analysis, uri) = setup(test_path.clone(), &content);
    let params = make_params(uri.clone(), &content, offset, false);
    let locations = find_references(&analysis, params).expect("should find fref references");
    let fref_location = locations
      .iter()
      .find(|location| location.uri == uri)
      .expect("should include file.td (the fref site)");
    // the fref call is on line 2
    // alice.td starts right after the opening quote at col 12, ends at col 20
    assert_eq!(fref_location.range.start.line, 2, "fref is on line 2");
    assert_eq!(
      fref_location.range.start.character, 12,
      "path string starts at col 12 (after opening quote)"
    );
    assert_eq!(
      fref_location.range.end.character, 20,
      "path string ends at col 20"
    );
  }
  // Cursor on a plain string value returns None, does not fall back to file_symbol
  #[test]
  fn references_on_plain_value_returns_none() {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let test_path = root.join("file.td");
    let (content, offset) = cursor(
      r#"---
_type: Person
name: Ali|ce
---
"#,
    );
    let (analysis, uri) = setup(test_path, &content);
    let params = make_params(uri, &content, offset, false);
    let result = find_references(&analysis, params);
    assert!(
      result.is_none(),
      "plain string value should have no references"
    );
  }
  // Exact range: the reference location covers just the identifier text
  #[test]
  fn references_ident_range_covers_identifier() {
    let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
    let test_path = root.join("file.td");
    let (content, offset) = cursor(
      r#"---
_type: Per|son
name: Alice
---
"#,
    );
    let (analysis, uri) = setup(test_path, &content);
    let params = make_params(uri.clone(), &content, offset, false);
    let locations = find_references(&analysis, params).expect("should find references");
    // Find the location in file.td itself
    let own_location = locations
      .iter()
      .find(|location| location.uri == uri)
      .expect("should have a location in file.td");
    // "Person" is on line 1, columns 7-13
    assert_eq!(own_location.range.start.line, 1);
    assert_eq!(own_location.range.start.character, 7);
    assert_eq!(own_location.range.end.line, 1);
    assert_eq!(own_location.range.end.character, 13);
  }
}
