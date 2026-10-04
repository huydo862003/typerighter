use regex::Regex;
use std::sync::OnceLock;

static ISO_DATE: OnceLock<Regex> = OnceLock::new();
static ISO_TIME: OnceLock<Regex> = OnceLock::new();
static ISO_DATETIME: OnceLock<Regex> = OnceLock::new();

fn iso_date_re() -> &'static Regex {
  ISO_DATE.get_or_init(|| Regex::new(r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])$").unwrap())
}

fn iso_time_re() -> &'static Regex {
  ISO_TIME.get_or_init(|| {
    Regex::new(r"^([01]\d|2[0-3]):[0-5]\d(:[0-5]\d(\.\d+)?)?(Z|[+-]([01]\d|2[0-3]):[0-5]\d)?$")
      .unwrap()
  })
}

fn iso_datetime_re() -> &'static Regex {
  ISO_DATETIME.get_or_init(|| {
    Regex::new(
      r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])[T ]([01]\d|2[0-3]):[0-5]\d(:[0-5]\d(\.\d+)?)?(Z|[+-]([01]\d|2[0-3]):[0-5]\d)?$",
    )
    .unwrap()
  })
}

pub(crate) fn is_valid_iso_date(s: &str) -> bool {
  iso_date_re().is_match(s)
}

pub(crate) fn is_valid_iso_time(s: &str) -> bool {
  iso_time_re().is_match(s)
}

pub(crate) fn is_valid_iso_datetime(s: &str) -> bool {
  iso_datetime_re().is_match(s)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn iso_date_valid() {
    assert!(is_valid_iso_date("2024-01-15"));
    assert!(is_valid_iso_date("2000-12-31"));
    assert!(is_valid_iso_date("0001-01-01"));
  }

  #[test]
  fn iso_date_invalid() {
    assert!(!is_valid_iso_date("2024-1-15")); // month not zero-padded
    assert!(!is_valid_iso_date("2024-13-01")); // month out of range
    assert!(!is_valid_iso_date("2024-00-01")); // month zero
    assert!(!is_valid_iso_date("2024-01-32")); // day out of range
    assert!(!is_valid_iso_date("2024-01-00")); // day zero
    assert!(!is_valid_iso_date("24-01-15")); // year not 4 digits
    assert!(!is_valid_iso_date("not-a-date"));
    assert!(!is_valid_iso_date("2024-01")); // missing day
  }

  #[test]
  fn iso_time_valid() {
    assert!(is_valid_iso_time("14:30"));
    assert!(is_valid_iso_time("14:30:00"));
    assert!(is_valid_iso_time("14:30:59.999"));
    assert!(is_valid_iso_time("00:00:00"));
    assert!(is_valid_iso_time("23:59:59"));
    assert!(is_valid_iso_time("14:30:00Z"));
    assert!(is_valid_iso_time("14:30:00+05:30"));
    assert!(is_valid_iso_time("14:30:00-08:00"));
  }

  #[test]
  fn iso_time_invalid() {
    assert!(!is_valid_iso_time("24:00")); // hour out of range
    assert!(!is_valid_iso_time("14:60")); // minute out of range
    assert!(!is_valid_iso_time("14:30:60")); // second out of range
    assert!(!is_valid_iso_time("4:30")); // hour not zero-padded
    assert!(!is_valid_iso_time("14:3")); // minute not zero-padded
    assert!(!is_valid_iso_time("not-a-time"));
  }

  #[test]
  fn iso_datetime_valid() {
    assert!(is_valid_iso_datetime("2024-01-15T14:30:00"));
    assert!(is_valid_iso_datetime("2024-01-15 14:30:00"));
    assert!(is_valid_iso_datetime("2024-01-15T14:30:00Z"));
    assert!(is_valid_iso_datetime("2024-01-15T14:30:00+05:30"));
    assert!(is_valid_iso_datetime("2024-01-15T14:30:00.123"));
    assert!(is_valid_iso_datetime("2024-01-15T14:30"));
  }

  #[test]
  fn iso_datetime_invalid() {
    assert!(!is_valid_iso_datetime("2024-01-15")); // date only, no time
    assert!(!is_valid_iso_datetime("14:30:00")); // time only, no date
    assert!(!is_valid_iso_datetime("2024-13-15T14:30:00")); // invalid month
    assert!(!is_valid_iso_datetime("2024-01-15T25:00:00")); // invalid hour
    assert!(!is_valid_iso_datetime("not-a-datetime"));
  }
}
