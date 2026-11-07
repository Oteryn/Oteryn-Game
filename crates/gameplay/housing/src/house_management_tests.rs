use super::*;
#[test]
fn house_move_out_dates_include_one_and_thirty_days_and_reject_overflow() {
    let now = 1_000_000;
    assert!(valid_move_out_date(now, now + DAY_MS));
    assert!(valid_move_out_date(now, now + 30 * DAY_MS));
    for due in [0, now - 1, now, now + DAY_MS - 1, now + 30 * DAY_MS + 1] {
        assert!(!valid_move_out_date(now, due));
    }
    assert!(!valid_move_out_date(u64::MAX, 0));
}
#[test]
fn house_management_list_keys_keep_native_door_coordinates() {
    assert_eq!(HouseManagementList::Guest.key().as_deref(), Some("guest"));
    assert_eq!(
        HouseManagementList::Subowner.key().as_deref(),
        Some("subowner")
    );
    assert_eq!(
        HouseManagementList::Door {
            x: 65535,
            y: 0,
            floor: -15
        }
        .key()
        .as_deref(),
        Some("door:65535:0:-15")
    );
    assert_eq!(
        HouseManagementList::Door {
            x: 0,
            y: 0,
            floor: 1
        }
        .key(),
        None
    );
}
