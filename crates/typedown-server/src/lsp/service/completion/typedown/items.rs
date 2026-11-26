//! Completion item helpers shared by several providers

use lsp_types::CompletionItemKind;
use typedown_lang::db::TypedownDatabase;
use typedown_lang::db::types::{Symbol, SymbolKind};

pub(super) fn scope_symbol_kind(db: &TypedownDatabase, symbol: Symbol) -> CompletionItemKind {
  match symbol.kind(db) {
    SymbolKind::BuiltinMacro(_) => CompletionItemKind::FUNCTION,
    SymbolKind::BuiltinGlobal(_) => CompletionItemKind::VARIABLE,
    SymbolKind::UserDefinedSchema(..) => CompletionItemKind::CLASS,
    _ => CompletionItemKind::VARIABLE,
  }
}
