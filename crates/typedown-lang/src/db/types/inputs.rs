//! Input types for the incremental database

use std::{collections::HashMap, fs, io, path::PathBuf, time::SystemTime};

use typedown_macros::query_input;

use typedown_types::{file_stream::FileStream, stream::Utf8Stream};

use strum::FromRepr;

use typedown_incremental::{Decodable, Decoder, Encodable, Encoder};

/// File metadata
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FileMetadata {
  /// Last modification time
  pub mtime: SystemTime,
  /// Creation time
  pub ctime: SystemTime,
}

impl Default for FileMetadata {
  fn default() -> Self {
    Self {
      mtime: SystemTime::UNIX_EPOCH,
      ctime: SystemTime::UNIX_EPOCH,
    }
  }
}

impl FileMetadata {
  pub fn mtime_epoch_secs(&self) -> u64 {
    self
      .mtime
      .duration_since(SystemTime::UNIX_EPOCH)
      .map(|d| d.as_secs())
      .unwrap_or(0)
  }

  pub fn ctime_epoch_secs(&self) -> u64 {
    self
      .ctime
      .duration_since(SystemTime::UNIX_EPOCH)
      .map(|d| d.as_secs())
      .unwrap_or(0)
  }
}

/// Types of file-handle: path-based or editor-managed content
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum FileHandle {
  /// A file on disk with metadata
  Path(PathBuf, FileMetadata),
  /// Content provided directly by the editor buffer, with a virtual file path
  Content(PathBuf, String, FileMetadata),
}

impl FileHandle {
  pub fn open(&self) -> io::Result<Box<dyn Utf8Stream>> {
    match self {
      FileHandle::Path(path, _) => {
        let file = fs::File::open(path)?;
        Ok(Box::new(FileStream::new(file)))
      }
      FileHandle::Content(_, content, _) => {
        let cursor = io::Cursor::new(content.as_bytes().to_vec());
        Ok(Box::new(FileStream::new(cursor)))
      }
    }
  }

  /// Return the path for this handle
  pub fn path(&self) -> Option<&PathBuf> {
    match self {
      FileHandle::Path(path, _) => Some(path),
      FileHandle::Content(path, _, _) => Some(path),
    }
  }

  /// Return the metadata for this handle
  pub fn metadata(&self) -> &FileMetadata {
    match self {
      FileHandle::Path(_, metadata) | FileHandle::Content(_, _, metadata) => metadata,
    }
  }
}

/// A file input struct
#[query_input]
pub struct File {
  handle: FileHandle,
}

#[derive(FromRepr)]
#[repr(u8)]
enum FileHandleTag {
  Path = 0,
  Content = 1,
}

fn encode_system_time(time: &SystemTime, buf: &mut Vec<u8>, encoder: &mut Encoder) {
  let duration = time
    .duration_since(SystemTime::UNIX_EPOCH)
    .unwrap_or_default();
  duration.as_secs().encode(buf, encoder);
  duration.subsec_nanos().encode(buf, encoder);
}

fn decode_system_time(data: &mut &[u8], decoder: &Decoder) -> SystemTime {
  let secs = u64::decode(data, decoder);
  let nanos = u32::decode(data, decoder);
  SystemTime::UNIX_EPOCH + std::time::Duration::new(secs, nanos)
}

impl Encodable for FileMetadata {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encode_system_time(&self.mtime, buf, encoder);
    encode_system_time(&self.ctime, buf, encoder);
  }
}

impl Decodable for FileMetadata {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let mtime = decode_system_time(data, decoder);
    let ctime = decode_system_time(data, decoder);
    FileMetadata { mtime, ctime }
  }
}

impl Encodable for FileHandle {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      FileHandle::Path(path, metadata) => {
        encoder.emit_u8(buf, FileHandleTag::Path as u8);
        path.encode(buf, encoder);
        metadata.encode(buf, encoder);
      }
      FileHandle::Content(path, content, metadata) => {
        encoder.emit_u8(buf, FileHandleTag::Content as u8);
        path.encode(buf, encoder);
        content.encode(buf, encoder);
        metadata.encode(buf, encoder);
      }
    }
  }
}

impl Decodable for FileHandle {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match FileHandleTag::from_repr(tag).expect("unknown FileHandle tag") {
      FileHandleTag::Path => {
        let path = PathBuf::decode(data, decoder);
        let metadata = FileMetadata::decode(data, decoder);
        FileHandle::Path(path, metadata)
      }
      FileHandleTag::Content => {
        let path = PathBuf::decode(data, decoder);
        let content = String::decode(data, decoder);
        let metadata = FileMetadata::decode(data, decoder);
        FileHandle::Content(path, content, metadata)
      }
    }
  }
}

/// A project input struct representing files in a project.
/// `files` maps each tracked path to its stable `File` ID.
/// It only changes when files are added or removed, not when their content changes.
#[query_input]
pub struct Project {
  root_dir: PathBuf,
  files: HashMap<PathBuf, File>,
}
