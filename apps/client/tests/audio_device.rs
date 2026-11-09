use oteryn_client::play::select_audio_device;
#[test]
fn requested_audio_device_is_exact_and_never_silently_falls_back() {
    let names = vec!["Speakers".to_owned(), "Headphones".to_owned()];
    assert_eq!(select_audio_device("", &names), Ok(None));
    assert_eq!(select_audio_device("Headphones", &names), Ok(Some(1)));
    assert!(select_audio_device("Disconnected", &names).is_err());
}

#[test]
fn missing_device_choice_survives_preference_reload() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client::play::{PreferenceStore, ReferencePreferences};
    let path =
        std::env::temp_dir().join(format!("oteryn-device-choice-{}.json", std::process::id()));
    let preferences = ReferencePreferences {
        audio_device: "Disconnected headphones".into(),
        ..ReferencePreferences::default()
    };
    PreferenceStore::export_to(&preferences, &path)?;
    let store = PreferenceStore::load(path.clone())?;
    assert_eq!(store.preferences().audio_device, "Disconnected headphones");
    assert!(select_audio_device(&store.preferences().audio_device, &[]).is_err());
    std::fs::remove_file(path)?;
    Ok(())
}
