use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types::{
  DocumentChangeOperation, DocumentChanges, OptionalVersionedTextDocumentIdentifier, RenameFile,
  ResourceOp, TextDocumentEdit, TextEdit, WorkspaceEdit,
};
use typedown_lang::db::derived::name_resolver::resolution_index::{Reference, ReferenceKind};
use typedown_lang::db::types::HirValueKind;
use typedown_lang::syntax::syntax_kind::SyntaxKind;
use typedown_types::path::normalize_path;

use crate::core::analysis::Analysis;
use crate::core::utils::position::node_trimmed_range;
use crate::core::utils::uri::path_to_uri;
use crate::lsp::service::utils::symbol::str_content_node;

/// Build text edits for all references to a symbol.
pub fn collect_reference_edits(
  analysis: &Analysis,
  refs: &[Reference],
  new_stem: &str,
  new_absolute: &Path,
  root_dir: &Path,
) -> Option<HashMap<PathBuf, Vec<TextEdit>>> {
  let db = &analysis.db;
  let mut edits: HashMap<PathBuf, Vec<TextEdit>> = HashMap::new();

  for r in refs {
    let ref_path = r.hir.file(db).handle(db).path()?.clone();
    let ref_rope = analysis.file_rope(&ref_path)?;
    let node = r.hir.node(db);

    let text_edit = match r.kind {
      // Ident references get replaced with `new_stem`
      ReferenceKind::Ident => TextEdit {
        range: node_trimmed_range(&ref_rope, &node),
        new_text: new_stem.to_string(),
      },
      // Fref references get their path argument replaced with the new relative path
      ReferenceKind::Fref => {
        let HirValueKind::Call { args, .. } = r.hir.kind(db) else {
          continue;
        };
        let Some(arg) = args.first() else { continue };
        let arg_node = arg.node(db);
        // Skip interpolated string arguments
        if arg_node
          .children()
          .any(|c| c.kind() == SyntaxKind::InterpFragment)
        {
          continue;
        }
        let Some(content) = str_content_node(&arg_node) else {
          continue;
        };
        let new_relative = new_absolute.strip_prefix(root_dir).ok()?;
        TextEdit {
          range: node_trimmed_range(&ref_rope, &content),
          new_text: normalize_path(new_relative).to_string(),
        }
      }
    };

    edits.entry(ref_path).or_default().push(text_edit);
  }

  Some(edits)
}

/// Assemble text edits and file renames into a WorkspaceEdit
pub fn build_workspace_edit(
  analysis: &Analysis,
  edits_by_path: HashMap<PathBuf, Vec<TextEdit>>,
  file_renames: Vec<(PathBuf, PathBuf)>,
) -> Option<WorkspaceEdit> {
  let mut changes: Vec<DocumentChangeOperation> = Vec::new();

  for (file_path, edits) in edits_by_path {
    let scheme = analysis
      .scheme_map
      .get(&file_path)
      .map(|s| s.as_str())
      .unwrap_or("file");
    changes.push(DocumentChangeOperation::Edit(TextDocumentEdit {
      text_document: OptionalVersionedTextDocumentIdentifier {
        uri: path_to_uri(&file_path, scheme),
        version: None,
      },
      edits: edits.into_iter().map(lsp_types::OneOf::Left).collect(),
    }));
  }

  for (old_path, new_path) in &file_renames {
    if old_path == new_path {
      continue;
    }
    let scheme = analysis
      .scheme_map
      .get(old_path)
      .map(|s| s.as_str())
      .unwrap_or("file");
    changes.push(DocumentChangeOperation::Op(ResourceOp::Rename(
      RenameFile {
        old_uri: path_to_uri(old_path, scheme),
        new_uri: path_to_uri(new_path, scheme),
        options: None,
        annotation_id: None,
      },
    )));
  }

  if changes.is_empty() {
    return None;
  }

  // Text edits before file renames
  // The LSP says that it applies edits in order
  changes.sort_by_key(|op| match op {
    DocumentChangeOperation::Edit(_) => 0,
    DocumentChangeOperation::Op(_) => 1,
  });

  Some(WorkspaceEdit {
    changes: None,
    document_changes: Some(DocumentChanges::Operations(changes)),
    change_annotations: None,
  })
}
