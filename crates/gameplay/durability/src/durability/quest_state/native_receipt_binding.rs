//! Exact replay of native-death bindings from main and the published PC3 lane.
pub(super) fn matches(stored: &[u8], canonical: &[u8; 33], legacy: Option<&[u8; 33]>) -> bool {
    stored == canonical || legacy.is_some_and(|legacy| stored == legacy)
}

#[cfg(test)]
mod tests {
    use super::matches;
    #[test]
    fn only_exact_canonical_or_explicit_legacy_receipts_replay() {
        let canonical = [1; 33];
        let legacy = [2; 33];
        assert!(matches(&canonical, &canonical, None));
        assert!(!matches(&legacy, &canonical, None));
        assert!(matches(&legacy, &canonical, Some(&legacy)));
        assert!(!matches(&[3; 33], &canonical, Some(&legacy)));
        assert!(!matches(&legacy[..32], &canonical, Some(&legacy)));
    }
}
