use serde::{Deserialize, Serialize};

/// Server-defined JSON-RPC error code for query cancellation
/// NOTE: -32000 to -32099 are server-reserved in JSON-RPC
pub const CANCELLED_ERROR_CODE: i32 = -32002;

/* JSON-RPC method names */

pub const METHOD_REQUEST_FILE: &str = "typedown_build.request_file";
pub const METHOD_REQUEST_FILES: &str = "typedown_build.request_files";
pub const METHOD_LIST_VAULT: &str = "typedown_build.list_vault";
pub const METHOD_LIST_FILES_GROUPED_BY_SCHEMA: &str = "typedown_build.list_files_grouped_by_schema";
pub const METHOD_LIST_SIDEBAR: &str = "typedown_build.list_sidebar";
pub const METHOD_LIST_SCHEMAS: &str = "typedown_build.list_schemas";
pub const METHOD_GET_SCHEMA: &str = "typedown_build.get_schema";
pub const METHOD_GET_VERSION: &str = "typedown_build.get_version";
pub const METHOD_GET_CONFIG: &str = "typedown_build.get_config";
pub const METHOD_CHECK_VAULT: &str = "typedown_build.check_vault";
pub const METHOD_FORMAT_FILE: &str = "typedown_build.format_file";

/* JSON-RPC notification names (server -> client) */

pub const NOTIF_CONTENT_CHANGED: &str = "typedown_build.content_changed";
pub const NOTIF_CONTENT_CREATED: &str = "typedown_build.content_created";
pub const NOTIF_CONTENT_DELETED: &str = "typedown_build.content_deleted";
pub const NOTIF_SCHEMA_CHANGED: &str = "typedown_build.schema_changed";
pub const NOTIF_SCHEMA_CREATED: &str = "typedown_build.schema_created";
pub const NOTIF_SCHEMA_DELETED: &str = "typedown_build.schema_deleted";
pub const NOTIF_CONFIG_CHANGED: &str = "typedown_build.config_changed";

/* RPC error types */

pub struct RpcError {
  pub code: i32,
  pub message: String,
}

impl RpcError {
  pub fn invalid_params(message: impl Into<String>) -> Self {
    Self {
      code: lsp_server::ErrorCode::InvalidParams as i32,
      message: message.into(),
    }
  }
}

pub type RpcResult<T> = Result<T, RpcError>;

/* RPC request params and results */

/// Site-wide configuration derived from typedown.yaml
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TdSiteConfig {
  pub version: String,
  /// URL base path (e.g. "/" or "/docs")
  pub base_path: String,
  /// Vault root directory path relative to the project root
  pub root_dir: String,
  /// Site title from typedown.yaml
  pub site_title: String,
  /// Site description from typedown.yaml
  pub site_description: String,
  /// Repository URL from typedown.yaml
  pub repo: Option<String>,
  /// Site author from typedown.yaml
  pub author: Option<String>,
  /// License name from typedown.yaml
  pub license: Option<String>,
  pub public_dir: String,
  pub nav: Vec<TdNavItem>,
}

/// Navigation link from typedown.yaml
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TdNavItem {
  pub title: String,
  pub link: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<String>,
}

/// Lightweight summary of a content file (no body content)
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TdContentSummary {
  /// File path relative to the vault root
  pub filepath: String,
  /// Schema type name
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  /// Human-readable schema label
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema_label: Option<String>,
  /// Display label
  #[serde(skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  /// Page icon
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<TdIcon>,
  /// Frontmatter header as JSON
  pub header: serde_json::Value,
  /// First paragraph of the body content
  #[serde(skip_serializing_if = "Option::is_none")]
  pub excerpt: Option<String>,
  /// File metadata
  pub metadata: TdFileMetadata,
}

/// Lightweight sidebar item (no header/content/excerpt)
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TdSidebarItem {
  pub filepath: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema_label: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<TdIcon>,
  pub metadata: TdFileMetadata,
}

/// Structured build result: header (frontmatter), HTML content, and extracted headings
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TdBuiltResource {
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema: Option<String>,
  /// Human-readable schema label
  #[serde(skip_serializing_if = "Option::is_none")]
  pub schema_label: Option<String>,
  /// Display label
  #[serde(skip_serializing_if = "Option::is_none")]
  pub label: Option<String>,
  /// Page icon
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<TdIcon>,
  pub header: serde_json::Value,
  /// HTML body with placeholders for code/math post-processing
  pub content: String,
  /// Headings extracted during HTML emission
  pub headings: Vec<TdHeading>,
  /// Page title extracted from the first h1
  #[serde(skip_serializing_if = "Option::is_none")]
  pub title: Option<String>,
  /// File metadata
  pub metadata: TdFileMetadata,
}

/// Heading extracted from page content
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TdHeading {
  pub level: u32,
  pub title: String,
  pub title_html: String,
  pub slug: String,
}

/// Page icon
#[derive(Serialize, Deserialize, Clone)]
pub struct TdIcon {
  /// Lucide icon name
  pub name: String,
}

/// File metadata
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TdFileMetadata {
  /// Last modification time as seconds since UNIX epoch
  pub mtime: u64,
  /// Creation time as seconds since UNIX epoch
  pub ctime: u64,
}

/// Schema metadata
#[derive(Serialize, Deserialize, Clone)]
pub struct TdSchemaInfo {
  pub schema: String,
  pub label: String,
  pub properties: serde_json::Value,
}

/* Formatting */

/// Result of formatting a single file
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TdFormatResult {
  /// Full formatted file content (frontmatter + body)
  pub content: String,
  /// Whether the content changed
  pub changed: bool,
}

/* Diagnostics */

/// A single diagnostic item with location and message
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TdDiagnosticItem {
  /// File path relative to the vault root
  pub filepath: String,
  /// 1-based line number
  pub line: u32,
  /// 1-based column number
  pub column: u32,
  /// "error" or "warning"
  pub severity: String,
  /// Kebab-case diagnostic code (e.g. "duplicate-key", "missing-required-field")
  pub code: String,
  /// Human-readable message
  pub message: String,
}

/// Result of checking all vault files for diagnostics
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TdDiagnosticReport {
  pub diagnostics: Vec<TdDiagnosticItem>,
  pub file_count: u32,
  pub error_count: u32,
  pub warning_count: u32,
}

/* Subscription notifications */

/// Content file event: A resource file was created, changed, or deleted
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TdContentNotification {
  pub content: String,
  #[serde(default, skip_serializing_if = "Vec::is_empty")]
  pub affected_files: Vec<String>,
}

/// Schema file event: A schema file was created, changed, or deleted
#[derive(Serialize, Deserialize, Clone)]
pub struct TdSchemaNotification {
  pub schema: String,
}
