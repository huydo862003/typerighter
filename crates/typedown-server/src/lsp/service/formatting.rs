use lsp_types::{DocumentFormattingParams, TextEdit};
use typedown_lang::db::derived::parse_file::parse_file;
use typedown_lang::integrations::format::format_markdown;
use typedown_lang::syntax::ast::{AstNode, SourceFile};

use crate::core::analysis::Analysis;
use crate::core::utils::position::text_offset_to_lsp_position;
use crate::core::utils::uri::uri_to_path;

pub fn formatting(analysis: &Analysis, params: DocumentFormattingParams) -> Option<Vec<TextEdit>> {
  let db = &analysis.db;
  let project = analysis.project;

  let path = uri_to_path(&params.text_document.uri)?;
  let file = *project.files(db).get(&path)?;
  let rope = analysis.file_rope(&path)?;

  let root = parse_file(db, project, file).ast(db).node.clone();
  let source_file = SourceFile::cast(root)?;
  let body = source_file.body()?;

  let formatted = format_markdown(&body);

  // Replace the entire body range with the formatted text
  let body_node = body.syntax();
  let start = body_node.offset();
  let end = start + body_node.text_len();

  let range = lsp_types::Range {
    start: text_offset_to_lsp_position(&rope, start),
    end: text_offset_to_lsp_position(&rope, end),
  };

  Some(vec![TextEdit {
    range,
    new_text: formatted,
  }])
}
