use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex};
use std::{fs, io};

use lsp_types::Uri;
use ropey::Rope;
use std::time::SystemTime;
use typedown_incremental::InputId;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::types::{File, FileHandle, FileMetadata, Project};
use typedown_lang::db::utils::is_content_file;

use crate::core::analysis::Analysis;
use crate::core::utils::fs::{disk_handle, is_asset_file, is_vault_config, scan_project_files};
use crate::core::utils::uri::{uri_scheme, uri_to_path};

pub struct AnalysisHost {
  db: TypedownDatabase,
  project: Project,
  project_dir: PathBuf,
  snapshot_counter: Arc<(Mutex<usize>, Condvar)>,
  open_files: Arc<HashMap<PathBuf, Rope>>, // editor-managed content
  scheme_map: Arc<HashMap<PathBuf, String>>, // URI scheme per path, set when editor opens a file
  project_files: HashSet<PathBuf>,         // all tracked files known on disk
  file_map: HashMap<PathBuf, File>,        // stable File IDs, one per tracked path
}

impl AnalysisHost {
  pub fn new(mut db: TypedownDatabase, project_dir: PathBuf) -> io::Result<Self> {
    // Scan project directory for .td files
    let project_files = scan_project_files(&project_dir)?;

    // Derived queries are keyed by File/Project entry ID, so we must reuse the same IDs from the previous session for cache hits
    let cached_files: HashMap<PathBuf, File> = File::iter(&db)
      .into_iter()
      .filter_map(|file| {
        let handle = file.handle(&db);
        handle.path().cloned().map(|path| (path, file))
      })
      .collect();

    let mut file_map = HashMap::new();
    let mut files = HashMap::new();
    for path in &project_files {
      let Some(handle) = disk_handle(path) else {
        continue;
      };
      let file = if let Some(&cached) = cached_files.get(path) {
        cached.set_handle(&mut db, handle);
        cached
      } else {
        File::new(&db, handle)
      };
      file_map.insert(path.clone(), file);
      files.insert(path.clone(), file);
    }

    let cached_project = Project::iter(&db)
      .into_iter()
      .find(|proj| proj.root_dir(&db) == project_dir);
    let project = if let Some(proj) = cached_project {
      proj.set_files(&mut db, files);
      proj
    } else {
      Project::new(&db, project_dir.clone(), files)
    };

    Ok(Self {
      db,
      project,
      project_dir,
      snapshot_counter: Arc::new((Mutex::new(1), Condvar::new())),
      open_files: Arc::new(HashMap::new()),
      scheme_map: Arc::new(HashMap::new()),
      project_files,
      file_map,
    })
  }

  /// Take a read-only snapshot of the current database state.
  pub fn snapshot(&self) -> Analysis {
    *self.snapshot_counter.0.lock().unwrap() += 1;
    Analysis::new(
      self.db.clone(),
      self.project,
      self.scheme_map.clone(),
      self.open_files.clone(),
      Arc::clone(&self.snapshot_counter),
    )
  }

  /// Cancel all in-flight snapshots, wait for them to finish, then apply a write.
  pub fn write<R>(&mut self, f: impl FnOnce(&mut TypedownDatabase) -> R) -> R {
    self.db.storage.cancelled.store(true, Ordering::Relaxed);

    let mut clones = self.snapshot_counter.0.lock().unwrap();
    while *clones != 1 {
      clones = self.snapshot_counter.1.wait(clones).unwrap();
    }
    drop(clones);

    self.db.storage.cancelled.store(false, Ordering::Relaxed);
    f(&mut self.db)
  }

  fn sync_files(&mut self) {
    // Editor-opened files are always tracked
    for path in self.open_files.keys() {
      self.project_files.insert(path.clone());
    }

    // Build desired handles, pruning files that no longer exist on disk
    let mut desired: HashMap<PathBuf, FileHandle> = HashMap::new();
    self.project_files.retain(|path| {
      if self.open_files.contains_key(path) {
        return true; // editor-owned files handled below
      }
      match disk_handle(path) {
        Some(handle) => {
          desired.insert(path.clone(), handle);
          true
        }
        None => false,
      }
    });
    for (path, rope) in self.open_files.iter() {
      desired.insert(
        path.clone(),
        FileHandle::Content(
          path.clone(),
          rope.to_string(),
          FileMetadata {
            mtime: SystemTime::now(),
            ctime: fs::metadata(path)
              .and_then(|m| m.created())
              .unwrap_or_else(|_| SystemTime::now()),
          },
        ),
      );
    }

    let project = self.project;
    let old_file_map = std::mem::take(&mut self.file_map);

    let new_file_map = self.write(|db| {
      let mut file_map = HashMap::new();
      let mut files = HashMap::new();

      for (path, handle) in desired {
        let file = if let Some(existing) = old_file_map.get(&path) {
          // Reuse stable ID, update handle for invalidation
          existing.set_handle(db, handle);
          *existing
        } else {
          File::new(db, handle)
        };
        files.insert(path.clone(), file);
        file_map.insert(path, file);
      }

      project.set_files(db, files);
      file_map
    });

    self.file_map = new_file_map;
  }

  /// Called on textDocument/didOpen.
  pub fn on_editor_open_file(&mut self, uri: &Uri, content: String) {
    if let Some(path) = uri_to_path(uri) {
      let scheme = uri_scheme(uri).to_string();
      Arc::make_mut(&mut self.scheme_map).insert(path.clone(), scheme);
      Arc::make_mut(&mut self.open_files).insert(path, Rope::from(content));
      self.sync_files();
    }
  }

  /// Called on textDocument/didChange.
  pub fn on_editor_change_file(&mut self, path: PathBuf, rope: Rope) {
    // Preserve ctime from the existing handle instead of re-reading from disk
    let ctime = self
      .file_map
      .get(&path)
      .map(|f| f.handle(&self.db).metadata().ctime)
      .unwrap_or_else(SystemTime::now);
    let handle = FileHandle::Content(
      path.clone(),
      rope.to_string(),
      FileMetadata {
        mtime: SystemTime::now(),
        ctime,
      },
    );
    Arc::make_mut(&mut self.open_files).insert(path.clone(), rope);
    let file_map = &self.file_map;

    if let Some(&file) = file_map.get(&path) {
      self.write(|db| {
        file.set_handle(db, handle);
      });
    } else {
      // File not tracked yet, fall back to full sync
      self.sync_files();
    }
  }

  /// Called on textDocument/didClose. Falls back to disk version.
  pub fn on_close_file(&mut self, path: &PathBuf) {
    Arc::make_mut(&mut self.open_files).remove(path);
    self.sync_files();
  }

  /// Called by the file watcher for disk changes to non-open files.
  pub fn on_disk_change(&mut self, path: PathBuf) {
    if self.open_files.contains_key(&path) {
      return; // editor owns this file, ignore disk change
    }
    if is_content_file(&path)
      || is_asset_file(&path)
      || (path.parent().is_some_and(|p| p == self.project_dir) && is_vault_config(&path))
    {
      self.project_files.insert(path);
      self.sync_files();
    }
  }

  /// Moves the old path entry to the new path
  pub fn on_did_rename_file(&mut self, old_path: PathBuf, new_path: PathBuf) {
    self.project_files.remove(&old_path);
    self.project_files.insert(new_path.clone());

    if let Some(rope) = Arc::make_mut(&mut self.open_files).remove(&old_path) {
      Arc::make_mut(&mut self.open_files).insert(new_path.clone(), rope);
    }
    if let Some(scheme) = Arc::make_mut(&mut self.scheme_map).remove(&old_path) {
      Arc::make_mut(&mut self.scheme_map).insert(new_path.clone(), scheme);
    }

    // Reuse the File ID, just update its handle path
    let Some(file) = self.file_map.remove(&old_path) else {
      return;
    };

    let content = match file.handle(&self.db) {
      FileHandle::Content(_, content, _) => content.clone(),
      FileHandle::Path(path, _) => fs::read_to_string(&path).unwrap_or_default(),
    };

    let handle = FileHandle::Content(
      new_path.clone(),
      content,
      FileMetadata {
        mtime: SystemTime::now(),
        ctime: SystemTime::now(),
      },
    );

    let project = self.project;

    self.write(|db| {
      file.set_handle(db, handle);
      let mut files = project.files(db).clone();
      files.remove(&old_path);
      files.insert(new_path.clone(), file);
      project.set_files(db, files);
    });

    self.file_map.insert(new_path, file);
  }

  /// Called by the file watcher when a file is deleted.
  pub fn on_disk_delete(&mut self, path: PathBuf) {
    if self.open_files.contains_key(&path) {
      return;
    }
    if self.project_files.remove(&path) {
      self.sync_files();
    }
  }

  pub fn into_db(self) -> TypedownDatabase {
    self.db
  }

  pub fn open_file_content(&self, path: &PathBuf) -> Option<&Rope> {
    self.open_files.get(path)
  }

  pub fn project_dir(&self) -> &PathBuf {
    &self.project_dir
  }
}
