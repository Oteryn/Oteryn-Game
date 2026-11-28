use crate::qualification::client_world::g17_validate_offer_source;

#[test]
fn g17_only_accepts_an_offer_created_for_the_current_admission() {
    let current = [1; 16];
    let prior = [2; 16];
    assert!(g17_validate_offer_source(Some(&current), &current).is_ok());
    assert!(g17_validate_offer_source(Some(&prior), &current).is_err());
    assert!(g17_validate_offer_source(None, &current).is_err());
}
