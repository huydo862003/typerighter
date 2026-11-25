use std::collections::{HashMap, HashSet};

use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::name_resolver::file_symbol::file_symbol;
use typedown_lang::db::derived::name_resolver::resolution_index::resolution_index;
use typedown_lang::db::types::{File, Project};

use super::Edge;

// Collect fref links between pages via the resolution index
pub fn collect_edges(
  db: &TypedownDatabase,
  project: Project,
  visible: &HashMap<String, File>,
  visible_paths: &HashSet<String>,
) -> Vec<Edge> {
  let def_id_to_path: HashMap<String, String> = visible
    .iter()
    .filter_map(|(relative, file)| {
      let sym = file_symbol(db, project, *file).value(db)?;
      Some((sym.definition_id(db).to_string(), relative.clone()))
    })
    .collect();

  let mut result = Vec::new();

  for (relative, file) in visible {
    let index = resolution_index(db, project, *file);

    for def_id in index.definition_ids(db) {
      if let Some(target) = def_id_to_path.get(&def_id)
        && *target != *relative
        && visible_paths.contains(target)
      {
        result.push(Edge {
          source: relative.clone(),
          target: target.clone(),
          kind: "reference",
        });
      }
    }
  }

  result
}
