use std::collections::HashMap;

use typedown_incremental::QueryDatabase;
use typedown_macros::query_derived;

use crate::db::TypedownDatabase;
use crate::db::derived::get_builtin_types::get_icon_type;
use crate::db::types::{LazyType, TdProductType};

pub struct IconEntry {
  pub name: &'static str,
  pub lucide_name: &'static str,
}

pub static ICON_ENTRIES: &[IconEntry] = &[
  // Documents
  IconEntry {
    name: "document",
    lucide_name: "file-text",
  },
  IconEntry {
    name: "page",
    lucide_name: "file",
  },
  IconEntry {
    name: "notebook",
    lucide_name: "notebook",
  },
  IconEntry {
    name: "article",
    lucide_name: "newspaper",
  },
  IconEntry {
    name: "scroll",
    lucide_name: "scroll",
  },
  IconEntry {
    name: "book",
    lucide_name: "book-open",
  },
  IconEntry {
    name: "book_marked",
    lucide_name: "book-marked",
  },
  IconEntry {
    name: "clipboard",
    lucide_name: "clipboard",
  },
  IconEntry {
    name: "note",
    lucide_name: "sticky-note",
  },
  IconEntry {
    name: "file_code",
    lucide_name: "file-code",
  },
  // Folders & Organization
  IconEntry {
    name: "folder",
    lucide_name: "folder",
  },
  IconEntry {
    name: "archive",
    lucide_name: "archive",
  },
  IconEntry {
    name: "container",
    lucide_name: "box",
  },
  IconEntry {
    name: "package",
    lucide_name: "package",
  },
  IconEntry {
    name: "briefcase",
    lucide_name: "briefcase",
  },
  IconEntry {
    name: "inbox",
    lucide_name: "inbox",
  },
  IconEntry {
    name: "layers",
    lucide_name: "layers",
  },
  IconEntry {
    name: "library",
    lucide_name: "library",
  },
  // Communication
  IconEntry {
    name: "mail",
    lucide_name: "mail",
  },
  IconEntry {
    name: "message",
    lucide_name: "message-circle",
  },
  IconEntry {
    name: "megaphone",
    lucide_name: "megaphone",
  },
  IconEntry {
    name: "bell",
    lucide_name: "bell",
  },
  IconEntry {
    name: "phone",
    lucide_name: "phone",
  },
  IconEntry {
    name: "radio",
    lucide_name: "radio",
  },
  IconEntry {
    name: "send",
    lucide_name: "send",
  },
  // People
  IconEntry {
    name: "user",
    lucide_name: "user",
  },
  IconEntry {
    name: "users",
    lucide_name: "users",
  },
  IconEntry {
    name: "contact",
    lucide_name: "contact",
  },
  IconEntry {
    name: "hand",
    lucide_name: "hand",
  },
  // Nature
  IconEntry {
    name: "leaf",
    lucide_name: "leaf",
  },
  IconEntry {
    name: "flower",
    lucide_name: "flower-2",
  },
  IconEntry {
    name: "tree",
    lucide_name: "tree-pine",
  },
  IconEntry {
    name: "sun",
    lucide_name: "sun",
  },
  IconEntry {
    name: "moon",
    lucide_name: "moon",
  },
  IconEntry {
    name: "cloud",
    lucide_name: "cloud",
  },
  IconEntry {
    name: "mountain",
    lucide_name: "mountain",
  },
  IconEntry {
    name: "waves",
    lucide_name: "waves",
  },
  IconEntry {
    name: "snowflake",
    lucide_name: "snowflake",
  },
  IconEntry {
    name: "flame",
    lucide_name: "flame",
  },
  IconEntry {
    name: "droplet",
    lucide_name: "droplet",
  },
  // Science & Tech
  IconEntry {
    name: "flask",
    lucide_name: "flask-conical",
  },
  IconEntry {
    name: "atom",
    lucide_name: "atom",
  },
  IconEntry {
    name: "cpu",
    lucide_name: "cpu",
  },
  IconEntry {
    name: "code",
    lucide_name: "code",
  },
  IconEntry {
    name: "terminal",
    lucide_name: "terminal",
  },
  IconEntry {
    name: "database",
    lucide_name: "database",
  },
  IconEntry {
    name: "server",
    lucide_name: "server",
  },
  IconEntry {
    name: "bug",
    lucide_name: "bug",
  },
  IconEntry {
    name: "circuit",
    lucide_name: "circuit-board",
  },
  IconEntry {
    name: "binary",
    lucide_name: "binary",
  },
  // Creative
  IconEntry {
    name: "palette",
    lucide_name: "palette",
  },
  IconEntry {
    name: "pen",
    lucide_name: "pen",
  },
  IconEntry {
    name: "brush",
    lucide_name: "paintbrush",
  },
  IconEntry {
    name: "camera",
    lucide_name: "camera",
  },
  IconEntry {
    name: "music",
    lucide_name: "music",
  },
  IconEntry {
    name: "film",
    lucide_name: "film",
  },
  IconEntry {
    name: "image",
    lucide_name: "image",
  },
  IconEntry {
    name: "mic",
    lucide_name: "mic",
  },
  IconEntry {
    name: "headphones",
    lucide_name: "headphones",
  },
  // Status & Emotion
  IconEntry {
    name: "heart",
    lucide_name: "heart",
  },
  IconEntry {
    name: "star",
    lucide_name: "star",
  },
  IconEntry {
    name: "flag",
    lucide_name: "flag",
  },
  IconEntry {
    name: "trophy",
    lucide_name: "trophy",
  },
  IconEntry {
    name: "target",
    lucide_name: "target",
  },
  IconEntry {
    name: "bookmark",
    lucide_name: "bookmark",
  },
  IconEntry {
    name: "thumbs_up",
    lucide_name: "thumbs-up",
  },
  IconEntry {
    name: "check",
    lucide_name: "circle-check",
  },
  IconEntry {
    name: "alert",
    lucide_name: "triangle-alert",
  },
  IconEntry {
    name: "info",
    lucide_name: "info",
  },
  IconEntry {
    name: "sparkles",
    lucide_name: "sparkles",
  },
  IconEntry {
    name: "crown",
    lucide_name: "crown",
  },
  // Abstract & Shapes
  IconEntry {
    name: "circle",
    lucide_name: "circle",
  },
  IconEntry {
    name: "square",
    lucide_name: "square",
  },
  IconEntry {
    name: "diamond",
    lucide_name: "diamond",
  },
  IconEntry {
    name: "hexagon",
    lucide_name: "hexagon",
  },
  IconEntry {
    name: "triangle",
    lucide_name: "triangle",
  },
  IconEntry {
    name: "zap",
    lucide_name: "zap",
  },
  IconEntry {
    name: "hash",
    lucide_name: "hash",
  },
  IconEntry {
    name: "at_sign",
    lucide_name: "at-sign",
  },
  IconEntry {
    name: "infinity",
    lucide_name: "infinity",
  },
  // Navigation & Travel
  IconEntry {
    name: "compass",
    lucide_name: "compass",
  },
  IconEntry {
    name: "map",
    lucide_name: "map",
  },
  IconEntry {
    name: "globe",
    lucide_name: "globe",
  },
  IconEntry {
    name: "home",
    lucide_name: "house",
  },
  IconEntry {
    name: "rocket",
    lucide_name: "rocket",
  },
  IconEntry {
    name: "plane",
    lucide_name: "plane",
  },
  IconEntry {
    name: "car",
    lucide_name: "car",
  },
  IconEntry {
    name: "ship",
    lucide_name: "ship",
  },
  IconEntry {
    name: "signpost",
    lucide_name: "signpost",
  },
  // Time & Calendar
  IconEntry {
    name: "clock",
    lucide_name: "clock",
  },
  IconEntry {
    name: "calendar",
    lucide_name: "calendar",
  },
  IconEntry {
    name: "timer",
    lucide_name: "timer",
  },
  IconEntry {
    name: "hourglass",
    lucide_name: "hourglass",
  },
  // Tools & Objects
  IconEntry {
    name: "wrench",
    lucide_name: "wrench",
  },
  IconEntry {
    name: "hammer",
    lucide_name: "hammer",
  },
  IconEntry {
    name: "key",
    lucide_name: "key",
  },
  IconEntry {
    name: "lock",
    lucide_name: "lock",
  },
  IconEntry {
    name: "shield",
    lucide_name: "shield",
  },
  IconEntry {
    name: "link",
    lucide_name: "link",
  },
  IconEntry {
    name: "magnet",
    lucide_name: "magnet",
  },
  IconEntry {
    name: "scissors",
    lucide_name: "scissors",
  },
  IconEntry {
    name: "lightbulb",
    lucide_name: "lightbulb",
  },
  IconEntry {
    name: "glasses",
    lucide_name: "glasses",
  },
  // Data & Charts
  IconEntry {
    name: "chart",
    lucide_name: "chart-bar",
  },
  IconEntry {
    name: "pie_chart",
    lucide_name: "chart-pie",
  },
  IconEntry {
    name: "trending",
    lucide_name: "trending-up",
  },
  IconEntry {
    name: "table",
    lucide_name: "table",
  },
  IconEntry {
    name: "kanban",
    lucide_name: "kanban",
  },
  // Education
  IconEntry {
    name: "graduation",
    lucide_name: "graduation-cap",
  },
  IconEntry {
    name: "school",
    lucide_name: "school",
  },
  IconEntry {
    name: "pencil",
    lucide_name: "pencil",
  },
  // Food & Drink
  IconEntry {
    name: "coffee",
    lucide_name: "coffee",
  },
  IconEntry {
    name: "pizza",
    lucide_name: "pizza",
  },
  IconEntry {
    name: "apple",
    lucide_name: "apple",
  },
  // Misc
  IconEntry {
    name: "gift",
    lucide_name: "gift",
  },
  IconEntry {
    name: "puzzle",
    lucide_name: "puzzle",
  },
  IconEntry {
    name: "gamepad",
    lucide_name: "gamepad-2",
  },
  IconEntry {
    name: "ticket",
    lucide_name: "ticket",
  },
  IconEntry {
    name: "tag",
    lucide_name: "tag",
  },
  IconEntry {
    name: "pin",
    lucide_name: "pin",
  },
  IconEntry {
    name: "trash",
    lucide_name: "trash-2",
  },
  IconEntry {
    name: "wand",
    lucide_name: "wand-sparkles",
  },
  IconEntry {
    name: "eye",
    lucide_name: "eye",
  },
  IconEntry {
    name: "search",
    lucide_name: "search",
  },
  IconEntry {
    name: "settings",
    lucide_name: "settings",
  },
];

#[query_derived]
pub fn get_icon_module_type<'db>(db: &'db TypedownDatabase) -> TdProductType<'db> {
  let icon_type = get_icon_type(db);
  let mut fields = HashMap::new();
  for entry in ICON_ENTRIES {
    fields.insert(entry.name.to_string(), LazyType::eager(icon_type.into()));
  }
  TdProductType::new(db, None, fields)
}
