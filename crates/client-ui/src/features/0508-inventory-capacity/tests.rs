use super::*;

#[test]
fn capacity_line_shows_whole_ounces_of_free_capacity_only_once_sent() {
    let mut panel = InventoryPanel::default();
    assert_eq!(cap_text(&panel), None);
    apply(
        &mut panel,
        InventoryCapacity {
            capacity: 60_000,
            free: 58_799,
        },
    );
    assert_eq!(cap_text(&panel).as_deref(), Some("Cap: 587"));
    apply(
        &mut panel,
        InventoryCapacity {
            capacity: 60_000,
            free: 0,
        },
    );
    assert_eq!(cap_text(&panel).as_deref(), Some("Cap: 0"));
}
