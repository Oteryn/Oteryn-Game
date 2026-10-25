//! Basic Oteryn world gathering: alternate reset epochs, until the next reset.
//! The reset owner selects this once; players can only read its announcement.

/// Epoch zero is uninitialized gameplay. Odd epochs announce a gathering; the
/// following even epoch closes it. This is Oteryn content, not a sourced map event.
pub fn announcement_for_epoch(epoch: u64) -> Option<&'static str> {
    (epoch % 2 == 1).then_some("World gathering is active until the next server save.")
}

/// Canary `global_server_save.lua:29,48` warns once a minute from
/// `globalServerSaveNotifyDuration` (5) down to 1 minute before the server save.
pub const SERVER_SAVE_WARNING_MINUTES_MAX: u8 = 5;
const MINUTE_MS: i64 = 60_000;

/// The countdown minute due at `now_unix_ms` before the planned reset at `reset_unix_ms`.
/// Minute `m` is due while `now` lies in `[reset - m min, reset - (m - 1) min)`. A minute
/// whose window opened before `serve_started_unix_ms` is never due, so a restarted serve
/// cannot repeat a notice the previous incarnation may already have sent.
pub fn server_save_warning_due(
    reset_unix_ms: i64,
    now_unix_ms: i64,
    serve_started_unix_ms: i64,
) -> Option<u8> {
    let remaining = reset_unix_ms.checked_sub(now_unix_ms)?;
    if remaining <= 0 {
        return None;
    }
    let minutes = (remaining - 1) / MINUTE_MS + 1;
    let minutes = u8::try_from(minutes)
        .ok()
        .filter(|minutes| *minutes <= SERVER_SAVE_WARNING_MINUTES_MAX)?;
    let opened = reset_unix_ms.checked_sub(i64::from(minutes) * MINUTE_MS)?;
    (opened >= serve_started_unix_ms).then_some(minutes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheduled_world_event_starts_and_stops_only_at_reset() {
        assert_eq!(announcement_for_epoch(0), None);
        assert!(announcement_for_epoch(1).is_some());
        assert_eq!(announcement_for_epoch(2), None);
        assert_eq!(announcement_for_epoch(3), announcement_for_epoch(1));
        assert!(announcement_for_epoch(u64::MAX).is_some());
    }

    const RESET: i64 = 1_800_000_000_000;

    #[test]
    fn server_save_warning_counts_down_each_minute_from_five_to_one() {
        let started = RESET - 10 * MINUTE_MS;
        let due = |now| server_save_warning_due(RESET, now, started);
        assert_eq!(due(RESET - 5 * MINUTE_MS - 1), None);
        assert_eq!(due(RESET - 5 * MINUTE_MS), Some(5));
        assert_eq!(due(RESET - 4 * MINUTE_MS - 1), Some(5));
        assert_eq!(due(RESET - 4 * MINUTE_MS), Some(4));
        assert_eq!(due(RESET - 2 * MINUTE_MS), Some(2));
        assert_eq!(due(RESET - MINUTE_MS), Some(1));
        assert_eq!(due(RESET - 1), Some(1));
        assert_eq!(due(RESET), None);
        assert_eq!(due(RESET + MINUTE_MS), None);
    }

    #[test]
    fn server_save_warning_never_repeats_a_minute_opened_before_the_serve() {
        // A restart 3.5 minutes before the save skips 4 (window already open), sends 3.
        let started = RESET - 7 * MINUTE_MS / 2;
        assert_eq!(server_save_warning_due(RESET, started, started), None);
        assert_eq!(
            server_save_warning_due(RESET, RESET - 3 * MINUTE_MS, started),
            Some(3)
        );
        // A serve that starts exactly as a window opens may still send it.
        let started = RESET - 2 * MINUTE_MS;
        assert_eq!(server_save_warning_due(RESET, started, started), Some(2));
    }

    #[test]
    fn server_save_warning_survives_extreme_clocks() {
        assert_eq!(server_save_warning_due(i64::MIN, i64::MAX, i64::MIN), None);
        assert_eq!(server_save_warning_due(i64::MAX, i64::MIN, i64::MIN), None);
        assert_eq!(
            server_save_warning_due(i64::MIN + 1, i64::MIN, i64::MIN),
            None
        );
    }
}
