use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};

pub(crate) fn is_valid_iso_date(string: &str) -> bool {
  normalize_to_iso_date(string).is_some()
}

// Parses a date string and returns the canonical ISO 8601 form (YYYY-MM-DD), or None if the input is not a valid ISO date
// Requires strict YYYY-MM-DD format (4-digit year, 2-digit month, 2-digit day)
pub(crate) fn normalize_to_iso_date(string: &str) -> Option<String> {
  // chrono's %Y-%m-%d accepts non-padded values, so check structure first
  let bytes = string.as_bytes();
  if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
    return None;
  }
  NaiveDate::parse_from_str(string, "%Y-%m-%d")
    .ok()
    .map(|date| date.format("%Y-%m-%d").to_string())
}

pub(crate) fn is_valid_iso_time(string: &str) -> bool {
  const NAIVE_FORMATS: &[&str] = &["%H:%M", "%H:%M:%S", "%H:%M:%S%.f"];

  if NAIVE_FORMATS
    .iter()
    .any(|format| NaiveTime::parse_from_str(string, format).is_ok())
  {
    return true;
  }

  // Time with timezone offset (+HH:MM / -HH:MM): prepend a fake date to use DateTime parsing
  let with_date = format!("1970-01-01T{string}");
  const TZ_FORMATS: &[&str] = &[
    "%Y-%m-%dT%H:%M%:z",
    "%Y-%m-%dT%H:%M:%S%:z",
    "%Y-%m-%dT%H:%M:%S%.f%:z",
  ];
  if TZ_FORMATS
    .iter()
    .any(|format| DateTime::<FixedOffset>::parse_from_str(&with_date, format).is_ok())
  {
    return true;
  }

  // Time with Z (UTC) suffix
  if let Some(without_z) = string.strip_suffix('Z') {
    return NAIVE_FORMATS
      .iter()
      .any(|format| NaiveTime::parse_from_str(without_z, format).is_ok());
  }

  false
}

pub(crate) fn is_valid_iso_datetime(string: &str) -> bool {
  const NAIVE_FORMATS: &[&str] = &[
    "%Y-%m-%dT%H:%M",
    "%Y-%m-%dT%H:%M:%S",
    "%Y-%m-%dT%H:%M:%S%.f",
    "%Y-%m-%d %H:%M",
    "%Y-%m-%d %H:%M:%S",
    "%Y-%m-%d %H:%M:%S%.f",
  ];

  if NAIVE_FORMATS
    .iter()
    .any(|format| NaiveDateTime::parse_from_str(string, format).is_ok())
  {
    return true;
  }

  const TZ_FORMATS: &[&str] = &[
    "%Y-%m-%dT%H:%M%:z",
    "%Y-%m-%dT%H:%M:%S%:z",
    "%Y-%m-%dT%H:%M:%S%.f%:z",
  ];
  if TZ_FORMATS
    .iter()
    .any(|format| DateTime::<FixedOffset>::parse_from_str(string, format).is_ok())
  {
    return true;
  }

  // Datetime with Z (UTC) suffix
  if let Some(without_z) = string.strip_suffix('Z') {
    return NAIVE_FORMATS
      .iter()
      .any(|format| NaiveDateTime::parse_from_str(without_z, format).is_ok());
  }

  false
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
    assert!(!is_valid_iso_date("2024-13-01")); // month out of range
    assert!(!is_valid_iso_date("2024-00-01")); // month zero
    assert!(!is_valid_iso_date("2024-01-32")); // day out of range
    assert!(!is_valid_iso_date("2024-01-00")); // day zero
    assert!(!is_valid_iso_date("not-a-date"));
    assert!(!is_valid_iso_date("2024-01")); // missing day
    assert!(!is_valid_iso_date("2024-1-15")); // non-padded month
    assert!(!is_valid_iso_date("24-01-15")); // two-digit year
    assert!(!is_valid_iso_date("15/01/2024")); // slash format not supported
    assert!(!is_valid_iso_date("2024/01/15")); // slash format not supported
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
