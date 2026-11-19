// Schema file variant of actual_node_typ
// Nodes inside descriptor `type` field return TdTypTyp

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_typs::get_typ_typ;
use crate::db::derived::typechecker::actual_node_typ::actual_node_typ_content;
use crate::db::derived::typechecker::utils::is_inside_descriptor_type_field;
use crate::db::types::{ProjectRedNode, TypResult};

pub fn actual_node_typ_schema<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
) -> TypResult<'db> {
  if is_inside_descriptor_type_field(&project_node.node) {
    return TypResult::new(db, Some(get_typ_typ(db).into()), vec![]);
  }

  actual_node_typ_content(db, project_node)
}
