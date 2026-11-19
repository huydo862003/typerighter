// Schema file variant of the typecheck query

use crate::db::TypedownDatabase;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::typechecker::actual_node_typ::actual_node_typ;
use crate::db::derived::typechecker::expected_node_typ::expected_node_typ;
use crate::db::derived::typechecker::typecheck::{
  typecheck, typecheck_body, typecheck_frontmatter,
};
use crate::db::types::derived::obj_system::TdStaticTyp;
use crate::db::types::typecheck::is_subtyp_of;
use crate::db::types::{ProjectRedNode, TypecheckResult};
use crate::syntax::ast::{AstNode, SourceFile, YamlFrontmatter};
use crate::syntax::diagnostic::*;

pub fn typecheck_schema<'db>(
  db: &'db TypedownDatabase,
  project_node: ProjectRedNode,
) -> TypecheckResult<'db> {
  let node = project_node.node.clone();

  // 1. SourceFile: delegate to frontmatter (no markdown body for schema files)
  if let Some(source_file) = SourceFile::cast(node.clone()) {
    let diagnostics = typecheck_frontmatter(db, &project_node, source_file.frontmatter());
    return TypecheckResult::new(db, diagnostics);
  }

  // 2. YamlFrontmatter: delegate to mapping
  if let Some(frontmatter) = YamlFrontmatter::cast(node.clone()) {
    if let Some(mapping) = frontmatter.mapping() {
      let mapping_node = ProjectRedNode::new(
        project_node.project,
        project_node.file,
        mapping.syntax().clone(),
      );
      return typecheck(db, mapping_node);
    }
    return TypecheckResult::new(db, vec![]);
  }

  // 3. Top-level schema mapping: handle _type/_extends/properties directly
  // Only the root frontmatter mapping has _type in schema files
  if let Some(entries) = project_node.mapping_entries()
    && entries.iter().any(|(key, _)| key == "_type")
  {
    return typecheck_schema_top_level(db, &entries);
  }

  // 4. Other nodes (expressions, nested mappings): standard typecheck flow
  let typ_result = actual_node_typ(db, project_node.clone());
  let mut diagnostics = typ_result.diagnostics(db).clone();

  let declared_typ = match expected_node_typ(db, project_node.clone()).typ(db) {
    Some(typ) => typ,
    None => match typ_result.typ(db) {
      Some(typ) => typ,
      None => return TypecheckResult::new(db, diagnostics),
    },
  };

  diagnostics.extend(typecheck_body(db, project_node, &declared_typ));
  TypecheckResult::new(db, diagnostics)
}

fn typecheck_schema_top_level<'db>(
  db: &'db TypedownDatabase,
  entries: &[(String, ProjectRedNode)],
) -> TypecheckResult<'db> {
  let mut diagnostics = vec![];

  for (key, value_node) in entries {
    if key == "_content" {
      continue;
    }

    // 1. Validate _type/_extends resolve to schema symbols
    if key == "_type" || key == "_extends" {
      let resolved = referee(db, value_node.clone());
      if let Some(symbol) = resolved.value(db)
        && !symbol.kind(db).is_schema()
      {
        let (trimmed_offset, trimmed_len) = value_node.node.trimmed_range();
        diagnostics.push(
          TdDiagnosticKind::FieldTypMismatch {
            field: key.clone(),
            expected: "schema".to_string(),
          }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        );
      }
      continue;
    }

    // 2. Validate properties structure and typecheck each descriptor
    if key == "properties" {
      let Some(descriptor_entries) = value_node.mapping_entries() else {
        let (trimmed_offset, trimmed_len) = value_node.node.trimmed_range();
        diagnostics.push(
          TdDiagnosticKind::FieldTypMismatch {
            field: "properties".to_string(),
            expected: "mapping".to_string(),
          }
          .into_error(trimmed_offset, trimmed_offset + trimmed_len),
        );
        continue;
      };
      for (_, descriptor_node) in &descriptor_entries {
        // 2a. Each descriptor must have a `type` field
        if let Some(descriptor_fields) = descriptor_node.mapping_entries() {
          let has_typ_field = descriptor_fields
            .iter()
            .any(|(field_key, _)| field_key == "type");
          if !has_typ_field {
            let (offset, len) = descriptor_node.node.trimmed_range();
            diagnostics.push(
              TdDiagnosticKind::MissingRequiredField {
                field: "type".to_string(),
              }
              .into_error(offset, offset + len),
            );
          }
        }
        // 2b. Typecheck descriptor fields (type, default, computed)
        diagnostics.extend(typecheck_descriptor_fields(db, descriptor_node));
      }
      continue;
    }

    // 3. Other entries: recurse normally
    let result = typecheck(db, value_node.clone());
    diagnostics.extend(result.diagnostics(db).iter().cloned());
  }

  TypecheckResult::new(db, diagnostics)
}

fn typecheck_descriptor_fields(
  db: &TypedownDatabase,
  descriptor_node: &ProjectRedNode,
) -> Vec<TdDiagnostic> {
  let mut diagnostics = vec![];

  let Some(fields) = descriptor_node.mapping_entries() else {
    return diagnostics;
  };

  for (field_key, field_value_node) in &fields {
    // 1. Recurse into value to catch body errors (e.g., 1 + true)
    let typecheck_result = typecheck(db, field_value_node.clone());
    diagnostics.extend(typecheck_result.diagnostics(db).iter().cloned());

    // 2. Check actual type against expected type
    let actual_result = actual_node_typ(db, field_value_node.clone());
    diagnostics.extend(actual_result.diagnostics(db).iter().cloned());

    let expected_typ = expected_node_typ(db, field_value_node.clone()).typ(db);
    let actual_typ = actual_result.typ(db);

    if let (Some(expected), Some(actual)) = (expected_typ, actual_typ)
      && !is_subtyp_of(db, &actual, &expected)
    {
      let (trimmed_offset, trimmed_len) = field_value_node.node.trimmed_range();
      diagnostics.push(
        TdDiagnosticKind::FieldTypMismatch {
          field: field_key.clone(),
          expected: expected.display_name(db),
        }
        .into_error(trimmed_offset, trimmed_offset + trimmed_len),
      );
    }
  }

  diagnostics
}
