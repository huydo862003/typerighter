#![allow(clippy::double_must_use)]

use std::collections::HashMap;

#[cfg(not(target_arch = "wasm32"))]
use jsonrpsee::core::RpcResult;
use jsonrpsee::proc_macros::rpc;
use serde::{Deserialize, Serialize};
#[cfg(target_arch = "wasm32")]
use tsify_next::Tsify;

#[cfg(not(target_arch = "wasm32"))]
use jsonrpsee::{
  IntoSubscriptionCloseResponse, SubscriptionCloseResponse, core::to_json_raw_value,
};

/// Server-defined JSON-RPC error code for query cancellation
/// NOTE: -32000 to -32099 are server-reserved in JSON-RPC
pub const CANCELLED_ERROR_CODE: i32 = -32002;

/// On native: generates both TdBuildRpcServer and TdBuildRpcClient traits
#[cfg_attr(
  not(target_arch = "wasm32"),
  rpc(
    server,
    client,
    namespace = "typedown_build",
    namespace_separator = "."
  )
)]
/// On WASM: generates only TdBuildRpcClient (no server types available)
#[cfg_attr(
  target_arch = "wasm32",
  rpc(client, namespace = "typedown_build", namespace_separator = ".")
)]
pub trait TdBuildRpc<Hash, StorageKey> {
  /* Requests */

  #[method(name = "request_file")]
  async fn request_file(&self, file_path: TdFilePath) -> RpcResult<TdBuiltResource>;

  #[method(name = "request_files")]
  async fn request_files(&self, file_paths: Vec<TdFilePath>) -> RpcResult<Vec<TdBuiltResource>>;

  #[method(name = "list_vault")]
  async fn list_vault(&self) -> RpcResult<Vec<String>>;

  #[method(name = "list_files_grouped_by_schema")]
  async fn list_files_grouped_by_schema(&self)
  -> RpcResult<HashMap<String, Vec<TdContentSummary>>>;

  #[method(name = "list_sidebar")]
  async fn list_sidebar(&self) -> RpcResult<Vec<TdSidebarItem>>;

  #[method(name = "list_schemas")]
  async fn list_schemas(&self) -> RpcResult<Vec<String>>;

  #[method(name = "get_schema")]
  async fn get_schema(&self, schema: String) -> RpcResult<TdSchemaInfo>;

  #[method(name = "get_version")]
  async fn get_version(&self) -> RpcResult<String>;

  #[method(name = "get_config")]
  async fn get_config(&self) -> RpcResult<TdSiteConfig>;

  #[method(name = "check_vault")]
  async fn check_vault(&self) -> RpcResult<TdDiagnosticReport>;

  #[method(name = "format_file")]
  async fn format_file(&self, file_path: TdFilePath) -> RpcResult<TdFormatResult>;

  /* Content subscriptions */

  #[subscription(name = "subscribe_content_changed", item = TdContentNotification)]
  async fn subscribe_content_changed(&self) -> TdRpcSubscriptionCloseResponse;

  #[subscription(name = "subscribe_content_created", item = TdContentNotification)]
  async fn subscribe_content_created(&self) -> TdRpcSubscriptionCloseResponse;

  #[subscription(name = "subscribe_content_deleted", item = TdContentNotification)]
  async fn subscribe_content_deleted(&self) -> TdRpcSubscriptionCloseResponse;

  /* Schema subscriptions */

  #[subscription(name = "subscribe_schema_changed", item = TdSchemaNotification)]
  async fn subscribe_schema_changed(&self) -> TdRpcSubscriptionCloseResponse;

  #[subscription(name = "subscribe_schema_created", item = TdSchemaNotification)]
  async fn subscribe_schema_created(&self) -> TdRpcSubscriptionCloseResponse;

  #[subscription(name = "subscribe_schema_deleted", item = TdSchemaNotification)]
  async fn subscribe_schema_deleted(&self) -> TdRpcSubscriptionCloseResponse;

  /* Config subscriptions */

  #[subscription(name = "subscribe_config_changed", item = TdSiteConfig)]
  async fn subscribe_config_changed(&self) -> TdRpcSubscriptionCloseResponse;
}

/* RPC request params and results */

/// Site-wide configuration derived from typedown.yaml
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, hashmap_as_object))]
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
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi))]
pub struct TdNavItem {
  pub title: String,
  pub link: String,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub icon: Option<String>,
}

/// Path relative to the vault root
#[derive(Serialize, Deserialize)]
pub struct TdFilePath(pub String);

/// Lightweight summary of a content file (no body content)
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, hashmap_as_object))]
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
  #[cfg_attr(target_arch = "wasm32", tsify(type = "Record<string, any>"))]
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
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi))]
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
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, hashmap_as_object))]
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
  #[cfg_attr(target_arch = "wasm32", tsify(type = "Record<string, any>"))]
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
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi))]
pub struct TdHeading {
  pub level: u32,
  pub title: String,
  pub slug: String,
}

/// Page icon
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, hashmap_as_object))]
pub struct TdIcon {
  /// Lucide icon name
  pub name: String,
}

/// File metadata
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, hashmap_as_object))]
pub struct TdFileMetadata {
  /// Last modification time as seconds since UNIX epoch
  pub mtime: u64,
  /// Creation time as seconds since UNIX epoch
  pub ctime: u64,
}

/// Schema metadata
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, hashmap_as_object))]
pub struct TdSchemaInfo {
  pub schema: String,
  pub label: String,
  #[cfg_attr(target_arch = "wasm32", tsify(type = "Record<string, any>"))]
  pub properties: serde_json::Value,
}

/* Formatting */

/// Result of formatting a single file
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, hashmap_as_object))]
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
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, hashmap_as_object))]
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
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, hashmap_as_object))]
pub struct TdDiagnosticReport {
  pub diagnostics: Vec<TdDiagnosticItem>,
  pub file_count: u32,
  pub error_count: u32,
  pub warning_count: u32,
}

/* Subscription notifications */

/// Content file event: A resource file was created, changed, or deleted
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, hashmap_as_object))]
pub struct TdContentNotification {
  pub content: String,
}

/// Schema file event: A schema file was created, changed, or deleted
#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(target_arch = "wasm32", derive(Tsify))]
#[cfg_attr(target_arch = "wasm32", tsify(into_wasm_abi, hashmap_as_object))]
pub struct TdSchemaNotification {
  pub schema: String,
}

/* Server's response to client subscription termination */

#[cfg(not(target_arch = "wasm32"))]
pub enum TdRpcSubscriptionCloseResponse {
  Ok,
  Err(String),
}

#[cfg(not(target_arch = "wasm32"))]
impl IntoSubscriptionCloseResponse for TdRpcSubscriptionCloseResponse {
  fn into_response(self) -> SubscriptionCloseResponse {
    match self {
      TdRpcSubscriptionCloseResponse::Ok => SubscriptionCloseResponse::None,
      TdRpcSubscriptionCloseResponse::Err(msg) => {
        let err = to_json_raw_value(&msg).unwrap();
        SubscriptionCloseResponse::Notif(err.into())
      }
    }
  }
}
