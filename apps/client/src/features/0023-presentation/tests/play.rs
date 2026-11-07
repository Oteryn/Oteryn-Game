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

#[test]
fn combat_review_overflow_keeps_newest_in_mailbox_and_view()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = crate::play::play_channel();
    for sequence in 1..=2 {
        outbox.push(vec![oteryn_session::SessionEvent::Presentation(
            AppliedDelta {
                server_sequence: sequence,
                base_revision: sequence - 1,
                new_revision: sequence,
                value: (1..=MAX_VISUAL_CUES + 1)
                    .map(|ordinal| PresentationEvent {
                        decision_ordinal: 1,
                        emission_ordinal: ordinal as u32,
                        kind: numbered_value(
                            (sequence - 1) * (MAX_VISUAL_CUES as u64 + 1) + ordinal as u64,
                        ),
                    })
                    .collect(),
            },
        )]);
    }
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    assert!(view.tick(&link).is_ok());
    assert_eq!(view.visuals.len(), MAX_VISUAL_CUES);
    assert_eq!(
        view.visuals[0].kind,
        numbered_value(MAX_VISUAL_CUES as u64 + 3)
    );
    assert_eq!(
        view.visuals[MAX_VISUAL_CUES - 1].kind,
        numbered_value(2 * (MAX_VISUAL_CUES as u64 + 1))
    );
    let now = std::time::Instant::now();
    view.receive_presentation(&[numbered_value(999)], now);
    assert_eq!(view.visuals.len(), MAX_VISUAL_CUES);
    assert_eq!(
        view.visuals[0].kind,
        numbered_value(MAX_VISUAL_CUES as u64 + 4)
    );
    assert_eq!(view.visuals[MAX_VISUAL_CUES - 1].kind, numbered_value(999));
    Ok(())
}

#[test]
fn combat_review_same_tile_values_reserve_separate_lines_in_order()
-> Result<(), Box<dyn std::error::Error>> {
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    let base = view.scene().sprites().len();
    let now = std::time::Instant::now();
    view.receive_presentation(&[numbered_value(8)], now);
    view.project_presentation(now)?;
    let first = view.scene().sprites().instances()[base..].to_vec();
    // The middle cue has a secondary number: it must reserve two lines.
    let mut middle = numbered_value(8);
    if let PresentationEventKind::ValueText {
        secondary, primary, ..
    } = &mut middle
    {
        *secondary = Some(*primary);
    }
    view.receive_presentation(&[middle, numbered_value(8)], now);
    view.project_presentation(now)?;
    let quads = &view.scene().sprites().instances()[base..];
    assert_eq!(quads.len(), first.len() * 4);
    for (line, group) in quads.chunks_exact(first.len()).enumerate() {
        for (a, b) in first.iter().zip(group) {
            assert_eq!(
                b.position,
                [a.position[0], a.position[1] - 12.0 * line as f32]
            );
        }
    }
    Ok(())
}

#[test]
fn novelty_server_effects_decode_into_distinct_native_bursts_and_expire()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_protocol_oteryn::presentation::{
        decode_item_effect_batch, encode_item_effect_batch,
    };
    let mut rendered = Vec::new();
    for effect_id in [29, 30, 31, 28, 27, 24] {
        let event = PresentationEvent {
            decision_ordinal: 1,
            emission_ordinal: 1,
            kind: PresentationEventKind::MagicEffect {
                position: oteryn_session::ActorPosition {
                    x: 0,
                    y: 0,
                    floor: 7,
                },
                effect_id,
                source: PresentationSource::Others,
            },
        };
        let bytes = encode_item_effect_batch(&[event]).map_err(|e| format!("{e:?}"))?;
        let decoded = decode_item_effect_batch(&bytes).map_err(|e| format!("{e:?}"))?;
        assert_eq!(decoded, vec![event]);
        let (link, _commands, outbox) = crate::play::play_channel();
        let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 7, &[])?;
        let base = view.scene().sprites().len();
        outbox.push(vec![oteryn_session::SessionEvent::ItemEffects(
            AppliedDelta {
                server_sequence: 42,
                base_revision: 0,
                new_revision: 1,
                value: decoded,
            },
        )]);
        view.tick(&link).map_err(|e| format!("{e:?}"))?;
        // Fix the animation instant so comparisons prove the server effect ID is used.
        let now = std::time::Instant::now();
        for cue in &mut view.visuals {
            cue.received = now;
        }
        view.project_presentation(now)?;
        let quads = view.scene().sprites().instances()[base..].to_vec();
        assert!(!quads.is_empty());
        rendered.push(quads);
        view.advance_presentation(now + VISUAL_LIFETIME)?;
        assert_eq!(view.scene().sprites().len(), base);
    }
    for (i, one) in rendered.iter().enumerate() {
        for other in &rendered[i + 1..] {
            assert_ne!(
                one, other,
                "distinct donor effect IDs must produce distinct native visuals"
            );
        }
    }
    Ok(())
}

#[test]
fn srx3_native_missile_mailbox_reaches_scene_and_expires() -> Result<(), Box<dyn std::error::Error>>
{
    let (link, _commands, outbox) = crate::play::play_channel();
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    let baseline = view.scene().sprites().instances().len();
    let missile = PresentationEventKind::Missile {
        from: oteryn_session::ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
        to: oteryn_session::ActorPosition {
            x: 2,
            y: 0,
            floor: 0,
        },
        missile_id: 31,
        source: PresentationSource::Creatures,
    };
    outbox.push(vec![oteryn_session::SessionEvent::Presentation(
        AppliedDelta {
            server_sequence: 42,
            base_revision: 0,
            new_revision: 1,
            value: vec![PresentationEvent {
                decision_ordinal: 1,
                emission_ordinal: 1,
                kind: missile,
            }],
        },
    )]);
    view.tick(&link)
        .map_err(|error| format!("native missile play: {error:?}"))?;
    assert!(
        view.scene().sprites().instances().len() > baseline,
        "decoded native missile must reach the play scene"
    );
    assert!(view.visuals.iter().any(|cue| cue.kind == missile));
    let received = view.visuals.last().ok_or("missing missile")?.received;
    view.advance_presentation(received + VISUAL_LIFETIME)?;
    assert_eq!(view.scene().sprites().instances().len(), baseline);
    Ok(())
}

#[test]
fn srx3_native_missile_animates_and_culls_other_floors() -> Result<(), Box<dyn std::error::Error>> {
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
    let baseline = view.scene().sprites().instances().len();
    let now = std::time::Instant::now();
    let missile = |floor| PresentationEventKind::Missile {
        from: oteryn_session::ActorPosition { x: 0, y: 0, floor },
        to: oteryn_session::ActorPosition { x: 2, y: 0, floor },
        missile_id: 31,
        source: PresentationSource::Creatures,
    };
    view.receive_presentation(&[missile(1)], now);
    view.project_presentation(now)?;
    assert_eq!(view.scene().sprites().instances().len(), baseline);
    view.receive_presentation(&[missile(0)], now);
    view.project_presentation(now)?;
    let before = view.scene().sprites().instances()[baseline..].to_vec();
    assert!(!before.is_empty());
    view.advance_presentation(now + std::time::Duration::from_millis(250))?;
    let after = &view.scene().sprites().instances()[baseline..];
    assert_eq!(before.len(), after.len());
    assert!(
        after
            .iter()
            .zip(&before)
            .all(|(a, b)| a.position[0] > b.position[0])
    );
    Ok(())
}

#[test]
fn srx3_native_missile_crosses_real_session_into_play_scene()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    type Error = Box<dyn std::error::Error + Send + Sync>;
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            oteryn_client_runtime::ClientRuntime::new()?.block_on(async {
                use oteryn_session::test_support::wire;
                use wire::world_spatial_entities::*;
                const CAPS: &[u32] = &[6, 23];
                let id = |value: u8| {
                    let mut bytes = [value; 16];
                    bytes[6] = 0x70;
                    bytes[8] = 0x80;
                    bytes
                };
                let own = WorldSpatialEntity {
                    display_name: None,
                    kind: EntityKind::Player,
                    entity: EntityRef {
                        identity: id(1),
                        generation: 1,
                    },
                    position: wire::world_spatial::ActorPosition {
                        x: 100,
                        y: 200,
                        floor: 0,
                    },
                    detail: EntityDetail::Actor {
                        direction: wire::world_spatial::StepDirection::South,
                        appearance_ref: 0,
                        health_percent: 100,
                    },
                };
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let (control, mut receiver) = tokio::sync::mpsc::channel(1);
                let peer = tokio::spawn(async move {
                    let bootstrap = oteryn_session::read_frame(&mut server).await?;
                    assert_eq!(
                        wire::decode_wire_envelope(&bootstrap)?
                            .client_bootstrap()?
                            .supported_capabilities,
                        CAPS
                    );
                    oteryn_session::write_frame(
                        &mut server,
                        &wire::encode_server_accepted(&wire::ServerAcceptedValue {
                            game_session_id: wire::GameSessionId::decode(&id(2))?,
                            world_id: wire::WorldId::decode(&id(3))?,
                            channel_id: wire::ChannelId::decode(&id(4))?,
                            connection_generation: 1,
                            current_server_sequence: 0,
                            next_command_id: 1,
                            schema_revision: 1,
                            selected_capabilities: CAPS,
                        })?,
                    )
                    .await?;
                    let entities = encode_world_spatial_entities_snapshot_with_fields(
                        &WorldSpatialEntitiesSnapshot {
                            content_generation: [7; 32],
                            actor_position: own.position,
                            own_identity: own.entity.identity,
                            entities: vec![own],
                        },
                        EntityFields::selected(CAPS),
                        &EntityLights::new(),
                    )
                    .map_err(|e| format!("{e:?}"))?;
                    let overlay = wire::world_object::encode_world_object_overlay_snapshot(&[])
                        .map_err(|e| format!("{e:?}"))?;
                    let presentation = wire::presentation::encode_presentation_snapshot();
                    let cooldowns = wire::presentation::encode_actor_cooldowns(
                        &wire::presentation::ActorCooldowns {
                            server_now_ms: 1,
                            rtt_ms: None,
                            entries: vec![],
                        },
                    )
                    .map_err(|error| format!("{error:?}"))?;
                    for frame in wire::encode_single_chunk_snapshot(
                        1,
                        1,
                        1,
                        &[
                            wire::DomainSnapshot {
                                domain_id: 1,
                                revision: 1,
                                snapshot_type: 2,
                                payload: &entities,
                            },
                            wire::DomainSnapshot {
                                domain_id: 2,
                                revision: 1,
                                snapshot_type: 1,
                                payload: &overlay,
                            },
                            wire::DomainSnapshot {
                                domain_id: 19,
                                revision: 1,
                                snapshot_type: 1,
                                payload: &presentation,
                            },
                            wire::DomainSnapshot {
                                domain_id: 20,
                                revision: 1,
                                snapshot_type: 1,
                                payload: &cooldowns,
                            },
                        ],
                    )? {
                        oteryn_session::write_frame(&mut server, &frame).await?;
                    }
                    receiver.recv().await.ok_or("missile control closed")?;
                    let events = [wire::presentation::PresentationEvent {
                        decision_ordinal: 1,
                        emission_ordinal: 1,
                        kind: wire::presentation::PresentationEventKind::Missile {
                            from: wire::world_spatial::ActorPosition {
                                x: 100,
                                y: 200,
                                floor: 0,
                            },
                            to: wire::world_spatial::ActorPosition {
                                x: 102,
                                y: 200,
                                floor: 0,
                            },
                            missile_id: 31,
                            source: wire::presentation::PresentationSource::Creatures,
                        },
                    }];
                    let bytes = wire::presentation::encode_presentation_batch(&events)
                        .map_err(|error| format!("{error:?}"))?;
                    oteryn_session::write_frame(
                        &mut server,
                        &wire::encode_state_delta(1, 2, 19, 1, 2, 1, &bytes)?,
                    )
                    .await?;
                    receiver
                        .recv()
                        .await
                        .ok_or("missile final control closed")?;
                    Ok::<(), Error>(())
                });
                let session = oteryn_session::Session::admit(
                    client,
                    oteryn_session::Admission {
                        appearance_items: None,
                        schema_revision: 1,
                        character_id: oteryn_session::CharacterId::decode(&id(5))?,
                        admission_material: b"native-missile",
                        client_build_id: "srx3-native-missile",
                        supported_capabilities: CAPS,
                        deadline: std::time::Duration::from_secs(3),
                    },
                )
                .await?;
                let mut view = crate::play::PlayView::from_join(session.join_snapshot())?;
                let baseline = view.scene().sprites().instances().len();
                let (link, commands, outbox) = crate::play::play_channel();
                let runner = tokio::spawn(crate::play::run_session(session, commands, outbox));
                control.send(()).await?;
                tokio::time::timeout(std::time::Duration::from_secs(3), async {
                    loop {
                        view.tick(&link)
                            .map_err(|error| format!("native missile play: {error:?}"))?;
                        if view.visuals.iter().any(|cue| {
                            matches!(
                                cue.kind,
                                PresentationEventKind::Missile { missile_id: 31, .. }
                            )
                        }) {
                            assert!(view.scene().sprites().instances().len() > baseline);
                            return Ok::<(), Error>(());
                        }
                        tokio::task::yield_now().await;
                    }
                })
                .await??;
                control.send(()).await?;
                drop(link);
                runner.await?;
                peer.await??;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "native missile play thread panicked")?
}
