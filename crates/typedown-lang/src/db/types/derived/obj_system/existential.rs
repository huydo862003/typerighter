use std::collections::BTreeMap;
use typedown_macros::query_derived;

use super::base::{TdRuntimeObj, TdStaticTyp, TdTypTyp};
use super::{TdObjEnum, TdTypEnum};
use crate::db::TypedownDatabase;
use crate::db::types::{FuncSignature, LazyTyp, TypParams};

/// Existential type: `exists <T0 <: Bound, ...>. Body`
#[query_derived]
pub struct TdExistentialTyp<'db> {
  #[id]
  pub typ_params: TypParams<'db>,
  #[id]
  pub body: Option<LazyTyp<'db>>,
}

impl<'db> TdStaticTyp<'db> for TdExistentialTyp<'db> {
  fn display_name(&self, db: &'db TypedownDatabase) -> String {
    let params = Self::typ_params(*self, db);
    let params_string = params
      .params(db)
      .iter()
      .enumerate()
      .map(|(index, param)| {
        if let Some(bound) = param.upper_bound(db).resolve(db) {
          format!("T{} <: {}", index, bound.display_name(db))
        } else {
          format!("T{}", index)
        }
      })
      .collect::<Vec<_>>()
      .join(", ");

    let body_string = self
      .body(db)
      .and_then(|body| body.resolve(db))
      .map(|body| body.display_name(db))
      .unwrap_or_else(|| "Never".to_string());

    if params_string.is_empty() {
      format!("exists. {}", body_string)
    } else {
      format!("exists <{}>. {}", params_string, body_string)
    }
  }

  fn static_vtable(&self, db: &'db TypedownDatabase) -> BTreeMap<String, TdTypEnum<'db>> {
    self
      .body(db)
      .and_then(|body| body.resolve(db))
      .map(|body| body.static_vtable(db))
      .unwrap_or_default()
  }

  fn get_fields(&self, db: &'db TypedownDatabase) -> BTreeMap<String, LazyTyp<'db>> {
    self
      .body(db)
      .and_then(|body| body.resolve(db))
      .map(|body| body.get_fields(db))
      .unwrap_or_default()
  }

  fn lookup_field_typ(&self, db: &'db TypedownDatabase, name: &str) -> Option<TdTypEnum<'db>> {
    self
      .body(db)
      .and_then(|body| body.resolve(db))?
      .lookup_field_typ(db, name)
  }

  fn index_typ(
    &self,
    db: &'db TypedownDatabase,
    key_typ: &TdTypEnum<'db>,
  ) -> Option<FuncSignature<'db>> {
    self
      .body(db)
      .and_then(|body| body.resolve(db))?
      .index_typ(db, key_typ)
  }

  fn call_typ(
    &self,
    db: &'db TypedownDatabase,
    arg_typs: Vec<TdTypEnum<'db>>,
  ) -> Option<FuncSignature<'db>> {
    self
      .body(db)
      .and_then(|body| body.resolve(db))?
      .call_typ(db, arg_typs)
  }
}

impl<'db> TdRuntimeObj<'db> for TdExistentialTyp<'db> {
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
  use crate::db::derived::get_builtin_typs::{
    get_func_typ, get_list_typ, get_number_typ, get_string_typ,
  };
  use crate::db::types::derived::obj_system::TdProductTyp;
  use crate::db::{QueryStorage, TypedownDatabase};

  fn make_db() -> TypedownDatabase {
    TypedownDatabase {
      storage: QueryStorage::default(),
    }
  }

  #[test]
  fn existential_typ_delegates_static_operations_to_body() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();

    let mut fields = BTreeMap::new();
    fields.insert("title".to_string(), LazyTyp::eager(string_typ.clone()));
    let body_struct: TdTypEnum = TdProductTyp::new(&db, None, fields).into();

    let existential_params = TypParams::new(&db, vec![], vec![]);
    let existential_typ =
      TdExistentialTyp::new(&db, existential_params, Some(LazyTyp::eager(body_struct)));

    // static operations delegate to body
    assert_eq!(
      existential_typ.lookup_field_typ(&db, "title"),
      Some(string_typ.clone())
    );
    assert_eq!(existential_typ.lookup_field_typ(&db, "nonexistent"), None);

    let fields_map = existential_typ.get_fields(&db);
    assert!(fields_map.contains_key("title"));
  }

  #[test]
  fn existential_typ_delegates_index_typ_to_body() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();

    let list_string = get_list_typ(&db)
      .instantiate(&db, vec![LazyTyp::eager(string_typ.clone())])
      .typ(&db);

    let existential_params = TypParams::new(&db, vec![], vec![]);
    let existential_typ =
      TdExistentialTyp::new(&db, existential_params, Some(LazyTyp::eager(list_string)));

    let index_signature = existential_typ.index_typ(&db, &number_typ).unwrap();
    assert_eq!(index_signature.ret(&db), string_typ);
  }

  #[test]
  fn existential_typ_delegates_call_typ_to_body() {
    let db = make_db();
    let string_typ: TdTypEnum = get_string_typ(&db).into();
    let number_typ: TdTypEnum = get_number_typ(&db).into();

    let signature = FuncSignature::new(&db, vec![string_typ.clone()], number_typ.clone());
    let func_typ: TdTypEnum = get_func_typ(&db, signature).into();

    let existential_params = TypParams::new(&db, vec![], vec![]);
    let existential_typ =
      TdExistentialTyp::new(&db, existential_params, Some(LazyTyp::eager(func_typ)));

    let call_signature = existential_typ.call_typ(&db, vec![string_typ]).unwrap();
    assert_eq!(call_signature.ret(&db), number_typ);
  }
}
