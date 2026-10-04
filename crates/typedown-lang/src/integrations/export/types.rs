//! Public types for resource export results

use super::html::ExportedHeading;

/// Structured export result for markdown output
#[derive(serde::Serialize)]
pub struct ExportedResourceMarkdown {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<ExportedIcon>,
  /// Frontmatter fields as a JSON object
  pub header: serde_json::Value,
  /// Commonmark-compatible markdown body
  pub content: String,
  /// File timestamps
  pub metadata: ExportedFileMetadata,
}

/// Exported page icon
#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct ExportedIcon {
  /// Lucide icon name
  pub name: String,
}

/// File timestamps exported alongside a resource
#[derive(serde::Serialize, Clone)]
pub struct ExportedFileMetadata {
  /// Last modification time as seconds since UNIX epoch
  pub mtime: u64,
  /// Creation time as seconds since UNIX epoch
  pub ctime: u64,
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

/// Structured export result with HTML content and extracted headings
#[derive(serde::Serialize)]
pub struct ExportedResourceHtml {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<ExportedIcon>,
  /// Frontmatter fields as a JSON object
  pub header: serde_json::Value,
  /// HTML body with placeholders for code/math post-processing
  pub content: String,
  pub headings: Vec<ExportedHeading>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  /// File timestamps
  pub metadata: ExportedFileMetadata,
  /// SEO fields from the _meta builtin field
  #[serde(skip_serializing_if = "Option::is_none")]
  pub seo_metadata: Option<ExportedSeoMeta>,
}

/// Content listing entry: includes header for filtering, no body
pub struct ExportedResourceSummary {
  pub schema: Option<String>,
  pub label: Option<String>,
  pub icon: Option<ExportedIcon>,
  pub header: serde_json::Value,
  pub excerpt: Option<String>,
  pub metadata: ExportedFileMetadata,
}

/// Lightweight sidebar nav entry: no header or body
pub struct ExportedResourceNav {
  pub schema: Option<String>,
  pub label: Option<String>,
  pub icon: Option<ExportedIcon>,
  pub metadata: ExportedFileMetadata,
  pub excerpt: Option<String>,
}
