use std::collections::BTreeMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::get_string_typ;
use crate::db::types::typecheck::make_nullable;
use crate::db::types::{LazyTyp, PropertyDescriptor, TdSchemaTyp};

#[query_derived]
pub fn get_schema_meta_builtin_field_typ<'db>(db: &'db TypedownDatabase) -> TdSchemaTyp<'db> {
  let string_typ = get_string_typ(db).into();
  let optional_string = LazyTyp::eager(make_nullable(db, string_typ));

  let fields = BTreeMap::from([
    (
      "title".to_string(),
      PropertyDescriptor {
        field_typ: optional_string.clone(),
        default_value: None,
        computed_func: None,
      },
    ),
    (
      "description".to_string(),
      PropertyDescriptor {
        field_typ: optional_string.clone(),
        default_value: None,
        computed_func: None,
      },
    ),
    (
      "image".to_string(),
      PropertyDescriptor {
        field_typ: optional_string.clone(),
        default_value: None,
        computed_func: None,
      },
    ),
    (
      "lang".to_string(),
      PropertyDescriptor {
        field_typ: optional_string,
        default_value: None,
        computed_func: None,
      },
    ),
  ]);

  TdSchemaTyp::new(
    db,
    "_meta".to_string(),
    BTreeMap::new(),
    fields,
    BTreeMap::new(),
    None,
    None,
  )
}
