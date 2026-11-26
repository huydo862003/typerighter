//! Schema names after `_type:`

use lsp_types::{CompletionItem, CompletionItemKind, InsertTextFormat};
use typedown_incremental::StableCompare;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::derived::evaluate::evaluate_typ::evaluate_typ;
use typedown_lang::db::derived::name_resolver::members::members;
use typedown_lang::db::types::{
  LazyTyp, LitValue, Project, Scope, ScopeKind, Symbol, SymbolKind, TdStaticTyp, TdTypEnum,
};

use crate::lsp::service::utils::cursor::CursorContext;

pub(super) fn complete(ctx: &CursorContext) -> Vec<CompletionItem> {
  collect_schema_completions(ctx.db, ctx.project)
}

// Suggest all user-defined schema names visible in the project scope
fn collect_schema_completions(db: &TypedownDatabase, project: Project) -> Vec<CompletionItem> {
  let scope = Scope::new(db, ScopeKind::Project(project));
  members(db, scope)
    .members(db)
    .iter()
    .filter(|(_, symbol)| matches!(symbol.kind(db), SymbolKind::UserDefinedSchema(..)))
    .map(|(name, symbol)| {
      let snippet = build_schema_snippet(db, name, symbol);

      CompletionItem {
        label: name.clone(),
        kind: Some(CompletionItemKind::CLASS),
        insert_text: Some(snippet),
        insert_text_format: Some(InsertTextFormat::SNIPPET),
        ..Default::default()
      }
    })
    .collect()
}

// Build a snippet with all schema fields as placeholders
fn build_schema_snippet(db: &TypedownDatabase, name: &str, symbol: &Symbol) -> String {
  let typ = evaluate_typ(db, *symbol).typ(db);
  let schema = typ.as_ref().and_then(|typ| typ.as_td_schema_typ());

  let Some(schema) = schema else {
    return name.to_string();
  };

  let fields = schema.fields(db);
  let mut snippet = name.to_string();
  for (tab_stop, (field_name, prop_descriptor)) in fields.iter().enumerate() {
    let placeholder = make_snippet_placeholder(db, &prop_descriptor.field_typ, 0);
    let index = tab_stop + 1;

    snippet.push_str(&format!("\n{field_name}: ${{{index}:{placeholder}}}"));
  }

  snippet
}

// Generate a placeholder string for a lazy type
fn make_snippet_placeholder(db: &TypedownDatabase, lazy: &LazyTyp, indent: usize) -> String {
  let Some(typ) = lazy.resolve(db) else {
    return "value".to_string();
  };
  match typ {
    TdTypEnum::TdSumTyp(sum) => {
      let mut members: Vec<_> = sum.members(db).into_iter().collect();
      members.sort_by(|left_member, right_member| left_member.stable_cmp(db, right_member));
      // Optional type: use non-null member's placeholder
      let non_null: Vec<_> = members
        .iter()
        .filter(|member| {
          member
            .resolve(db)
            .is_none_or(|typ| typ.as_td_null_typ().is_none())
        })
        .collect();
      if non_null.len() == 1 {
        return make_snippet_placeholder(db, non_null[0], indent);
      }
      // Enum: use first literal string option as default
      let first = members.iter().find_map(|member| {
        if let Some(TdTypEnum::TdLitTyp(lit)) = member.resolve(db)
          && let LitValue::String(string) = lit.value(db)
        {
          return Some(string);
        }
        None
      });
      first.unwrap_or_else(|| "value".to_string())
    }
    _ => make_simple_typ_placeholder(db, &typ, indent),
  }
}

fn make_simple_typ_placeholder(db: &TypedownDatabase, typ: &TdTypEnum, indent: usize) -> String {
  match typ {
    TdTypEnum::TdStringTyp(_) => "string".to_string(),
    TdTypEnum::TdNumberTyp(_) => "0".to_string(),
    TdTypEnum::TdBoolTyp(_) => "true".to_string(),
    TdTypEnum::TdDateTyp(_) => "date".to_string(),
    TdTypEnum::TdDateTimeTyp(_) => "datetime".to_string(),
    TdTypEnum::TdTimeTyp(_) => "time".to_string(),
    TdTypEnum::TdListTyp(list) => {
      let inner = list
        .element(db)
        .and_then(|element| element.resolve(db))
        .map(|element| make_simple_typ_placeholder(db, &element, indent + 1))
        .unwrap_or_else(|| "value".to_string());
      let pad = "  ".repeat(indent);

      format!("\n{pad}- {inner}")
    }
    TdTypEnum::TdSchemaTyp(schema) => {
      format!("fref(\"{}\")", schema.name(db))
    }
    TdTypEnum::TdProductTyp(product) => {
      let fields = product.get_fields(db);
      let pad = "  ".repeat(indent + 1);
      let mut nested = String::new();
      for (field_name, field_lazy) in &fields {
        let placeholder = make_snippet_placeholder(db, field_lazy, indent + 1);
        nested.push_str(&format!("\n{pad}{field_name}: {placeholder}"));
      }
      nested
    }
    _ => "value".to_string(),
  }
}
