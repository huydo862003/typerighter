//! Component names, props and slots for `:::` containers and `[[...]]` shorthand

use lsp_types::{CompletionItem, CompletionItemKind};
use typedown_lang::db::derived::get_component_contract::get_component_contract;
use typedown_lang::db::utils::{kebab_to_pascal, pascal_to_kebab};
use typedown_lang::syntax::ast::{AstNode, MdContainerBlock, MdContainerShorthand};
use typedown_lang::syntax::red::RedNode;

use crate::lsp::service::utils::cursor::CursorContext;

/// Component names after `:::` or `[[`; callouts are only valid for `:::`
pub(super) fn complete_label(
  ctx: &CursorContext,
  builtin_callouts: bool,
) -> Option<Vec<CompletionItem>> {
  let mut items = vec![];

  if builtin_callouts {
    for name in [
      "tip",
      "info",
      "warning",
      "danger",
      "details",
      "note",
      "important",
      "caution",
    ] {
      items.push(CompletionItem {
        label: name.to_string(),
        kind: Some(CompletionItemKind::SNIPPET),
        detail: Some("builtin callout".to_string()),
        ..Default::default()
      });
    }
  }

  // Custom components from contract
  if let Some(contract_file) = ctx.project.custom_component_contract(ctx.db) {
    let contract = get_component_contract(ctx.db, contract_file);
    for entry in contract.entries(ctx.db) {
      items.push(CompletionItem {
        label: pascal_to_kebab(&entry.name),
        kind: Some(CompletionItemKind::MODULE),
        detail: Some(format!("component ({})", entry.source)),
        ..Default::default()
      });
    }
  }

  if items.is_empty() {
    return None;
  }
  Some(items)
}

/// Prop names inside a container's `{...}`, minus the ones already present
pub(super) fn complete_props(
  ctx: &CursorContext,
  container: &RedNode,
) -> Option<Vec<CompletionItem>> {
  let (component_name, existing) = if let Some(block) = MdContainerBlock::cast(container.clone()) {
    (block.label()?, block.prop_names())
  } else {
    let shorthand = MdContainerShorthand::cast(container.clone())?;
    (shorthand.label()?, shorthand.prop_names())
  };
  let pascal = kebab_to_pascal(&component_name);

  let contract_file = ctx.project.custom_component_contract(ctx.db)?;
  let contract = get_component_contract(ctx.db, contract_file);
  let entries = contract.entries(ctx.db).clone();
  let entry = entries
    .iter()
    .find(|entry| entry.name == component_name || entry.name == pascal)?;

  let props = entry.props.as_ref()?;

  let items = props
    .iter()
    .filter(|prop| !existing.iter().any(|entry| entry == &prop.name))
    .map(|prop| {
      let insert = if prop.required {
        format!("{}=", prop.name)
      } else {
        prop.name.clone()
      };
      CompletionItem {
        label: prop.name.clone(),
        kind: Some(CompletionItemKind::PROPERTY),
        detail: Some(format!(
          "{}{}",
          prop.typ_name,
          if prop.required { "" } else { "?" }
        )),
        documentation: prop.description.as_ref().map(|documentation| {
          lsp_types::Documentation::MarkupContent(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: documentation.clone(),
          })
        }),
        insert_text: Some(insert),
        ..Default::default()
      }
    })
    .collect();

  Some(items)
}

/// Slot names on a `===` separator, minus the ones already used
pub(super) fn complete_slots(
  ctx: &CursorContext,
  separator: &RedNode,
) -> Option<Vec<CompletionItem>> {
  let block = MdContainerBlock::cast(separator.parent()?)?;
  let component_name = block.label()?;
  let pascal = kebab_to_pascal(&component_name);

  let contract_file = ctx.project.custom_component_contract(ctx.db)?;
  let contract = get_component_contract(ctx.db, contract_file);
  let entries = contract.entries(ctx.db).clone();
  let entry = entries
    .iter()
    .find(|entry| entry.name == component_name || entry.name == pascal)?;

  if entry.slots.is_empty() {
    return None;
  }

  let existing = block.slot_names();
  let items = entry
    .slots
    .iter()
    .filter(|slot| !existing.iter().any(|entry| entry == *slot))
    .map(|slot| CompletionItem {
      label: slot.clone(),
      kind: Some(CompletionItemKind::ENUM_MEMBER),
      detail: Some("slot".to_string()),
      ..Default::default()
    })
    .collect();

  Some(items)
}
