// A property descriptor inside a schema's `properties` field
// Has a required `type` field and an optional `default` field

use std::collections::BTreeMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{
  get_bool_typ, get_dict_typ, get_func_typ, get_list_typ, get_null_typ, get_number_typ,
  get_obj_typ, get_string_typ, get_sum_typ, get_typ_typ,
};
use crate::db::types::{
  BuiltinSchemaKind, FuncSignature, LazyTyp, PropertyDescriptor, Symbol, SymbolKind, TdSchemaTyp,
  TdStaticTyp,
};

fn get_schema_prop_symbol(db: &TypedownDatabase) -> Symbol {
  Symbol::new(db, SymbolKind::BuiltinSchema(BuiltinSchemaKind::SchemaProp))
}

#[query_derived]
pub fn get_schema_prop_typ<'db>(db: &'db TypedownDatabase) -> TdSchemaTyp<'db> {
  let typ_typ = get_typ_typ(db).into();
  let string_typ = get_string_typ(db).into();
  let bool_typ = get_bool_typ(db).into();
  let number_typ = get_number_typ(db).into();
  let obj_typ = get_obj_typ(db).into();

  // The base scalar types that the `type` field accepts
  let base_typ_lazys = vec![
    LazyTyp::eager(typ_typ),
    LazyTyp::eager(string_typ),
    LazyTyp::eager(bool_typ),
    LazyTyp::eager(number_typ),
  ];

  // Lazy self-reference to avoid recursive query
  let self_symbol = get_schema_prop_symbol(db);
  let self_lazy = LazyTyp::lazy(self_symbol);

  // list[base | self]
  let list_element_sum = get_sum_typ(
    db,
    [base_typ_lazys.clone(), vec![self_lazy.clone()]]
      .concat()
      .into_iter()
      .collect(),
  );
  let list = get_list_typ(db);
  let list_typ = list
    .instantiate(db, vec![LazyTyp::eager(list_element_sum.into())])
    .typ(db);

  // dict[base | self]
  let dict_element_sum = get_sum_typ(
    db,
    [base_typ_lazys.clone(), vec![LazyTyp::lazy(self_symbol)]]
      .concat()
      .into_iter()
      .collect(),
  );
  let dict_typ = get_dict_typ(db)
    .instantiate(
      db,
      vec![
        LazyTyp::eager(get_string_typ(db).into()),
        LazyTyp::eager(dict_element_sum.into()),
      ],
    )
    .typ(db);

  // type field: sum of [base types, list[...], dict[...]]
  let typ_field_typ = LazyTyp::eager(
    get_sum_typ(
      db,
      [
        base_typ_lazys,
        vec![LazyTyp::eager(list_typ), LazyTyp::eager(dict_typ)],
      ]
      .concat()
      .into_iter()
      .collect(),
    )
    .into(),
  );

  // default field: Object | Null
  let nullable_obj = LazyTyp::eager(
    get_sum_typ(
      db,
      vec![
        LazyTyp::eager(obj_typ),
        LazyTyp::eager(get_null_typ(db).into()),
      ],
    )
    .into(),
  );

  // computed field: function (obj) -> obj | null
  let computed_func_typ = LazyTyp::eager(
    get_sum_typ(
      db,
      vec![
        LazyTyp::eager(
          get_func_typ(
            db,
            FuncSignature::new(
              db,
              None,
              vec![LazyTyp::eager(get_obj_typ(db).into())],
              LazyTyp::eager(get_obj_typ(db).into()),
            ),
          )
          .into(),
        ),
        LazyTyp::eager(get_null_typ(db).into()),
      ],
    )
    .into(),
  );

  let fields = BTreeMap::from([
    (
      "type".to_string(),
      PropertyDescriptor {
        field_typ: typ_field_typ,
        default_value: None,
        computed_func: None,
      },
    ),
    (
      "default".to_string(),
      PropertyDescriptor {
        field_typ: nullable_obj.clone(),
        default_value: None,
        computed_func: None,
      },
    ),
    (
      "computed".to_string(),
      PropertyDescriptor {
        field_typ: computed_func_typ,
        default_value: None,
        computed_func: None,
      },
    ),
  ]);

  TdSchemaTyp::new(
    db,
    "SchemaProperty".to_string(),
    BTreeMap::new(),
    fields,
    BTreeMap::new(),
    None,
    None,
  )
}
