//! Per-file index mapping each symbol to the HIR nodes that reference it

use std::collections::HashMap;
use std::hash::Hasher;

use strum::FromRepr;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::types::{
  File, HirValue, HirValueKind, InterpolatedPart, Project, Symbol, SymbolKind,
};
use crate::db::utils::lower_file;

/// How a symbol is referenced at a particular site
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, FromRepr)]
#[repr(u8)]
pub enum ReferenceKind {
  /// An identifier that resolves to the symbol (e.g. `_type: Person`)
  Ident = 0,
  /// A fref call whose path resolves to the symbol (e.g. `fref("summary.td")`)
  Fref = 1,
}

impl Encodable for ReferenceKind {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    (*self as u8).encode(buf, encoder);
  }
}

impl Decodable for ReferenceKind {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    let tag = u8::decode(data, decoder);
    ReferenceKind::from_repr(tag).expect("unknown ReferenceKind tag")
  }
}

impl StableHash for ReferenceKind {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, _db: &DB, hasher: &mut StableHasher) {
    hasher.write_u8(*self as u8);
  }
}

/// A single reference to a symbol
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Reference {
  pub hir: HirValue,
  pub kind: ReferenceKind,
}

impl Encodable for Reference {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.hir.encode(buf, encoder);
    self.kind.encode(buf, encoder);
  }
}

impl Decodable for Reference {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    Reference {
      hir: HirValue::decode(data, decoder),
      kind: ReferenceKind::decode(data, decoder),
    }
  }
}

impl StableHash for Reference {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.hir.stable_hash(db, hasher);
    self.kind.stable_hash(db, hasher);
  }
}

#[query_derived]
pub struct ResolutionIndex {
  references: HashMap<Symbol, Vec<Reference>>,
}

impl ResolutionIndex {
  /// Look up all references to a given symbol
  pub fn get_references(&self, db: &TypedownDatabase, symbol: Symbol) -> Vec<Reference> {
    self
      .references(db)
      .get(&symbol)
      .cloned()
      .unwrap_or_default()
  }

  /// Get all symbols referenced in this file
  pub fn symbols(&self, db: &TypedownDatabase) -> Vec<Symbol> {
    self.references(db).keys().copied().collect()
  }
}

/// Build an index of all symbol references in a file
#[query_derived]
pub fn resolution_index(db: &TypedownDatabase, project: Project, file: File) -> ResolutionIndex {
  let mut map: HashMap<Symbol, Vec<Reference>> = HashMap::new();
  let (hir, _) = lower_file(db, project, file);
  if let Some(hir) = hir {
    collect_references(db, hir, &mut map);
  }
  ResolutionIndex::new(db, map)
}

fn collect_references(
  db: &TypedownDatabase,
  hir: HirValue,
  map: &mut HashMap<Symbol, Vec<Reference>>,
) {
  match hir.kind(db) {
    HirValueKind::Mapping(values) => {
      for (_, value) in values {
        collect_references(db, value, map);
      }
    }
    HirValueKind::Sequence(items) => {
      for item in items {
        collect_references(db, item, map);
      }
    }
    HirValueKind::Interpolated(parts) | HirValueKind::Markdown(parts) => {
      for part in parts {
        if let InterpolatedPart::Expr(expr) = part {
          collect_references(db, expr, map);
        }
      }
    }
    HirValueKind::Tag { tag, inner } => {
      collect_references(db, *tag, map);
      collect_references(db, *inner, map);
    }
    HirValueKind::Prefix { operand, .. } | HirValueKind::Postfix { operand, .. } => {
      collect_references(db, *operand, map);
    }
    HirValueKind::Binary { left, right, .. } => {
      collect_references(db, *left, map);
      collect_references(db, *right, map);
    }
    // Only fref calls produce file references
    HirValueKind::Call { callee, args } => {
      if let Some(callee_symbol) = referee(db, *callee).value(db)
        && matches!(callee_symbol.kind(db), SymbolKind::BuiltinMacro(_))
        && let Some(target_symbol) = referee(db, hir).value(db)
      {
        map.entry(target_symbol).or_default().push(Reference {
          hir,
          kind: ReferenceKind::Fref,
        });
      }
      collect_references(db, *callee, map);
      for arg in args {
        collect_references(db, arg, map);
      }
    }
    HirValueKind::Index { expr, indices } => {
      collect_references(db, *expr, map);
      for idx in indices {
        collect_references(db, idx, map);
      }
    }
    HirValueKind::Closure { body, .. } => {
      collect_references(db, *body, map);
    }
    // Only Ident nodes resolve to symbols via referee
    HirValueKind::Ident(_) => {
      if let Some(symbol) = referee(db, hir).value(db) {
        map.entry(symbol).or_default().push(Reference {
          hir,
          kind: ReferenceKind::Ident,
        });
      }
    }
    HirValueKind::Str(_)
    | HirValueKind::Num(_)
    | HirValueKind::Math(_)
    | HirValueKind::Bool(_)
    | HirValueKind::Null => {}
  }
}

/// Find all references to a symbol across the project
pub fn references(db: &TypedownDatabase, project: Project, symbol: Symbol) -> Vec<Reference> {
  let mut refs = vec![];
  for file in project.files(db).values() {
    let idx = resolution_index(db, project, *file);
    refs.extend(idx.get_references(db, symbol));
  }
  refs
}

#[cfg(test)]
mod tests {
  use super::{ReferenceKind, references, resolution_index};
  use crate::db::derived::name_resolver::file_symbol::file_symbol;
  use crate::db::fixtures::load_vault_fixture;

  // resolution_index finds schema references in a typed content file
  #[test]
  fn resolution_index_finds_type_reference() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/valid_person.td");
    let idx = resolution_index(&db, project, file);
    // valid_person.td has _type: Person, so "Person" should be in the index
    let syms = idx.symbols(&db);
    assert!(!syms.is_empty(), "should have symbols");
    let refs = idx.get_references(&db, syms[0]);
    assert!(!refs.is_empty(), "should have references");
    assert_eq!(refs[0].kind, ReferenceKind::Ident);
  }

  // resolution_index returns empty for a file with no references
  #[test]
  fn resolution_index_empty_for_untyped() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "content/literal_value.td");
    let idx = resolution_index(&db, project, file);
    assert!(
      idx.symbols(&db).is_empty(),
      "untyped file with no identifiers should have no symbols"
    );
  }

  // references finds all files that reference a given schema symbol
  #[test]
  fn references_finds_usages_across_files() {
    let (db, project, _) = load_vault_fixture("typecheck/my_vault", "content/valid_person.td");
    // Get the Person schema symbol
    let schema_files = project.files(&db);
    let person_path = schema_files
      .keys()
      .find(|path| path.to_string_lossy().contains("Person.td"))
      .expect("should have Person.td");
    let person_file = schema_files[person_path];
    let person_symbol = file_symbol(&db, project, person_file)
      .value(&db)
      .expect("Person.td should have a symbol");

    // valid_person.td references Person via _type
    let refs = references(&db, project, person_symbol);
    assert!(
      !refs.is_empty(),
      "Person should have at least one reference"
    );
  }

  // resolution_index finds fref references
  #[test]
  fn resolution_index_finds_fref() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/article_fref_status.td");
    // article_fref_status.td has fref("summary.td") which resolves to a symbol
    let idx = resolution_index(&db, project, file);
    let all_refs: Vec<_> = idx
      .symbols(&db)
      .into_iter()
      .flat_map(|sym| idx.get_references(&db, sym))
      .collect();
    assert!(
      all_refs.len() >= 2,
      "should have at least 2 references, got {}",
      all_refs.len()
    );
    let has_ident = all_refs.iter().any(|r| r.kind == ReferenceKind::Ident);
    let has_fref = all_refs.iter().any(|r| r.kind == ReferenceKind::Fref);
    assert!(has_ident, "should have an Ident reference (Article)");
    assert!(has_fref, "should have a Fref reference");
  }

  // get_references filters by symbol
  #[test]
  fn get_references_filters_correctly() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "content/article_fref_status.td");
    let idx = resolution_index(&db, project, file);

    // Get the Article schema symbol
    let schema_files = project.files(&db);
    let article_path = schema_files
      .keys()
      .find(|path| path.to_string_lossy().contains("Article.td"))
      .expect("should have Article.td");
    let article_file = schema_files[article_path];
    let article_symbol = file_symbol(&db, project, article_file)
      .value(&db)
      .expect("Article.td should have a symbol");

    let article_refs = idx.get_references(&db, article_symbol);
    assert!(!article_refs.is_empty(), "should find Article reference");
    assert_eq!(article_refs[0].kind, ReferenceKind::Ident);

    // Fref references content/summary.td
    let summary_path = schema_files
      .keys()
      .find(|path| path.ends_with("content/summary.td"))
      .expect("should have content/summary.td");
    let summary_file = schema_files[summary_path];
    let summary_symbol = file_symbol(&db, project, summary_file)
      .value(&db)
      .expect("summary.td should have a symbol");

    let summary_refs = idx.get_references(&db, summary_symbol);
    assert!(
      !summary_refs.is_empty(),
      "should find summary reference via fref"
    );
    assert_eq!(summary_refs[0].kind, ReferenceKind::Fref);
  }
}
