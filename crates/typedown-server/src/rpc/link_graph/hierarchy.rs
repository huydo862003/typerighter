use std::collections::HashSet;

use super::Edge;

// Build hierarchy edges from index.td (or virtual indexes) to child files
pub fn collect_edges(visible_paths: &HashSet<String>) -> (Vec<Edge>, Vec<String>) {
  let mut edges = Vec::new();
  let mut virtual_indexes: Vec<String> = Vec::new();

  // Collect all directories that contain visible files
  let mut directories: HashSet<String> = HashSet::new();

  for relative in visible_paths {
    if let Some(separator) = relative.rfind('/') {
      directories.insert(relative[..separator].to_string());
    } else {
      directories.insert(String::new());
    }
  }

  for directory in &directories {
    let index_path = make_index_path(directory);
    let has_real_index = visible_paths.contains(&index_path);

    if !has_real_index {
      virtual_indexes.push(index_path.clone());
    }

    // Connect this index to all direct children in the directory
    for relative in visible_paths {
      if *relative == index_path {
        continue;
      }

      let parent_dir = relative
        .rfind('/')
        .map(|separator| &relative[..separator])
        .unwrap_or("");

      if parent_dir == directory {
        edges.push(Edge {
          source: index_path.clone(),
          target: relative.clone(),
          kind: "hierarchy",
        });
      }
    }
  }

  // Connect nested directory indexes to their parent directory index
  for directory in &directories {
    if directory.is_empty() {
      continue;
    }

    let child_index = make_index_path(directory);
    let parent_dir = directory
      .rfind('/')
      .map(|separator| &directory[..separator])
      .unwrap_or("");
    let parent_index = make_index_path(parent_dir);

    if child_index != parent_index {
      edges.push(Edge {
        source: parent_index,
        target: child_index,
        kind: "hierarchy",
      });
    }
  }

  (edges, virtual_indexes)
}

fn make_index_path(directory: &str) -> String {
  if directory.is_empty() {
    "index.td".to_string()
  } else {
    format!("{directory}/index.td")
  }
}
