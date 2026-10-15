//! LOGOUT-WIRE-1 (ARCH-KILL-REWARD-LOGOUT-1 §1.6, §2.2): the in-fight retry and the terminal
//! release of a logout.

use super::logout::in_fight_retry_after_ms;

#[test]
fn the_in_fight_retry_rounds_up_to_whole_milliseconds_within_1_to_60000() {
    assert_eq!(in_fight_retry_after_ms(1_000_001, 1_000_000), 1);
    assert_eq!(in_fight_retry_after_ms(1_001_000, 1_000_000), 1);
    assert_eq!(in_fight_retry_after_ms(1_001_001, 1_000_000), 2);
    assert_eq!(in_fight_retry_after_ms(61_000_000, 1_000_000), 60_000);
    assert_eq!(in_fight_retry_after_ms(u64::MAX, 0), 60_000);
    // A deadline that already ran out between the read and the reply still says 1.
    assert_eq!(in_fight_retry_after_ms(1_000_000, 1_000_000), 1);
}
