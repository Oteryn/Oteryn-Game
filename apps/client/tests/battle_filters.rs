use oteryn_client::play::{AudioSettings, BattleClass, SoundChannel};
use oteryn_session::PresentationSource;
#[test]
fn battle_filters_separate_source_and_spell_class() {
    let mut settings = AudioSettings::default();
    settings.battle_filters[0][1] = false;
    assert_eq!(
        settings.classified_gain(
            PresentationSource::Own,
            SoundChannel::Battle,
            Some(BattleClass::Healing)
        ),
        0.0
    );
    assert_eq!(
        settings.classified_gain(
            PresentationSource::Others,
            SoundChannel::Battle,
            Some(BattleClass::Healing)
        ),
        1.0
    );
    assert_eq!(
        settings.classified_gain(
            PresentationSource::Own,
            SoundChannel::Battle,
            Some(BattleClass::Attack)
        ),
        1.0
    );
}

#[test]
fn confirmed_battle_cues_respect_each_source_and_class() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client::play::{PlayView, SoundCatalog, play_channel};
    use oteryn_session::{
        ActorPosition, AppliedDelta, PresentationEvent, PresentationEventKind, SessionEvent,
    };
    let root = std::env::temp_dir().join(format!("oteryn-battleclass-{}", std::process::id()));
    std::fs::create_dir_all(&root)?;
    let metadata = b"test metadata";
    let audio = b"test audio bytes";
    std::fs::write(root.join("sounds.dat"), metadata)?;
    std::fs::write(root.join("track.ogg"), audio)?;
    let manifest = serde_json::json!({
        "schema": "OTERYN_CLIENT_ASSET_MANIFEST/v1", "sound_data": "sounds.dat",
        "sound_cues": { "1": ["track.ogg"], "2": ["track.ogg"], "3": ["track.ogg"] },
        "battle_classes": { "1": "attack", "2": "healing", "3": "support" },
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
    preferences.audio.battle_volumes = [80, 60, 40];
    preferences.audio.battle_filters = [
        [true, false, true],
        [false, true, true],
        [true, true, false],
    ];
    view.set_preferences(preferences)?;
    outbox.push(vec![SessionEvent::Presentation(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: (0..9)
            .map(|n| PresentationEvent {
                decision_ordinal: n + 1,
                emission_ordinal: 1,
                kind: PresentationEventKind::Sound {
                    position: ActorPosition {
                        x: 0,
                        y: 0,
                        floor: 7,
                    },
                    cue_id: n % 3 + 1,
                    source: [
                        PresentationSource::Own,
                        PresentationSource::Others,
                        PresentationSource::Creatures,
                    ][n as usize / 3],
                    secondary_cue_id: 0,
                },
            })
            .collect(),
    })]);
    assert!(view.tick(&link).is_ok());
    let cues = view.take_sound_cues();
    assert_eq!(cues.len(), 9);
    for (cue, expected) in cues
        .iter()
        .zip([0.4, 0.0, 0.4, 0.0, 0.3, 0.3, 0.2, 0.2, 0.0])
    {
        assert_eq!(catalog.read(cue.id, 0)?, Some(audio.to_vec()));
        let gain = view.preferences().audio.classified_gain(
            cue.source,
            catalog.channel(cue.id),
            catalog.battle_class(cue.id),
        );
        assert!((gain - expected).abs() < 0.001);
    }
    let mut invalid = manifest;
    invalid["battle_classes"]["999"] = "attack".into();
    assert!(oteryn_client::play::decode_mapping(&serde_json::to_vec(&invalid)?).is_err());
    std::fs::remove_dir_all(root)?;
    Ok(())
}
