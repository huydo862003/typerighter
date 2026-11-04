use std::collections::BTreeMap;

use typedown_macros::query_interned;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{CallTypResult, FuncSignature, LazyTyp, Variance};

#[query_interned]
pub struct TdBoundVariableTyp<'db> {
  pub index: usize,
  pub upper_bound: LazyTyp<'db>,
  pub variance: Variance,
}

impl<'db> TdBoundVariableTyp<'db> {
  fn resolve_bound(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    Self::upper_bound(*self, db).resolve(db)
  }
}

impl<'db> TdStaticTyp<'db> for TdBoundVariableTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    let name = format!("T{}", Self::index(*self, db));
    match self.resolve_bound(db) {
      Some(bound) if bound.as_td_obj_typ().is_none() => {
        format!("{} <: {}", name, bound.display_name(db))
      }
      _ => name,
    }
  }

  fn static_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdTypEnum<'db>> {
    self
      .resolve_bound(db)
      .map(|upper| upper.static_vtable(db))
      .unwrap_or_default()
  }

  fn get_fields(&self, db: &'db TypedownDatabase) -> BTreeMap<String, LazyTyp<'db>> {
    self
      .resolve_bound(db)
      .map(|upper| upper.get_fields(db))
      .unwrap_or_default()
  }

  fn lookup_field_typ(&self, db: &'db TypedownDatabase, name: &str) -> Option<TdTypEnum<'db>> {
    self.resolve_bound(db)?.lookup_field_typ(db, name)
  }

  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    key_typ: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    self.resolve_bound(db)?.index_typ(db, key_typ)
  }

  fn call_typ(
    &self,
    db: &'db TypedownDatabase,
    arg_typs: Vec<TdTypEnum<'db>>,
  ) -> CallTypResult<'db> {
    match self.resolve_bound(db) {
      Some(resolved) => resolved.call_typ(db, arg_typs),
      None => CallTypResult {
        signature: None,
        diagnostics: vec![],
      },
    }
  }
}

impl<'db> TdRuntimeObj<'db> for TdBoundVariableTyp<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdTypTyp::get(db).into()
  }
  fn get_owned_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.display_name(db)
  }
}
