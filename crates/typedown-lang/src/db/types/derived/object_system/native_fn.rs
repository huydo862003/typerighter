use std::path::PathBuf;

use strum::FromRepr;

use super::base::{TdObjectLike, TdTypeLike};
use super::list::TdListObj;
use super::num::TdNumObj;
use super::str::TdStrObj;
use super::{TdObjectEnum, TdTypeEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_resource::evaluate_resource;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::utils::is_content_file;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};
use typedown_types::either::Either;

type NativeFn = fn(&TypedownDatabase, TdObjectEnum, Vec<TdObjectEnum>) -> Option<TdObjectEnum>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromRepr)]
#[repr(u8)]
pub enum NativeFnKind {
  StrToString = 0,
  NumToString = 1,
  BoolToString = 2,
  MathToString = 3,
  ObjectToString = 4,
  FuncToString = 5,
  DateTimeToString = 6,
  DateToString = 7,
  TimeToString = 8,

  VaultFiles = 9,
  VaultFilesWhere = 10,
  VaultCount = 11,
  VaultCountWhere = 12,
}

impl StableHash for NativeFnKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    (*self as u8).stable_hash(db, hasher);
  }
}

impl Encodable for NativeFnKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    encoder.emit_u8(buf, *self as u8);
  }
}

impl Decodable for NativeFnKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = decoder.read_u8(data);
    NativeFnKind::from_repr(tag).expect("unknown NativeFnKind tag")
  }
}

impl NativeFnKind {
  pub fn resolve(self) -> NativeFn {
    match self {
      NativeFnKind::StrToString => str_to_string,
      NativeFnKind::NumToString => num_to_string,
      NativeFnKind::BoolToString => bool_to_string,
      NativeFnKind::MathToString => math_to_string,
      NativeFnKind::ObjectToString => object_to_string,
      NativeFnKind::FuncToString => func_to_string,
      NativeFnKind::DateTimeToString => datetime_to_string,
      NativeFnKind::DateToString => date_to_string,
      NativeFnKind::TimeToString => time_to_string,
      NativeFnKind::VaultFiles => vault_files,
      NativeFnKind::VaultFilesWhere => vault_files_where,
      NativeFnKind::VaultCount => vault_count,
      NativeFnKind::VaultCountWhere => vault_count_where,
    }
  }
}

fn str_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_str_obj()?;
  Some(TdStrObj::new(db, obj.value(db)).into())
}

fn num_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_num_obj()?;
  Some(TdStrObj::new(db, obj.value(db).to_string()).into())
}

fn bool_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_bool_obj()?;
  Some(TdStrObj::new(db, obj.value(db).to_string()).into())
}

fn math_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_math_obj()?;
  Some(TdStrObj::new(db, format!("${}$", obj.value(db))).into())
}

fn object_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  Some(TdStrObj::new(db, this.source_path(db)).into())
}

fn func_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let func = this.as_td_func_obj()?;
  Some(TdStrObj::new(db, func.name(db)).into())
}

fn datetime_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_date_time_obj()?;
  Some(TdStrObj::new(db, obj.value(db)).into())
}

fn date_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_date_obj()?;
  Some(TdStrObj::new(db, obj.value(db)).into())
}

fn time_to_string(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let obj = this.as_td_time_obj()?;
  Some(TdStrObj::new(db, obj.value(db)).into())
}

// A content resource with its path relative to content_dir
struct VaultResource {
  obj: TdObjectEnum,
  relative_path: PathBuf,
}

// Collect all content file objects from the vault
fn collect_vault_resources(
  db: &TypedownDatabase,
  this: &TdObjectEnum,
) -> Option<Vec<VaultResource>> {
  let vault_obj = this.as_td_vault_obj()?;
  let project = vault_obj.project(db);
  let config = get_vault_config(db, project);
  let content_dir = config.content_dir(db);
  let schema_dir = config.schema_dir(db);

  let mut resources = vec![];
  for (path, file) in project.files(db).iter() {
    if !path.starts_with(&content_dir) || path.starts_with(&schema_dir) || !is_content_file(path) {
      continue;
    }
    let Some(symbol) = file_symbol(db, project, *file).value(db) else {
      continue;
    };
    let Some(obj) = evaluate_resource(db, symbol).value(db) else {
      continue;
    };
    let relative_path = path
      .strip_prefix(&content_dir)
      .unwrap_or(path)
      .to_path_buf();
    resources.push(VaultResource { obj, relative_path });
  }
  Some(resources)
}

fn vault_files(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let resources = collect_vault_resources(db, &this)?;
  let items = resources
    .into_iter()
    .map(|r| Either::Right(r.obj))
    .collect();
  Some(TdListObj::new(db, items).into())
}

struct VaultFilter {
  schema: Option<TdTypeEnum>,
  path: Option<String>,
}

// Extract filters from { schema: Article, path: "blog/" }
fn extract_vault_filter(db: &TypedownDatabase, args: &[TdObjectEnum]) -> VaultFilter {
  let filter = args.first().and_then(|a| a.as_td_dict_obj());
  let schema = filter
    .and_then(|f| f.get_owned_field(db, "schema"))
    .and_then(|obj| obj.as_type());
  let path = filter
    .and_then(|f| f.get_owned_field(db, "path"))
    .and_then(|obj| obj.as_td_str_obj().map(|s| s.value(db)));
  VaultFilter { schema, path }
}

fn matches_filter(db: &TypedownDatabase, resource: &VaultResource, filter: &VaultFilter) -> bool {
  if let Some(ref schema) = filter.schema
    && !schema.accepts(db, &resource.obj.get_type(db))
  {
    return false;
  }
  if let Some(ref path_prefix) = filter.path
    && !resource.relative_path.starts_with(path_prefix)
  {
    return false;
  }
  true
}

fn vault_files_where(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let filter = extract_vault_filter(db, &args);
  let resources = collect_vault_resources(db, &this)?;
  let filtered: Vec<_> = resources
    .into_iter()
    .filter(|r| matches_filter(db, r, &filter))
    .map(|r| Either::Right(r.obj))
    .collect();
  Some(TdListObj::new(db, filtered).into())
}

fn vault_count(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  _args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let resources = collect_vault_resources(db, &this)?;
  Some(TdNumObj::new(db, resources.len() as f64).into())
}

fn vault_count_where(
  db: &TypedownDatabase,
  this: TdObjectEnum,
  args: Vec<TdObjectEnum>,
) -> Option<TdObjectEnum> {
  let filter = extract_vault_filter(db, &args);
  let resources = collect_vault_resources(db, &this)?;
  let count = resources
    .iter()
    .filter(|r| matches_filter(db, r, &filter))
    .count();
  Some(TdNumObj::new(db, count as f64).into())
}
