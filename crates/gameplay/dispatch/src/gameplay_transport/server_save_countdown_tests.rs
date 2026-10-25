use super::*;

const RESET: i64 = 1_800_000_000_000;
const MINUTE: i64 = 60_000;

fn planned(epoch: u64, reset: i64, now: i64) -> Option<PlannedWorldReset> {
    Some(PlannedWorldReset {
        ending_epoch: epoch,
        reset_at_unix_ms: reset,
        database_now_unix_ms: now,
    })
}

#[test]
fn a_channel_publishes_each_countdown_minute_once_and_sessions_take_it_once() {
    let mut pass = ServerSaveCountdownPass::default();
    let mut sent = None;
    let mut delivered = Vec::new();
    // Every 5 s from 10 minutes before the reset until 1 minute after it.
    for now in (RESET - 10 * MINUTE..RESET + MINUTE).step_by(5_000) {
        if let Some(due) = pass.observe(planned(7, RESET, now))
            && due.follows(sent)
        {
            sent = Some(due.receipt());
            delivered.push(due.notice);
        }
    }
    let minutes: Vec<_> = delivered.iter().map(|n| n.minutes_remaining).collect();
    assert_eq!(minutes, [5, 4, 3, 2, 1]);
    assert!(delivered.iter().all(|n| n.ending_epoch == 7));
}

#[test]
fn a_restarted_channel_never_repeats_a_minute_whose_window_already_opened() {
    let mut pass = ServerSaveCountdownPass::default();
    assert_eq!(
        pass.observe(planned(7, RESET, RESET - 4 * MINUTE + 1)),
        None
    );
    let due = pass.observe(planned(7, RESET, RESET - 3 * MINUTE));
    assert_eq!(due.map(|d| d.notice.minutes_remaining), Some(3));
}

#[test]
fn no_planned_reset_or_a_retiring_one_publishes_nothing() {
    let mut pass = ServerSaveCountdownPass::default();
    assert_eq!(pass.observe(None), None);
    assert_eq!(pass.observe(planned(7, RESET, RESET - 6 * MINUTE)), None);
}

#[test]
fn a_clock_step_back_and_a_skipped_reset_are_told_apart() {
    let due = |reset, minutes| DueServerSaveCountdown {
        reset_at_unix_ms: reset,
        notice: ServerSaveCountdown {
            ending_epoch: 7,
            minutes_remaining: minutes,
        },
    };
    assert!(!due(RESET, 2).follows(Some((RESET, 2))));
    assert!(!due(RESET, 3).follows(Some((RESET, 2))));
    assert!(due(RESET, 1).follows(Some((RESET, 2))));
    // The same epoch a day later: Control did not reset, so the countdown runs again.
    assert!(due(RESET + 1_440 * MINUTE, 5).follows(Some((RESET, 1))));
}
