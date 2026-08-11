//! Tracked query to get the type of a symbol

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::typechecker::actual_node_type_member::actual_node_type_member;
use crate::db::types::{
  MemberType, Symbol, SymbolKind, TdBlobType, TdTypeType, TypeMember, TypeMemberDescriptors,
  TypeMemberResult,
};
use crate::db::utils::lower_file;
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn get_symbol_type_member(db: &TypedownDatabase, symbol: Symbol) -> TypeMemberResult {
  match symbol.kind(db) {
    SymbolKind::BuiltinSchema(_) | SymbolKind::UserDefinedSchema(_, _) => TypeMemberResult::new(
      db,
      Some(TypeMember::new(
        db,
        MemberType::simple(TdTypeType::get(db).into()),
        TypeMemberDescriptors::empty(),
      )),
      vec![],
    ),
    SymbolKind::UserDefinedResource(project, file) => {
      let (hir, _) = lower_file(db, project, file);
      match hir {
        Some(hir) => actual_node_type_member(db, hir),
        None => TypeMemberResult::new(db, None, vec![]),
      }
    }
    SymbolKind::Asset(_, _, _) => TypeMemberResult::new(
      db,
      Some(TypeMember::new(
        db,
        MemberType::simple(TdBlobType::get(db).into()),
        TypeMemberDescriptors::empty(),
      )),
      vec![],
    ),
    SymbolKind::BuiltinMacro(_) => TypeMemberResult::new(db, None, vec![]),
  }
}
