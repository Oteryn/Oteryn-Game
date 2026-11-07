use super::*;

#[test]
fn house_move_out_absolute_bounds_and_overflow() {
    for (offset, valid) in [
        (NOTICE_MIN_MS - 1, false),
        (NOTICE_MIN_MS, true),
        (NOTICE_MAX_MS, true),
        (NOTICE_MAX_MS + 1, false),
    ] {
        assert_eq!(valid_deadline(100, 100 + offset), valid);
    }
    assert!(!valid_deadline(-1, i64::MAX));
    assert!(!valid_deadline(i64::MAX, i64::MIN));
    assert!(!valid_deadline(100, 99));
}
