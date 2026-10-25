/// Unescape a string literal content (the part between delimiters)
pub fn unescape(input: &str) -> Result<String, unescaper::Error> {
  unescaper::unescape(input)
}

/// Decode an HTML entity into a String
pub fn unescape_html_entity(input: &str) -> String {
  html_escape::decode_html_entities(input).into_owned()
}
