use std::collections::BTreeMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{get_bool_typ, get_true};
use crate::db::types::{LazyTyp, PropertyDescriptor, TdObjEnum, TdSchemaTyp};

#[query_derived]
pub fn get_schema_ssg_builtin_field_typ<'db>(db: &'db TypedownDatabase) -> TdSchemaTyp<'db> {
  let bool_typ = LazyTyp::eager(get_bool_typ(db).into());
  let default_true: TdObjEnum = get_true(db).into();

  let fields = BTreeMap::from([
    (
      "show_in_sidebar".to_string(),
      PropertyDescriptor {
        field_typ: bool_typ.clone(),
        default_value: Some(default_true.clone()),
        computed_func: None,
      },
    ),
    (
      "show_in_search".to_string(),
      PropertyDescriptor {
        field_typ: bool_typ.clone(),
        default_value: Some(default_true.clone()),
        computed_func: None,
      },
    ),
    (
      "crawlable".to_string(),
      PropertyDescriptor {
        field_typ: bool_typ,
        default_value: Some(default_true),
        computed_func: None,
      },
    ),
  ]);

  TdSchemaTyp::new(
    db,
    "_ssg".to_string(),
    BTreeMap::new(),
    fields,
    BTreeMap::new(),
    None,
    None,
  )
}
