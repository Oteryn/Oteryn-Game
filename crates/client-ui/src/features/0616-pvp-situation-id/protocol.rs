//! Incoming situation identities distinguish a renewed attack from later observations.
pub fn renewed(current: &[u64], previous: &[u64]) -> bool {
    current.iter().any(|id| !previous.contains(id))
}

#[cfg(test)]
#[test]
fn observations() {
    assert!(renewed(&[3], &[2]));
    assert!(!renewed(&[2, 1], &[1, 2]));
}
