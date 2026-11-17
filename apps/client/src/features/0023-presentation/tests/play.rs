use crate::play::feature_0023_presentation::{input::audio_input, state::*};
use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
use oteryn_session::{AppliedDelta, PresentationEvent, PresentationEventKind, PresentationSource};

#[test]
fn audio_controls_preserve_volume_across_mute_and_ignore_repeat()
-> Result<(), Box<dyn std::error::Error>> {
    let mut settings = AudioSettings::default();
    let code = KeyCode::new(67)?;
    let key = |repeat| NormalizedInputEvent::Key {
        code,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat,
    };
    assert!(audio_input(&mut settings, &[key(false)]));
    assert_eq!(settings.gain(), 0.0);
    assert!(!audio_input(&mut settings, &[key(true)]));
    assert!(settings.muted);
    assert!(audio_input(&mut settings, &[key(false)]));
    assert_eq!(settings.gain(), 1.0);
    settings.master_volume = 40;
    assert_eq!(settings.gain(), 0.4);
    for (code, expected) in [(68, 0), (69, 100)] {
        settings.master_volume = if code == 68 { 0 } else { 100 };
        assert!(audio_input(
            &mut settings,
            &[NormalizedInputEvent::Key {
                code: KeyCode::new(code)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::NONE,
                repeat: false,
            }]
        ));
        assert_eq!(settings.master_volume, expected);
    }
    settings.master_volume = 40;
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    let mut preferences = view.preferences().clone();
    preferences.audio = settings;
    view.set_preferences(preferences.clone())?;
    assert_eq!(view.preferences().audio.master_volume, 40);
    preferences.audio.master_volume = 101;
    assert!(view.set_preferences(preferences).is_err());
    Ok(())
}

#[test]
fn audio_synthetic_flow_keeps_sources_secondary_cues_and_bounds_pending_events()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = crate::play::play_channel();
    let events = [
        PresentationSource::Own,
        PresentationSource::Others,
        PresentationSource::Creatures,
        PresentationSource::Global,
    ]
    .into_iter()
    .map(|source| PresentationEvent {
        decision_ordinal: 1,
        emission_ordinal: 1,
        kind: PresentationEventKind::Sound {
            position: oteryn_session::ActorPosition {
                x: 0,
                y: 0,
                floor: 0,
            },
            cue_id: 41,
            source,
            secondary_cue_id: 42,
        },
    })
    .collect();
    outbox.push(vec![oteryn_session::SessionEvent::Presentation(
        AppliedDelta {
            server_sequence: 41,
            base_revision: 0,
            new_revision: 1,
            value: events,
        },
    )]);
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    assert!(view.tick(&link).is_ok());
    let cues = view.take_sound_cues();
    assert_eq!(cues.len(), 8);
    assert_eq!(
        cues[0],
        SoundCue {
            id: 41,
            source: PresentationSource::Own,
            ambient: false,
        }
    );
    assert_eq!(
        cues[7],
        SoundCue {
            id: 42,
            source: PresentationSource::Global,
            ambient: false,
        }
    );
    assert!(view.take_sound_cues().is_empty());
    for _ in 0..200 {
        outbox.push(vec![oteryn_session::SessionEvent::Presentation(
            AppliedDelta {
                server_sequence: 42,
                base_revision: 1,
                new_revision: 2,
                value: vec![PresentationEvent {
                    decision_ordinal: 1,
                    emission_ordinal: 1,
                    kind: PresentationEventKind::Sound {
                        position: oteryn_session::ActorPosition {
                            x: 0,
                            y: 0,
                            floor: 0,
                        },
                        cue_id: 41,
                        source: PresentationSource::Own,
                        secondary_cue_id: 0,
                    },
                }],
            },
        )]);
    }
    assert!(view.tick(&link).is_ok());
    assert_eq!(view.take_sound_cues().len(), MAX_PENDING_SOUNDS);
    Ok(())
}

#[test]
fn audio_mapping_rejects_unsafe_paths_and_catalog_verifies_audio_bytes()
-> Result<(), Box<dyn std::error::Error>> {
    use std::fs;
    let dir = std::env::temp_dir().join(format!("oteryn-c28-{}", std::process::id()));
    fs::create_dir_all(&dir)?;
    let metadata = b"synthetic metadata";
    let audio = b"synthetic audio placeholder";
    fs::write(dir.join("sounds.dat"), metadata)?;
    fs::write(dir.join("a.ogg"), audio)?;
    let manifest = serde_json::json!({
        "schema": "OTERYN_CLIENT_ASSET_MANIFEST/v1", "sound_data": "sounds.dat",
        "sound_cues": {"41": ["a.ogg"]},
        "files": [
            {"name": "sounds.dat", "sha256": oteryn_client_assets::text_sha256(metadata)},
            {"name": "a.ogg", "sha256": oteryn_client_assets::text_sha256(audio)}
        ]
    });
    let bytes = serde_json::to_vec(&manifest)?;
    let path = dir.join("manifest.json");
    fs::write(&path, &bytes)?;
    let pin = oteryn_client_assets::text_sha256(&bytes);
    let catalog = SoundCatalog::open(&dir, &path, &pin)?;
    assert_eq!(catalog.read(41, 7)?, Some(audio.to_vec()));
    assert_eq!(catalog.read(65535, 0)?, None);
    fs::write(dir.join("a.ogg"), b"changed")?;
    assert!(catalog.read(41, 0).is_err());
    assert!(SoundCatalog::open(&dir, &path, &"0".repeat(64)).is_err());
    let mut invalid = manifest;
    invalid["sound_cues"]["41"] = serde_json::json!(["../a.ogg"]);
    assert!(decode_mapping(&serde_json::to_vec(&invalid)?).is_err());
    assert!(decode_mapping(b"{").is_err());
    fs::remove_dir_all(dir)?;
    Ok(())
}

#[test]
fn combat_presentation_synthetic_flow_reaches_scene_and_expires()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = crate::play::play_channel();
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    let baseline = view.scene().sprites().instances().len();
    outbox.push(vec![oteryn_session::SessionEvent::Presentation(
        AppliedDelta {
            server_sequence: 42,
            base_revision: 0,
            new_revision: 1,
            value: vec![PresentationEvent {
                decision_ordinal: 1,
                emission_ordinal: 1,
                kind: PresentationEventKind::MagicEffect {
                    position: oteryn_session::ActorPosition {
                        x: 0,
                        y: 0,
                        floor: 0,
                    },
                    effect_id: 1,
                    source: PresentationSource::Own,
                },
            }],
        },
    )]);
    assert!(view.tick(&link).is_ok());
    assert!(view.scene().sprites().instances().len() > baseline);
    view.advance_presentation(std::time::Instant::now() + std::time::Duration::from_secs(2))?;
    assert_eq!(view.scene().sprites().instances().len(), baseline);
    Ok(())
}

fn value_event(floor: i16) -> PresentationEventKind {
    PresentationEventKind::ValueText {
        kind: oteryn_session::PresentationValueKind::Healed,
        position: oteryn_session::ActorPosition { x: 0, y: 0, floor },
        target: oteryn_session::EntityRef {
            identity: [1; 16],
            generation: 1,
        },
        attacker: None,
        primary: oteryn_session::PresentationValue {
            value: u64::MAX,
            color: oteryn_session::PresentationColor::LightGreen,
        },
        secondary: Some(oteryn_session::PresentationValue {
            value: 12,
            color: oteryn_session::PresentationColor::Blue,
        }),
    }
}

#[test]
fn combat_values_float_keep_secondary_and_cull_other_floors()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    let base = view.scene().sprites().len();
    let now = std::time::Instant::now();
    view.receive_presentation(&[value_event(1)], now);
    view.project_presentation(now)?;
    assert_eq!(view.scene().sprites().len(), base);
    view.receive_presentation(&[value_event(0)], now);
    view.project_presentation(now)?;
    let before = view.scene().sprites().instances()[base..].to_vec();
    assert!(!before.is_empty());
    assert!(
        before
            .iter()
            .any(|quad| quad.position[1] < before[0].position[1])
    );
    view.advance_presentation(now + std::time::Duration::from_millis(250))?;
    let after = &view.scene().sprites().instances()[base..];
    assert_eq!(before.len(), after.len());
    for (a, b) in before.iter().zip(after) {
        assert_eq!(a.position[0], b.position[0]);
        assert_eq!(a.position[1] - 8.0, b.position[1]);
    }
    view.advance_presentation(now + VISUAL_LIFETIME)?;
    assert_eq!(view.scene().sprites().len(), base);
    Ok(())
}

#[test]
fn combat_visual_queues_are_bounded_across_mailbox_coalescing()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = crate::play::play_channel();
    for sequence in 1..=3 {
        outbox.push(vec![oteryn_session::SessionEvent::Presentation(
            AppliedDelta {
                server_sequence: sequence,
                base_revision: sequence - 1,
                new_revision: sequence,
                value: (1..=MAX_VISUAL_CUES)
                    .map(|ordinal| PresentationEvent {
                        decision_ordinal: 1,
                        emission_ordinal: ordinal as u32,
                        kind: value_event(0),
                    })
                    .collect(),
            },
        )]);
    }
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    assert!(view.tick(&link).is_ok());
    assert_eq!(view.visuals.len(), MAX_VISUAL_CUES);
    view.receive_presentation(&[value_event(0)], std::time::Instant::now());
    assert_eq!(view.visuals.len(), MAX_VISUAL_CUES);
    assert!(view.scene().sprites().len() < oteryn_renderer::MAX_BATCH_QUADS);
    Ok(())
}

#[test]
fn combat_visuals_keep_world_position_when_camera_moves_and_cull_offscreen()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    let base = view.scene().sprites().len();
    let now = std::time::Instant::now();
    view.receive_presentation(&[value_event(0)], now);
    view.project_presentation(now)?;
    let before = view.scene().sprites().instances()[base].position[0];
    view.apply_pushed(&crate::play::Pushed {
        own: Some(oteryn_session::ActorPosition {
            x: 1,
            y: 0,
            floor: 0,
        }),
        ..Default::default()
    })?;
    view.project_presentation(now)?;
    assert_eq!(
        view.scene().sprites().instances()[base].position[0],
        before - crate::scene::SCENE_TILE_PX as f32
    );
    view.apply_pushed(&crate::play::Pushed {
        own: Some(oteryn_session::ActorPosition {
            x: 100,
            y: 100,
            floor: 0,
        }),
        ..Default::default()
    })?;
    assert_eq!(view.scene().sprites().len(), base);
    Ok(())
}

fn numbered_value(value: u64) -> PresentationEventKind {
    let mut event = value_event(0);
    if let PresentationEventKind::ValueText {
        primary, secondary, ..
    } = &mut event
    {
        primary.value = value;
        *secondary = None;
    }
    event
}

#[path = "play_additional.rs"]
mod additional;

#[path = "visual_sprites.rs"]
mod visual_sprites;

#[path = "boss_area.rs"]
mod boss_area;

#[path = "native_missile_merged.rs"]
mod native_missile_merged;
