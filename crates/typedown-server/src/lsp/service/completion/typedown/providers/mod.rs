//! One provider per `CursorPosition`; `complete` is the dispatch

mod code_block;
mod container;
mod field_key;
mod fref;
mod imports;
mod interpolation;
mod member_access;
mod type_name;
mod value;

use lsp_types::CompletionItem;

use super::position::CursorPosition;
use crate::lsp::service::utils::cursor::CursorContext;

/// `None` means there is nothing to offer at this position
pub(super) fn complete(
  ctx: &CursorContext,
  position: CursorPosition,
) -> Option<Vec<CompletionItem>> {
  match position {
    CursorPosition::CodeBlockFence => code_block::complete(ctx),
    CursorPosition::ContainerLabel { builtin_callouts } => {
      container::complete_label(ctx, builtin_callouts)
    }
    CursorPosition::ContainerProps { container } => container::complete_props(ctx, &container),
    CursorPosition::ContainerSlot { separator } => container::complete_slots(ctx, &separator),
    CursorPosition::TypeName => Some(type_name::complete(ctx)),
    CursorPosition::MemberAccess { binary } => member_access::complete(ctx, &binary),
    CursorPosition::FrefArg => Some(fref::complete_arg(ctx)),
    CursorPosition::ImportsValue => Some(imports::complete(ctx)),
    CursorPosition::Interpolation => Some(interpolation::complete(ctx)),
    CursorPosition::Value => Some(value::complete(ctx)),
    CursorPosition::FieldKey => field_key::complete(ctx),
  }
}
