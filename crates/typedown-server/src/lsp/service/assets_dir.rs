use lsp_types::TextDocumentIdentifier;
use serde::{Deserialize, Serialize};
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::types::AssetsDirMode;

use crate::core::analysis::Analysis;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentIdentifierParams {
  pub text_document: TextDocumentIdentifier,
}

#[derive(Serialize)]
pub struct AssetsDirResponse {
  pub mode: String,
  pub path: String,
}

pub fn get_assets_dir(
  analysis: &Analysis,
  _params: TextDocumentIdentifierParams,
) -> Option<AssetsDirResponse> {
  let db = &analysis.db;
  let project = analysis.project;
  let config = get_vault_config(db, project);
  let assets_dir = config.assets_dir(db);

  let mode = match assets_dir.mode {
    AssetsDirMode::Local => "local".to_string(),
  };

  Some(AssetsDirResponse {
    mode,
    path: assets_dir.path.clone(),
  })
}

#[cfg(test)]
mod tests {
  use std::collections::HashMap;
  use std::path::{Path, PathBuf};
  use std::sync::{Arc, Condvar, Mutex};

  use lsp_types::TextDocumentIdentifier;
  use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
  use typedown_lang::db::{QueryStorage, TypedownDatabase};

  use crate::core::analysis::Analysis;
  use crate::core::utils::uri::path_to_uri;

  use super::{TextDocumentIdentifierParams, get_assets_dir};

  fn make_analysis(config_content: &str) -> (Analysis, PathBuf) {
    let db = TypedownDatabase {
      storage: QueryStorage::default(),
    };

    let root = PathBuf::from("/test-vault");
    let config_path = root.join("typedown.yaml");
    let file_path = root.join("content/note.td");

    let config_file = File::new(
      &db,
      FileHandle::Content(
        config_path.clone(),
        config_content.to_string(),
        FileMetadata::default(),
      ),
    );
    let td_file = File::new(
      &db,
      FileHandle::Content(file_path.clone(), String::new(), FileMetadata::default()),
    );

    let files: HashMap<PathBuf, File> = [(config_path, config_file), (file_path.clone(), td_file)]
      .into_iter()
      .collect();

    let project = Project::new(&db, root, files);
    let counter = Arc::new((Mutex::new(1), Condvar::new()));

    let analysis = Analysis::new(
      db,
      project,
      Arc::new(HashMap::new()),
      Arc::new(HashMap::new()),
      counter,
    );

    (analysis, file_path)
  }

  fn make_params(file_path: &Path) -> TextDocumentIdentifierParams {
    let uri = path_to_uri(file_path, "file");
    TextDocumentIdentifierParams {
      text_document: TextDocumentIdentifier { uri },
    }
  }

  #[test]
  fn default_assets_dir() {
    let config = r#"version: "1"
vault:
  content_dir: content
  schema_dir: schemas
"#;
    let (analysis, file_path) = make_analysis(config);
    let params = make_params(&file_path);

    let response = get_assets_dir(&analysis, params).expect("should return response");
    assert_eq!(response.mode, "local");
    assert_eq!(response.path, "assets");
  }

  #[test]
  fn custom_assets_dir() {
    let config = r#"version: "1"
vault:
  content_dir: content
  schema_dir: schemas
  assets_dir:
    mode: local
    path: media
"#;
    let (analysis, file_path) = make_analysis(config);
    let params = make_params(&file_path);

    let response = get_assets_dir(&analysis, params).expect("should return response");
    assert_eq!(response.mode, "local");
    assert_eq!(response.path, "media");
  }
}
