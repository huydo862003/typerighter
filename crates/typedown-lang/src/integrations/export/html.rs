//! Emit HTML directly from the typedown AST, with placeholders for shiki and KaTeX

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::db::TypedownDatabase;
use crate::db::derived::evaluate::evaluate_node::evaluate_node;

use crate::db::derived::get_component_contract::get_component_contract;
use crate::db::derived::name_resolver::scope::get_file_runtime_scope;
use crate::db::types::{File, Project, ProjectRedNode, TdRuntimeObj};
use crate::integrations::export::utils::is_string_quoted;
use crate::syntax::ast::{
  AstNode, CodeBlock, InlineCode, InlineMath, InterpFragment, MathBlock, MdBody, MdBold,
  MdBoldItalic, MdHeading, MdInlineElement, MdItalic, MdLink, MdStrikethrough, MdTable,
  MdTableCell,
};
use crate::syntax::red::RedNode;
use crate::syntax::syntax_kind::SyntaxKind;

use super::utils::{
  extract_plain_text, html_escape, is_callout, resolve_vault_url, slugify, strip_quotes,
};
use crate::db::utils::is_external_url;
use crate::syntax::ast::MdContainerShorthand;

/// Heading extracted during HTML emission
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExportedHeading {
  pub level: u32,
  pub title: String,
  pub title_html: String,
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
  slug_counts: BTreeMap<String, usize>,
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
      slug_counts: BTreeMap::new(),
    }
  }

  fn resolve_url(&self, url: &str) -> String {
    let config = crate::db::derived::get_vault_config::get_vault_config(self.db, self.project);
    let handle = self.file.handle(self.db);
    let empty = PathBuf::new();
    let file_dir = handle
      .path()
      .unwrap_or(&empty)
      .parent()
      .and_then(|parent| parent.strip_prefix(config.root_dir(self.db)).ok())
      .unwrap_or(Path::new(""));

    resolve_vault_url(url, &config.base_path(self.db), file_dir)
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

    // Extract plain text for slug and title metadata
    let plain_text = {
      let mut text = String::new();
      for child in heading.inline_elements() {
        text.push_str(&extract_plain_text(child.syntax()));
      }
      text.trim().to_string()
    };
    let slug = self.make_unique_slug(&plain_text);

    if self.title.is_none() && level == 1 {
      self.title = Some(plain_text.clone());
    }

    let escaped_slug = html_escape(&slug);
    self.write(&format!("<h{level} id=\"{escaped_slug}\">"));
    let title_html_start = self.out.len();
    // Emit rich inline content (links, bold, etc) not just plain text
    for child in heading.inline_elements() {
      self.emit_inline(child.syntax());
    }
    let title_html = self.out[title_html_start..].to_string();

    self.headings.push(ExportedHeading {
      level: level as u32,
      title: plain_text.clone(),
      title_html,
      slug: slug.clone(),
    });
    self.write(&format!(
      " <a class=\"td-header-anchor\" href=\"#{escaped_slug}\" aria-label=\"Permalink to &quot;{}&quot;\">&#8203;</a>",
      html_escape(&plain_text)
    ));
    self.write(&format!("</h{level}>\n"));
  }

  fn emit_paragraph(&mut self, node: &RedNode) {
    self.write("<p>");
    self.emit_inline(node);
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
      .any(|child| child.kind() == SyntaxKind::MdTaskListItem);

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
    let start = extract_ordered_list_start_number(node);
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

    let has_block = node.children().any(|child| child.kind().is_md_block());

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
      self.emit_inline(node);
    }

    self.write("</li>\n");
  }

  fn emit_task_list_item(&mut self, node: &RedNode) {
    let checked = node
      .children()
      .any(|child| child.kind() == SyntaxKind::MdCheckbox && child.text().contains('x'));

    self.write("<li class=\"td-task-list-item\">");
    if checked {
      self.write("<input type=\"checkbox\" disabled checked> ");
    } else {
      self.write("<input type=\"checkbox\" disabled> ");
    }

    // Emit first paragraph in a <span> so it stays next to the checkbox
    // Block children (nested lists) go after the span
    self.write("<span>");
    let mut first_paragraph = true;
    let mut blocks: Vec<RedNode> = Vec::new();
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
        self.emit_inline(&child);
      } else if kind.is_md_block() {
        blocks.push(child);
      } else {
        self.emit_inline(&child);
      }
    }
    self.write("</span>");

    for block in blocks {
      self.write("\n");
      self.emit_block(&block);
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
    self.emit_inline(cell.syntax());
    self.write(&format!("</{tag}>\n"));
  }

  fn emit_code_block(&mut self, node: &RedNode) {
    let Some(block) = CodeBlock::cast(node.clone()) else {
      return;
    };
    let lang = block.language().unwrap_or_default();
    let label = block.label().unwrap_or_default();
    let value = block.value().unwrap_or_default();

    // Mermaid blocks use a separate placeholder for client-side rendering
    if lang == "mermaid" {
      self.write("<pre class=\"td-mermaid-placeholder\"><code>");
      self.write_escaped(&value);
      self.write("</code></pre>\n");
      return;
    }

    // D2 blocks use a separate placeholder for client-side rendering
    if lang == "d2" {
      self.write("<pre class=\"td-d2-placeholder\"><code>");
      self.write_escaped(&value);
      self.write("</code></pre>\n");
      return;
    }

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
    for (key, value) in block.props() {
      self.write(" data-");
      self.write_escaped(&key);
      self.write("=\"");
      self.write_escaped(&value.unwrap_or_default());
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

    let contract = self
      .project
      .custom_component_contract(self.db)
      .map(|file| get_component_contract(self.db, file));

    if is_callout(&label)
      && !contract.is_some_and(|result| {
        result
          .entries(self.db)
          .iter()
          .find(|entry| entry.name == label)
          .is_some()
      })
    {
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
    let prop_string = if props.is_empty() {
      String::new()
    } else {
      format!(" {props}")
    };

    self.write(&format!("<{label}{prop_string}>\n"));

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

    // Close any trailing open slot template (empty named slot)
    if current_slot_name.is_some() {
      self.write("</template>\n");
    }

    self.write(&format!("</{label}>\n"));
  }

  fn emit_container_shorthand(&mut self, node: &RedNode) {
    let Some(shorthand) = MdContainerShorthand::cast(node.clone()) else {
      return;
    };
    let Some(label) = shorthand.label() else {
      return;
    };
    let props = extract_container_props(node);

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

  fn emit_inline(&mut self, node: &RedNode) {
    match node.kind() {
      SyntaxKind::MdBold => self.emit_wrapped(
        "strong",
        MdBold::cast(node.clone())
          .unwrap()
          .inline_elements()
          .map(|element| element.syntax().clone()),
      ),
      SyntaxKind::MdItalic => self.emit_wrapped(
        "em",
        MdItalic::cast(node.clone())
          .unwrap()
          .inline_elements()
          .map(|element| element.syntax().clone()),
      ),
      SyntaxKind::MdBoldItalic => {
        self.write("<strong><em>");
        MdBoldItalic::cast(node.clone())
          .unwrap()
          .inline_elements()
          .for_each(|element| self.emit_inline(element.syntax()));
        self.write("</em></strong>");
      }
      SyntaxKind::MdStrikethrough => self.emit_wrapped(
        "s",
        MdStrikethrough::cast(node.clone())
          .unwrap()
          .inline_elements()
          .map(|element| element.syntax().clone()),
      ),
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
          self.write_escaped(&text);
        } else {
          for child in node.children() {
            self.emit_inline(&child);
          }
        }
      }
    }
  }

  fn emit_wrapped(&mut self, tag: &str, children: impl Iterator<Item = RedNode>) {
    self.write(&format!("<{tag}>"));
    children.for_each(|child| self.emit_inline(&child));
    self.write(&format!("</{tag}>"));
  }

  fn emit_link(&mut self, node: &RedNode) {
    let Some(link) = MdLink::cast(node.clone()) else {
      return;
    };
    let raw_url = link.url().map(|text| text.value()).unwrap_or_default();
    let url = self.resolve_url(&raw_url);

    // Check for an embedded image (linked image / badge syntax)
    let has_media = node
      .children()
      .any(|child| child.kind() == SyntaxKind::MdMedia);

    if is_external_url(&url) && !url.starts_with("mailto:") && !has_media {
      self.write("<LucideIcon name=\"arrow-up-right\" class=\"td-external-link-icon\" />");
      self.write(&format!(
        "<a href=\"{}\" class=\"td-external-link\" target=\"_blank\" rel=\"noopener noreferrer\">",
        html_escape(&url)
      ));
    } else if is_external_url(&url) {
      self.write(&format!(
        "<a href=\"{}\" target=\"_blank\" rel=\"noopener noreferrer\">",
        html_escape(&url)
      ));
    } else {
      self.write(&format!("<a href=\"{}\">", html_escape(&url)));
    }

    if has_media {
      for child in node.children() {
        if child.kind() == SyntaxKind::MdMedia {
          self.emit_media(&child);
        }
      }
    } else {
      // Emit inline children between [ and ] (link text, not URL)
      let mut in_text = false;
      for child in node.children() {
        if child.kind() == SyntaxKind::LBracket {
          in_text = true;
          continue;
        }
        if child.kind() == SyntaxKind::RBracket {
          break;
        }
        if in_text && MdInlineElement::cast(child.clone()).is_some() {
          self.emit_inline(&child);
        }
      }
    }
    self.write("</a>");
  }

  fn emit_media(&mut self, node: &RedNode) {
    let Some(media) = crate::syntax::ast::MdMedia::cast(node.clone()) else {
      return;
    };
    let alt = media.alt().map(|text| text.value()).unwrap_or_default();
    let raw_url = media.url().map(|text| text.value()).unwrap_or_default();
    let url = self.resolve_url(&raw_url);

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
    let project_node = ProjectRedNode::new(self.project, self.file, expr_node);
    let scope = get_file_runtime_scope(self.db, self.project, self.file);
    let evaluated_result = evaluate_node(self.db, project_node, scope);
    if let Some(obj) = evaluated_result.value(self.db) {
      self.write_escaped(&obj.to_display_string(self.db));
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
      .map(|name| format!("<LucideIcon name=\"{name}\" class=\"td-inline-icon\" />"))
      .unwrap_or_default();

    Some(format!(
      "<a href=\"{}\" class=\"td-fref-link\">{icon_html}{}</a>",
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
        .filter(|string| !string.trim().is_empty())
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

// Extract the container label (possibly kebab like `td-alert`) and optional title text
// (for callouts, e.g. `:::warning My title`) in a single pass
fn extract_container_label_and_title(node: &RedNode) -> (String, Option<String>) {
  let mut label = String::new();
  let mut title_raw = String::new();
  let mut seen_opening = false;
  // Once the label is complete we switch to collecting title text
  let mut label_done = false;

  for child in node.children() {
    if child.kind() == SyntaxKind::MdSymbol && child.text() == ":::" && !seen_opening {
      seen_opening = true;
      continue;
    }
    if !seen_opening {
      continue;
    }
    match child.kind() {
      SyntaxKind::Newline => break,
      SyntaxKind::MdContainerPropBlock => break,
      SyntaxKind::Whitespace => {
        // Whitespace before the label: skip; after: label is done, append to title
        if !label.is_empty() {
          label_done = true;
          title_raw.push_str(&child.text());
        }
      }
      SyntaxKind::Ident if !label_done => {
        label.push_str(&child.text());
      }
      SyntaxKind::MdSymbol if !label_done && child.text() == "-" && !label.is_empty() => {
        label.push('-');
      }
      _ => {
        if !label.is_empty() {
          label_done = true;
          title_raw.push_str(&child.text());
        }
      }
    }
  }

  if label.is_empty() {
    return (String::new(), None);
  }

  let title = {
    let trimmed = title_raw.trim();
    if trimmed.is_empty() {
      None
    } else {
      Some(trimmed.to_string())
    }
  };

  (label, title)
}

fn extract_container_props(node: &RedNode) -> String {
  for child in node.children() {
    if child.kind() == SyntaxKind::MdContainerPropBlock {
      let mut props = Vec::new();
      for item in child.children() {
        if item.kind() != SyntaxKind::MdContainerPropItem {
          continue;
        }
        let text = item.text().trim().to_string();
        if text.is_empty() {
          continue;
        }
        let Some(eq) = text.find('=') else {
          props.push(text);
          continue;
        };
        let key = &text[..eq];

        let raw_value = &text[eq + 1..];
        let value = strip_quotes(raw_value);

        if is_string_quoted(raw_value) {
          // Quoted: pass as string attribute
          props.push(format!("{key}=\"{}\"", html_escape(value)));
        } else {
          // Unquoted: use v-bind so Vue preserves the JS type
          props.push(format!(":{key}=\"{}\"", html_escape(value)));
        }
      }
      return props.join(" ");
    }
  }
  String::new()
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

fn extract_ordered_list_start_number(node: &RedNode) -> usize {
  for child in node.children() {
    if child.kind() == SyntaxKind::MdOrderedListItem {
      for child in child.children() {
        if child.kind() == SyntaxKind::MdNumber {
          return child.text().parse().unwrap_or(1);
        }
      }
    }
  }
  1
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::syntax::green::cache::Cache;
  use crate::syntax::parse::ctx::{ParseCtx, ParseResult};
  use std::cell::RefCell;
  use std::rc::Rc;
  use typedown_types::string_stream::StringStream;

  // Colon position in separator cells determines alignment
  #[test]
  fn table_alignment_parsing() {
    let text = "| :--- | :---: | ---: | --- |";
    let aligns: Vec<Option<&str>> = text
      .split('|')
      .filter(|string| !string.trim().is_empty())
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
    let mut counts = BTreeMap::new();
    let make = |counts: &mut BTreeMap<String, usize>, text: &str| -> String {
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

  fn parse_body(input: &str) -> RedNode {
    let full = format!("---\n---\n{}", input);
    let stream = StringStream::new(&full);
    let cache = Rc::new(RefCell::new(Cache::new()));
    let ctx = ParseCtx::new(stream, cache);
    let ParseResult { ast, .. } = ctx.parse();
    RedNode::from_green(0, ast)
  }

  fn find_first(node: &RedNode, kind: SyntaxKind) -> Option<RedNode> {
    if node.kind() == kind {
      return Some(node.clone());
    }
    for child in node.children() {
      if let Some(found) = find_first(&child, kind) {
        return Some(found);
      }
    }
    None
  }

  fn export_props(input: &str) -> String {
    let root = parse_body(input);
    let block = find_first(&root, SyntaxKind::MdContainerBlock).unwrap();
    extract_container_props(&block)
  }

  #[test]
  fn export_bare_ident_as_vbind() {
    assert_eq!(
      export_props(
        r#"
:::flashcard{revealed=true}
:::
"#
      ),
      r#":revealed="true""#
    );
  }

  #[test]
  fn export_number_as_vbind() {
    assert_eq!(
      export_props(
        r#"
:::chart{count=42}
:::
"#
      ),
      r#":count="42""#
    );
  }

  #[test]
  fn export_quoted_string_as_attribute() {
    assert_eq!(
      export_props(
        r#"
:::card{title="hello"}
:::
"#
      ),
      r#"title="hello""#
    );
  }

  #[test]
  fn export_array_as_vbind() {
    assert_eq!(
      export_props(
        r#"
:::chart{data=[1, 2, 3]}
:::
"#
      ),
      r#":data="[1, 2, 3]""#
    );
  }

  #[test]
  fn export_nested_array_as_vbind() {
    assert_eq!(
      export_props(
        r#"
:::chart{matrix=[[1, 2], [3, 4]]}
:::
"#
      ),
      r#":matrix="[[1, 2], [3, 4]]""#
    );
  }

  #[test]
  fn export_negative_number_as_vbind() {
    assert_eq!(
      export_props(
        r#"
:::widget{offset=-1}
:::
"#
      ),
      r#":offset="-1""#
    );
  }

  #[test]
  fn export_dict_as_vbind() {
    // Inner quotes become &quot; via HTML attribute escaping
    // Vue decodes them back to " before evaluating the v-bind expression
    assert_eq!(
      export_props(
        r#"
:::chart{config={color: "red", size: 10}}
:::
"#,
      ),
      r#":config="{color: &quot;red&quot;, size: 10}""#
    );
  }

  #[test]
  fn export_mixed_props() {
    assert_eq!(
      export_props(
        r#"
:::comp{a=true b="hello" c=[1, 2] d=42}
:::
"#
      ),
      r#":a="true" b="hello" :c="[1, 2]" :d="42""#
    );
  }

  #[test]
  fn export_flag_prop() {
    assert_eq!(
      export_props(
        r#"
:::flashcard{disabled}
:::
"#
      ),
      "disabled"
    );
  }
}
