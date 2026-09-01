/// Split PascalCase into space-separated words
/// e.g. "PltConcept" -> "Plt Concept"
pub fn split_pascal_case(name: &str) -> String {
  let mut result = String::with_capacity(name.len() + 4);
  for (i, ch) in name.chars().enumerate() {
    if i > 0 && ch.is_uppercase() {
      result.push(' ');
    }
    result.push(ch);
  }
  result
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn split_pascal_case_simple() {
    assert_eq!(split_pascal_case("PltConcept"), "Plt Concept");
    assert_eq!(split_pascal_case("DistSysJourney"), "Dist Sys Journey");
    assert_eq!(split_pascal_case("Person"), "Person");
    assert_eq!(split_pascal_case("simple"), "simple");
    assert_eq!(split_pascal_case(""), "");
    assert_eq!(split_pascal_case("A"), "A");
    assert_eq!(split_pascal_case("ABC"), "A B C");
  }
}
