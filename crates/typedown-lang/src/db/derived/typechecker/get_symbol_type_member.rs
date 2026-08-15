//! Tracked query to get the type of a symbol

use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::{get_schema_type, get_type_type};
use crate::db::derived::typechecker::actual_node_type_member::actual_node_type_member;
use crate::db::derived::vault::get_vault_type;
use crate::db::types::{
  BuiltinGlobalKind, LazyType, MemberType, Symbol, SymbolKind, TdBlobType, TypeMember,
  TypeMemberDescriptors, TypeMemberResult,
};
use crate::db::utils::lower_file;
use typedown_incremental::QueryDatabase;

#[query_derived]
pub fn get_symbol_type_member(db: &TypedownDatabase, symbol: Symbol) -> TypeMemberResult {
  match symbol.kind(db) {
    SymbolKind::BuiltinSchema(_) => TypeMemberResult::new(
      db,
      Some(TypeMember::new(
        db,
        MemberType::Simple(LazyType::eager(get_type_type(db).into())),
        TypeMemberDescriptors::empty(),
      )),
      vec![],
    ),
    SymbolKind::UserDefinedSchema(_, _) => TypeMemberResult::new(
      db,
      Some(TypeMember::new(
        db,
        MemberType::Simple(LazyType::eager(get_schema_type(db).into())),
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
        MemberType::Simple(LazyType::eager(TdBlobType::get(db).into())),
        TypeMemberDescriptors::empty(),
      )),
      vec![],
    ),
    SymbolKind::BuiltinMacro(_) => TypeMemberResult::new(db, None, vec![]),
    SymbolKind::BuiltinGlobal(kind) => {
      let typ = match kind {
        BuiltinGlobalKind::Vault => get_vault_type(db).into(),
      };
      TypeMemberResult::new(
        db,
        Some(TypeMember::new(
          db,
          MemberType::Simple(LazyType::eager(typ)),
          TypeMemberDescriptors::empty(),
        )),
        vec![],
      )
    }
  }
}
