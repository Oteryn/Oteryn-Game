//! VIP0 metadata bounds. Presence is deliberately owned by VIP-2.
use std::collections::BTreeSet;

/// Returns canonical metadata only when every accepted VIP0 bound holds.
pub fn metadata(text: &str, icon: u8, groups: &[u8]) -> Option<String> {
    let text = text.trim();
    (text.chars().count() <= 128
        && text.len() <= 512
        && !text.chars().any(char::is_control)
        && icon <= 10
        && groups.len() <= 8
        && groups.iter().all(|g| (1..=8).contains(g))
        && groups.iter().collect::<BTreeSet<_>>().len() == groups.len())
    .then(|| text.to_owned())
}
/// Canonical custom group label; comparison is case insensitive in the store.
pub fn group_name(text: &str) -> Option<String> {
    let text = text.trim();
    (!text.is_empty()
        && text.chars().count() <= 25
        && text.len() <= 100
        && !text.chars().any(char::is_control))
    .then(|| text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn vip_metadata_and_group_bounds() {
        assert_eq!(metadata("  friend  ", 10, &[1, 8]), Some("friend".into()));
        assert!(metadata(&"𐀀".repeat(128), 0, &[]).is_some());
        assert!(metadata(&"a".repeat(129), 0, &[]).is_none());
        for (text, icon, groups) in [
            ("a\nb", 0, vec![]),
            ("ok", 11, vec![]),
            ("ok", 0, vec![1, 1]),
            ("ok", 0, vec![9]),
        ] {
            assert!(metadata(text, icon, &groups).is_none());
        }
        assert!(group_name(&"𐀀".repeat(25)).is_some());
        assert!(group_name(&"a".repeat(26)).is_none());
        assert!(group_name(" ").is_none());
        assert!(group_name("A\tB").is_none());
    }
}
