use super::*;

#[test]
fn every_catalog_key_has_a_help_text_and_unknown_keys_none() {
    for key in KEYS {
        assert!(help_text(key).is_some_and(|text| !text.is_empty()), "{key}");
    }
    assert_eq!(help_text("unknown-panel"), None);
}

#[test]
fn lens_mode_toggles_and_a_click_ends_it() {
    let mut lens = LensHelp::default();
    assert!(!lens.active);
    assert!(lens.toggle());
    assert_ne!(mode_text(true), mode_text(false));
    lens.finish();
    assert!(!lens.active);
    assert!(lens.toggle());
    assert!(!lens.toggle());
}
