use super::*;

#[test]
fn promotion_requires_level_twenty_and_is_bought_once() {
    assert!(!eligible(19, false));
    assert!(eligible(20, false));
    assert!(!eligible(20, true));
    assert_eq!(PROMOTION_PRICE, 20_000);
}

#[test]
fn promotion_after_delivery_fails_closed_and_login_snapshot_expires_only_at_login() {
    assert!(!purchase_permitted(&PremiumStatus::NotCurrent));
    assert!(purchase_permitted(&PremiumStatus::Current));
    assert!(login_projection(true, true));
    assert!(!login_projection(true, false));
    assert!(!login_projection(false, true));
}
