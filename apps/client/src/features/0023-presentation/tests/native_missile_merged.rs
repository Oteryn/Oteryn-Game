use super::*;
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
    // Two tiles at 50 ms per tile: sample mid-flight, before its 100 ms arrival.
    view.advance_presentation(now + std::time::Duration::from_millis(50))?;
    let after = &view.scene().sprites().instances()[baseline..];
    assert_eq!(before.len(), after.len());
    assert!(
        after
            .iter()
            .zip(&before)
            .all(|(a, b)| a.position[0] > b.position[0])
    );
    view.advance_presentation(now + std::time::Duration::from_millis(100))?;
    assert_eq!(view.scene().sprites().instances().len(), baseline);
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
