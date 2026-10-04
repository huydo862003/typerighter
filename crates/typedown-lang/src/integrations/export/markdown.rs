//! Export typedown markdown body to CommonMark

use std::path::{Path, PathBuf};

use super::file_ref::try_resolve_fref;
use super::utils;
use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::get_vault_config::get_vault_config;
use crate::db::derived::hir::lower_node;
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::types::{File, FileRedNode, Project, TdRuntimeObject};
use crate::syntax::ast::{AstNode, InterpFragment, MdBody, MdLink, MdMedia};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

pub fn export_markdown_body(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  body: &MdBody,
) -> String {
  let mut emitter = MarkdownExporter::new(db, project, file);
  emitter.emit_body(body);
  emitter.finish()
}

struct MarkdownExporter<'a> {
  db: &'a TypedownDatabase,
  project: Project,
  file: File,
  out: String,
  prefix: String,
  at_line_start: bool,
}

impl<'a> MarkdownExporter<'a> {
  fn new(db: &'a TypedownDatabase, project: Project, file: File) -> Self {
    Self {
      db,
      project,
      file,
      out: String::new(),
      prefix: String::new(),
      at_line_start: true,
    }
  }

  fn resolve_url(&self, url: &str) -> String {
    let config = get_vault_config(self.db, self.project);
    let handle = self.file.handle(self.db);
    let empty = PathBuf::new();
    let file_dir = handle
      .path()
      .unwrap_or(&empty)
      .parent()
      .and_then(|p| p.strip_prefix(config.root_dir(self.db)).ok())
      .unwrap_or(Path::new(""));

    utils::resolve_vault_url(url, &config.base_path(self.db), file_dir)
  }

  fn finish(mut self) -> String {
    if !self.out.ends_with('\n') {
      self.out.push('\n');
    }
    self.out
  }

  fn write(&mut self, text: &str) {
    for ch in text.chars() {
      if ch == '\n' {
        self.out.push('\n');
        self.at_line_start = true;
      } else {
        if self.at_line_start {
          self.out.push_str(&self.prefix);
          self.at_line_start = false;
        }
        self.out.push(ch);
      }
    }
  }

  fn newline(&mut self) {
    self.out.push('\n');
    self.at_line_start = true;
  }

  fn emit_body(&mut self, body: &MdBody) {
    let mut first = true;
    for child in body.syntax().children() {
      if child.kind() == SyntaxKind::Whitespace || child.kind() == SyntaxKind::Newline {
        continue;
      }
      if !first {
        self.newline();
      }
      first = false;
      self.emit_block(&child);
    }
  }

  fn emit_block(&mut self, node: &RedNode) {
    match node.kind() {
      SyntaxKind::MdHeading => self.emit_heading(node),
      SyntaxKind::MdHorizontalRule => self.emit_horizontal_rule(node),
      SyntaxKind::MdParagraph => self.emit_paragraph(node),
      SyntaxKind::MdBlockquote => self.emit_blockquote(node),
      SyntaxKind::MdBulletList => self.emit_list(node),
      SyntaxKind::MdOrderedList => self.emit_list(node),
      SyntaxKind::MdContainerBlock => self.emit_container(node),
      SyntaxKind::MdContainerShorthand => self.emit_container_shorthand(node),
      SyntaxKind::MdTable => self.emit_passthrough(node),
      SyntaxKind::CodeBlock | SyntaxKind::MathBlock => self.emit_passthrough(node),
      _ => self.emit_passthrough(node),
    }
  }

  fn emit_child_blocks(&mut self, node: &RedNode) {
    let mut first = true;
    for child in node.children() {
      let kind = child.kind();
      if kind == SyntaxKind::Whitespace || kind == SyntaxKind::Newline {
        continue;
      }
      if !first {
        self.newline();
      }
      first = false;
      self.emit_block(&child);
    }
  }

  fn emit_heading(&mut self, node: &RedNode) {
    self.emit_inline_children(node);
    self.newline();
  }

  fn emit_horizontal_rule(&mut self, node: &RedNode) {
    self.write(&node.text());
    self.newline();
  }

  fn emit_paragraph(&mut self, node: &RedNode) {
    self.emit_inline_children(node);
    self.newline();
  }

  fn emit_blockquote(&mut self, node: &RedNode) {
    let old_prefix = self.prefix.clone();
    self.prefix.push_str("> ");

    let mut first = true;
    for child in node.children() {
      let kind = child.kind();
      if kind == SyntaxKind::MdSymbol
        || kind == SyntaxKind::Whitespace
        || kind == SyntaxKind::Newline
      {
        continue;
      }
      if !first {
        self.newline();
      }
      first = false;
      self.emit_block(&child);
    }

    self.prefix = old_prefix;
  }

  fn emit_list(&mut self, node: &RedNode) {
    for child in node.children() {
      match child.kind() {
        SyntaxKind::MdBulletListItem | SyntaxKind::MdTaskListItem => {
          self.emit_list_item(&child, "- ");
        }
        SyntaxKind::MdOrderedListItem => {
          let marker = self.extract_ordered_marker(&child);
          self.emit_list_item(&child, &marker);
        }
        _ => {}
      }
    }
  }

  fn extract_ordered_marker(&self, node: &RedNode) -> String {
    let mut num = String::new();
    for child in node.children() {
      match child.kind() {
        SyntaxKind::MdNumber => num = child.text().to_string(),
        SyntaxKind::MdSymbol if child.text() == "." => {
          return format!("{num}. ");
        }
        _ => {
          if !num.is_empty() {
            break;
          }
        }
      }
    }
    "1. ".to_string()
  }

  fn emit_list_item(&mut self, node: &RedNode, marker: &str) {
    let old_prefix = self.prefix.clone();
    let continuation = " ".repeat(marker.len());

    self.write(marker);
    self.prefix.push_str(&continuation);

    if node.kind() == SyntaxKind::MdTaskListItem {
      for child in node.children() {
        if child.kind() == SyntaxKind::MdCheckbox {
          self.write(&child.text());
          self.write(" ");
          break;
        }
      }
    }

    let mut first = true;
    for child in node.children() {
      let kind = child.kind();
      if kind == SyntaxKind::MdSymbol
        || kind == SyntaxKind::MdNumber
        || kind == SyntaxKind::Whitespace
        || kind == SyntaxKind::Newline
        || kind == SyntaxKind::MdCheckbox
      {
        continue;
      }
      if !first {
        self.newline();
      }
      first = false;
      self.emit_block(&child);
    }

    self.prefix = old_prefix;
  }

  fn emit_container(&mut self, node: &RedNode) {
    self.write(":::");
    let mut seen_opening = false;
    for child in node.children() {
      if child.kind() == SyntaxKind::MdSymbol && child.text() == ":::" && !seen_opening {
        seen_opening = true;
        continue;
      }
      if child.kind() == SyntaxKind::Newline {
        break;
      }
      if seen_opening {
        self.emit_inline(&child);
      }
    }
    self.newline();

    for child in node.children() {
      match child.kind() {
        SyntaxKind::MdContainerSlot => {
          self.emit_child_blocks(&child);
        }
        SyntaxKind::MdContainerSlotSeparator => {
          self.write(&child.text());
          self.newline();
        }
        _ => {}
      }
    }

    self.write(":::");
    self.newline();
  }

  fn emit_container_shorthand(&mut self, node: &RedNode) {
    let mut label = String::new();
    let mut props = String::new();

    for child in node.children() {
      match child.kind() {
        SyntaxKind::Ident => label.push_str(&child.text()),
        SyntaxKind::MdSymbol if child.text() == "-" => label.push('-'),
        SyntaxKind::MdContainerPropBlock => props = child.text().trim().to_string(),
        _ => {}
      }
    }

    self.write("::: ");
    self.write(&label);
    if !props.is_empty() {
      self.write(" ");
      self.write(&props);
    }
    self.newline();
    self.write(":::");
    self.newline();
  }

  fn emit_passthrough(&mut self, node: &RedNode) {
    let text = node.text().to_string();

    let mut min_indent = usize::MAX;
    for line in text.lines() {
      if !line.trim().is_empty() {
        let indent = line.len() - line.trim_start().len();
        if indent < min_indent {
          min_indent = indent;
        }
      }
    }
    if min_indent == usize::MAX {
      min_indent = 0;
    }

    for (index, line) in text.lines().enumerate() {
      if index > 0 {
        self.newline();
      }
      if line.len() >= min_indent {
        self.write(&line[min_indent..]);
      } else {
        self.write(line.trim_start());
      }
    }
    self.newline();
  }

  fn emit_inline_children(&mut self, node: &RedNode) {
    let mut started = false;
    self.emit_inline_children_inner(node, &mut started);
  }

  fn emit_inline_children_inner(&mut self, node: &RedNode, started: &mut bool) {
    for child in node.children() {
      let kind = child.kind();
      if kind == SyntaxKind::Newline {
        continue;
      }
      if !*started && kind == SyntaxKind::Whitespace {
        continue;
      }
      if !*started && child.as_token().is_none() && kind != SyntaxKind::InterpFragment {
        self.emit_inline_children_inner(&child, started);
        continue;
      }
      *started = true;
      self.emit_inline(&child);
    }
  }

  fn emit_inline(&mut self, node: &RedNode) {
    if node.as_token().is_some() {
      self.write(&node.text());
      return;
    }

    if node.kind() == SyntaxKind::InterpFragment {
      let Some(fragment) = InterpFragment::cast(node.clone()) else {
        return;
      };
      let Some(expr) = fragment.expr() else { return };
      let expr_node = expr.syntax().clone();

      if let Some(link) = try_resolve_fref(self.db, self.project, self.file, &expr_node) {
        self.write(&link);
        return;
      }
      let hir = lower_node(
        self.db,
        self.project,
        FileRedNode::new(self.file, expr_node),
      );
      let scope = get_file_runtime_scope(self.db, self.project, self.file);
      let eval = evaluate_node(self.db, hir, scope);
      let obj = eval.value(self.db);
      if let Some(obj) = obj
        && let Some(func) = obj.lookup_method(self.db, "to_string")
        && let Ok(result) = func.call(self.db, self.project, Some(obj), vec![])
        && let Some(str_obj) = result.as_td_str_obj()
      {
        self.write(&str_obj.value(self.db));
      }
      return;
    }

    // Resolve URLs in links and images
    if node.kind() == SyntaxKind::MdLink
      && let Some(link) = MdLink::cast(node.clone())
    {
      let url = link.url().map(|t| t.value()).unwrap_or_default();
      let resolved = self.resolve_url(&url);
      self.write("[");
      if let Some(alt) = link.alt() {
        self.emit_inline_children(alt.syntax());
      }
      self.write(&format!("]({})", resolved));
      return;
    }

    if node.kind() == SyntaxKind::MdMedia
      && let Some(media) = MdMedia::cast(node.clone())
    {
      let alt = media.alt().map(|t| t.value()).unwrap_or_default();
      let url = media.url().map(|t| t.value()).unwrap_or_default();
      let resolved = self.resolve_url(&url);
      self.write(&format!("![{}]({})", alt, resolved));
      return;
    }

    for child in node.children() {
      self.emit_inline(&child);
    }
  }
}
