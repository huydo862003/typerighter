use std::collections::HashMap;

use strum::FromRepr;
use typedown_macros::{query_derived, query_interned};

use crate::db::types::{File, HirValue, Project};
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, FieldDecodable, FieldEncodable, QueryDatabase,
  StableCompare, StableHash, StableHasher,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolKind {
  UserDefinedSchema(Project, File),
  UserDefinedResource(Project, File),
  Asset(AssetKind, Project, File),
  BuiltinSchema(BuiltinSchemaKind),
  BuiltinMacro(BuiltinMacroKind),
  BuiltinGlobal(BuiltinGlobalKind),
}

#[derive(FromRepr)]
#[repr(u8)]
enum SymbolKindTag {
  UserDefinedSchema = 0,
  UserDefinedResource = 1,
  BuiltinSchema = 2,
  BuiltinMacro = 3,
  Asset = 4,
  BuiltinGlobal = 5,
}

impl SymbolKind {
  pub fn is_schema(&self) -> bool {
    matches!(
      self,
      SymbolKind::UserDefinedSchema(_, _) | SymbolKind::BuiltinSchema(_)
    )
  }

  pub fn is_resource(&self) -> bool {
    matches!(self, SymbolKind::UserDefinedResource(_, _))
  }

  pub fn is_asset(&self) -> bool {
    matches!(self, SymbolKind::Asset(_, _, _))
  }

  pub fn is_user_defined(&self) -> bool {
    matches!(
      self,
      SymbolKind::UserDefinedSchema(_, _)
        | SymbolKind::UserDefinedResource(_, _)
        | SymbolKind::Asset(_, _, _)
    )
  }

  pub fn is_builtin(&self) -> bool {
    matches!(self, SymbolKind::BuiltinSchema(_))
  }
}

impl StableHash for SymbolKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      SymbolKind::UserDefinedSchema(project, file)
      | SymbolKind::UserDefinedResource(project, file) => {
        project.stable_hash(db, hasher);
        file.stable_hash(db, hasher);
      }
      SymbolKind::Asset(asset_kind, project, file) => {
        asset_kind.stable_hash(db, hasher);
        project.stable_hash(db, hasher);
        file.stable_hash(db, hasher);
      }
      SymbolKind::BuiltinSchema(kind) => kind.stable_hash(db, hasher),
      SymbolKind::BuiltinMacro(kind) => kind.stable_hash(db, hasher),
      SymbolKind::BuiltinGlobal(kind) => kind.stable_hash(db, hasher),
    }
  }
}

impl Encodable for SymbolKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      SymbolKind::UserDefinedSchema(project, file) => {
        encoder.emit_u8(buf, SymbolKindTag::UserDefinedSchema as u8);
        project.encode_field(buf, encoder);
        file.encode_field(buf, encoder);
      }
      SymbolKind::UserDefinedResource(project, file) => {
        encoder.emit_u8(buf, SymbolKindTag::UserDefinedResource as u8);
        project.encode_field(buf, encoder);
        file.encode_field(buf, encoder);
      }
      SymbolKind::Asset(asset_kind, project, file) => {
        encoder.emit_u8(buf, SymbolKindTag::Asset as u8);
        asset_kind.encode(buf, encoder);
        project.encode_field(buf, encoder);
        file.encode_field(buf, encoder);
      }
      SymbolKind::BuiltinSchema(kind) => {
        encoder.emit_u8(buf, SymbolKindTag::BuiltinSchema as u8);
        kind.encode(buf, encoder);
      }
      SymbolKind::BuiltinMacro(kind) => {
        encoder.emit_u8(buf, SymbolKindTag::BuiltinMacro as u8);
        kind.encode(buf, encoder);
      }
      SymbolKind::BuiltinGlobal(kind) => {
        encoder.emit_u8(buf, SymbolKindTag::BuiltinGlobal as u8);
        kind.encode(buf, encoder);
      }
    }
  }
}

impl Decodable for SymbolKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match SymbolKindTag::from_repr(tag).expect("unknown SymbolKind tag") {
      SymbolKindTag::UserDefinedSchema => SymbolKind::UserDefinedSchema(
        Project::decode_field(data, decoder),
        File::decode_field(data, decoder),
      ),
      SymbolKindTag::UserDefinedResource => SymbolKind::UserDefinedResource(
        Project::decode_field(data, decoder),
        File::decode_field(data, decoder),
      ),
      SymbolKindTag::Asset => SymbolKind::Asset(
        AssetKind::decode(data, decoder),
        Project::decode_field(data, decoder),
        File::decode_field(data, decoder),
      ),
      SymbolKindTag::BuiltinSchema => {
        SymbolKind::BuiltinSchema(BuiltinSchemaKind::decode(data, decoder))
      }
      SymbolKindTag::BuiltinMacro => {
        SymbolKind::BuiltinMacro(BuiltinMacroKind::decode(data, decoder))
      }
      SymbolKindTag::BuiltinGlobal => {
        SymbolKind::BuiltinGlobal(BuiltinGlobalKind::decode(data, decoder))
      }
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr)]
#[repr(u8)]
pub enum AssetKind {
  Pdf = 0,
  Svg = 1,
  Png = 2,
  Jpg = 3,
  Webp = 4,
  UnknownBinary = 5,
}

impl AssetKind {
  pub fn from_extension(ext: &str) -> Option<AssetKind> {
    match ext {
      "pdf" => Some(AssetKind::Pdf),
      "svg" => Some(AssetKind::Svg),
      "png" => Some(AssetKind::Png),
      "jpg" | "jpeg" => Some(AssetKind::Jpg),
      "webp" => Some(AssetKind::Webp),
      _ => None,
    }
  }

  pub fn is_image(&self) -> bool {
    matches!(
      self,
      AssetKind::Png | AssetKind::Jpg | AssetKind::Svg | AssetKind::Webp
    )
  }

  pub fn as_format_str(&self) -> &'static str {
    match self {
      AssetKind::Pdf => "pdf",
      AssetKind::Svg => "svg",
      AssetKind::Png => "png",
      AssetKind::Jpg => "jpg",
      AssetKind::Webp => "webp",
      AssetKind::UnknownBinary => "unknown",
    }
  }
}

impl StableHash for AssetKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
  }
}

impl Encodable for AssetKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buf, *self as u8);
  }
}

impl Decodable for AssetKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    AssetKind::from_repr(tag).expect("unknown AssetKind tag")
  }
}

#[cfg(test)]
mod tests {
  use super::AssetKind;

  #[test]
  fn is_image_returns_true_for_image_formats() {
    assert!(AssetKind::Png.is_image());
    assert!(AssetKind::Jpg.is_image());
    assert!(AssetKind::Svg.is_image());
    assert!(AssetKind::Webp.is_image());
  }

  #[test]
  fn is_image_returns_false_for_non_image_formats() {
    assert!(!AssetKind::Pdf.is_image());
    assert!(!AssetKind::UnknownBinary.is_image());
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr)]
#[repr(u8)]
pub enum BuiltinMacroKind {
  Fref = 0,
}

impl StableHash for BuiltinMacroKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
  }
}

impl Encodable for BuiltinMacroKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buf, *self as u8);
  }
}

impl Decodable for BuiltinMacroKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    BuiltinMacroKind::from_repr(tag).expect("unknown BuiltinMacroKind tag")
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr)]
#[repr(u8)]
pub enum BuiltinGlobalKind {
  Vault = 0,
}

impl StableHash for BuiltinGlobalKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
  }
}

impl Encodable for BuiltinGlobalKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buf, *self as u8);
  }
}

impl Decodable for BuiltinGlobalKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    BuiltinGlobalKind::from_repr(tag).expect("unknown BuiltinGlobalKind tag")
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr)]
#[repr(u8)]
pub enum BuiltinSchemaKind {
  TypeType = 0,
  Schema = 1,
  Str = 2,
  Num = 3,
  Bool = 4,
  Date = 5,
  DateTime = 6,
  Time = 7,
  List = 8,
  Dict = 9,
  Math = 10,
  SchemaProperty = 11,
}

impl StableHash for BuiltinSchemaKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
  }
}

impl Encodable for BuiltinSchemaKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buf, *self as u8);
  }
}

impl Decodable for BuiltinSchemaKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    BuiltinSchemaKind::from_repr(tag).expect("unknown BuiltinSchemaKind tag")
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ScopeKind {
  Builtin,
  Project(Project),
  File(Project, File),
  Fn(Project, File, HirValue),
}

#[derive(FromRepr)]
#[repr(u8)]
enum ScopeKindTag {
  Builtin = 0,
  Project = 1,
  File = 2,
  Fn = 3,
}

impl StableHash for ScopeKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      ScopeKind::Builtin => {}
      ScopeKind::Project(project) => project.stable_hash(db, hasher),
      ScopeKind::File(project, file) => {
        project.stable_hash(db, hasher);
        file.stable_hash(db, hasher);
      },
      ScopeKind::Fn(project, file, value) => {
        project.stable_hash(db, hasher);
        file.stable_hash(db, hasher);
        value.stable_hash(db, hasher);
      },
    }
  }
}

impl Encodable for ScopeKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      ScopeKind::Builtin => {
        encoder.emit_u8(buf, ScopeKindTag::Builtin as u8);
      }
      ScopeKind::Project(project) => {
        encoder.emit_u8(buf, ScopeKindTag::Project as u8);
        project.encode_field(buf, encoder);
      }
      ScopeKind::File(project, file) => {
        encoder.emit_u8(buf, ScopeKindTag::File as u8);
        project.encode_field(buf, encoder);
        file.encode_field(buf, encoder);
      }
      ScopeKind::Fn(project, file, value) => {
        encoder.emit_u8(buf, ScopeKindTag::Fn as u8);
        project.encode_field(buf, encoder);
        file.encode_field(buf, encoder);
        value.encode_field(buf, encoder);
      }
    }
  }
}

impl Decodable for ScopeKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match ScopeKindTag::from_repr(tag).expect("unknown ScopeKind tag") {
      ScopeKindTag::Builtin => ScopeKind::Builtin,
      ScopeKindTag::Project => ScopeKind::Project(Project::decode_field(data, decoder)),
      ScopeKindTag::File => ScopeKind::File(
        Project::decode_field(data, decoder),
        File::decode_field(data, decoder),
      ),
      ScopeKindTag::Fn => ScopeKind::Fn(
        Project::decode_field(data, decoder),
        File::decode_field(data, decoder),
        HirValue::decode_field(data, decoder),
      ),
    }
  }
}

#[query_derived]
pub struct Scope {
  #[id]
  kind: ScopeKind,
}

impl Scope {
  pub fn builtin_scope(db: &(impl QueryDatabase + ?Sized)) -> Self {
    Self::new(db, ScopeKind::Builtin)
  }

  pub fn project_scope(db: &(impl QueryDatabase + ?Sized), project: Project) -> Self {
    Self::new(db, ScopeKind::Project(project))
  }

  pub fn file_scope(db: &(impl QueryDatabase + ?Sized), project: Project, file: File) -> Self {
    Self::new(db, ScopeKind::File(project, file))
  }

  pub fn fn_scope(
    db: &(impl QueryDatabase + ?Sized),
    project: Project,
    file: File,
    value: HirValue,
  ) -> Self {
    Self::new(db, ScopeKind::Fn(project, file, value))
  }
}

#[query_interned]
pub struct Symbol {
  kind: SymbolKind,
  name: String,
  def_id: String,
}

impl StableCompare for Symbol {
  const CAN_USE_UNSTABLE_SORT: bool = true;

  fn stable_cmp<DB: QueryDatabase + ?Sized>(&self, db: &DB, other: &Self) -> std::cmp::Ordering {
    self.def_id(db).cmp(&other.def_id(db))
  }
}

#[query_derived]
pub struct ProjectSchemaResult {
  members: HashMap<String, Symbol>,
}

#[query_derived]
pub struct MembersResult {
  members: HashMap<String, Symbol>,
}
