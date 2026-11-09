use oteryn_client::play::AudioSettings;
use oteryn_session::PresentationSource;
#[test]
fn battle_sound_source_gain_multiplies_master_and_mute() {
    let mut settings = AudioSettings {
        master_volume: 50,
        battle_volumes: [0, 40, 80],
        ..AudioSettings::default()
    };
    assert_eq!(settings.source_gain(PresentationSource::Own), 0.0);
    assert!((settings.source_gain(PresentationSource::Others) - 0.2).abs() < 0.001);
    assert!((settings.source_gain(PresentationSource::Creatures) - 0.4).abs() < 0.001);
    assert_eq!(settings.source_gain(PresentationSource::Global), 0.5);
    settings.muted = true;
    assert_eq!(settings.source_gain(PresentationSource::Creatures), 0.0);
}
