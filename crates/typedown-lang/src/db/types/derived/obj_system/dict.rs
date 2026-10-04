use std::collections::BTreeMap;
use typedown_incremental::{QueryDatabase, StableHash, StableHasher};
use typedown_macros::query_derived;
use typedown_types::either::Either;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::typecheck::validate_typ_params;
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_typs::{get_dict_typ, get_obj_typ, get_string_typ};
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::types::Project;
use crate::db::types::{
  FuncSignature, HirValue, InstantiateResult, LazyTyp, TypParams, TypVariable,
};
use crate::syntax::diagnostic::Diagnostic;

#[query_derived]
pub struct TdDictTyp<'db> {
  pub key_type: Option<LazyTyp<'db>>,
  pub value_type: Option<LazyTyp<'db>>,
}

impl<'db> TdDictTyp<'db> {
  pub fn key(self, db: &'db TypedownDatabase) -> Option<LazyTyp<'db>> {
    self.key_type(db)
  }

  pub fn value(self, db: &'db TypedownDatabase) -> Option<LazyTyp<'db>> {
    self.value_type(db)
  }
}

impl<'db> TdRuntimeObj<'db> for TdDictTyp<'db> {
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
    match (
      self.key(db).and_then(|l| l.resolve(db)),
      self.value(db).and_then(|l| l.resolve(db)),
    ) {
      (Some(key), Some(value)) => format!(
        "@builtin::dict[{}, {}]",
        key.source_path(db),
        value.source_path(db)
      ),
      _ => "@builtin::dict".to_string(),
    }
  }
}

impl<'db> TdStaticTyp<'db> for TdDictTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    match (
      self.key(db).and_then(|l| l.resolve(db)),
      self.value(db).and_then(|l| l.resolve(db)),
    ) {
      (Some(key), Some(value)) => {
        format!("dict[{}, {}]", key.display_name(db), value.display_name(db))
      }
      _ => "dict".to_string(),
    }
  }

  fn runtime_typ(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    if self.key(db).is_none() || self.value(db).is_none() {
      return None;
    }
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    _project: Project,
    args: Vec<TdObjEnum<'db>>,
  ) -> Option<TdObjEnum<'db>> {
    let mut entries = BTreeMap::new();
    for arg in args {
      let pair = *arg.as_td_list_obj()?;
      if pair.len(db) != 2 {
        return None;
      }
      let key_obj = pair.get(db, 0)?;
      let key_str = key_obj.as_td_string_obj()?.value(db);
      let val = pair.get(db, 1)?;
      entries.insert(key_str, Either::Right(val));
    }
    Some(TdDictObj::new(db, entries).into())
  }

  fn arity(&self, db: &'db TypedownDatabase) -> usize {
    if self.key(db).is_some() && self.value(db).is_some() {
      0
    } else {
      2
    }
  }

  fn get_typ_args(&self, db: &'db TypedownDatabase) -> Vec<TdTypEnum<'db>> {
    match (
      self.key(db).and_then(|l| l.resolve(db)),
      self.value(db).and_then(|l| l.resolve(db)),
    ) {
      (Some(key), Some(value)) => vec![key, value],
      _ => vec![],
    }
  }

  fn to_typ_enum(&self, _db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    (*self).into()
  }

  fn instantiate(
    &self,
    db: &'db TypedownDatabase,
    args: Vec<LazyTyp<'db>>,
  ) -> InstantiateResult<'db> {
    let self_type: TdTypEnum = (*self).into();
    let params = self.typ_params(db);
    let diagnostics = validate_typ_params(db, params.as_ref(), &args);
    if diagnostics
      .iter()
      .any(|d| matches!(d, Diagnostic::WrongTypArgCount { .. }))
    {
      return InstantiateResult::new(db, self_type, diagnostics);
    }
    if args.len() == 2 {
      InstantiateResult::new(
        db,
        TdDictTyp::new(db, Some(args[0].clone()), Some(args[1].clone())).into(),
        diagnostics,
      )
    } else {
      InstantiateResult::new(db, self_type, diagnostics)
    }
  }

  fn typ_params(&self, db: &'db TypedownDatabase) -> Option<TypParams<'db>> {
    let obj_type = LazyTyp::eager(get_obj_typ(db).into());
    let mut bindings = Vec::new();
    if let Some(k) = self.key(db) {
      bindings.push(k);
    }
    if let Some(v) = self.value(db) {
      bindings.push(v);
    }
    Some(TypParams::new(
      db,
      vec![
        TypVariable::get(db, Some(obj_type.clone())),
        TypVariable::get(db, Some(obj_type)),
      ],
      bindings,
    ))
  }

  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    _key_type: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    let value = self.value(db).and_then(|v| v.resolve(db))?;
    let key = self
      .key(db)
      .and_then(|k| k.resolve(db))
      .unwrap_or_else(|| get_string_typ(db).into());
    Some(FuncSignature::new(db, vec![key], value))
  }
}

impl<'db> TdDictTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdDictTyp<'db> {
    get_dict_typ(db)
  }
}

#[query_derived(custom_hash)]
pub struct TdDictObj<'db> {
  pub entries: BTreeMap<String, Either<HirValue<'db>, TdObjEnum<'db>>>,
}

// Hash entry count + keys only, skip values to avoid deep recursion
impl<'db> StableHash for TdDictObj<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    let storage = unsafe { db.storage() };
    let cache_key = (Self::ingredient_start_index(), self.0);
    if let Some(cached) = storage.derived_fingerprints.get(&cache_key) {
      ::std::hash::Hasher::write(hasher, &cached.0);
      return;
    }
    let mut inner_hasher = StableHasher::new();
    if let Some(entries) = Self::try_entries(*self, db) {
      entries.len().stable_hash(db, &mut inner_hasher);
      for key in entries.keys() {
        key.stable_hash(db, &mut inner_hasher);
      }
    }
    let fingerprint = typedown_incremental::Fingerprint::from_hasher(inner_hasher);
    storage.derived_fingerprints.insert(cache_key, fingerprint);
    ::std::hash::Hasher::write(hasher, &fingerprint.0);
  }
}

impl<'db> TdRuntimeObj<'db> for TdDictObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdDictTyp::get(db).into()
  }
  fn get_owned_field(&self, db: &'db TypedownDatabase, key: &str) -> Option<TdObjEnum<'db>> {
    match self.entries(db).get(key).cloned()? {
      Either::Left(hir) => {
        let file_scope = get_file_runtime_scope(db, hir.project(db), hir.node(db).owner_file);
        evaluate_node(db, hir, file_scope).value(db)
      }
      Either::Right(obj) => Some(obj),
    }
  }
  fn get_builtin_field(&self, db: &'db TypedownDatabase, key: &str) -> Option<TdObjEnum<'db>> {
    if !key.starts_with('_') {
      return None;
    }
    match self.entries(db).get(key).cloned()? {
      Either::Left(hir) => {
        let file_scope = get_file_runtime_scope(db, hir.project(db), hir.node(db).owner_file);
        evaluate_node(db, hir, file_scope).value(db)
      }
      Either::Right(obj) => Some(obj),
    }
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_typ(db).source_path(db)
  }
  fn index(&self, db: &'db TypedownDatabase, key: &TdObjEnum<'db>) -> Option<TdObjEnum<'db>> {
    let str_key = key.as_td_string_obj()?;
    self.get_owned_field(db, &str_key.value(db))
  }
  fn len(&self, db: &'db TypedownDatabase) -> Option<usize> {
    Some(self.entries(db).len())
  }
}
