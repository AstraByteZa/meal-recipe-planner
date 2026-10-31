//! Last-write-wins comparison for synced rows.

/// Returns true if an incoming change should replace the current version of a row.
///
/// Timestamps are compared as text, so both must use the `updated_at` format
/// (`%Y-%m-%dT%H:%M:%fZ`, UTC, fixed width). Ties on the timestamp go to the larger client id,
/// so every replica picks the same winner. An identical pair is not newer, which makes replaying
/// the same change a no-op.
pub fn lww_wins(incoming_ts: &str, incoming_client: &str, current_ts: &str, current_client: &str) -> bool {
    (incoming_ts, incoming_client) > (current_ts, current_client)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EARLY: &str = "2026-10-31T10:00:00.000Z";
    const LATE: &str = "2026-10-31T10:00:00.001Z";

    #[test]
    fn later_timestamp_wins() {
        assert!(lww_wins(LATE, "a", EARLY, "z"));
        assert!(!lww_wins(EARLY, "z", LATE, "a"));
    }

    #[test]
    fn tie_goes_to_the_larger_client_id() {
        assert!(lww_wins(EARLY, "b", EARLY, "a"));
        assert!(!lww_wins(EARLY, "a", EARLY, "b"));
    }

    #[test]
    fn replaying_the_same_change_is_not_newer() {
        assert!(!lww_wins(EARLY, "a", EARLY, "a"));
    }

    #[test]
    fn fixed_width_text_orders_by_time() {
        // Milliseconds with a fixed width keep text order the same as time order.
        assert!(lww_wins("2026-10-31T10:00:00.100Z", "a", "2026-10-31T10:00:00.099Z", "a"));
        assert!(lww_wins("2026-11-01T00:00:00.000Z", "a", "2026-10-31T23:59:59.999Z", "a"));
    }
}
