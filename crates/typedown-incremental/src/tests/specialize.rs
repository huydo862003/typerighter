use std::fmt::Display;
use typedown_macros::specialize;

#[test]
fn dispatches_to_display_for_i32() {
  let val = 42i32;
  let result = specialize!(val {
    where Display => "display";
    default => "other";
  });
  assert_eq!(result, "display");
}

#[test]
fn dispatches_to_default_for_non_display() {
  struct Opaque;
  let val = Opaque;
  let result = specialize!(val {
    where Display => "display";
    default => "other";
  });
  assert_eq!(result, "other");
}

#[test]
fn type_dispatch_to_display() {
  let result = specialize!(type i32 {
    where Display => "display";
    default => "other";
  });
  assert_eq!(result, "display");
}

#[test]
fn type_dispatch_to_default() {
  struct Opaque;
  let result = specialize!(type Opaque {
    where Display => "display";
    default => "other";
  });
  assert_eq!(result, "other");
}

#[test]
fn captures_outer_scope() {
  let mut buf: Vec<String> = Vec::new();
  let val = 42i32;
  specialize!(val {
    where Display => buf.push(format!("{}", val));
    default => buf.push("opaque".to_string());
  });
  assert_eq!(buf, vec!["42"]);
}
