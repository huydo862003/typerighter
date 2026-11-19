// Shared schema template utilities for code actions and commands

use typedown_incremental::StableCompare;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_typ::evaluate_typ;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::types::typecheck::is_nullable;
use typedown_lang::db::types::{
  LazyTyp, LitValue, Project, PropertyDescriptor, Scope, ScopeKind, SymbolKind, TdObjEnum,
  TdTypEnum,
};

// Collect all user-defined schemas with their field templates (for init actions)
pub fn collect_schemas(db: &TypedownDatabase, project: Project) -> Vec<(String, String)> {
  let scope = Scope::new(db, ScopeKind::Project(project));

  members(db, scope)
    .members(db)
    .iter()
    .filter(|(_, symbol)| matches!(symbol.kind(db), SymbolKind::UserDefinedSchema(..)))
    .map(|(name, _)| {
      let template = build_template(db, project, name, None);
      // Strip the _type line and delimiters since init action adds those separately
      let inner = template
        .strip_prefix("---\n")
        .and_then(|rest| rest.strip_suffix("---\n"))
        .and_then(|rest| rest.strip_prefix(&format!("_type: {name}\n")))
        .unwrap_or("")
        .to_string();
      (name.clone(), inner)
    })
    .collect()
}

// Just the schema names sorted alphabetically, for use in Select prompts
pub fn collect_schema_names(db: &TypedownDatabase, project: Project) -> Vec<String> {
  let scope = Scope::new(db, ScopeKind::Project(project));
  let mut names: Vec<String> = members(db, scope)
    .members(db)
    .iter()
    .filter(|(_, symbol)| matches!(symbol.kind(db), SymbolKind::UserDefinedSchema(..)))
    .map(|(name, _)| name.clone())
    .collect();
  names.sort();
  names
}

// Build a frontmatter template for a schema
pub fn build_template(
  db: &TypedownDatabase,
  project: Project,
  schema_name: &str,
  backward_link: Option<(&str, &str)>,
) -> String {
  let mut template = format!(
    "---
_type: {schema_name}
"
  );

  let typ = members(db, Scope::new(db, ScopeKind::Project(project)))
    .members(db)
    .get(schema_name)
    .copied()
    .and_then(|symbol| evaluate_typ(db, symbol).typ(db));

  if let Some(schema) = typ.as_ref().and_then(|typ| typ.as_td_schema_typ()) {
    // Builtin metadata fields
    template.push_str(&format!("# _label: \"{schema_name}\"\n"));
    template.push_str("# _icon: icon.file\n");

    for (field_name, descriptor) in schema.fields(db) {
      // Pre-fill backward link field
      if let Some((link_field, link_path)) = backward_link
        && field_name == link_field
      {
        template.push_str(&format!("{field_name}: fref(\"{link_path}\")\n"));
        continue;
      }

      // Skip computed fields (auto-calculated, user doesn't fill these)
      if descriptor.computed_func.is_some() {
        continue;
      }

      let default = format_field_default(db, &descriptor);
      let optional = is_optional_field(db, &descriptor);

      if optional {
        template.push_str(&format!("# {field_name}: {default}\n"));
      } else {
        template.push_str(&format!("{field_name}: {default}\n"));
      }
    }
  }

  template.push_str("---\n");
  template
}

// A field is optional if it's nullable or has a default value
fn is_optional_field(db: &TypedownDatabase, descriptor: &PropertyDescriptor) -> bool {
  descriptor.default_value.is_some()
    || descriptor
      .field_typ
      .resolve(db)
      .is_some_and(|typ| is_nullable(db, &typ))
}

// Generate a default value string, using the schema-defined default if available
fn format_field_default(db: &TypedownDatabase, descriptor: &PropertyDescriptor) -> String {
  // Use the actual default value if the schema defines one
  if let Some(ref default_obj) = descriptor.default_value {
    return format_obj_value(db, default_obj);
  }
  format_default_value(db, &descriptor.field_typ)
}

// Format a runtime object as a frontmatter value string
fn format_obj_value(db: &TypedownDatabase, obj: &TdObjEnum) -> String {
  if let Some(string_obj) = obj.as_td_string_obj() {
    return format!(r#""{}""#, string_obj.value(db));
  }
  if let Some(number_obj) = obj.as_td_number_obj() {
    let value = number_obj.value(db);
    return if value.fract() == 0.0 {
      format!("{}", value as i64)
    } else {
      format!("{value}")
    };
  }
  if let Some(bool_obj) = obj.as_td_bool_obj() {
    return if bool_obj.value(db) {
      "true".to_string()
    } else {
      "false".to_string()
    };
  }
  r#""""#.to_string()
}

// Generate a default value from the type (when no schema default exists)
pub fn format_default_value(db: &TypedownDatabase, lazy: &LazyTyp) -> String {
  let Some(typ) = lazy.resolve(db) else {
    return r#""""#.to_string();
  };
  match typ {
    TdTypEnum::TdLitTyp(lit) => match lit.value(db) {
      LitValue::String(string) => format!(r#""{string}""#),
      _ => r#""""#.to_string(),
    },
    TdTypEnum::TdStringTyp(_) => r#""""#.to_string(),
    TdTypEnum::TdNumberTyp(_) => "0".to_string(),
    TdTypEnum::TdBoolTyp(_) => "false".to_string(),
    TdTypEnum::TdDateTyp(_) => r#""""#.to_string(),
    TdTypEnum::TdDateTimeTyp(_) => r#""""#.to_string(),
    TdTypEnum::TdTimeTyp(_) => r#""""#.to_string(),
    TdTypEnum::TdSchemaTyp(_) => r#""""#.to_string(),
    TdTypEnum::TdListTyp(_) => "[]".to_string(),
    TdTypEnum::TdSumTyp(sum) => {
      let mut members: Vec<_> = sum.members(db).into_iter().collect();
      members.sort_by(|left_member, right_member| left_member.stable_cmp(db, right_member));
      // Optional type: use non-null member's default
      let non_null: Vec<_> = members
        .iter()
        .filter(|member| {
          member
            .resolve(db)
            .is_none_or(|typ| typ.as_td_null_typ().is_none())
        })
        .collect();
      if non_null.len() == 1 {
        return format_default_value(db, non_null[0]);
      }
      // Enum: use first literal as default
      members
        .iter()
        .find_map(|member| {
          if let Some(TdTypEnum::TdLitTyp(lit)) = member.resolve(db)
            && let LitValue::String(string) = lit.value(db)
          {
            return Some(format!("\"{string}\""));
          }
          None
        })
        .unwrap_or_else(|| r#""""#.to_string())
    }
    _ => r#""""#.to_string(),
  }
}

// Convert text to a filename-safe slug
pub fn slugify(text: &str) -> String {
  text
    .to_lowercase()
    .chars()
    .map(|ch| if ch.is_alphanumeric() { ch } else { '-' })
    .collect::<String>()
    .split('-')
    .filter(|segment| !segment.is_empty())
    .collect::<Vec<_>>()
    .join("-")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn slugify_basic() {
    assert_eq!(slugify("Hello World"), "hello-world");
  }

  #[test]
  fn slugify_special_chars() {
    assert_eq!(
      slugify("Dependency Injection (DI)"),
      "dependency-injection-di"
    );
  }

  #[test]
  fn slugify_already_slug() {
    assert_eq!(slugify("my-file"), "my-file");
  }

  #[test]
  fn slugify_leading_trailing() {
    assert_eq!(slugify("  hello  "), "hello");
  }

  #[test]
  fn slugify_unicode() {
    assert_eq!(slugify("cafe au lait"), "cafe-au-lait");
  }
}
