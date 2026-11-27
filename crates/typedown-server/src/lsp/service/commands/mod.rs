pub mod backlinks;
pub mod create_linked_resource;
pub mod forward_links;

use serde::{Deserialize, Serialize};

pub const COMMAND_PREFIX: &str = "_typerighter.";
pub const CREATE_LINKED_RESOURCE: &str = "_typerighter.createLinkedResource";
pub const BACKLINKS: &str = "_typerighter.backlinks";
pub const FORWARD_LINKS: &str = "_typerighter.forwardLinks";

pub fn command_ids() -> Vec<String> {
  vec![
    CREATE_LINKED_RESOURCE.to_string(),
    BACKLINKS.to_string(),
    FORWARD_LINKS.to_string(),
  ]
}

// A prompt that the editor should show before sending the command
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum Prompt {
  // Free text input
  Input {
    field: String,
    prompt: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    default: Option<String>,
  },
  // Pick from a list of choices
  Select {
    field: String,
    prompt: String,
    choices: Vec<String>,
  },
}
