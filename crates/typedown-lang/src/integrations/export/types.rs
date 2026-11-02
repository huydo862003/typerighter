//! Public types for resource export results

use crate::integrations::bindings::types::TdValue;

use super::content::html::ExportedHeading;

/// Resource metadata (label, icon, excerpt, seo, file timestamps)
#[derive(serde::Serialize, Clone)]
pub struct ExportedMeta {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<ExportedIcon>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub excerpt: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub seo: Option<ExportedSeoMeta>,
  pub mtime: u64,
  pub ctime: u64,
}

/// Exported page icon
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ExportedIcon {
  pub name: String,
}

/// SEO fields from the _meta builtin field
#[derive(serde::Serialize, Clone, Default)]
pub struct ExportedSeoMeta {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub image: Option<String>,
}

/// Structured export result for markdown output
#[derive(serde::Serialize)]
pub struct ExportedResourceMarkdown {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  pub meta: ExportedMeta,
  pub frontmatter: TdValue,
  pub content: String,
}

/// Structured export result with HTML content and extracted headings
#[derive(serde::Serialize)]
pub struct ExportedResourceHtml {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  pub meta: ExportedMeta,
  pub frontmatter: TdValue,
  pub content: String,
  pub headings: Vec<ExportedHeading>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
}

/// Content listing entry: includes frontmatter for filtering, no body
pub struct ExportedResourceSummary {
  pub schema: Option<String>,
  pub meta: ExportedMeta,
  pub frontmatter: TdValue,
}

/// Lightweight sidebar nav entry: no frontmatter or body
pub struct ExportedResourceNav {
  pub schema: Option<String>,
  pub meta: ExportedMeta,
}
