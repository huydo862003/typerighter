use typedown_incremental::{QueryDatabase, StableHash, StableHasher};
use typedown_macros::query_derived;
use typedown_types::either::Either;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::typecheck::validate_typ_params;
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_builtin_typs::{get_list_typ, get_number_typ, get_obj_typ};
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::types::Project;
use crate::db::types::{
  FuncSignature, HirValue, InstantiateResult, LazyTyp, TypParams, TypVariable,
};
use crate::syntax::diagnostic::Diagnostic;

#[query_derived]
pub struct TdListTyp<'db> {
  pub element_typ: Option<LazyTyp<'db>>,
}

impl<'db> TdListTyp<'db> {
  pub fn element(self, db: &'db TypedownDatabase) -> Option<LazyTyp<'db>> {
    self.element_typ(db)
  }
}

impl<'db> TdRuntimeObj<'db> for TdListTyp<'db> {
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
    match self.element(db).and_then(|e| e.resolve(db)) {
      Some(elem) => format!("@builtin::list[{}]", elem.source_path(db)),
      None => "@builtin::list".to_string(),
    }
  }
  // Type instantiation: list[string] at runtime
  fn index(&self, db: &'db TypedownDatabase, key: &TdObjEnum<'db>) -> Option<TdObjEnum<'db>> {
    let arg_type = key.as_td_typ_obj()?.clone();
    let result = self.instantiate(db, vec![LazyTyp::eager(arg_type)]).typ(db);
    Some(TdObjEnum::from(result))
  }
}

impl<'db> TdStaticTyp<'db> for TdListTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    match self.element(db).and_then(|e| e.resolve(db)) {
      Some(elem) => format!("list[{}]", elem.display_name(db)),
      None => "list".to_string(),
    }
  }

  fn runtime_typ(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    self.element(db)?;
    Some((*self).into())
  }
  fn construct(
    &self,
    db: &'db TypedownDatabase,
    _project: Project,
    args: Vec<TdObjEnum<'db>>,
  ) -> Option<TdObjEnum<'db>> {
    let items = args.into_iter().map(Either::Right).collect();
    Some(TdListObj::new(db, items).into())
  }

  fn arity(&self, db: &'db TypedownDatabase) -> usize {
    if self.element(db).is_some() { 0 } else { 1 }
  }

  fn get_typ_args(&self, db: &'db TypedownDatabase) -> Vec<TdTypEnum<'db>> {
    match self.element(db).and_then(|e| e.resolve(db)) {
      Some(elem) => vec![elem],
      None => vec![],
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
    if args.len() == 1 {
      InstantiateResult::new(
        db,
        TdListTyp::new(db, Some(args[0].clone())).into(),
        diagnostics,
      )
    } else {
      InstantiateResult::new(db, self_type, diagnostics)
    }
  }

  fn typ_params(&self, db: &'db TypedownDatabase) -> Option<TypParams<'db>> {
    let param = TypVariable::get(db, Some(LazyTyp::eager(get_obj_typ(db).into())));
    let bindings = self.element(db).into_iter().collect();
    Some(TypParams::new(db, vec![param], bindings))
  }

  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    _key_type: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    let elem = self.element(db).and_then(|e| e.resolve(db))?;
    let key_type: TdTypEnum = get_number_typ(db).into();
    Some(FuncSignature::new(db, vec![key_type], elem))
  }
}

impl<'db> TdListTyp<'db> {
  pub fn get(db: &'db TypedownDatabase) -> TdListTyp<'db> {
    get_list_typ(db)
  }
}

#[query_derived(custom_hash)]
pub struct TdListObj<'db> {
  pub items: Vec<Either<HirValue<'db>, TdObjEnum<'db>>>,
}

// Hash item count only to avoid walking the full list
impl<'db> StableHash for TdListObj<'db> {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    let storage = unsafe { db.storage() };
    let cache_key = (Self::ingredient_start_index(), self.0);
    if let Some(cached) = storage.derived_fingerprints.get(&cache_key) {
      ::std::hash::Hasher::write(hasher, &cached.0);
      return;
    }
    let mut inner_hasher = StableHasher::new();
    if let Some(items) = Self::try_items(*self, db) {
      items.len().stable_hash(db, &mut inner_hasher);
    }
    let fingerprint = typedown_incremental::Fingerprint::from_hasher(inner_hasher);
    storage.derived_fingerprints.insert(cache_key, fingerprint);
    ::std::hash::Hasher::write(hasher, &fingerprint.0);
  }
}

impl<'db> TdRuntimeObj<'db> for TdListObj<'db> {
  fn get_typ(&self, db: &'db TypedownDatabase) -> TdTypEnum<'db> {
    TdListTyp::get(db).into()
  }
  fn get_owned_field(&self, db: &'db TypedownDatabase, key: &str) -> Option<TdObjEnum<'db>> {
    let idx: usize = key.parse().ok()?;
    self.get(db, idx)
  }
  fn get_builtin_field(&self, _db: &'db TypedownDatabase, _key: &str) -> Option<TdObjEnum<'db>> {
    None
  }
  fn source_path(&self, db: &'db TypedownDatabase) -> String {
    self.get_typ(db).source_path(db)
  }
  fn index(&self, db: &'db TypedownDatabase, key: &TdObjEnum<'db>) -> Option<TdObjEnum<'db>> {
    let num = key.as_td_number_obj()?;
    let idx = num.value(db) as usize;
    self.get(db, idx)
  }
  fn len(&self, db: &'db TypedownDatabase) -> Option<usize> {
    Some(self.items(db).len())
  }
}

impl<'db> TdListObj<'db> {
  pub fn len(self, db: &'db TypedownDatabase) -> usize {
    self.items(db).len()
  }

  pub fn get(self, db: &'db TypedownDatabase, idx: usize) -> Option<TdObjEnum<'db>> {
    match self.items(db).into_iter().nth(idx)? {
      Either::Left(hir) => {
        let file_scope = get_file_runtime_scope(db, hir.project(db), hir.node(db).owner_file);
        evaluate_node(db, hir, file_scope).value(db)
      }
      Either::Right(obj) => Some(obj),
    }
  }
}
