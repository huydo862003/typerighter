//! Emit HTML directly from the typedown AST, with placeholders for shiki and KaTeX

use std::collections::HashMap;

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;
use crate::db::derived::hir::lower_node;
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::types::{File, Project, TdRuntimeObject};
use crate::syntax::ast::{
  AstNode, CodeBlock, InlineCode, InlineMath, InterpFragment, MathBlock, MdBody, MdHeading,
  MdTable, MdTableCell,
};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

use super::utils::{
  collect_inline_children, extract_plain_text, html_escape, is_delimiter, is_external_url, slugify,
  strip_quotes,
};

/// Heading extracted during HTML emission
#[derive(Debug, Clone)]
#[cfg_attr(feature = "export", derive(serde::Serialize))]
pub struct ExportedHeading {
  pub level: u32,
  pub title: String,
  pub slug: String,
}

/// Result of HTML body emission
pub struct HtmlBodyResult {
  pub html: String,
  pub headings: Vec<ExportedHeading>,
  pub title: Option<String>,
}

pub fn export_html_body(
  db: &TypedownDatabase,
  project: Project,
  file: File,
  body: &MdBody,
) -> HtmlBodyResult {
  let mut emitter = HtmlEmitter::new(db, project, file);
  emitter.emit_body(body);
  emitter.finish()
}

struct HtmlEmitter<'a> {
  db: &'a TypedownDatabase,
  project: Project,
  file: File,
  out: String,
  headings: Vec<ExportedHeading>,
  title: Option<String>,
  slug_counts: HashMap<String, usize>,
}

impl<'a> HtmlEmitter<'a> {
  fn new(db: &'a TypedownDatabase, project: Project, file: File) -> Self {
    Self {
      db,
      project,
      file,
      out: String::new(),
      headings: Vec::new(),
      title: None,
      slug_counts: HashMap::new(),
    }
  }

  fn finish(self) -> HtmlBodyResult {
    HtmlBodyResult {
      html: self.out,
      headings: self.headings,
      title: self.title,
    }
  }

  fn write(&mut self, text: &str) {
    self.out.push_str(text);
  }

  fn write_escaped(&mut self, text: &str) {
    self.out.push_str(&html_escape(text));
  }

  // Block emission

  fn emit_body(&mut self, body: &MdBody) {
    for child in body.syntax().children() {
      if child.kind() == SyntaxKind::Whitespace || child.kind() == SyntaxKind::Newline {
        continue;
      }
      self.emit_block(&child);
    }
  }

  fn emit_block(&mut self, node: &RedNode) {
    match node.kind() {
      SyntaxKind::MdHeading => self.emit_heading(node),
      SyntaxKind::MdHorizontalRule => self.write("<hr>\n"),
      SyntaxKind::MdParagraph => self.emit_paragraph(node),
      SyntaxKind::MdBlockquote => self.emit_blockquote(node),
      SyntaxKind::MdBulletList => self.emit_bullet_list(node),
      SyntaxKind::MdOrderedList => self.emit_ordered_list(node),
      SyntaxKind::MdTable => self.emit_table(node),
      SyntaxKind::MdContainerBlock => self.emit_container(node),
      SyntaxKind::MdContainerShorthand => self.emit_container_shorthand(node),
      SyntaxKind::CodeBlock => self.emit_code_block(node),
      SyntaxKind::MathBlock => self.emit_math_block(node),
      _ => {}
    }
  }

  fn emit_heading(&mut self, node: &RedNode) {
    let Some(heading) = MdHeading::cast(node.clone()) else {
      return;
    };
    let level = heading.level();
    if level == 0 || level > 6 {
      return;
    }

    let plain_text = extract_plain_text(node);
    let slug = self.make_unique_slug(&plain_text);

    if self.title.is_none() && level == 1 {
      self.title = Some(plain_text.clone());
    }

    self.headings.push(ExportedHeading {
      level: level as u32,
      title: plain_text.clone(),
      slug: slug.clone(),
    });

    let escaped_slug = html_escape(&slug);
    self.write(&format!("<h{level} id=\"{escaped_slug}\">"));
    self.emit_inline_children(node);
    self.write(&format!(
      " <a class=\"td-header-anchor\" href=\"#{escaped_slug}\" aria-label=\"Permalink to &quot;{}&quot;\">&#8203;</a>",
      html_escape(&plain_text)
    ));
    self.write(&format!("</h{level}>\n"));
  }

  fn emit_paragraph(&mut self, node: &RedNode) {
    self.write("<p>");
    self.emit_inline_children(node);
    self.write("</p>\n");
  }

  fn emit_blockquote(&mut self, node: &RedNode) {
    self.write("<blockquote>\n");
    for child in node.children() {
      let kind = child.kind();
      if kind == SyntaxKind::MdSymbol
        || kind == SyntaxKind::Whitespace
        || kind == SyntaxKind::Newline
      {
        continue;
      }
      self.emit_block(&child);
    }
    self.write("</blockquote>\n");
  }

  fn emit_bullet_list(&mut self, node: &RedNode) {
    let has_task = node
      .children()
      .any(|c| c.kind() == SyntaxKind::MdTaskListItem);

    if has_task {
      self.write("<ul class=\"td-task-list\">\n");
    } else {
      self.write("<ul>\n");
    }

    for child in node.children() {
      match child.kind() {
        SyntaxKind::MdBulletListItem => self.emit_list_item(&child),
        SyntaxKind::MdTaskListItem => self.emit_task_list_item(&child),
        _ => {}
      }
    }
    self.write("</ul>\n");
  }

  fn emit_ordered_list(&mut self, node: &RedNode) {
    let start = extract_ordered_start(node);
    if start > 1 {
      self.write(&format!("<ol start=\"{start}\">\n"));
    } else {
      self.write("<ol>\n");
    }

    for child in node.children() {
      if child.kind() == SyntaxKind::MdOrderedListItem {
        self.emit_list_item(&child);
      }
    }
    self.write("</ol>\n");
  }

  fn emit_list_item(&mut self, node: &RedNode) {
    self.write("<li>");

    let has_block = node.children().any(|c| c.kind().is_md_block());

    if has_block {
      self.write("\n");
      for child in node.children() {
        let kind = child.kind();
        if kind == SyntaxKind::MdSymbol
          || kind == SyntaxKind::MdNumber
          || kind == SyntaxKind::Whitespace
          || kind == SyntaxKind::Newline
        {
          continue;
        }
        if kind.is_md_block() {
          self.emit_block(&child);
        }
      }
    } else {
      self.emit_inline_children(node);
    }

    self.write("</li>\n");
  }

  fn emit_task_list_item(&mut self, node: &RedNode) {
    let checked = node
      .children()
      .any(|c| c.kind() == SyntaxKind::MdCheckbox && c.text().contains('x'));

    self.write("<li class=\"td-task-list-item\">");
    if checked {
      self.write("<input type=\"checkbox\" disabled checked> ");
    } else {
      self.write("<input type=\"checkbox\" disabled> ");
    }

    // Emit first paragraph inline (no <p> wrap) so text stays next to checkbox
    let mut first_paragraph = true;
    for child in node.children() {
      let kind = child.kind();
      if kind == SyntaxKind::MdSymbol
        || kind == SyntaxKind::Whitespace
        || kind == SyntaxKind::Newline
        || kind == SyntaxKind::MdCheckbox
      {
        continue;
      }
      if kind == SyntaxKind::MdParagraph && first_paragraph {
        first_paragraph = false;
        self.emit_inline_children(&child);
      } else if kind.is_md_block() {
        self.write("\n");
        self.emit_block(&child);
      } else {
        self.emit_inline(&child);
      }
    }

    self.write("</li>\n");
  }

  fn emit_table(&mut self, node: &RedNode) {
    let Some(table) = MdTable::cast(node.clone()) else {
      return;
    };

    let alignments = parse_separator_alignments(node);

    self.write("<table tabindex=\"0\">\n");

    if let Some(header) = table.header() {
      self.write("<thead><tr>\n");
      for (i, cell) in header.cells().enumerate() {
        self.emit_table_cell(&cell, "th", alignments.get(i).copied().flatten());
      }
      self.write("</tr></thead>\n");
    }

    self.write("<tbody>\n");
    for row in table.rows() {
      self.write("<tr>\n");
      for (i, cell) in row.cells().enumerate() {
        self.emit_table_cell(&cell, "td", alignments.get(i).copied().flatten());
      }
      self.write("</tr>\n");
    }
    self.write("</tbody>\n</table>\n");
  }

  fn emit_table_cell(&mut self, cell: &MdTableCell, tag: &str, align: Option<&str>) {
    if let Some(a) = align {
      self.write(&format!("<{tag} style=\"text-align:{a}\">"));
    } else {
      self.write(&format!("<{tag}>"));
    }
    for elem in cell.inline_elements() {
      self.emit_inline(elem.syntax());
    }
    self.write(&format!("</{tag}>\n"));
  }

  fn emit_code_block(&mut self, node: &RedNode) {
    let Some(block) = CodeBlock::cast(node.clone()) else {
      return;
    };
    let lang = block.language().unwrap_or_default();
    let label = block.label().unwrap_or_default();
    let value = block.value().unwrap_or_default();

    self.write("<pre class=\"td-code-placeholder\"");
    if !lang.is_empty() {
      self.write(" data-lang=\"");
      self.write_escaped(&lang);
      self.write("\"");
    }
    if !label.is_empty() {
      self.write(" data-meta=\"");
      self.write_escaped(&label);
      self.write("\"");
    }
    self.write("><code>");
    self.write_escaped(&value);
    self.write("</code></pre>\n");
  }

  fn emit_math_block(&mut self, node: &RedNode) {
    let Some(block) = MathBlock::cast(node.clone()) else {
      return;
    };
    let value = block.value().unwrap_or_default();
    self.write("<div class=\"td-math-block\">");
    self.write_escaped(&value);
    self.write("</div>\n");
  }

  fn emit_container(&mut self, node: &RedNode) {
    let (label, title) = extract_container_label_and_title(node);

    if is_callout(&label) {
      self.emit_callout_container(node, &label, title.as_deref());
    } else {
      self.emit_component_container(node, &label);
    }
  }

  fn emit_callout_container(&mut self, node: &RedNode, name: &str, custom_title: Option<&str>) {
    let default_title = callout_default_title(name);
    // Default titles are static strings, custom titles come from user content
    let title_text = custom_title
      .map(html_escape)
      .unwrap_or_else(|| default_title.to_string());
    let has_custom_title = custom_title.is_some();

    if name == "details" {
      self.write(&format!(
        "<details class=\"details td-callout\"><summary>{title_text}</summary>\n"
      ));
    } else {
      let title_class = if has_custom_title {
        "td-callout-title"
      } else {
        "td-callout-title td-callout-title-default"
      };
      self.write(&format!(
        "<div class=\"{name} td-callout\"><p class=\"{title_class}\">{title_text}</p>\n"
      ));
    }

    for child in node.children() {
      if child.kind() == SyntaxKind::MdContainerSlot {
        self.emit_slot_blocks(&child);
      }
    }

    if name == "details" {
      self.write("</details>\n");
    } else {
      self.write("</div>\n");
    }
  }

  fn emit_component_container(&mut self, node: &RedNode, label: &str) {
    let props = extract_container_props(node);
    let prop_str = if props.is_empty() {
      String::new()
    } else {
      format!(" {props}")
    };

    self.write(&format!("<{label}{prop_str}>\n"));

    let mut saw_separator = false;
    let mut current_slot_name: Option<String> = None;

    for child in node.children() {
      match child.kind() {
        SyntaxKind::MdContainerSlot => {
          if !saw_separator {
            self.emit_slot_blocks(&child);
          } else if current_slot_name.is_some() {
            self.emit_slot_blocks(&child);
            self.write("</template>\n");
            current_slot_name = None;
          }
        }
        SyntaxKind::MdContainerSlotSeparator => {
          saw_separator = true;
          let name = extract_slot_name(&child);
          if let Some(ref n) = name {
            self.write(&format!("<template #{n}>\n"));
            current_slot_name = name;
          }
        }
        _ => {}
      }
    }

    self.write(&format!("</{label}>\n"));
  }

  fn emit_container_shorthand(&mut self, node: &RedNode) {
    let mut label = String::new();
    let mut props = String::new();

    for child in node.children() {
      match child.kind() {
        SyntaxKind::Ident => label.push_str(&child.text()),
        SyntaxKind::MdSymbol if child.text() == "-" => label.push('-'),
        SyntaxKind::MdContainerPropBlock => {
          props = extract_props_from_block(&child);
        }
        _ => {}
      }
    }

    if props.is_empty() {
      self.write(&format!("<{label} />\n"));
    } else {
      self.write(&format!("<{label} {props} />\n"));
    }
  }

  fn emit_slot_blocks(&mut self, node: &RedNode) {
    for child in node.children() {
      let kind = child.kind();
      if kind == SyntaxKind::Whitespace || kind == SyntaxKind::Newline {
        continue;
      }
      self.emit_block(&child);
    }
  }

  // Inline emission

  // Emit inline children, skipping leading whitespace and heading markers
  fn emit_inline_children(&mut self, node: &RedNode) {
    let children: Vec<_> = collect_inline_children(node);
    // Skip leading whitespace
    let start = children
      .iter()
      .position(|c| {
        let kind = c.kind();
        kind != SyntaxKind::Whitespace && kind != SyntaxKind::MdSymbol
      })
      .unwrap_or(children.len());

    for child in &children[start..] {
      if child.kind() == SyntaxKind::Newline {
        continue;
      }
      self.emit_inline(child);
    }
  }

  fn emit_inline(&mut self, node: &RedNode) {
    match node.kind() {
      SyntaxKind::MdBold => self.emit_wrapped("strong", node),
      SyntaxKind::MdItalic => self.emit_wrapped("em", node),
      SyntaxKind::MdBoldItalic => {
        self.write("<strong><em>");
        self.emit_content_children(node);
        self.write("</em></strong>");
      }
      SyntaxKind::MdStrikethrough => self.emit_wrapped("s", node),
      SyntaxKind::MdLink => self.emit_link(node),
      SyntaxKind::MdMedia => self.emit_media(node),
      SyntaxKind::MdText => self.write_escaped(&node.text()),
      SyntaxKind::MdHtmlEntity => self.write(&node.text()),
      SyntaxKind::InlineCode => self.emit_inline_code(node),
      SyntaxKind::InlineMath => self.emit_inline_math(node),
      SyntaxKind::InterpFragment => self.emit_interp(node),
      SyntaxKind::CodeLit => self.emit_code_lit(node),
      SyntaxKind::MathLit => self.emit_math_lit(node),
      _ => {
        if node.as_token().is_some() {
          let text = node.text();
          if !is_delimiter(&text) {
            self.write_escaped(&text);
          }
        } else {
          for child in node.children() {
            self.emit_inline(&child);
          }
        }
      }
    }
  }

  fn emit_wrapped(&mut self, tag: &str, node: &RedNode) {
    self.write(&format!("<{tag}>"));
    self.emit_content_children(node);
    self.write(&format!("</{tag}>"));
  }

  // Emit children skipping delimiter tokens like ** * ~~ etc
  fn emit_content_children(&mut self, node: &RedNode) {
    for child in node.children() {
      if child.as_token().is_some() && is_delimiter(&child.text()) {
        continue;
      }
      self.emit_inline(&child);
    }
  }

  fn emit_link(&mut self, node: &RedNode) {
    let Some(link) = crate::syntax::ast::MdLink::cast(node.clone()) else {
      return;
    };
    let alt = link.alt().map(|t| t.value()).unwrap_or_default();
    let url = link.url().map(|t| t.value()).unwrap_or_default();

    if is_external_url(&url) {
      self.write("<LucideIcon name=\"arrow-up-right\" />");
      self.write(&format!(
        "<a href=\"{}\" class=\"td-external-link\" target=\"_blank\" rel=\"noreferrer\">",
        html_escape(&url)
      ));
    } else {
      self.write(&format!("<a href=\"{}\">", html_escape(&url)));
    }
    self.write_escaped(&alt);
    self.write("</a>");
  }

  fn emit_media(&mut self, node: &RedNode) {
    let Some(media) = crate::syntax::ast::MdMedia::cast(node.clone()) else {
      return;
    };
    let alt = media.alt().map(|t| t.value()).unwrap_or_default();
    let url = media.url().map(|t| t.value()).unwrap_or_default();

    self.write(&format!(
      "<img src=\"{}\" alt=\"{}\" loading=\"lazy\">",
      html_escape(&url),
      html_escape(&alt)
    ));
  }

  fn emit_inline_code(&mut self, node: &RedNode) {
    let Some(code) = InlineCode::cast(node.clone()) else {
      return;
    };
    let value = code.value().unwrap_or_default();
    self.write("<code>");
    self.write_escaped(&value);
    self.write("</code>");
  }

  fn emit_inline_math(&mut self, node: &RedNode) {
    let Some(math) = InlineMath::cast(node.clone()) else {
      return;
    };
    let value = math.value().unwrap_or_default();
    self.write("<span class=\"td-math-inline\">");
    self.write_escaped(&value);
    self.write("</span>");
  }

  fn emit_code_lit(&mut self, node: &RedNode) {
    for child in node.children() {
      match child.kind() {
        SyntaxKind::InlineCode => return self.emit_inline_code(&child),
        SyntaxKind::CodeBlock => return self.emit_code_block(&child),
        _ => {}
      }
    }
  }

  fn emit_math_lit(&mut self, node: &RedNode) {
    for child in node.children() {
      match child.kind() {
        SyntaxKind::InlineMath => return self.emit_inline_math(&child),
        SyntaxKind::MathBlock => return self.emit_math_block(&child),
        _ => {}
      }
    }
  }

  fn emit_interp(&mut self, node: &RedNode) {
    let Some(fragment) = InterpFragment::cast(node.clone()) else {
      return;
    };
    let Some(expr) = fragment.expr() else {
      return;
    };
    let expr_node = expr.syntax().clone();

    // Try fref resolution first
    if let Some(html) = self.try_resolve_fref_html(&expr_node) {
      self.write(&html);
      return;
    }

    // Fall back to expression evaluation
    let hir = lower_node(self.db, self.project, self.file, expr_node);
    let scope = get_file_runtime_scope(self.db, self.project, self.file);
    let eval = evaluate_node(self.db, hir, scope);
    if let Some(obj) = eval.value(self.db)
      && let Some(func) = obj.lookup_method(self.db, "to_string")
      && let Ok(result) = func.call(self.db, self.project, Some(obj), vec![])
      && let Some(str_obj) = result.as_td_str_obj()
    {
      self.write_escaped(&str_obj.value(self.db));
    }
  }

  fn try_resolve_fref_html(&self, node: &RedNode) -> Option<String> {
    let target = super::resolve_fref_target(self.db, self.project, self.file, node)?;

    if target.is_image {
      return Some(format!(
        "<img src=\"{}\" alt=\"{}\" loading=\"lazy\">",
        html_escape(&target.url),
        html_escape(&target.name)
      ));
    }

    let icon_html = target
      .icon
      .map(|name| format!("<LucideIcon name=\"{name}\" />"))
      .unwrap_or_default();

    Some(format!(
      "{icon_html}<a href=\"{}\">{}</a>",
      html_escape(&target.url),
      html_escape(&target.name)
    ))
  }

  // Utilities

  fn make_unique_slug(&mut self, text: &str) -> String {
    let base = slugify(text);
    let count = self.slug_counts.entry(base.clone()).or_insert(0);
    let slug = if *count == 0 {
      base.clone()
    } else {
      format!("{base}-{count}")
    };
    *count += 1;
    slug
  }
}

// HTML-specific helpers

fn is_callout(name: &str) -> bool {
  matches!(
    name,
    "tip" | "info" | "warning" | "danger" | "details" | "note" | "important" | "caution"
  )
}

fn callout_default_title(name: &str) -> &'static str {
  match name {
    "tip" => "TIP",
    "info" => "INFO",
    "warning" => "WARNING",
    "danger" => "DANGER",
    "details" => "Details",
    "note" => "NOTE",
    "important" => "IMPORTANT",
    "caution" => "CAUTION",
    _ => "",
  }
}

fn parse_separator_alignments(table_node: &RedNode) -> Vec<Option<&'static str>> {
  for child in table_node.children() {
    if child.kind() == SyntaxKind::MdTableSeparatorRow {
      let text = child.text();
      return text
        .split('|')
        .filter(|s| !s.trim().is_empty())
        .map(|cell| {
          let cell = cell.trim();
          let starts = cell.starts_with(':');
          let ends = cell.ends_with(':');
          match (starts, ends) {
            (true, true) => Some("center"),
            (true, false) => Some("left"),
            (false, true) => Some("right"),
            (false, false) => None,
          }
        })
        .collect();
    }
  }
  Vec::new()
}

fn extract_container_label_and_title(node: &RedNode) -> (String, Option<String>) {
  let mut label = String::new();
  let mut title_parts = Vec::new();
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
      let text = child.text();
      let trimmed = text.trim();
      if !trimmed.is_empty() {
        if label.is_empty() {
          label = trimmed.to_string();
        } else {
          title_parts.push(trimmed.to_string());
        }
      }
    }
  }

  let title = if title_parts.is_empty() {
    None
  } else {
    Some(title_parts.join(" "))
  };

  (label, title)
}

fn extract_container_props(node: &RedNode) -> String {
  for child in node.children() {
    if child.kind() == SyntaxKind::MdContainerPropBlock {
      return extract_props_from_block(&child);
    }
  }
  String::new()
}

fn extract_props_from_block(node: &RedNode) -> String {
  let mut props = Vec::new();
  for item in node.children() {
    if item.kind() == SyntaxKind::MdContainerPropItem {
      let text = item.text().trim().to_string();
      if !text.is_empty() {
        if let Some(eq) = text.find('=') {
          let key = &text[..eq];
          let value = strip_quotes(&text[eq + 1..]);
          props.push(format!("{key}=\"{}\"", html_escape(value)));
        } else {
          props.push(text);
        }
      }
    }
  }
  props.join(" ")
}

fn extract_slot_name(node: &RedNode) -> Option<String> {
  let text = node.text();
  let rest = text.trim().strip_prefix("===")?;
  let name = rest.trim();
  if name.is_empty() {
    None
  } else {
    Some(name.to_string())
  }
}

fn extract_ordered_start(node: &RedNode) -> usize {
  for child in node.children() {
    if child.kind() == SyntaxKind::MdOrderedListItem {
      for c in child.children() {
        if c.kind() == SyntaxKind::MdNumber {
          return c.text().parse().unwrap_or(1);
        }
      }
    }
  }
  1
}

#[cfg(test)]
mod tests {
  use super::*;

  // Colon position in separator cells determines alignment
  #[test]
  fn table_alignment_parsing() {
    let text = "| :--- | :---: | ---: | --- |";
    let aligns: Vec<Option<&str>> = text
      .split('|')
      .filter(|s| !s.trim().is_empty())
      .map(|cell| {
        let cell = cell.trim();
        let starts = cell.starts_with(':');
        let ends = cell.ends_with(':');
        match (starts, ends) {
          (true, true) => Some("center"),
          (true, false) => Some("left"),
          (false, true) => Some("right"),
          (false, false) => None,
        }
      })
      .collect();
    assert_eq!(
      aligns,
      vec![Some("left"), Some("center"), Some("right"), None]
    );
  }

  // Identical headings get -1, -2 suffixes
  #[test]
  fn slug_dedup() {
    let mut counts = HashMap::new();
    let make = |counts: &mut HashMap<String, usize>, text: &str| -> String {
      let base = slugify(text);
      let count = counts.entry(base.clone()).or_insert(0);
      let slug = if *count == 0 {
        base.clone()
      } else {
        format!("{base}-{count}")
      };
      *count += 1;
      slug
    };

    assert_eq!(make(&mut counts, "Foo"), "foo");
    assert_eq!(make(&mut counts, "Foo"), "foo-1");
    assert_eq!(make(&mut counts, "Foo"), "foo-2");
    assert_eq!(make(&mut counts, "Bar"), "bar");
  }
}
