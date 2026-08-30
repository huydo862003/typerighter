use std::collections::HashMap;

use strum::FromRepr;
use typedown_macros::{StableCompare, query_derived, query_interned};

use crate::db::TypedownDatabase;
use crate::db::derived::name_resolver::scope::{
  get_builtin_runtime_scope, get_file_runtime_scope, get_project_runtime_scope, parent_scope,
};
use crate::db::types::{File, HirValue, Project, TdObjectEnum};
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase,
  StableHash, StableHasher,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub enum SymbolKind<'db> {
  UserDefinedSchema(Project, File),
  UserDefinedResource(Project, File),
  Asset(AssetKind, Project, File),
  BuiltinSchema(BuiltinSchemaKind),
  BuiltinMacro(BuiltinMacroKind),
  BuiltinGlobal(BuiltinGlobalKind),
  FnParam(Project, File, HirValue<'db>),
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
  FnParam = 6,
}

impl<'db> SymbolKind<'db> {
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
        | SymbolKind::FnParam(_, _, _)
    )
  }

  pub fn is_builtin(&self) -> bool {
    matches!(self, SymbolKind::BuiltinSchema(_))
  }
}

impl<'db> StableHash for SymbolKind<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      SymbolKind::UserDefinedSchema(project, file)
      | SymbolKind::UserDefinedResource(project, file) => {
        project.stable_hash(db, hasher);
        file.stable_hash(db, hasher);
      }
      SymbolKind::FnParam(project, file, closure) => {
        project.stable_hash(db, hasher);
        file.stable_hash(db, hasher);
        closure.stable_hash(db, hasher);
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

impl<'db> Encodable for SymbolKind<'db> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      SymbolKind::UserDefinedSchema(project, file) => {
        encoder.emit_u8(buf, SymbolKindTag::UserDefinedSchema as u8);
        project.field_encode(buf, encoder);
        file.field_encode(buf, encoder);
      }
      SymbolKind::UserDefinedResource(project, file) => {
        encoder.emit_u8(buf, SymbolKindTag::UserDefinedResource as u8);
        project.field_encode(buf, encoder);
        file.field_encode(buf, encoder);
      }
      SymbolKind::Asset(asset_kind, project, file) => {
        encoder.emit_u8(buf, SymbolKindTag::Asset as u8);
        asset_kind.encode(buf, encoder);
        project.field_encode(buf, encoder);
        file.field_encode(buf, encoder);
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
      SymbolKind::FnParam(project, file, closure) => {
        encoder.emit_u8(buf, SymbolKindTag::FnParam as u8);
        project.field_encode(buf, encoder);
        file.field_encode(buf, encoder);
        closure.field_encode(buf, encoder);
      }
    }
  }
}

impl<'db> Decodable for SymbolKind<'db> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match SymbolKindTag::from_repr(tag).expect("unknown SymbolKind tag") {
      SymbolKindTag::UserDefinedSchema => SymbolKind::UserDefinedSchema(
        Project::field_decode(data, decoder),
        File::field_decode(data, decoder),
      ),
      SymbolKindTag::UserDefinedResource => SymbolKind::UserDefinedResource(
        Project::field_decode(data, decoder),
        File::field_decode(data, decoder),
      ),
      SymbolKindTag::Asset => SymbolKind::Asset(
        AssetKind::decode(data, decoder),
        Project::field_decode(data, decoder),
        File::field_decode(data, decoder),
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
      SymbolKindTag::FnParam => SymbolKind::FnParam(
        Project::field_decode(data, decoder),
        File::field_decode(data, decoder),
        HirValue::field_decode(data, decoder),
      ),
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, StableCompare)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, StableCompare)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, StableCompare)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr, StableCompare)]
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
  Object = 12,
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, StableCompare)]
pub enum ScopeKind<'db> {
  Builtin(Project),
  Project(Project),
  File(Project, File),
  Fn(Project, File, HirValue<'db>),
}

#[derive(FromRepr)]
#[repr(u8)]
enum ScopeKindTag {
  Builtin = 0,
  Project = 1,
  File = 2,
  Fn = 3,
}

impl<'db> StableHash for ScopeKind<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    std::mem::discriminant(self).stable_hash(db, hasher);
    match self {
      ScopeKind::Builtin(project) => project.stable_hash(db, hasher),
      ScopeKind::Project(project) => project.stable_hash(db, hasher),
      ScopeKind::File(project, file) => {
        project.stable_hash(db, hasher);
        file.stable_hash(db, hasher);
      }
      ScopeKind::Fn(project, file, value) => {
        project.stable_hash(db, hasher);
        file.stable_hash(db, hasher);
        value.stable_hash(db, hasher);
      }
    }
  }
}

impl<'db> Encodable for ScopeKind<'db> {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    match self {
      ScopeKind::Builtin(project) => {
        encoder.emit_u8(buf, ScopeKindTag::Builtin as u8);
        project.field_encode(buf, encoder);
      }
      ScopeKind::Project(project) => {
        encoder.emit_u8(buf, ScopeKindTag::Project as u8);
        project.field_encode(buf, encoder);
      }
      ScopeKind::File(project, file) => {
        encoder.emit_u8(buf, ScopeKindTag::File as u8);
        project.field_encode(buf, encoder);
        file.field_encode(buf, encoder);
      }
      ScopeKind::Fn(project, file, value) => {
        encoder.emit_u8(buf, ScopeKindTag::Fn as u8);
        project.field_encode(buf, encoder);
        file.field_encode(buf, encoder);
        value.field_encode(buf, encoder);
      }
    }
  }
}

impl<'db> Decodable for ScopeKind<'db> {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    match ScopeKindTag::from_repr(tag).expect("unknown ScopeKind tag") {
      ScopeKindTag::Builtin => ScopeKind::Builtin(Project::field_decode(data, decoder)),
      ScopeKindTag::Project => ScopeKind::Project(Project::field_decode(data, decoder)),
      ScopeKindTag::File => ScopeKind::File(
        Project::field_decode(data, decoder),
        File::field_decode(data, decoder),
      ),
      ScopeKindTag::Fn => ScopeKind::Fn(
        Project::field_decode(data, decoder),
        File::field_decode(data, decoder),
        HirValue::field_decode(data, decoder),
      ),
    }
  }
}

#[query_derived]
pub struct Scope<'db> {
  #[id]
  kind: ScopeKind<'db>,
}

impl<'db> Scope<'db> {
  pub fn builtin_scope(db: &'db (impl QueryDatabase + ?Sized), project: Project) -> Self {
    Self::new(db, ScopeKind::Builtin(project))
  }

  pub fn project_scope(db: &'db (impl QueryDatabase + ?Sized), project: Project) -> Self {
    Self::new(db, ScopeKind::Project(project))
  }

  pub fn file_scope(db: &'db (impl QueryDatabase + ?Sized), project: Project, file: File) -> Self {
    Self::new(db, ScopeKind::File(project, file))
  }

  pub fn fn_scope(
    db: &'db (impl QueryDatabase + ?Sized),
    project: Project,
    file: File,
    value: HirValue<'db>,
  ) -> Self {
    Self::new(db, ScopeKind::Fn(project, file, value))
  }

  pub fn project(&self, db: &(impl QueryDatabase + ?Sized)) -> Project {
    match self.kind(db) {
      ScopeKind::Builtin(project)
      | ScopeKind::Project(project)
      | ScopeKind::File(project, _)
      | ScopeKind::Fn(project, _, _) => project,
    }
  }

  pub fn runtime_scope(&self, db: &'db TypedownDatabase) -> RuntimeScope<'db> {
    match self.kind(db) {
      ScopeKind::Builtin(project) => get_builtin_runtime_scope(db, project),
      ScopeKind::Project(project) => get_project_runtime_scope(db, project),
      ScopeKind::File(project, file) => get_file_runtime_scope(db, project, file),
      ScopeKind::Fn(project, file, _) => {
        let scope = parent_scope(db, *self);
        let parent_static = scope.value(db);
        if let Some(parent) = parent_static
          && matches!(parent.kind(db), ScopeKind::Fn(..))
        {
          panic!(
            "Cannot construct static RuntimeScope for a nested closure scope. Nested closures require the dynamic defining RuntimeScope captured when the closure instance was created"
          );
        }
        let parent = get_file_runtime_scope(db, project, file);
        RuntimeScope::new(db, *self, vec![], Some(Box::new(parent)))
      }
    }
  }
}

// Runtime scope for closure evaluation
// Carries param bindings and a reference to the syntactic scope
#[query_derived]
pub struct RuntimeScope<'db> {
  scope: Scope<'db>,
  bindings: Vec<(String, TdObjectEnum<'db>)>,
  parent: Option<Box<RuntimeScope<'db>>>,
}

impl<'db> RuntimeScope<'db> {
  pub fn lookup(
    &self,
    db: &'db (impl QueryDatabase + ?Sized),
    name: &str,
  ) -> Option<TdObjectEnum<'db>> {
    for (key, val) in &self.bindings(db) {
      if key == name {
        return Some(val.clone());
      }
    }
    if let Some(parent) = self.parent(db).as_ref() {
      return parent.lookup(db, name);
    }
    None
  }
}

#[query_interned]
pub struct Symbol<'db> {
  kind: SymbolKind<'db>,
  name: String,
  def_id: String,
}

#[query_derived]
pub struct ProjectSchemaResult<'db> {
  members: HashMap<String, Symbol<'db>>,
}

#[query_derived]
pub struct MembersResult<'db> {
  members: HashMap<String, Symbol<'db>>,
}
