//! Export resource content as markdown or HTML (with Vue component support)

pub mod html;
pub mod markdown;
pub mod media;

use crate::db::TypedownDatabase;
use crate::db::types::derived::obj_system::TdStaticTyp;
use crate::db::types::{File, Project, TdBlobTyp};
use crate::integrations::export::frontmatter::{ResourceKind, extract_resource};
use crate::integrations::export::types::*;

/// Export a resource file as structured header and CommonMark body
pub fn export_resource_markdown(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<ExportedResourceMarkdown> {
  match extract_resource(db, project, file)? {
    ResourceKind::Blob(blob) => Some(ExportedResourceMarkdown {
      schema: Some(TdBlobTyp::get(db).display_name(db)),
      label: None,
      icon: None,
      header: blob.header,
      content: String::new(),
      metadata: blob.metadata,
    }),
    ResourceKind::Content(fields) => {
      let content = markdown::export_markdown_body(db, project, file, &fields.body);
      Some(ExportedResourceMarkdown {
        schema: fields.schema,
        label: fields.label,
        icon: fields.icon,
        header: fields.header,
        content,
        metadata: fields.metadata,
      })
    }
  }
}

/// Export a resource file as structured header and HTML body (with Vue components).
pub fn export_resource_html(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<ExportedResourceHtml> {
  match extract_resource(db, project, file)? {
    ResourceKind::Blob(blob) => Some(ExportedResourceHtml {
      schema: Some(TdBlobTyp::get(db).display_name(db)),
      label: None,
      icon: None,
      header: blob.header,
      content: String::new(),
      headings: Vec::new(),
      title: None,
      metadata: blob.metadata,
      seo_metadata: None,
    }),
    ResourceKind::Content(fields) => {
      let content = html::export_html_body(db, project, file, &fields.body);
      Some(ExportedResourceHtml {
        schema: fields.schema,
        label: fields.label,
        icon: fields.icon,
        header: fields.header,
        content: content.html,
        headings: content.headings,
        title: content.title,
        metadata: fields.metadata,
        seo_metadata: fields.seo_metadata,
      })
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::fixtures::load_vault_fixture;

  #[test]
  fn exports_content_body() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let result = export_resource_markdown(&db, project, file);
    let exported = result.expect("should export");
    assert!(
      exported.content.contains("Hello world"),
      "content should contain body text: {}",
      exported.content
    );
  }

  #[test]
  fn exports_markdown_body() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert_eq!(
      exported.content,
      r"Hello world
"
    );
  }

  #[test]
  fn exports_all_markdown_elements() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(content.contains("# Heading 1"), "should contain h1");
    assert!(content.contains("## Heading 2"), "should contain h2");
    assert!(content.contains("**bold**"), "should contain bold");
    assert!(content.contains("- bullet one"), "should contain bullet");
    assert!(content.contains("[link text]"), "should contain link");
    assert!(
      content.contains("```js{1,3}"),
      "should preserve code block range indicator: {content}",
    );
  }

  #[test]
  fn exports_container_shorthand() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_container_shorthand.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains(
        r"::: toc
:::
"
      ),
      "should expand [[toc]] to empty container: {content}",
    );
    assert!(
      content.contains(
        r"::: grid {cols=2}
:::
"
      ),
      "should expand [[grid {{cols=2}}]] to container with props: {content}",
    );
    assert!(
      content.contains(
        r"::: directory-index
:::
"
      ),
      "should expand [[directory-index]] with kebab-case: {content}",
    );
  }

  #[test]
  fn exports_unresolved_interpolation_as_empty() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_unresolved_interp.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      !content.contains("question"),
      "unresolved interpolation should not appear in output: {content}",
    );
    assert!(
      content.contains(
        r"::: info
"
      ),
      "container should still be present: {content}",
    );
  }

  #[test]
  fn exports_nested_blocks_without_extra_indentation() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_nested_blocks.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let content = &exported.content;

    for line in content.lines() {
      let indent = line.len() - line.trim_start().len();
      assert!(
        indent < 4 || line.trim_start().is_empty(),
        r"line has {indent} spaces of indentation (would become code block): '{line}'
full content:
{content}",
      );
    }

    assert!(
      content.contains(
        r"nested paragraph
"
      ),
      "nested paragraph should not have leading whitespace: {content}",
    );
    assert!(
      content.contains(
        r"> blockquote content
"
      ),
      "blockquote should use > prefix: {content}",
    );
    assert!(
      content.contains(
        r"- item one
"
      ),
      "list item should use - marker: {content}",
    );
  }

  #[test]
  fn exports_container_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("::: details Click to expand"),
      "should contain container with title: {}",
      exported.content,
    );
  }

  #[test]
  fn fref_uses_base_path() {
    let (db, project, file) = load_vault_fixture("evaluate/base_path_vault", "with_fref.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("[Alice](/blog/alice)"),
      "fref should use base_path /blog: {}",
      exported.content
    );
  }

  #[test]
  fn fref_resolves_image_asset_to_markdown_image() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_asset_fref.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("![icon](/icon.svg)"),
      "image asset fref should produce markdown image: {}",
      exported.content
    );
  }

  #[test]
  fn fref_default_base_path() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(
      !exported.content.contains("/blog"),
      "default base_path should not have /blog prefix: {}",
      exported.content
    );
  }

  #[test]
  fn export_separates_blocks_with_blank_lines() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let lines: Vec<&str> = exported.content.lines().collect();
    let heading_indices: Vec<usize> = lines
      .iter()
      .enumerate()
      .filter(|(_, line)| line.starts_with('#'))
      .map(|(index, _)| index)
      .collect();
    for &index in &heading_indices {
      if index > 0 {
        assert_eq!(
          lines[index - 1],
          "",
          r"heading at line {index} should be preceded by blank line:
{}",
          exported.content
        );
      }
    }
  }

  #[test]
  fn markdown_export_body_only_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "body_only.td");
    let exported =
      export_resource_markdown(&db, project, file).expect("body-only file should export");
    assert!(
      exported
        .content
        .contains("This file has no frontmatter at all.")
    );
    assert!(exported.schema.is_none());
  }

  #[test]
  fn html_export_simple_paragraph() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert_eq!(
      exported.content,
      r"<p>Hello world</p>
"
    );
  }

  #[test]
  fn html_export_headings() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    eprintln!("{:?}", exported.content);
    assert!(
      exported.content.contains(
        r##"<h1 id="heading-1">Heading 1 <a class="td-header-anchor" href="#heading-1""##
      )
    );
    assert!(
      exported.content.contains(
        r##"<h2 id="heading-2">Heading 2 <a class="td-header-anchor" href="#heading-2""##
      )
    );
    assert_eq!(exported.headings.len(), 2);
    assert_eq!(exported.headings[0].level, 1);
    assert_eq!(exported.headings[0].title, "Heading 1");
    assert_eq!(exported.headings[0].slug, "heading-1");
    assert_eq!(exported.headings[1].level, 2);
    assert_eq!(exported.headings[1].slug, "heading-2");
    assert_eq!(exported.title.as_deref(), Some("Heading 1"));
  }

  #[test]
  fn html_export_heading_titles_preserve_special_chars() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_special_headings.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let titles: Vec<&str> = exported
      .headings
      .iter()
      .map(|heading| heading.title.as_str())
      .collect();
    assert!(
      titles.contains(&"Version v1.2.3"),
      "dots preserved: {titles:?}"
    );
    assert!(
      titles.contains(&"Key: Value Pairs"),
      "colon preserved: {titles:?}"
    );
    assert!(
      titles.contains(&"C++ and C#"),
      "plus and hash preserved: {titles:?}"
    );
    assert!(
      titles.contains(&"node.js vs Deno"),
      "dots preserved: {titles:?}"
    );
  }

  #[test]
  fn html_export_heading_with_inline_markup() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_special_headings.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains(r#"href="https://example.com""#),
      r"heading should contain rendered link:
{}",
      exported.content
    );
    assert!(
      exported.content.contains("`def1`"),
      r"heading should preserve inline code in link:
{}",
      exported.content
    );
    assert!(
      exported.content.contains("<strong>bold heading</strong>"),
      r"heading should contain rendered bold:
{}",
      exported.content
    );
  }

  #[test]
  fn html_export_inline_markup() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<strong>bold</strong>"));
    assert!(exported.content.contains("<em>italic</em>"));
    assert!(
      exported
        .content
        .contains("<strong><em>bold italic</em></strong>")
    );
    assert!(exported.content.contains("<s>strikethrough</s>"));
  }

  #[test]
  fn html_export_inline_code() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<code>inline code</code>"));
  }

  #[test]
  fn html_export_code_block_placeholder() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r#"<pre class="td-code-placeholder" data-lang="python" data-meta="python"><code>print(&quot;hello&quot;)"#
    ));
    assert!(exported.content.contains(
      r#"<pre class="td-code-placeholder" data-lang="js" data-meta="js{1,3}"><code>const x = 1;"#
    ));
  }

  #[test]
  fn html_export_code_block_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"data-title="config.ts""#));
  }

  #[test]
  fn html_export_code_block_with_line_numbers() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"data-line-numbers="5""#));
  }

  #[test]
  fn html_export_inline_math_placeholder() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<span class="td-math-inline">E = mc^2</span>"#)
    );
  }

  #[test]
  fn html_export_math_block_placeholder() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<div class="td-math-block">\int_0^\infty e^{-x^2} dx"#)
    );
  }

  #[test]
  fn html_export_blockquote() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r"<blockquote>
<p>blockquote line</p>
</blockquote>"
    ));
  }

  #[test]
  fn html_export_table() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"<table tabindex="0">"#));
    assert!(exported.content.contains("<th>"));
    assert!(exported.content.contains("Col1"));
    assert!(exported.content.contains("Col2"));
    assert!(exported.content.contains("<td>"));
    let th_count = exported.content.matches("<th>").count();
    let td_count = exported.content.matches("<td>").count();
    assert_eq!(th_count, 2, "should have 2 header cells");
    assert_eq!(td_count, 2, "should have 2 data cells");
  }

  #[test]
  fn html_export_table_inline_code() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_table_inline_code.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("<code>_type</code>"),
      "inline code _type should be preserved in table cell: {}",
      exported.content
    );
    assert!(
      exported.content.contains("<code>_label</code>"),
      "inline code _label should be preserved: {}",
      exported.content
    );
    assert!(
      exported.content.contains("<code>_content</code>"),
      "inline code _content should be preserved: {}",
      exported.content
    );
    assert!(
      exported.content.contains("<code>icon.rocket</code>"),
      "inline code icon.rocket should be preserved: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_bullet_list() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r"<ul>
<li>"
    ));
    assert!(exported.content.contains("bullet one"));
    assert!(exported.content.contains("bullet two"));
  }

  #[test]
  fn html_export_ordered_list() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r"<ol>
<li>"
    ));
    assert!(exported.content.contains("ordered one"));
  }

  #[test]
  fn html_export_callout_container() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r#"<div class="note td-callout"><p class="td-callout-title td-callout-title-default">NOTE</p>"#
    ));
    assert!(exported.content.contains("<p>callout content</p>"));
  }

  #[test]
  fn html_export_code_block_in_callout() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<pre class="td-code-placeholder" data-lang="python" data-meta="python"><code>print(&quot;in callout&quot;)"#)
    );
  }

  #[test]
  fn html_export_details_container() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<details class="details td-callout"><summary>Click to expand</summary>"#)
    );
    assert!(exported.content.contains("<p>details content</p>"));
  }

  #[test]
  fn html_export_external_link() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r#"<LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://example.com" class="td-external-link" target="_blank" rel="noopener noreferrer">link text</a>"#
    ));
  }

  #[test]
  fn html_export_code_in_link_text() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("<code>code</code> in link</a>"),
      "inline code in link should render as <code>code</code> inside <a>: {}",
      exported.content
    );
    assert!(
      !exported.content.contains("`code` in link"),
      "backticks should not appear as raw text in rendered link: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_container_shorthand() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_container_shorthand.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(
      r"<toc />
"
    ));
    assert!(exported.content.contains(
      r"<directory-index />
"
    ));
    assert!(exported.content.contains(
      r#"<grid :cols="2" />
"#
    ));
  }

  #[test]
  fn html_export_container_empty_named_slot() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_container_empty_slot.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("<template #key>"),
      "should have named slot: {content}"
    );
    assert!(
      content.contains("</template>"),
      "named slot template should be closed: {content}"
    );
    assert!(
      content.contains("</flashcard>"),
      "container should be closed: {content}"
    );
  }

  #[test]
  fn html_export_blob_empty() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "icon.svg");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert_eq!(exported.schema, Some("blob".to_string()));
    assert!(exported.content.is_empty());
    assert!(exported.headings.is_empty());
    assert!(exported.title.is_none());
  }

  #[test]
  fn html_export_schema_and_header() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert_eq!(exported.schema, Some("Person".to_string()));
    // Header now uses $kind-tagged bindings format
    let fields = &exported.header["fields"];
    assert_eq!(fields["name"], "Alice");
    assert_eq!(fields["age"], 30.0);
  }

  #[test]
  fn html_export_fref_link() {
    let (db, project, file) = load_vault_fixture("evaluate/base_path_vault", "with_fref.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<a href="/blog/alice" class="td-fref-link">Alice</a>"#)
    );
  }

  #[test]
  fn html_export_fref_image() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_asset_fref.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<img src="/icon.svg" alt="icon" loading="lazy">"#)
    );
  }

  #[test]
  fn html_export_relative_fref_image() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "subdir/relative_fref.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains("<img src=") && exported.content.contains("test.svg"),
      "relative fref should render as img tag: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_no_raw_markdown() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(!exported.content.contains(
      r"
> "
    ));
    assert!(!exported.content.contains(
      r"
- "
    ));
    assert!(!exported.content.contains(
      r"
1. "
    ));
    assert!(!exported.content.contains("**"));
    assert!(!exported.content.contains("~~"));
  }

  #[test]
  fn html_export_heading_slugs_unique() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let slugs: Vec<&str> = exported
      .headings
      .iter()
      .map(|heading| heading.slug.as_str())
      .collect();
    let mut deduped = slugs.clone();
    deduped.sort();
    deduped.dedup();
    assert_eq!(
      slugs.len(),
      deduped.len(),
      "slugs should be unique: {:?}",
      slugs
    );
  }

  #[test]
  fn html_export_body_only_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "body_only.td");
    let exported = export_resource_html(&db, project, file).expect("body-only file should export");
    assert!(
      exported
        .content
        .contains("<p>This file has no frontmatter at all.</p>")
    );
    assert!(exported.content.contains("<h2"));
    assert!(exported.schema.is_none());
    assert!(exported.label.is_none());
    // Body-only files produce an empty product with bindings format
    assert!(exported.header.is_object(), "header should be object");
  }

  #[test]
  fn html_linked_image_badge() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "linked_image.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains(r#"<a href="mailto:huydo862003@gmail.com""#),
      "should contain mailto link: {content}",
    );
    assert!(
      content.contains(
        r#"<img src="https://img.shields.io/badge/Email-blue" alt="Email badge" loading="lazy">"#
      ),
      "should contain image inside link: {content}",
    );
    assert!(
      !content.contains("LucideIcon"),
      "mailto links should not have arrow icon: {content}",
    );
  }

  #[test]
  fn html_export_body_only_nested_vault() {
    let (db, project, file) = load_vault_fixture("evaluate/nested_vault", "vault/body_only.td");
    let exported = export_resource_html(&db, project, file);
    assert!(
      exported.is_some(),
      "body-only file in nested vault should export"
    );
    let exported = exported.unwrap();
    assert!(
      exported
        .content
        .contains("<p>This file has no frontmatter.</p>")
    );
  }

  #[test]
  fn html_export_preserves_external_url() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "url_resolution.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains(r#"href="https://example.com""#),
      "external URL should pass through unchanged: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_preserves_anchor_url() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "url_resolution.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains(r##"href="#section""##),
      "anchor URL should pass through unchanged: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_resolves_relative_link() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "url_resolution.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported.content.contains(r#"href="/other.td""#),
      "relative link should resolve with base path: {}",
      exported.content
    );
  }

  #[test]
  fn html_export_media_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "media_with_title.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("<figure>"),
      "image with title should be wrapped in <figure>: {content}",
    );
    assert!(
      content.contains(r#"title="The Typedown project logo""#),
      "image should have title attribute: {content}",
    );
    assert!(
      content.contains("<figcaption>The Typedown project logo</figcaption>"),
      "image should have figcaption: {content}",
    );
  }

  #[test]
  fn html_export_link_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "link_with_title.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains(r#"title="Visit Example""#),
      "link should have title attribute: {content}",
    );
    assert!(
      content.contains("href=\"https://example.com"),
      "link should have href: {content}",
    );
  }

  #[test]
  fn html_export_github_file_embed() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "github_embed.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("class=\"td-github-placeholder\""),
      "github file URL should emit placeholder: {content}",
    );
    assert!(
      content.contains("data-url=\"https://github.com/vuejs/vite/blob/main/packages/vite/src/node/server/index.ts#L150-L165"),
      "placeholder should have data-url: {content}",
    );
    assert!(
      content.contains("data-label=\"server init\""),
      "placeholder should have data-label from alt text: {content}",
    );
    assert!(
      content.contains("data-title=\"Vite dev server\""),
      "placeholder should have data-title: {content}",
    );
  }

  #[test]
  fn html_export_github_gist_embed() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "github_embed.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("data-url=\"https://gist.github.com/octocat/6cad326836d38bd3a7ae\""),
      "gist URL should emit placeholder with data-url: {content}",
    );
  }

  #[test]
  fn html_export_nested_bullet_list() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "nested_list.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert_eq!(
      exported.content,
      r##"<ul>
<li>
<p><strong><LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://github.com/huydo862003/typerighter" class="td-external-link" target="_blank" rel="noopener noreferrer">typerighter</a></strong>: A typed content framework.</p>
<ul>
<li>
<p>Somewhat stabilized with lots of optimizations!</p>
</li>
<li>
<p><code>npm</code> package: <LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://www.npmjs.com/package/typerighter" class="td-external-link" target="_blank" rel="noopener noreferrer">npm</a></p>
</li>
<li>
<p>documentation: <LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://huydo862003.github.io/typerighter" class="td-external-link" target="_blank" rel="noopener noreferrer">docs</a></p>
</li>
</ul>
</li>
<li>
<p><strong><LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://github.com/huydo862003/sqlingo" class="td-external-link" target="_blank" rel="noopener noreferrer">sqlingo</a></strong>: Porting sqlglot to TypeScript.</p>
<ul>
<li>
<p>Stabilized &amp; Synced with sqlglot version 30.0.0.</p>
</li>
<li>
<p><code>npm</code> package: <LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://www.npmjs.com/package/sqlingo" class="td-external-link" target="_blank" rel="noopener noreferrer">npm</a></p>
</li>
<li>
<p>documentation: <LucideIcon name="arrow-up-right" class="td-external-link-icon" /><a href="https://huydo862003.github.io/sqlingo/" class="td-external-link" target="_blank" rel="noopener noreferrer">docs</a></p>
</li>
</ul>
</li>
</ul>
"##
    );
  }

  #[test]
  fn html_export_nested_container_indented() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_nested_container.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("<grid"),
      "should contain outer grid component: {content}"
    );
    assert!(
      content.contains("<grid-cell>"),
      "should contain inner grid-cell component: {content}"
    );
    assert!(
      content.contains("</grid-cell>"),
      "grid-cell should be closed: {content}"
    );
    assert!(
      content.contains("<p>Alpha</p>"),
      "first cell content should render: {content}"
    );
    assert!(
      content.contains("<p>Beta</p>"),
      "second cell content should render: {content}"
    );
  }

  #[test]
  fn html_export_nested_container_not_indented() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "md_nested_container_flat.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains("<grid-cell>"),
      "should contain grid-cell: {content}"
    );
    assert!(
      content.contains("<p>Alpha</p>"),
      "first cell content should render: {content}"
    );
  }

  #[test]
  fn html_export_nested_container_with_props() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "md_nested_container_props.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(
      content.contains(r#":columns="3""#),
      "outer grid should have columns prop: {content}"
    );
    assert!(
      content.contains(r#":col-span="2""#),
      "inner grid-cell should have col-span prop: {content}"
    );
  }
}
