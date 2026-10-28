//! Per-file index mapping each symbol to the AST nodes that reference it

use std::collections::BTreeMap;
use std::hash::Hasher;

use strum::FromRepr;
use typedown_incremental::{
  Decodable, Decoder, Encodable, Encoder, QueryDatabase, StableHash, StableHasher,
};
use typedown_macros::{StableCompare, query_derived};

use crate::db::TypedownDatabase;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::name_resolver::file_symbol::file_symbol;
use crate::db::derived::name_resolver::referee::referee;
use crate::db::derived::parse_file::parse_file;
use crate::db::types::{File, Project, ProjectRedNode, Symbol, SymbolKind};
use crate::db::utils::{is_content_file, is_external_url, resolve_md_link_path};
use crate::syntax::ast::{
  AstNode, BinaryExpr, CallExpr, ClosureExpr, Expr, IdentLit, IndexExpr, MdBlockElement, MdBody,
  MdInlineElement, MdLink, MdMedia, MdNode, PostfixExpr, PrefixExpr, SourceFile, StrLit,
  YamlFrontmatter, YamlMapping,
};
use crate::syntax::red::RedNode;
use typedown_types::either::Either;

/// How a symbol is referenced at a particular site
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, FromRepr, StableCompare)]
#[repr(u8)]
pub enum ReferenceKind {
  /// An identifier that resolves to the symbol (e.g. `_type: Person`)
  Ident = 0,
  /// A fref call whose path resolves to the symbol (e.g. `fref("summary.td")`)
  Fref = 1,
  /// A markdown link/media URL that resolves to the symbol (e.g. `[text](path.td)`)
  MdLink = 2,
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
#[derive(Clone, Debug, PartialEq, Eq, Hash, StableCompare)]
pub struct Reference {
  pub node: ProjectRedNode,
  pub kind: ReferenceKind,
}

impl Encodable for Reference {
  fn encode(&self, buf: &mut Vec<u8>, encoder: &mut Encoder) {
    self.node.encode(buf, encoder);
    self.kind.encode(buf, encoder);
  }
}

impl Decodable for Reference {
  fn decode(data: &mut &[u8], decoder: &Decoder) -> Self {
    Reference {
      node: ProjectRedNode::decode(data, decoder),
      kind: ReferenceKind::decode(data, decoder),
    }
  }
}

impl StableHash for Reference {
  fn stable_hash<DB: QueryDatabase + ?Sized>(&self, db: &DB, hasher: &mut StableHasher) {
    self.node.stable_hash(db, hasher);
    self.kind.stable_hash(db, hasher);
  }
}

#[query_derived]
pub struct ResolutionIndex<'db> {
  // Keyed by Symbol.def_id for stable ordering (Symbol is interned with session-local IDs)
  references: BTreeMap<String, Vec<Reference>>,
}

impl<'db> ResolutionIndex<'db> {
  /// Look up all references to a given symbol
  pub fn get_references(&self, db: &'db TypedownDatabase, symbol: Symbol) -> Vec<Reference> {
    self
      .references(db)
      .get(&symbol.definition_id(db).to_string())
      .cloned()
      .unwrap_or_default()
  }

  /// Get all def_ids referenced in this file
  pub fn def_ids(&self, db: &'db TypedownDatabase) -> Vec<String> {
    self.references(db).keys().cloned().collect()
  }
}

/// Build an index of all symbol references in a file
#[query_derived]
pub fn resolution_index<'db>(
  db: &'db TypedownDatabase,
  project: Project,
  file: File,
) -> ResolutionIndex<'db> {
  let mut map: BTreeMap<String, Vec<Reference>> = BTreeMap::new();
  let root = parse_file(db, project, file).ast(db);
  collect_references_in_source(db, root, &mut map);
  ResolutionIndex::new(db, map)
}

fn collect_references_in_source(
  db: &TypedownDatabase,
  project_node: ProjectRedNode,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  let source_file = match SourceFile::cast(project_node.node.clone()) {
    Some(source_file) => source_file,
    None => return,
  };
  if let Some(frontmatter) = source_file.frontmatter() {
    collect_references_in_frontmatter(
      db,
      frontmatter,
      project_node.project,
      project_node.file,
      map,
    );
  }
  if let Some(body) = source_file.body() {
    collect_references_in_md_body(db, body, project_node.project, project_node.file, map);
  }
}

fn collect_references_in_frontmatter(
  db: &TypedownDatabase,
  frontmatter: YamlFrontmatter,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  if let Some(mapping) = frontmatter.mapping() {
    collect_references_in_mapping(db, mapping, project, file, map);
  }
}

fn collect_references_in_mapping(
  db: &TypedownDatabase,
  mapping: YamlMapping,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  for (_, value_expr) in mapping.entries() {
    let child = ProjectRedNode::new(project, file, value_expr.syntax().clone());
    collect_references_in_expr(db, child, map);
  }
}

fn collect_references_in_md_body(
  db: &TypedownDatabase,
  body: MdBody,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  for block in body.block_elements() {
    collect_references_in_md_block(db, block, project, file, map);
  }
}

fn collect_references_in_md_block(
  db: &TypedownDatabase,
  block: MdBlockElement,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  match block {
    MdBlockElement::MdParagraph(p) => {
      for inline in p.inline_elements() {
        collect_references_in_md_inline(db, inline, project, file, map);
      }
    }
    MdBlockElement::MdHeading(h) => {
      for inline in h.inline_elements() {
        collect_references_in_md_inline(db, inline, project, file, map);
      }
    }
    MdBlockElement::MdBlockquote(bq) => {
      for inner_block in bq.block_elements() {
        collect_references_in_md_block(db, inner_block, project, file, map);
      }
    }
    MdBlockElement::MdTable(table) => {
      for row in table.rows() {
        for cell in row.cells() {
          for inline in cell.inline_elements() {
            collect_references_in_md_inline(db, inline, project, file, map);
          }
        }
      }
    }
    MdBlockElement::MdBulletList(list) => {
      for item in list.items() {
        for inner_block in item.block_elements() {
          collect_references_in_md_block(db, inner_block, project, file, map);
        }
      }
    }
    MdBlockElement::MdOrderedList(list) => {
      for item in list.items() {
        for inner_block in item.block_elements() {
          collect_references_in_md_block(db, inner_block, project, file, map);
        }
      }
    }
    MdBlockElement::MdContainerBlock(container) => {
      for node in container.value() {
        collect_references_in_md_node(db, node, project, file, map);
      }
    }
    MdBlockElement::MdMedia(media) => {
      collect_references_in_md_link_url(db, project, file, media.syntax(), map);
    }
    MdBlockElement::MdContainerShorthand(_)
    | MdBlockElement::MathBlock(_)
    | MdBlockElement::CodeBlock(_)
    | MdBlockElement::MdHorizontalRule(_) => {}
  }
}

fn collect_references_in_md_node(
  db: &TypedownDatabase,
  node: MdNode,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  match node {
    MdNode::MdHeading(heading) => {
      for inline in heading.inline_elements() {
        collect_references_in_md_inline(db, inline, project, file, map);
      }
    }
    MdNode::MdParagraph(paragraph) => {
      for inline in paragraph.inline_elements() {
        collect_references_in_md_inline(db, inline, project, file, map);
      }
    }
    MdNode::MdBlockquote(block_quote) => {
      for block in block_quote.block_elements() {
        collect_references_in_md_block(db, block, project, file, map);
      }
    }
    MdNode::MdContainerBlock(container) => {
      for inner in container.value() {
        collect_references_in_md_node(db, inner, project, file, map);
      }
    }
    MdNode::MdBold(bold) => {
      for inline in bold.inline_elements() {
        collect_references_in_md_inline(db, inline, project, file, map);
      }
    }
    MdNode::MdItalic(italic) => {
      for inline in italic.inline_elements() {
        collect_references_in_md_inline(db, inline, project, file, map);
      }
    }
    MdNode::MdBoldItalic(bold_italic) => {
      for inline in bold_italic.inline_elements() {
        collect_references_in_md_inline(db, inline, project, file, map);
      }
    }
    MdNode::MdStrikethrough(strikethrough) => {
      for inline in strikethrough.inline_elements() {
        collect_references_in_md_inline(db, inline, project, file, map);
      }
    }
    MdNode::MdLink(link) => {
      collect_references_in_md_link_url(db, project, file, link.syntax(), map);
    }
    MdNode::MdMedia(media) => {
      collect_references_in_md_link_url(db, project, file, media.syntax(), map);
    }
    MdNode::MdBulletList(_)
    | MdNode::MdOrderedList(_)
    | MdNode::MdTable(_)
    | MdNode::MdContainerShorthand(_)
    | MdNode::MdText(_)
    | MdNode::MdHtmlEntity(_) => {}
  }
}

fn collect_references_in_md_inline(
  db: &TypedownDatabase,
  inline: MdInlineElement,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  match inline {
    MdInlineElement::InterpFragment(frag) => {
      if let Some(expr) = frag.expr() {
        let child = ProjectRedNode::new(project, file, expr.syntax().clone());
        collect_references_in_expr(db, child, map);
      }
    }
    MdInlineElement::MdBold(b) => {
      for inner in b.inline_elements() {
        collect_references_in_md_inline(db, inner, project, file, map);
      }
    }
    MdInlineElement::MdItalic(i) => {
      for inner in i.inline_elements() {
        collect_references_in_md_inline(db, inner, project, file, map);
      }
    }
    MdInlineElement::MdBoldItalic(bi) => {
      for inner in bi.inline_elements() {
        collect_references_in_md_inline(db, inner, project, file, map);
      }
    }
    MdInlineElement::MdStrikethrough(s) => {
      for inner in s.inline_elements() {
        collect_references_in_md_inline(db, inner, project, file, map);
      }
    }
    MdInlineElement::MdLink(link) => {
      collect_references_in_md_link_url(db, project, file, link.syntax(), map);
    }
    MdInlineElement::MdMedia(media) => {
      collect_references_in_md_link_url(db, project, file, media.syntax(), map);
    }
    MdInlineElement::MdText(_)
    | MdInlineElement::MdHtmlEntity(_)
    | MdInlineElement::InlineCode(_)
    | MdInlineElement::InlineMath(_)
    | MdInlineElement::CodeLit(_)
    | MdInlineElement::MathLit(_) => {}
  }
}

/// Collect MdLink reference for a markdown link or media URL that points to a vault file
fn collect_references_in_md_link_url(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  node: &RedNode,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  let url_text = if let Some(link) = MdLink::cast(node.clone()) {
    link.url().map(|text| text.value())
  } else if let Some(media) = MdMedia::cast(node.clone()) {
    media.url().map(|text| text.value())
  } else {
    None
  };
  let url_text = match url_text {
    Some(text) => text,
    None => return,
  };

  // Skip external URLs and anchors
  if is_external_url(&url_text) || url_text.starts_with('#') {
    return;
  }

  let config = get_vault_config(db, project);
  let root_dir = config.root_dir(db);
  let base_path = config.base_path(db);

  let handle = file.handle(db);
  let file_dir = match handle.path().and_then(|path| path.parent()) {
    Some(dir) => dir.to_path_buf(),
    None => return,
  };
  let target_path = resolve_md_link_path(&url_text, &file_dir, &root_dir, &base_path);

  let target_file = match project.files(db).get(&target_path) {
    Some(file) => *file,
    None => return,
  };

  let target_symbol = match file_symbol(db, project, target_file).value(db) {
    Some(symbol) => symbol,
    None => return,
  };

  let project_node = ProjectRedNode::new(project, file, node.clone());
  map
    .entry(target_symbol.definition_id(db).to_string())
    .or_default()
    .push(Reference {
      node: project_node,
      kind: ReferenceKind::MdLink,
    });
}

fn collect_references_in_expr(
  db: &TypedownDatabase,
  project_node: ProjectRedNode,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  let project = project_node.project;
  let file = project_node.file;

  let expr = match Expr::cast(project_node.node.clone()) {
    Some(expr) => expr,
    None => return,
  };

  match expr {
    Expr::IdentLit(ident) => {
      collect_references_in_ident(db, ident, project_node, map);
    }
    Expr::StrLit(string_lit) => {
      collect_references_in_string_lit(db, string_lit, project, file, map);
    }
    Expr::CallExpr(call) => {
      collect_references_in_call(db, call, project_node, map);
    }
    Expr::YamlMapping(mapping) => {
      collect_references_in_mapping(db, mapping, project, file, map);
    }
    Expr::YamlSequence(seq) => {
      for value in seq.values() {
        let child = ProjectRedNode::new(project, file, value.syntax().clone());
        collect_references_in_expr(db, child, map);
      }
    }
    Expr::DictLit(dict) => {
      for value in dict.values() {
        let child = ProjectRedNode::new(project, file, value.syntax().clone());
        collect_references_in_expr(db, child, map);
      }
    }
    Expr::ListLit(list) => {
      for item in list.items() {
        if let Some(value) = item.value() {
          let child = ProjectRedNode::new(project, file, value.syntax().clone());
          collect_references_in_expr(db, child, map);
        }
      }
    }
    Expr::BinaryExpr(binary) => {
      collect_references_in_binary(db, binary, project, file, map);
    }
    Expr::PrefixExpr(prefix) => {
      collect_references_in_prefix(db, prefix, project, file, map);
    }
    Expr::PostfixExpr(postfix) => {
      collect_references_in_postfix(db, postfix, project, file, map);
    }
    Expr::IndexExpr(index) => {
      collect_references_in_index(db, index, project, file, map);
    }
    Expr::ClosureExpr(closure) => {
      collect_references_in_closure(db, closure, project, file, map);
    }
    Expr::ParenExpr(paren) => {
      if let Some(inner) = paren.expr() {
        let child = ProjectRedNode::new(project, file, inner.syntax().clone());
        collect_references_in_expr(db, child, map);
      }
    }
    Expr::NumberLit(_) | Expr::CodeLit(_) | Expr::MathLit(_) => {}
  }
}

fn collect_references_in_ident(
  db: &TypedownDatabase,
  ident: IdentLit,
  project_node: ProjectRedNode,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  let name = ident.value().unwrap_or_default();
  if matches!(name.as_str(), "null" | "true" | "false") {
    return;
  }
  let resolved = referee(db, project_node.clone());
  if let Some(symbol) = resolved.value(db) {
    map
      .entry(symbol.definition_id(db).to_string())
      .or_default()
      .push(Reference {
        node: project_node,
        kind: ReferenceKind::Ident,
      });
  }
}

fn collect_references_in_string_lit(
  db: &TypedownDatabase,
  string_lit: StrLit,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  for fragment in string_lit.fragments() {
    if let Either::Right(interp) = fragment
      && let Some(inner_expr) = interp.expr()
    {
      let child = ProjectRedNode::new(project, file, inner_expr.syntax().clone());
      collect_references_in_expr(db, child, map);
    }
  }
}

fn collect_references_in_call(
  db: &TypedownDatabase,
  call: CallExpr,
  call_project_node: ProjectRedNode,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  let project = call_project_node.project;
  let file = call_project_node.file;

  let callee_expr = match call.callee() {
    Some(c) => c,
    None => return,
  };
  let callee_project_node = ProjectRedNode::new(project, file, callee_expr.syntax().clone());
  let callee_ref = referee(db, callee_project_node.clone());

  // fref call: record the call node as a Fref reference to the target symbol
  if let Some(callee_symbol) = callee_ref.value(db)
    && matches!(callee_symbol.kind(db), SymbolKind::BuiltinMacro(_))
    && let Some(target_symbol) = referee(db, call_project_node.clone()).value(db)
  {
    map
      .entry(target_symbol.definition_id(db).to_string())
      .or_default()
      .push(Reference {
        node: call_project_node,
        kind: ReferenceKind::Fref,
      });
  }

  collect_references_in_expr(db, callee_project_node, map);
  for arg in call.args() {
    let child = ProjectRedNode::new(project, file, arg.syntax().clone());
    collect_references_in_expr(db, child, map);
  }
}

fn collect_references_in_binary(
  db: &TypedownDatabase,
  binary: BinaryExpr,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  if let Some(left) = binary.left() {
    let child = ProjectRedNode::new(project, file, left.syntax().clone());
    collect_references_in_expr(db, child, map);
  }
  if let Some(right) = binary.right() {
    let child = ProjectRedNode::new(project, file, right.syntax().clone());
    collect_references_in_expr(db, child, map);
  }
}

fn collect_references_in_prefix(
  db: &TypedownDatabase,
  prefix: PrefixExpr,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  if let Some(operand) = prefix.expr() {
    let child = ProjectRedNode::new(project, file, operand.syntax().clone());
    collect_references_in_expr(db, child, map);
  }
}

fn collect_references_in_postfix(
  db: &TypedownDatabase,
  postfix: PostfixExpr,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  if let Some(operand) = postfix.expr() {
    let child = ProjectRedNode::new(project, file, operand.syntax().clone());
    collect_references_in_expr(db, child, map);
  }
}

fn collect_references_in_index(
  db: &TypedownDatabase,
  index: IndexExpr,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  if let Some(expr) = index.expr() {
    let child = ProjectRedNode::new(project, file, expr.syntax().clone());
    collect_references_in_expr(db, child, map);
  }
  for idx in index.indices() {
    let child = ProjectRedNode::new(project, file, idx.syntax().clone());
    collect_references_in_expr(db, child, map);
  }
}

fn collect_references_in_closure(
  db: &TypedownDatabase,
  closure: ClosureExpr,
  project: Project,
  file: File,
  map: &mut BTreeMap<String, Vec<Reference>>,
) {
  if let Some(body) = closure.body() {
    let child = ProjectRedNode::new(project, file, body.syntax().clone());
    collect_references_in_expr(db, child, map);
  }
}

/// Find all references to a symbol across the project
pub fn references(db: &TypedownDatabase, project: Project, symbol: Symbol) -> Vec<Reference> {
  let mut references = vec![];
  for (path, file) in project.files(db).iter() {
    if !is_content_file(path) {
      continue;
    }
    let index = resolution_index(db, project, *file);
    references.extend(index.get_references(db, symbol));
  }
  references
}

const MAX_AFFECTED_DEPTH: usize = 5;

// Collect files that transitively reference the given file, up to MAX_AFFECTED_DEPTH levels
pub fn find_transitive_referrers(db: &TypedownDatabase, project: Project, file: File) -> Vec<File> {
  let symbol = match file_symbol(db, project, file).value(db) {
    Some(symbol) => symbol,
    None => return vec![],
  };

  let mut affected = std::collections::HashSet::new();
  let mut current_level = vec![symbol];

  for _ in 0..MAX_AFFECTED_DEPTH {
    let mut next_level = vec![];
    for symbol in &current_level {
      for reference in references(db, project, *symbol) {
        let ref_file = reference.node.file;
        if ref_file == file {
          continue;
        }
        if affected.insert(ref_file)
          && let Some(ref_symbol) = file_symbol(db, project, ref_file).value(db)
        {
          next_level.push(ref_symbol);
        }
      }
    }
    if next_level.is_empty() {
      break;
    }
    current_level = next_level;
  }

  affected.into_iter().collect()
}

#[cfg(test)]
mod tests {
  use super::{ReferenceKind, references, resolution_index};
  use crate::db::derived::name_resolver::file_symbol::file_symbol;
  use crate::db::fixtures::load_vault_fixture;

  // resolution_index finds schema references in a typed content file
  #[test]
  fn resolution_index_finds_type_reference() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
    let index = resolution_index(&db, project, file);
    // valid_person.td has _type: Person, so "Person" should be in the index
    let definition_ids = index.def_ids(&db);
    assert!(!definition_ids.is_empty(), "should have symbols");
    // Look up the first symbol by def_id
    let references: Vec<_> = index
      .references(&db)
      .values()
      .next()
      .cloned()
      .unwrap_or_default();
    assert!(!references.is_empty(), "should have references");
    assert_eq!(references[0].kind, ReferenceKind::Ident);
  }

  // resolution_index returns empty for a file with no references
  #[test]
  fn resolution_index_empty_for_untyped() {
    let (db, project, file) = load_vault_fixture("typecheck/my_vault", "literal_value.td");
    let index = resolution_index(&db, project, file);
    assert!(
      index.def_ids(&db).is_empty(),
      "untyped file with no identifiers should have no symbols"
    );
  }

  // references finds all files that reference a given schema symbol
  #[test]
  fn references_finds_usages_across_files() {
    let (db, project, _) = load_vault_fixture("typecheck/my_vault", "valid_person.td");
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
    let references = references(&db, project, person_symbol);
    assert!(
      !references.is_empty(),
      "Person should have at least one reference"
    );
  }

  // resolution_index finds fref references
  #[test]
  fn resolution_index_finds_fref() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "article_fref_status.td");
    // article_fref_status.td has fref("summary.td") which resolves to a symbol
    let index = resolution_index(&db, project, file);
    let all_references: Vec<_> = index
      .references(&db)
      .values()
      .flat_map(|references| references.iter().cloned())
      .collect();
    assert!(
      all_references.len() >= 2,
      "should have at least 2 references, got {}",
      all_references.len()
    );
    let has_ident = all_references
      .iter()
      .any(|reference| reference.kind == ReferenceKind::Ident);
    let has_fref = all_references
      .iter()
      .any(|reference| reference.kind == ReferenceKind::Fref);
    assert!(has_ident, "should have an Ident reference (Article)");
    assert!(has_fref, "should have a Fref reference");
  }

  // get_references filters by symbol
  #[test]
  fn get_references_filters_correctly() {
    let (db, project, file) =
      load_vault_fixture("typecheck/narrow_vault", "article_fref_status.td");
    let index = resolution_index(&db, project, file);

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

    let article_references = index.get_references(&db, article_symbol);
    assert!(
      !article_references.is_empty(),
      "should find Article reference"
    );
    assert_eq!(article_references[0].kind, ReferenceKind::Ident);

    // Fref references summary.td
    let summary_path = schema_files
      .keys()
      .find(|path| path.ends_with("summary.td"))
      .expect("should have summary.td");
    let summary_file = schema_files[summary_path];
    let summary_symbol = file_symbol(&db, project, summary_file)
      .value(&db)
      .expect("summary.td should have a symbol");

    let summary_references = index.get_references(&db, summary_symbol);
    assert!(
      !summary_references.is_empty(),
      "should find summary reference via fref"
    );
    assert_eq!(summary_references[0].kind, ReferenceKind::Fref);
  }

  // references() on an asset symbol finds content files that fref it without panicking
  #[test]
  fn references_finds_asset_referrers() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "icon.svg");
    let symbol = file_symbol(&db, project, file)
      .value(&db)
      .expect("asset should have a symbol");
    let references = references(&db, project, symbol);
    assert!(
      !references.is_empty(),
      "icon.svg is referenced by with_asset_fref.td"
    );
    assert_eq!(references[0].kind, ReferenceKind::Fref);
  }
}
