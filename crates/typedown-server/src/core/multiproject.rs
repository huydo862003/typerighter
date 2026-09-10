use std::{
  fmt::Debug,
  path::{Path, PathBuf},
  sync::{Arc, RwLock, atomic::Ordering},
};

use dashmap::DashMap;
use dashmap::mapref::entry::Entry;
use typedown_incremental::{CacheSession, QueryStorage, SerializableQueryDatabase};
use typedown_lang::db::TypedownDatabase;

use crate::core::analysis_host::AnalysisHost;

pub struct ProjectEntry {
  pub root_dir: PathBuf,
  pub cache: CacheSession,
  pub host: RwLock<AnalysisHost>,
}

impl Debug for ProjectEntry {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(self.root_dir.to_str().unwrap_or("<unknown path>"))?;
    Ok(())
  }
}

pub struct Multiproject {
  projects: DashMap<PathBuf, Arc<ProjectEntry>>,
}

impl Default for Multiproject {
  fn default() -> Self {
    Multiproject {
      projects: DashMap::new(),
    }
  }
}

impl Drop for Multiproject {
  fn drop(&mut self) {
    if !self.projects.is_empty() {
      log::warn!(
        "Multiproject dropped without calling save(), {} project caches will not be persisted",
        self.projects.len()
      );
    }
  }
}

impl Multiproject {
  pub fn projects(&self) -> impl Iterator<Item = Arc<ProjectEntry>> {
    self.projects.iter().map(|e| e.value().clone())
  }

  /// Load or return an existing project for the nearest vault root above `nested_dir`.
  pub fn load_nearest_project(&self, nested_dir: &Path) -> anyhow::Result<Arc<ProjectEntry>> {
    let project_dir = find_project_root(nested_dir)?;

    // Use entry API to avoid TOCTOU race between get and insert
    match self.projects.entry(project_dir.clone()) {
      Entry::Occupied(entry) => Ok(entry.get().clone()),
      Entry::Vacant(entry) => {
        let project = load_project(&project_dir)?;
        entry.insert(project.clone());
        Ok(project)
      }
    }
  }

  /// Save all project caches to disk.
  /// Must be called on the main thread after the event loop exits.
  pub fn save(mut self) {
    // Take ownership so Drop sees an empty map and does not warn
    let projects = std::mem::take(&mut self.projects);
    for (root_dir, project) in projects.into_iter() {
      let entry = match Arc::try_unwrap(project) {
        Ok(entry) => entry,
        Err(_) => {
          log::error!(
            "Cannot save project {}: still referenced elsewhere",
            root_dir.display()
          );
          continue;
        }
      };

      let ProjectEntry {
        host,
        cache: session,
        ..
      } = entry;

      let db = host
        .into_inner()
        .expect("RwLock should not be poisoned at shutdown")
        .into_db();

      // TODO: Cache dump temporarily disabled while investigating cache corruption bug
      // let revision = db.storage.revision.load(Ordering::Acquire) as u64;
      // let serialized = db.dump();
      // if let Err(err) = session.finalize(&serialized, revision) {
      //   log::error!(
      //     "Failed to save incremental cache for {}: {err}",
      //     root_dir.display()
      //   );
      // }
      let _ = (db, session);
    }
  }
}

/// Create a new project entry for the given directory.
fn load_project(project_dir: &Path) -> anyhow::Result<Arc<ProjectEntry>> {
  log::info!("Loading project: {}", project_dir.display());
  let cache_dir = project_dir.join(".typedown/.local/cache");

  let (session, serialized) = CacheSession::open(&cache_dir).unwrap_or_else(|_| {
    // If cache dir is inaccessible, proceed without cache
    (CacheSession::empty(), None)
  });

  // catch_unwind guards against panics in corrupted cache deserialization
  let storage = match serialized {
    Some(data) => {
      match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let arc = QueryStorage::from_serialized(data);
        Arc::try_unwrap(arc).unwrap_or_else(|arc| (*arc).clone())
      })) {
        Ok(storage) => storage,
        Err(_) => {
          log::warn!("Failed to load incremental cache, starting fresh");
          let _ = std::fs::remove_dir_all(&cache_dir);
          QueryStorage::default()
        }
      }
    }
    None => QueryStorage::default(),
  };

  let db = TypedownDatabase { storage };

  let host = RwLock::new(AnalysisHost::new(db, project_dir.into())?);

  Ok(Arc::new(ProjectEntry {
    root_dir: project_dir.into(),
    host,
    cache: session,
  }))
}

/// Walk up from `start` until a directory containing `typedown.yaml` or `typedown.yml` is found.
fn find_project_root(start: &Path) -> anyhow::Result<PathBuf> {
  let mut current = start;
  loop {
    if current.join("typedown.yaml").exists() || current.join("typedown.yml").exists() {
      return Ok(current.to_path_buf());
    }
    match current.parent() {
      Some(parent) => current = parent,
      None => {
        anyhow::bail!(
          "No typedown.yaml found in any ancestor of {}",
          start.display()
        );
      }
    }
  }
}
