#[test]
fn unregistered_and_sibling_chords_are_reserved() -> Result<(), Box<dyn std::error::Error>> {
    for chord in [
        "CTRL+G",
        "ENTER",
        "KEYPADENTER",
        "CTRL+SHIFT+G",
        "CTRL+SHIFT+L",
        "CTRL+P",
    ] {
        let mut preferences = crate::play::ReferencePreferences::default();
        assert!(
            preferences.rebind("0009-highscores-open", chord).is_err(),
            "{chord}"
        );
    }
    Ok(())
}
