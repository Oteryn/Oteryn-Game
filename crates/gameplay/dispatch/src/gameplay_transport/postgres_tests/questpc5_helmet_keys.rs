//! Reuse the delivered native key wire/replay/relog program at the exact helmet source doors.
use super::TestResult;
#[test]
fn questpc5_helmet_source_key_doors_use_replay_and_relog() -> TestResult {
    super::m06_keys::source_key_doors(&[
        (33211, 31634, -13, 6010),
        (32462, 32151, -15, 3610),
        (32462, 32153, -14, 3610),
        (32592, 32102, -14, 3667),
    ])
}
