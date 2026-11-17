use oteryn_client::play::AudioSettings;
#[test]
fn ambient_channel_gain_is_independent_and_honours_master_mute() {
    let mut settings = AudioSettings {
        master_volume: 50,
        ambient_volume: 40,
        ..AudioSettings::default()
    };
    assert!((settings.ambient_gain() - 0.2).abs() < 0.001);
    assert_eq!(settings.gain(), 0.5);
    settings.ambient_volume = 0;
    assert_eq!(settings.ambient_gain(), 0.0);
    settings.ambient_volume = 100;
    settings.muted = true;
    assert_eq!(settings.ambient_gain(), 0.0);
}
