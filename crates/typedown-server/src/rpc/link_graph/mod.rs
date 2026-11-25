mod hierarchy;
mod references;

use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde::Serialize;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::get_vault_config::get_vault_config;
use typedown_lang::db::types::{File, Project};
use typedown_lang::db::utils::{is_content_file, unslugify};
use typedown_lang::integrations::export::{export_page_resource_nav, export_ssg_flags};
use typedown_types::path::normalize_path;

use crate::core::analysis::Analysis;
use crate::rpc::contract::RpcResult;

#[derive(Serialize, Clone, Hash, Eq, PartialEq)]
pub struct Edge {
  pub source: String,
  pub target: String,
  pub kind: &'static str,
}

pub fn get_link_graph(analysis: &Analysis) -> RpcResult<serde_json::Value> {
  let db = &analysis.db;
  let project = analysis.project;
  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);

  let visible = collect_visible_files(db, project, &root_dir);
  let mut nodes = build_nodes(db, project, &visible);
  let visible_paths: HashSet<String> = visible.keys().cloned().collect();

  let (hierarchy_edges, virtual_indexes) = hierarchy::collect_edges(&visible_paths);

  // Add virtual index nodes for directories without index.td
  for virtual_path in &virtual_indexes {
    nodes.push(serde_json::json!({
      "id": virtual_path,
      "label": get_fallback_label(virtual_path),
      "schema": null,
      "nodeType": "index",
    }));
  }

  let mut all_visible = visible_paths;
  all_visible.extend(virtual_indexes);

  let mut edges: HashSet<Edge> = HashSet::new();
  edges.extend(hierarchy_edges);
  edges.extend(references::collect_edges(
    db,
    project,
    &visible,
    &all_visible,
  ));

  Ok(serde_json::json!({
    "nodes": nodes,
    "edges": edges.into_iter().collect::<Vec<_>>(),
  }))
}

fn collect_visible_files(
  db: &TypedownDatabase,
  project: Project,
  root_dir: &Path,
) -> HashMap<String, File> {
  let files = project.files(db);
  let mut result = HashMap::new();

  for (path, file) in files.iter() {
    if !path.starts_with(root_dir) || !is_content_file(path) {
      continue;
    }
    if !export_ssg_flags(db, project, *file).show_in_sidebar {
      continue;
    }

    let relative = normalize_path(path.strip_prefix(root_dir).unwrap_or(path));
    result.insert(relative, *file);
  }

  result
}

fn build_nodes(
  db: &TypedownDatabase,
  project: Project,
  visible: &HashMap<String, File>,
) -> Vec<serde_json::Value> {
  visible
    .iter()
    .map(|(relative, file)| {
      let nav = export_page_resource_nav(db, project, *file);
      let label = nav
        .as_ref()
        .and_then(|nav| nav.label.clone())
        .unwrap_or_else(|| get_fallback_label(relative));
      let schema = nav.as_ref().and_then(|nav| nav.schema.clone());
      let is_index = Path::new(relative)
        .file_stem()
        .is_some_and(|stem| stem == "index");
      let node_type = if is_index { "index" } else { "page" };

      serde_json::json!({
        "id": relative,
        "label": label,
        "schema": schema,
        "nodeType": node_type,
      })
    })
    .collect()
}

// For index.td, use parent directory name; for others, use the file stem
fn get_fallback_label(relative: &str) -> String {
  let stem = Path::new(relative)
    .file_stem()
    .and_then(|stem| stem.to_str())
    .unwrap_or(relative);

  if stem == "index" {
    // Use parent directory name, unslugified
    let parent = Path::new(relative)
      .parent()
      .and_then(|parent| parent.file_name())
      .and_then(|name| name.to_str())
      .unwrap_or("Home");
    unslugify(parent)
  } else {
    unslugify(stem)
  }
}
