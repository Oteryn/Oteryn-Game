use super::*;

const fn assert_house_preflight() {
    assert!(
        matches!(validate_house_qualification(Some(true), true, false), Some(true)),
        "reset preflight refuses a verified same-content House payload"
    );
    assert!(validate_house_qualification(Some(true), false, false).is_none());
    assert!(validate_house_qualification(None, true, true).is_none());
    assert!(validate_house_qualification(Some(false), true, false).is_none());
    assert!(matches!(validate_house_qualification(Some(false), false, true), Some(false)));
}

// Exercise the same assertions in the Cloud's compile-only test lane.
const _: () = assert_house_preflight();

#[test]
fn reset_preflight_requires_verified_house_tiles_and_same_content() {
    assert_house_preflight();
}
