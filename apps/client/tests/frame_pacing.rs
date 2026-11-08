use oteryn_client::play::ReferencePreferences;

#[test]
fn frame_limits_select_focus_and_reject_zero() -> Result<(), Box<dyn std::error::Error>> {
    let mut preferences = ReferencePreferences::default();
    assert_eq!(
        preferences.frame_interval(true).as_nanos(),
        1_000_000_000 / 60
    );
    assert_eq!(
        preferences.frame_interval(false).as_nanos(),
        1_000_000_000 / 15
    );
    preferences.foreground_fps = 144;
    preferences.background_fps = 5;
    assert_eq!(
        preferences.frame_interval(true).as_nanos(),
        1_000_000_000 / 144
    );
    assert_eq!(preferences.frame_interval(false).as_nanos(), 200_000_000);
    let path = std::env::temp_dir().join(format!("frame-limit-{}.json", std::process::id()));
    preferences.foreground_fps = 0;
    assert!(oteryn_client::play::PreferenceStore::export_to(&preferences, &path).is_err());
    Ok(())
}
