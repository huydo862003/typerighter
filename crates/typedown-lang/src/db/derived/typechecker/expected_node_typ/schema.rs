// Schema file variant of expected_node_typ
// type -> TdTypTyp, default -> sibling type, computed -> fn(Self) -> sibling type

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_typ::{evaluate_typ, resolve_typ_lazy};
use crate::db::derived::get_builtin_typs::{get_func_typ, get_typ_typ};
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::typechecker::expected_node_typ::expected_node_typ_content;
use crate::db::derived::typechecker::utils::{
  DescriptorField, get_descriptor_context, is_inside_descriptor_type_field,
};
use crate::db::types::{FuncSignature, LazyTyp, ProjectRedNode, TdTypEnum, TypResult};
use crate::syntax::ast::{AstNode, YamlMapping};
use crate::syntax::red::RedNode;

pub fn expected_node_typ_schema<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
) -> TypResult<'db> {
  // 1. Direct descriptor field values: return precise expected types
  if let Some(field_kind) = get_descriptor_context(&project_node.node) {
    match field_kind {
      DescriptorField::Type => {
        return TypResult::new(db, Some(get_typ_typ(db).into()), vec![]);
      }
      DescriptorField::Default(descriptor_mapping) => {
        let sibling_typ = resolve_sibling_type(db, &descriptor_mapping, &project_node);
        return TypResult::new(db, sibling_typ, vec![]);
      }
      DescriptorField::Computed(descriptor_mapping) => {
        let sibling_typ = resolve_sibling_type(db, &descriptor_mapping, &project_node);
        let self_typ = get_schema_self_type(db, &project_node);
        if let (Some(ret_typ), Some(self_typ)) = (sibling_typ, self_typ) {
          let signature = FuncSignature::new(
            db,
            None,
            vec![LazyTyp::eager(self_typ)],
            LazyTyp::eager(ret_typ),
          );
          let func_typ = get_func_typ(db, signature);
          return TypResult::new(db, Some(func_typ.into()), vec![]);
        }
        return TypResult::new(db, None, vec![]);
      }
    }
  }

  // 2. Sub-expressions inside a type field (e.g., `string` in `string?`)
  if is_inside_descriptor_type_field(&project_node.node) {
    return TypResult::new(db, Some(get_typ_typ(db).into()), vec![]);
  }

  // 3. Everything else: delegate to normal expected_node_typ
  expected_node_typ_content(db, project_node)
}

fn resolve_sibling_type<'db>(
  db: &'db TypedownDatabase,
  descriptor_mapping: &RedNode,
  project_node: &ProjectRedNode,
) -> Option<TdTypEnum<'db>> {
  let mapping = YamlMapping::cast(descriptor_mapping.clone())?;
  let (_, type_value_expr) = mapping.entries().find(|(key, _)| key == "type")?;
  let type_node = ProjectRedNode::new(
    project_node.project,
    project_node.file,
    type_value_expr.syntax().clone(),
  );
  resolve_typ_lazy(db, type_node, &mut vec![])?.resolve(db)
}

fn get_schema_self_type<'db>(
  db: &'db TypedownDatabase,
  project_node: &ProjectRedNode,
) -> Option<TdTypEnum<'db>> {
  let symbol = file_symbol(db, project_node.project, project_node.file).value(db)?;
  evaluate_typ(db, symbol).typ(db)
}
