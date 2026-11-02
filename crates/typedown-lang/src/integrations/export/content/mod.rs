//! Export resource content as markdown or HTML (with Vue component support)

pub mod html;
pub mod markdown;
pub mod media;

use crate::db::TypedownDatabase;
use crate::db::types::derived::obj_system::TdStaticTyp;
use crate::db::types::{File, Project, TdBlobTyp};
use crate::integrations::export::frontmatter::{ResourceKind, extract_resource};
use crate::integrations::export::types::*;

/// Export a resource file as structured frontmatter and CommonMark body
pub fn export_resource_markdown(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<ExportedResourceMarkdown> {
  match extract_resource(db, project, file)? {
    ResourceKind::Blob(blob) => Some(ExportedResourceMarkdown {
      schema: Some(TdBlobTyp::get(db).display_name(db)),
      meta: blob.meta,
      frontmatter: blob.frontmatter,
      content: String::new(),
    }),
    ResourceKind::Content(fields) => {
      let content = markdown::export_markdown_body(db, project, file, &fields.body);
      Some(ExportedResourceMarkdown {
        schema: fields.schema,
        meta: fields.meta,
        frontmatter: fields.frontmatter,
        content,
      })
    }
  }
}

/// Export a resource file as structured frontmatter and HTML body (with Vue components)
pub fn export_resource_html(
  db: &TypedownDatabase,
  project: Project,
  file: File,
) -> Option<ExportedResourceHtml> {
  match extract_resource(db, project, file)? {
    ResourceKind::Blob(blob) => Some(ExportedResourceHtml {
      schema: Some(TdBlobTyp::get(db).display_name(db)),
      meta: blob.meta,
      frontmatter: blob.frontmatter,
      content: String::new(),
      headings: Vec::new(),
      title: None,
    }),
    ResourceKind::Content(fields) => {
      let content = html::export_html_body(db, project, file, &fields.body);
      Some(ExportedResourceHtml {
        schema: fields.schema,
        meta: fields.meta,
        frontmatter: fields.frontmatter,
        content: content.html,
        headings: content.headings,
        title: content.title,
      })
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::db::fixtures::load_vault_fixture;
  use crate::integrations::bindings::types::TdValue;

  fn to_json(value: &TdValue) -> serde_json::Value {
    serde_json::to_value(value).unwrap()
  }

  // Markdown content tests

  #[test]
  fn exports_content_body() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(exported.content.contains("Hello world"));
  }

  #[test]
  fn exports_markdown_body() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert_eq!(exported.content, "Hello world\n");
  }

  #[test]
  fn exports_all_markdown_elements() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(content.contains("# Heading 1"));
    assert!(content.contains("## Heading 2"));
    assert!(content.contains("**bold**"));
    assert!(content.contains("- bullet one"));
    assert!(content.contains("[link text]"));
    assert!(content.contains("```js{1,3}"));
  }

  #[test]
  fn exports_container_shorthand() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_container_shorthand.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let content = &exported.content;
    assert!(content.contains("::: toc\n:::\n"));
    assert!(content.contains("::: grid {cols=2}\n:::\n"));
    assert!(content.contains("::: directory-index\n:::\n"));
  }

  #[test]
  fn exports_unresolved_interpolation_as_empty() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_unresolved_interp.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(!exported.content.contains("question"));
    assert!(exported.content.contains("::: info\n"));
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
        "line has {indent} spaces of indentation: '{line}'"
      );
    }
    assert!(content.contains("nested paragraph\n"));
    assert!(content.contains("> blockquote content\n"));
    assert!(content.contains("- item one\n"));
  }

  #[test]
  fn exports_container_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(exported.content.contains("::: details Click to expand"));
  }

  #[test]
  fn fref_uses_base_path() {
    let (db, project, file) = load_vault_fixture("evaluate/base_path_vault", "with_fref.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(exported.content.contains("[Alice](/blog/alice)"));
  }

  #[test]
  fn fref_resolves_image_asset() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_asset_fref.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(exported.content.contains("![icon](/icon.svg)"));
  }

  #[test]
  fn fref_default_base_path() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_fref.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(!exported.content.contains("/blog"));
  }

  #[test]
  fn separates_blocks_with_blank_lines() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    let lines: Vec<&str> = exported.content.lines().collect();
    let heading_indices: Vec<usize> = lines
      .iter()
      .enumerate()
      .filter(|(_, line)| line.starts_with('#'))
      .map(|(index, _)| index)
      .collect();
    for &idx in &heading_indices {
      if idx > 0 {
        assert_eq!(lines[idx - 1], "");
      }
    }
  }

  #[test]
  fn markdown_body_only_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "body_only.td");
    let exported = export_resource_markdown(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains("This file has no frontmatter at all.")
    );
    assert!(exported.schema.is_none());
  }

  // HTML content tests

  #[test]
  fn html_simple_paragraph() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_with_content.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert_eq!(exported.content, "<p>Hello world</p>\n");
  }

  #[test]
  fn html_headings() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
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
    assert_eq!(exported.headings[0].title, "Heading 1");
    assert_eq!(exported.headings[0].slug, "heading-1");
    assert_eq!(exported.title.as_deref(), Some("Heading 1"));
  }

  #[test]
  fn html_heading_titles_preserve_special_chars() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_special_headings.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let titles: Vec<&str> = exported.headings.iter().map(|h| h.title.as_str()).collect();
    assert!(titles.contains(&"Version v1.2.3"));
    assert!(titles.contains(&"Key: Value Pairs"));
    assert!(titles.contains(&"C++ and C#"));
    assert!(titles.contains(&"node.js vs Deno"));
  }

  #[test]
  fn html_heading_with_inline_markup() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_special_headings.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"href="https://example.com""#));
    assert!(exported.content.contains("`def1`"));
    assert!(exported.content.contains("<strong>bold heading</strong>"));
  }

  #[test]
  fn html_inline_markup() {
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
    assert!(exported.content.contains("<code>inline code</code>"));
  }

  #[test]
  fn html_code_block_placeholder() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"<pre class="td-code-placeholder" data-lang="python" data-meta="python"><code>print(&quot;hello&quot;)"#));
    assert!(exported.content.contains(
      r#"<pre class="td-code-placeholder" data-lang="js" data-meta="js{1,3}"><code>const x = 1;"#
    ));
    assert!(exported.content.contains(r#"data-title="config.ts""#));
    assert!(exported.content.contains(r#"data-line-numbers="5""#));
  }

  #[test]
  fn html_math_placeholders() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<span class="td-math-inline">E = mc^2</span>"#)
    );
    assert!(
      exported
        .content
        .contains(r#"<div class="td-math-block">\int_0^\infty e^{-x^2} dx"#)
    );
  }

  #[test]
  fn html_blockquote() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains("<blockquote>\n<p>blockquote line</p>\n</blockquote>")
    );
  }

  #[test]
  fn html_table() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"<table tabindex="0">"#));
    assert_eq!(exported.content.matches("<th>").count(), 2);
    assert_eq!(exported.content.matches("<td>").count(), 2);
  }

  #[test]
  fn html_table_inline_code() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_table_inline_code.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<code>_type</code>"));
    assert!(exported.content.contains("<code>_label</code>"));
    assert!(exported.content.contains("<code>_content</code>"));
    assert!(exported.content.contains("<code>icon.rocket</code>"));
  }

  #[test]
  fn html_lists() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<ul>\n<li>"));
    assert!(exported.content.contains("bullet one"));
    assert!(exported.content.contains("bullet two"));
    assert!(exported.content.contains("<ol>\n<li>"));
    assert!(exported.content.contains("ordered one"));
  }

  #[test]
  fn html_callout_and_details() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"<div class="note td-callout"><p class="td-callout-title td-callout-title-default">NOTE</p>"#));
    assert!(exported.content.contains("<p>callout content</p>"));
    assert!(
      exported
        .content
        .contains(r#"<details class="details td-callout"><summary>Click to expand</summary>"#)
    );
    assert!(exported.content.contains("<p>details content</p>"));
  }

  #[test]
  fn html_external_link() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<a href="https://example.com" class="td-external-link""#)
    );
  }

  #[test]
  fn html_code_in_link_text() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<code>code</code> in link</a>"));
    assert!(!exported.content.contains("`code` in link"));
  }

  #[test]
  fn html_container_shorthand() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_container_shorthand.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<toc />\n"));
    assert!(exported.content.contains("<directory-index />\n"));
    assert!(exported.content.contains("<grid :cols=\"2\" />\n"));
  }

  #[test]
  fn html_container_empty_named_slot() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_container_empty_slot.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<template #key>"));
    assert!(exported.content.contains("</template>"));
    assert!(exported.content.contains("</flashcard>"));
  }

  #[test]
  fn html_blob_empty() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "icon.svg");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert_eq!(exported.schema, Some("blob".to_string()));
    assert!(exported.content.is_empty());
    assert!(exported.headings.is_empty());
    assert!(exported.title.is_none());
  }

  #[test]
  fn html_schema_and_frontmatter() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "valid_person.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert_eq!(exported.schema, Some("Person".to_string()));
    let json = to_json(&exported.frontmatter);
    assert_eq!(json["$kind"], "schema");
    assert_eq!(json["fields"]["name"], "Alice");
    assert_eq!(json["fields"]["age"], 30.0);
  }

  #[test]
  fn html_fref_link() {
    let (db, project, file) = load_vault_fixture("evaluate/base_path_vault", "with_fref.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<a href="/blog/alice" class="td-fref-link">Alice</a>"#)
    );
  }

  #[test]
  fn html_fref_image() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "with_asset_fref.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<img src="/icon.svg" alt="icon" loading="lazy">"#)
    );
  }

  #[test]
  fn html_relative_fref_image() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "subdir/relative_fref.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<img src=") && exported.content.contains("test.svg"));
  }

  #[test]
  fn html_no_raw_markdown() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(!exported.content.contains("\n> "));
    assert!(!exported.content.contains("\n- "));
    assert!(!exported.content.contains("**"));
    assert!(!exported.content.contains("~~"));
  }

  #[test]
  fn html_heading_slugs_unique() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "all_md_elements.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    let slugs: Vec<&str> = exported.headings.iter().map(|h| h.slug.as_str()).collect();
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
  fn html_body_only_file() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "body_only.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains("<p>This file has no frontmatter at all.</p>")
    );
    assert!(exported.content.contains("<h2"));
    assert!(exported.schema.is_none());
    assert!(exported.meta.label.is_none());
  }

  #[test]
  fn html_linked_image_badge() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "linked_image.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains(r#"<a href="mailto:huydo862003@gmail.com""#)
    );
    assert!(exported.content.contains(
      r#"<img src="https://img.shields.io/badge/Email-blue" alt="Email badge" loading="lazy">"#
    ));
    assert!(!exported.content.contains("LucideIcon"));
  }

  #[test]
  fn html_body_only_nested_vault() {
    let (db, project, file) = load_vault_fixture("evaluate/nested_vault", "vault/body_only.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(
      exported
        .content
        .contains("<p>This file has no frontmatter.</p>")
    );
  }

  #[test]
  fn html_url_resolution() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "url_resolution.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"href="https://example.com""#));
    assert!(exported.content.contains(r##"href="#section""##));
    assert!(exported.content.contains(r#"href="/other.td""#));
  }

  #[test]
  fn html_media_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "media_with_title.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<figure>"));
    assert!(
      exported
        .content
        .contains(r#"title="The Typedown project logo""#)
    );
    assert!(
      exported
        .content
        .contains("<figcaption>The Typedown project logo</figcaption>")
    );
  }

  #[test]
  fn html_link_with_title() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "link_with_title.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#"title="Visit Example""#));
    assert!(exported.content.contains("href=\"https://example.com"));
  }

  #[test]
  fn html_github_embeds() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "github_embed.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("class=\"td-github-placeholder\""));
    assert!(exported.content.contains("data-url=\"https://github.com/vuejs/vite/blob/main/packages/vite/src/node/server/index.ts#L150-L165"));
    assert!(exported.content.contains("data-label=\"server init\""));
    assert!(exported.content.contains("data-title=\"Vite dev server\""));
    assert!(
      exported
        .content
        .contains("data-url=\"https://gist.github.com/octocat/6cad326836d38bd3a7ae\"")
    );
  }

  #[test]
  fn html_nested_bullet_list() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "nested_list.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<ul>"));
    assert!(
      exported
        .content
        .contains("typerighter</a></strong>: A typed content framework.")
    );
    assert!(
      exported
        .content
        .contains("sqlingo</a></strong>: Porting sqlglot to TypeScript.")
    );
  }

  #[test]
  fn html_nested_containers() {
    let (db, project, file) = load_vault_fixture("evaluate/my_vault", "md_nested_container.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<grid"));
    assert!(exported.content.contains("<grid-cell>"));
    assert!(exported.content.contains("</grid-cell>"));
    assert!(exported.content.contains("<p>Alpha</p>"));
    assert!(exported.content.contains("<p>Beta</p>"));
  }

  #[test]
  fn html_nested_container_flat() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "md_nested_container_flat.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains("<grid-cell>"));
    assert!(exported.content.contains("<p>Alpha</p>"));
  }

  #[test]
  fn html_nested_container_with_props() {
    let (db, project, file) =
      load_vault_fixture("evaluate/my_vault", "md_nested_container_props.td");
    let exported = export_resource_html(&db, project, file).expect("should export");
    assert!(exported.content.contains(r#":columns="3""#));
    assert!(exported.content.contains(r#":col-span="2""#));
  }
}
