use super::*;
#[test]
fn actual_condition_update_preserves_longer_only_for_source_allowed_ticks() {
    assert_eq!(
        merge_familiar_cooldown_deadline(Some(u64::MAX), 2000, 1000, 1000),
        u64::MAX
    );
    assert_eq!(
        merge_familiar_cooldown_deadline(Some(u64::MAX), 1000, 0, 1000),
        1000
    );
    assert_eq!(
        merge_familiar_cooldown_deadline(Some(u64::MAX), 1000, -2, 1000),
        1000
    );
    assert_eq!(
        merge_familiar_cooldown_deadline(Some(5000), 2000, 1000, 1000),
        5000
    );
    assert_eq!(
        merge_familiar_cooldown_deadline(Some(5000), u64::MAX, -1, 1000),
        u64::MAX
    );
    assert_eq!(
        merge_familiar_cooldown_deadline(Some(5000), 1000, -2, 1000),
        1000
    );
}
