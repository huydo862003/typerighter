use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_blob_typ, get_string_typ};
use crate::db::types::{AssetKind, File, TdStringObj};

#[query_derived]
pub struct TdBlobTyp<'db> {}

impl<'db> TdRuntimeObj<'db> for TdBlobTyp<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdTypTyp::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, _db: &'db TypedownDatabase) -> String {
    "@builtin::blob".to_string()
  }
}

impl<'db> TdStaticTyp<'db> for TdBlobTyp<'db> {
  fn display_name(&self, _db: &'db TypedownDatabase) -> String {
    "blob".to_string()
  }
  fn runtime_typ(&self, _db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Some((*self).into())
  }
  fn get_owned_field_typ(&self, db: &'db TypedownDatabase, name: &str) -> Option<TdTypEnum<'db>> {
    match name {
      "format" => Some(get_string_typ(db).into()),
      _ => None,
    }
  }
}

impl<'db> TdBlobTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdBlobTyp<'db> {
    get_blob_typ(db)
  }
}

#[query_derived]
pub struct TdBlobObj<'db> {
  asset_kind: AssetKind,
  file: File,
}

impl<'db> TdRuntimeObj<'db> for TdBlobObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdBlobTyp::get(db).into()
  }
  fn get_owned_field(&self, db: &'db TypedownDatabase, key: &str) -> Option<TdObjEnum<'db>> {
    match key {
      "format" => {
        Some(TdStringObj::new(db, self.asset_kind(db).as_format_str().to_string()).into())
      }
      _ => None,
    }
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_typ(db).source_path(db)
  }
}
