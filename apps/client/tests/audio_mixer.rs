use oteryn_client::play::{AudioSettings, SoundChannel};
use oteryn_session::PresentationSource;
#[test]
fn mixer_channels_are_independent_and_master_mute_wins() {
    let mut settings = AudioSettings {
        master_volume: 50,
        channel_volumes: [20, 40, 60, 80],
        ..AudioSettings::default()
    };
    for (channel, expected) in [
        (SoundChannel::Music, 0.1),
        (SoundChannel::Items, 0.2),
        (SoundChannel::Events, 0.3),
        (SoundChannel::Ui, 0.4),
    ] {
        assert!(
            (settings.channel_gain(PresentationSource::Global, channel) - expected).abs() < 0.001
        );
    }
    settings.muted = true;
    assert_eq!(
        settings.channel_gain(PresentationSource::Global, SoundChannel::Music),
        0.0
    );
}

#[test]
fn confirmed_sound_cues_use_pinned_mixer_routes() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client::play::{PlayView, SoundCatalog, play_channel};
    use oteryn_session::{
        ActorPosition, AppliedDelta, PresentationEvent, PresentationEventKind, SessionEvent,
    };
    let root = std::env::temp_dir().join(format!("oteryn-mixer-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let metadata = b"test metadata";
    let audio = b"test audio bytes";
    std::fs::write(root.join("sounds.dat"), metadata)?;
    std::fs::write(root.join("track.ogg"), audio)?;
    let manifest = serde_json::json!({
        "schema": "OTERYN_CLIENT_ASSET_MANIFEST/v1", "sound_data": "sounds.dat",
        "sound_cues": { "1": ["track.ogg"], "2": ["track.ogg"], "3": ["track.ogg"] },
        "sound_channels": { "1": "music", "2": "items", "3": "events" },
        "files": [
            { "name": "sounds.dat", "sha256": oteryn_client_assets::text_sha256(metadata) },
            { "name": "track.ogg", "sha256": oteryn_client_assets::text_sha256(audio) }
        ]
    });
    let bytes = serde_json::to_vec(&manifest)?;
    let path = root.join("manifest.json");
    std::fs::write(&path, &bytes)?;
    let catalog = SoundCatalog::open(&root, &path, &oteryn_client_assets::text_sha256(&bytes))?;
    let (link, _commands, outbox) = play_channel();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 7, &[])?;
    let mut preferences = view.preferences().clone();
    preferences.audio.master_volume = 50;
    preferences.audio.channel_volumes = [20, 40, 60, 80];
    view.set_preferences(preferences)?;
    outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: (1..=3)
            .map(|id| PresentationEvent {
                decision_ordinal: id,
                emission_ordinal: 1,
                kind: PresentationEventKind::Sound {
                    position: ActorPosition {
                        x: 0,
                        y: 0,
                        floor: 7,
                    },
                    cue_id: id,
                    source: PresentationSource::Global,
                    secondary_cue_id: 0,
                },
            })
            .collect(),
    })]);
    assert!(view.tick(&link).is_ok());
    let cues = view.take_sound_cues();
    assert_eq!(cues.len(), 3);
    for (cue, expected) in cues.iter().zip([0.1, 0.2, 0.3]) {
        assert_eq!(catalog.read(cue.id, 0)?, Some(audio.to_vec()));
        let gain = view
            .preferences()
            .audio
            .channel_gain(cue.source, catalog.channel(cue.id));
        assert!((gain - expected).abs() < 0.001);
    }
    let mut invalid = manifest;
    invalid["sound_channels"]["999"] = "music".into();
    assert!(oteryn_client::play::decode_mapping(&serde_json::to_vec(&invalid)?).is_err());
    std::fs::remove_dir_all(root)?;
    Ok(())
}
