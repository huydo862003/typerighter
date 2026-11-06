use std::collections::BTreeMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::{
  get_bool_typ, get_func_typ, get_null_typ, get_number_typ, get_obj_typ, get_schema_meta_typ,
  get_string_typ, get_sum_typ,
};
use crate::db::types::{
  Binder, BuiltinSchemaKind, FuncKind, FuncSignature, LazyTyp, NativeFuncKind, TdBoundVariableTyp,
  TdComputedTyp, TdFuncObj, TdListTyp, TdMetatypeInstanceTyp, TdSchemaTyp, TdStaticTyp, TdTypEnum,
  TypFn, TypParams, Variance,
};

#[query_derived]
pub fn get_node_set_typ<'db>(db: &'db TypedownDatabase) -> TdSchemaTyp<'db> {
  let element_bound_variable = TdBoundVariableTyp::new(
    db,
    Binder::BuiltinSchema(BuiltinSchemaKind::NodeSet),
    0,
    LazyTyp::eager(get_obj_typ(db).into()),
    Variance::Invariant,
  );
  let element_typ: TdTypEnum = element_bound_variable.into();

  let bool_typ: TdTypEnum = get_bool_typ(db).into();
  let string_typ: TdTypEnum = get_string_typ(db).into();
  let number_typ: TdTypEnum = get_number_typ(db).into();
  let null_typ: TdTypEnum = get_null_typ(db).into();

  let nullable_element: TdTypEnum = get_sum_typ(
    db,
    vec![
      LazyTyp::eager(element_typ.clone()),
      LazyTyp::eager(null_typ),
    ],
  )
  .into();

  let nullable_obj: TdTypEnum = get_sum_typ(
    db,
    vec![
      LazyTyp::eager(get_obj_typ(db).into()),
      LazyTyp::eager(get_null_typ(db).into()),
    ],
  )
  .into();

  let list_of_element: TdTypEnum =
    TdListTyp::new(db, Some(LazyTyp::eager(element_typ.clone()))).into();

  // Step 1: build non-self-referential vtable entries
  let mut vtable = BTreeMap::new();

  vtable.insert(
    "to_list".to_string(),
    TdFuncObj::new(
      db,
      "to_list".to_string(),
      FuncSignature::new(db, None, vec![], list_of_element),
      FuncKind::Native(NativeFuncKind::NodeSetToList),
    ),
  );

  vtable.insert(
    "first".to_string(),
    TdFuncObj::new(
      db,
      "first".to_string(),
      FuncSignature::new(db, None, vec![], nullable_element),
      FuncKind::Native(NativeFuncKind::NodeSetFirst),
    ),
  );

  vtable.insert(
    "count".to_string(),
    TdFuncObj::new(
      db,
      "count".to_string(),
      FuncSignature::new(db, None, vec![], number_typ),
      FuncKind::Native(NativeFuncKind::NodeSetCount),
    ),
  );

  vtable.insert(
    "contains".to_string(),
    TdFuncObj::new(
      db,
      "contains".to_string(),
      FuncSignature::new(db, None, vec![element_typ.clone()], bool_typ.clone()),
      FuncKind::Native(NativeFuncKind::NodeSetContains),
    ),
  );

  // Step 2: build node_set with non-self-ref vtable to get a type we can use as self-ref return
  let node_set_base_schema = TdSchemaTyp::new(
    db,
    "node_set".to_string(),
    BTreeMap::new(),
    BTreeMap::new(),
    vtable.clone(),
    None,
    Some(TypParams::new(db, vec![element_bound_variable], vec![])),
  );
  let node_set_base: TdTypEnum = node_set_base_schema.into();

  // Step 3: add self-referential vtable entries
  let filter_pred_typ: TdTypEnum = get_func_typ(
    db,
    FuncSignature::new(db, None, vec![element_typ.clone()], bool_typ),
  )
  .into();

  vtable.insert(
    "filter".to_string(),
    TdFuncObj::new(
      db,
      "filter".to_string(),
      FuncSignature::new(db, None, vec![filter_pred_typ], node_set_base.clone()),
      FuncKind::Native(NativeFuncKind::NodeSetFilter),
    ),
  );

  // follow<K: string>(field: K, opts?) -> node_set<Unwrap<Projection<T, K>>>
  // K is bounded by string, T is the element type from the enclosing node_set
  let follow_key_variable = TdBoundVariableTyp::new(
    db,
    Binder::BuiltinFunc(NativeFuncKind::NodeSetFollow),
    1,
    LazyTyp::eager(get_string_typ(db).into()),
    Variance::Invariant,
  );
  let follow_key_typ: TdTypEnum = follow_key_variable.into();
  // Projection<T, K> looks up field K on element type T
  let projection: TdTypEnum = TdComputedTyp::new(
    db,
    TypFn::Projection,
    vec![element_typ.clone(), follow_key_typ.clone()],
  )
  .into();
  // Unwrap strips nullable/list to get the referenced schema
  let unwrapped: TdTypEnum = TdComputedTyp::new(db, TypFn::Unwrap, vec![projection]).into();
  // Instantiate node_set with the unwrapped type as element
  let follow_return: TdTypEnum = node_set_base_schema
    .instantiate(db, vec![LazyTyp::eager(unwrapped)])
    .typ(db);
  let follow_params = TypParams::new(db, vec![follow_key_variable], vec![]);
  vtable.insert(
    "follow".to_string(),
    TdFuncObj::new(
      db,
      "follow".to_string(),
      FuncSignature::new(
        db,
        Some(follow_params),
        vec![follow_key_typ, nullable_obj],
        follow_return,
      ),
      FuncKind::Native(NativeFuncKind::NodeSetFollow),
    ),
  );

  // follow_back<S: schema_meta>(source: S, field: string) -> node_set<metatype_instance<S>>
  let follow_back_source_variable = TdBoundVariableTyp::new(
    db,
    Binder::BuiltinFunc(NativeFuncKind::NodeSetFollowBack),
    1,
    LazyTyp::eager(get_schema_meta_typ(db).into()),
    Variance::Invariant,
  );
  let follow_back_source_typ: TdTypEnum = follow_back_source_variable.into();
  let follow_back_element: TdTypEnum =
    TdMetatypeInstanceTyp::new(db, follow_back_source_typ.clone()).into();
  let follow_back_return: TdTypEnum = node_set_base_schema
    .instantiate(db, vec![LazyTyp::eager(follow_back_element)])
    .typ(db);
  let follow_back_params = TypParams::new(db, vec![follow_back_source_variable], vec![]);
  vtable.insert(
    "follow_back".to_string(),
    TdFuncObj::new(
      db,
      "follow_back".to_string(),
      FuncSignature::new(
        db,
        Some(follow_back_params),
        vec![follow_back_source_typ, string_typ],
        follow_back_return,
      ),
      FuncKind::Native(NativeFuncKind::NodeSetFollowBack),
    ),
  );

  // Step 4: rebuild with complete vtable
  TdSchemaTyp::new(
    db,
    "node_set".to_string(),
    BTreeMap::new(),
    BTreeMap::new(),
    vtable,
    None,
    Some(TypParams::new(db, vec![element_bound_variable], vec![])),
  )
}
