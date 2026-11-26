use crate::core::analysis::Analysis;
use crate::core::utils::uri::path_to_uri;
use lsp_types::{
  CompletionItem, CompletionItemKind, CompletionParams, PartialResultParams, Position,
  TextDocumentIdentifier, TextDocumentPositionParams, Uri, WorkDoneProgressParams,
};
use ropey::Rope;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
use typedown_lang::db::{QueryStorage, TypedownDatabase};

pub(super) use crate::lsp::service::completion::typedown::resolve_completion as completion;

pub(super) const VAULT_CONFIG: &str = r#"version: "1"
vault:
  root_dir: "."
"#;

pub(super) const SCHEMA_PERSON: &str = r#"---
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

pub(super) const SCHEMA_EVENT: &str = r#"---
_type: schema
properties:
  name:
    type: string
  date:
    type: date
  start_time:
    type: time
  created_at:
    type: datetime
---
"#;

pub(super) const SCHEMA_TASK: &str = r#"---
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
pub(super) const SCHEMA_PERSON_WITH_ADDRESS: &str = r#"---
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
pub(super) fn cursor(content: &str) -> (String, usize) {
  let offset = content
    .find('|')
    .expect("content must have a '|' cursor marker");
  (content.replacen('|', "", 1), offset)
}

/// Build CompletionParams from a URI, content string, and char offset
pub(super) fn make_params(uri: Uri, content: &str, offset: usize) -> CompletionParams {
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
pub(super) fn setup(content: &str) -> (Analysis, Uri) {
  let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });

  let typ_root = root.join("_types");

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
      typ_root.join("Person.td"),
      SCHEMA_PERSON.to_string(),
      FileMetadata::default(),
    ),
  );
  let event_file = File::new(
    &db,
    FileHandle::Content(
      typ_root.join("Event.td"),
      SCHEMA_EVENT.to_string(),
      FileMetadata::default(),
    ),
  );
  let task_file = File::new(
    &db,
    FileHandle::Content(
      typ_root.join("Task.td"),
      SCHEMA_TASK.to_string(),
      FileMetadata::default(),
    ),
  );
  let person_with_address_file = File::new(
    &db,
    FileHandle::Content(
      typ_root.join("PersonWithAddress.td"),
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

// A schema with a field typed as another schema (Person)
pub(super) const SCHEMA_DIRECTORY: &str = r#"---
_type: schema
properties:
  featured:
    type: Person
---
"#;

pub(super) const SCHEMA_TASK_REF: &str = r#"---
_type: schema
properties:
  title:
    type: string
  assignee:
    type: Person?
---
"#;

// Schema with list[string], list[number], dict[string,string], and list[Person] fields
pub(super) const SCHEMA_COLLECTION: &str = r#"---
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
  members_by_role:
    type: dict[string, Person]
---
"#;

pub(super) const CONTENT_ALICE: &str = r#"---
_type: Person
_label: "Alice Chen"
name: Alice
age: 30
---
"#;

pub(super) const CONTENT_BIRTHDAY: &str = r#"---
_type: Event
name: Birthday
date: 2024-01-01
---
"#;

/// Build a vault with Person, Event, Directory schemas plus two content files
pub(super) fn setup_with_content(content: &str) -> (Analysis, Uri) {
  let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });

  let typ_root = root.join("_types");

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
      typ_root.join("Person.td"),
      SCHEMA_PERSON.to_string(),
      FileMetadata::default(),
    ),
  );
  let event_file = File::new(
    &db,
    FileHandle::Content(
      typ_root.join("Event.td"),
      SCHEMA_EVENT.to_string(),
      FileMetadata::default(),
    ),
  );
  let directory_file = File::new(
    &db,
    FileHandle::Content(
      typ_root.join("Directory.td"),
      SCHEMA_DIRECTORY.to_string(),
      FileMetadata::default(),
    ),
  );
  let task_reference_file = File::new(
    &db,
    FileHandle::Content(
      typ_root.join("TaskRef.td"),
      SCHEMA_TASK_REF.to_string(),
      FileMetadata::default(),
    ),
  );
  let collection_file = File::new(
    &db,
    FileHandle::Content(
      typ_root.join("Collection.td"),
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

pub(super) fn assert_no_reference_items(items: &[CompletionItem], ctx: &str) {
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

pub(super) fn setup_with_contract(content: &str, contract_json: &str) -> (Analysis, Uri) {
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

// Place test file inside _types/ so it's treated as a schema file
pub(super) fn setup_schema(content: &str) -> (Analysis, Uri) {
  let root = PathBuf::from(if cfg!(windows) { "C:\\vault" } else { "/vault" });
  let typ_root = root.join("_types");
  let test_path = typ_root.join("TestSchema.td");
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
