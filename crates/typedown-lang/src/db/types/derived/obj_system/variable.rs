use std::collections::BTreeMap;
use typedown_macros::query_interned;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::bound_variable::TdBoundVariableTyp;
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{CallTypResult, FuncSignature, LazyTyp, ProjectRedNode};

// Free variable instantiated from a bound variable at a specific site
#[query_interned]
pub struct TdFreeVariableTyp<'db> {
  pub bound_origin: TdBoundVariableTyp<'db>,
  pub instantiation_site: ProjectRedNode,
}

impl<'db> TdFreeVariableTyp<'db> {
  fn resolve_bound(&self, db: &'db TypedownDatabase) -> Option<TdTypEnum<'db>> {
    let bound = Self::bound_origin(*self, db);
    TdBoundVariableTyp::upper_bound(bound, db).resolve(db)
  }
}

impl<'db> TdStaticTyp<'db> for TdFreeVariableTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    let bound = Self::bound_origin(*self, db);
    let name = format!("T{}", TdBoundVariableTyp::offset(bound, db));
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

impl<'db> TdRuntimeObj<'db> for TdFreeVariableTyp<'db> {
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::derived::get_builtin_typs::{get_obj_typ, get_string_typ};
  use crate::db::derived::parse_file::parse_file;
  use crate::db::fixtures::load_vault_fixture;
  use crate::db::types::derived::obj_system::TdProductTyp;
  use crate::db::types::{Binder, BuiltinSchemaKind, TdBoundVariableTyp, Variance};

  #[test]
  fn check_free_variable_delegates_to_upper_bound() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let site = parse_file(&db, project, file).ast(&db);

    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let mut fields = BTreeMap::new();
    fields.insert("name".to_string(), LazyTyp::eager(string_typ.clone()));
    let struct_typ: TdTypEnum = TdProductTyp::new(&db, None, fields).into();

    let bound = TdBoundVariableTyp::new(
      &db,
      Binder::BuiltinSchema(BuiltinSchemaKind::Obj),
      0,
      LazyTyp::eager(struct_typ),
      Variance::Covariant,
    );
    let free_variable = TdFreeVariableTyp::new(&db, bound, site);

    assert_eq!(
      free_variable.lookup_field_typ(&db, "name"),
      Some(string_typ)
    );
    assert_eq!(free_variable.lookup_field_typ(&db, "nonexistent"), None);
  }

  #[test]
  fn check_different_sites_produce_distinct_variables() {
    let (db, project, _) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let files: Vec<_> = project.files(&db).values().copied().collect();
    let upper_bound = LazyTyp::eager(get_obj_typ(&db).into());

    let site_a = parse_file(&db, project, files[0]).ast(&db);
    let site_b = parse_file(&db, project, files[1]).ast(&db);

    let bound_a = TdBoundVariableTyp::new(
      &db,
      Binder::BuiltinSchema(BuiltinSchemaKind::Obj),
      0,
      upper_bound.clone(),
      Variance::Covariant,
    );
    let variable_a = TdFreeVariableTyp::new(&db, bound_a, site_a);
    let bound_b = TdBoundVariableTyp::new(
      &db,
      Binder::BuiltinSchema(BuiltinSchemaKind::Obj),
      0,
      upper_bound,
      Variance::Covariant,
    );
    let variable_b = TdFreeVariableTyp::new(&db, bound_b, site_b);

    // Same index and upper bound, different sites - must be distinct
    let typ_a: TdTypEnum = variable_a.into();
    let typ_b: TdTypEnum = variable_b.into();
    assert_ne!(typ_a, typ_b);
  }
}
