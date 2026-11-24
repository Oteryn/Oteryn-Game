//! Personal boss defeat domain revisions trigger the screenshot preference.
pub fn advanced(current: u64, previous: u64) -> bool {
    current > previous
}

#[cfg(test)]
#[test]
fn observations() {
    assert!(advanced(3, 2));
    assert!(!advanced(3, 3));
}
