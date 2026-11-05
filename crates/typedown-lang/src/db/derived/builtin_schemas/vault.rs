use std::collections::BTreeMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::builtin_schemas::node_set::get_node_set_typ;
use crate::db::derived::get_builtin_typs::{get_obj_typ, get_schema_meta_typ};
use crate::db::types::derived::obj_system::TdStaticTyp;
use crate::db::types::typecheck::make_nullable;
use crate::db::types::{
  Binder, FuncKind, FuncSignature, LazyTyp, NativeFuncKind, TdBoundVariableTyp, TdFuncObj,
  TdMetatypeInstanceTyp, TdSchemaTyp, TdTypEnum, TypParams, Variance,
};

#[query_derived]
pub fn get_vault_typ<'db>(db: &'db TypedownDatabase) -> TdSchemaTyp<'db> {
  let schema_meta_typ: TdTypEnum = get_schema_meta_typ(db).into();
  let obj_typ: TdTypEnum = get_obj_typ(db).into();
  let nullable_options: TdTypEnum = make_nullable(db, obj_typ.clone());
  let node_set_typ = get_node_set_typ(db);

  /* from_nodes<T: schema_meta>(typ: T, opts?) -> node_set<metatype_instance<T>> */

  // Type variable T bounded by schema_meta, rejects non-schema args
  let from_nodes_variable = TdBoundVariableTyp::new(
    db,
    Binder::BuiltinFunc(NativeFuncKind::NodeSetFromVault),
    0,
    LazyTyp::eager(schema_meta_typ),
    Variance::Invariant,
  );
  let from_nodes_variable_typ: TdTypEnum = from_nodes_variable.into();

  // Return type: instantiate node_set with metatype_instance<T> as element
  // When T = lit<Person>, metatype_instance resolves to Person
  let from_nodes_element =
    LazyTyp::eager(TdMetatypeInstanceTyp::new(db, from_nodes_variable_typ.clone()).into());
  let from_nodes_return: TdTypEnum = node_set_typ
    .instantiate(db, vec![from_nodes_element])
    .typ(db);

  let from_nodes_func = TdFuncObj::new(
    db,
    "from_nodes".to_string(),
    FuncSignature::new(
      db,
      Some(TypParams::new(db, vec![from_nodes_variable], vec![])),
      vec![from_nodes_variable_typ, nullable_options],
      from_nodes_return,
    ),
    FuncKind::Native(NativeFuncKind::NodeSetFromVault),
  );

  /* from_node<U: metatype_instance<schema_meta>>(node: U) -> node_set<U> */

  let instance_of_schema_meta: TdTypEnum =
    TdMetatypeInstanceTyp::new(db, get_schema_meta_typ(db).into()).into();
  let from_node_variable = TdBoundVariableTyp::new(
    db,
    Binder::BuiltinFunc(NativeFuncKind::NodeSetFromNode),
    0,
    LazyTyp::eager(instance_of_schema_meta),
    Variance::Invariant,
  );
  let from_node_variable_typ: TdTypEnum = from_node_variable.into();

  let from_node_return: TdTypEnum = node_set_typ
    .instantiate(db, vec![LazyTyp::eager(from_node_variable_typ.clone())])
    .typ(db);

  let from_node_func = TdFuncObj::new(
    db,
    "from_node".to_string(),
    FuncSignature::new(
      db,
      Some(TypParams::new(db, vec![from_node_variable], vec![])),
      vec![from_node_variable_typ],
      from_node_return,
    ),
    FuncKind::Native(NativeFuncKind::NodeSetFromNode),
  );

  let vtable = BTreeMap::from([
    ("from_nodes".to_string(), from_nodes_func),
    ("from_node".to_string(), from_node_func),
  ]);

  TdSchemaTyp::new(
    db,
    "vault".to_string(),
    BTreeMap::new(),
    BTreeMap::new(),
    vtable,
    None,
    None,
  )
}
