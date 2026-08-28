// Schema property type definition
// A property descriptor inside a schema's `properties` field
// Has a required `type` field and an optional `default` field

use std::collections::HashMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{
  get_bool_type, get_dict_type, get_list_type, get_null_type, get_num_type, get_object_type,
  get_str_type, get_sum_type, get_type_type,
};
use crate::db::types::{
  BuiltinSchemaKind, LazyType, Symbol, SymbolKind, TdProductType, TdStaticType,
};

fn get_schema_property_symbol<'db>(db: &'db TypedownDatabase) -> Symbol<'db> {
  Symbol::new(
    db,
    SymbolKind::BuiltinSchema(BuiltinSchemaKind::SchemaProperty),
    "SchemaProperty".to_string(),
    "@builtin::schema_property".to_string(),
  )
}

#[query_derived]
pub fn get_schema_property_type<'db>(db: &'db TypedownDatabase) -> TdProductType<'db> {
  let type_type = get_type_type(db).into();
  let str_type = get_str_type(db).into();
  let bool_type = get_bool_type(db).into();
  let num_type = get_num_type(db).into();

  // The base scalar types that the `type` field accepts
  let base_type_lazys = vec![
    LazyType::eager(type_type),
    LazyType::eager(str_type),
    LazyType::eager(bool_type),
    LazyType::eager(num_type),
  ];

  // Lazy self-reference to avoid recursive query
  let self_symbol = get_schema_property_symbol(db);
  let self_lazy = LazyType::lazy(self_symbol);

  // list[base | self]
  let list_elem_sum = get_sum_type(
    db,
    [base_type_lazys.clone(), vec![self_lazy.clone()]]
      .concat()
      .into_iter()
      .collect(),
  );
  let list_type = get_list_type(db)
    .instantiate(db, vec![LazyType::eager(list_elem_sum.into())])
    .typ(db);

  // dict[base | self]
  let dict_elem_sum = get_sum_type(
    db,
    [base_type_lazys.clone(), vec![LazyType::lazy(self_symbol)]]
      .concat()
      .into_iter()
      .collect(),
  );
  let dict_type = get_dict_type(db)
    .instantiate(
      db,
      vec![
        LazyType::eager(get_str_type(db).into()),
        LazyType::eager(dict_elem_sum.into()),
      ],
    )
    .typ(db);

  // type field: sum of [base types, list[...], dict[...]]
  let type_field = LazyType::eager(
    get_sum_type(
      db,
      [
        base_type_lazys,
        vec![LazyType::eager(list_type), LazyType::eager(dict_type)],
      ]
      .concat()
      .into_iter()
      .collect(),
    )
    .into(),
  );

  // default field: Object | Null
  // evaluate_type will enforce that the default value actually of the type declared in field "type"
  let default_field = LazyType::eager(
    get_sum_type(
      db,
      vec![
        LazyType::eager(get_object_type(db).into()),
        LazyType::eager(get_null_type(db).into()),
      ],
    )
    .into(),
  );

  let fields = HashMap::from([
    ("type".to_string(), type_field),
    ("default".to_string(), default_field),
  ]);

  TdProductType::new(db, Some("SchemaProperty".to_string()), fields)
}
