//! The timestamp format used by `updated_at` on both the client and the server.

use chrono::{DateTime, SecondsFormat, Utc};

/// Formats a time the way `updated_at` is stored: UTC with milliseconds and a `Z` suffix.
/// Fixed width, so text comparison matches time order (see `lww::lww_wins`).
pub fn format_timestamp(time: DateTime<Utc>) -> String {
    time.to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Parses a timestamp sent by a client. Any RFC 3339 offset is accepted.
pub fn parse_timestamp(text: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(text)
        .map(|time| time.with_timezone(&Utc))
        .map_err(|_| format!("'{text}' is not an RFC 3339 timestamp"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_is_fixed_width_utc_milliseconds() {
        let time = parse_timestamp("2026-10-31T12:00:00+02:00").unwrap();
        assert_eq!(format_timestamp(time), "2026-10-31T10:00:00.000Z");
    }

    #[test]
    fn rejects_text_that_is_not_a_timestamp() {
        assert!(parse_timestamp("yesterday").is_err());
    }
}
