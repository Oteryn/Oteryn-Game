//! Exercise the actual route USE sender over TLS with no optional capabilities.
//! The peer checks native wire frames; this is a transport regression, not a
//! replacement for the hosted gameplay-owner qualification.
use super::*;
use oteryn_protocol_oteryn::{self as wire, world_object as object};
type Peer = tokio_rustls::server::TlsStream<TcpStream>;

async fn read(peer: &mut Peer) -> TestResult<Vec<u8>> {
    let size = peer.read_u32().await?;
    if size == 0 || size > wire::MAX_WIRE_FRAME_BYTES {
        return Err("route peer frame size".into());
    }
    let mut frame = vec![0; usize::try_from(size)?];
    peer.read_exact(&mut frame).await?;
    Ok(frame)
}
async fn send(peer: &mut Peer, frame: &[u8]) -> TestResult {
    peer.write_all(&framed(frame)).await?;
    peer.flush().await?;
    Ok(())
}
fn position(at: TilePos) -> ActorPosition {
    ActorPosition {
        x: i32::from(at.x),
        y: i32::from(at.y),
        floor: -i16::from(at.floor),
    }
}
fn spatial(at: TilePos) -> Vec<u8> {
    encode_world_spatial(&WorldSpatialObservation {
        content_generation: [0x11; 32],
        actor_position: position(at),
    })
}

#[test]
fn qualification_nbroute_map_use_crosses_tls_without_capability_15() -> TestResult {
    on_route_thread(|| {
        let (world, _, _) = compiled_town()?;
        let path = route_toward(world, TOWN, "rat descent", |at| {
            at.floor == -8 && at.x == 32368 && at.y == 32240
        })
        .ok_or("rat route missing")?;
        let mut from = TOWN;
        let mut first_use = None;
        for action in path {
            if let WalkAction::Use { at, ordinal } = action {
                first_use = Some((at, ordinal));
                break;
            }
            from = destination(world, from, action).ok_or("route step refused")?;
        }
        let (at, ordinal) = first_use.ok_or("rat descent has no USE")?;
        let expected = use_destination(world, from, at, ordinal).ok_or("USE destination")?;
        let key = crate::map::view::placement_key(at, ordinal).ok_or("USE key")?;
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        runtime.block_on(tokio::task::LocalSet::new().run_until(async {
            tokio::time::timeout(Duration::from_secs(10), async {
                let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
                let certificate = generated.cert.der().clone();
                let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
                let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
                    rustls::crypto::aws_lc_rs::default_provider(),
                ))
                .with_protocol_versions(&[&rustls::version::TLS13])?
                .with_no_client_auth()
                .with_single_cert(vec![certificate.clone()], key.into())?;
                config.alpn_protocols = vec![wire::ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
                let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
                let listener = TcpListener::bind("127.0.0.1:0").await?;
                let address = listener.local_addr()?;
                let server = tokio::task::spawn_local(async move {
                    let (tcp, _) = listener.accept().await?;
                    let mut peer = acceptor.accept(tcp).await?;
                    let bootstrap = read(&mut peer).await?;
                    assert!(
                        wire::decode_wire_envelope(&bootstrap)?
                            .client_bootstrap()?
                            .supported_capabilities
                            .contains(&15)
                    );
                    send(
                        &mut peer,
                        &wire::encode_server_accepted(&wire::ServerAcceptedValue {
                            game_session_id: wire::GameSessionId::decode(&v7(1, 1))?,
                            world_id: wire::WorldId::decode(&v7(2, 2))?,
                            channel_id: wire::ChannelId::decode(&v7(3, 3))?,
                            connection_generation: 1,
                            current_server_sequence: 40,
                            next_command_id: 7,
                            schema_revision: 1,
                            selected_capabilities: &[],
                        })?,
                    )
                    .await?;
                    let initial_spatial = spatial(from);
                    let overlay = object::encode_world_object_overlay_snapshot(&[])
                        .map_err(|e| format!("overlay: {e:?}"))?;
                    for frame in wire::encode_single_chunk_snapshot(
                        1,
                        1,
                        40,
                        &[
                            DomainSnapshot {
                                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                                revision: 0,
                                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                                payload: &initial_spatial,
                            },
                            DomainSnapshot {
                                domain_id: object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                                revision: 0,
                                snapshot_type: object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                                payload: &overlay,
                            },
                        ],
                    )? {
                        send(&mut peer, &frame).await?;
                    }
                    let bytes = read(&mut peer).await?;
                    let command = wire::decode_wire_envelope(&bytes)?.client_command(1)?;
                    assert_eq!(
                        (command.command_id, command.command_type),
                        (7, object::COMMAND_TYPE_USE_INTENT)
                    );
                    let target = object::decode_use_intent(command.payload)
                        .map_err(|e| format!("USE: {e:?}"))?;
                    assert_eq!(
                        target,
                        object::WorldObjectTarget {
                            placement,
                            expected_revision: 0
                        }
                    );
                    let result = object::encode_use_result(object::UseDisposition::Committed)
                        .map_err(|e| format!("result: {e:?}"))?;
                    send(
                        &mut peer,
                        &wire::encode_command_result(1, 41, 7, CommandStatus::Accepted, &result)?,
                    )
                    .await?;
                    send(
                        &mut peer,
                        &wire::encode_state_delta(
                            1,
                            42,
                            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                            0,
                            1,
                            DELTA_TYPE_WORLD_SPATIAL_V1,
                            &spatial(expected),
                        )?,
                    )
                    .await?;
                    let mut byte = [0];
                    // Dropping DevClientSession closes TCP without TLS close_notify.
                    let closed = peer.read(&mut byte).await;
                    assert!(
                        matches!(closed, Ok(0))
                            || matches!(closed, Err(ref error) if error.kind() == std::io::ErrorKind::UnexpectedEof),
                        "unexpected client teardown: {closed:?}"
                    );
                    Ok::<_, Box<dyn std::error::Error>>(())
                });
                let mut client =
                    oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
                        address,
                        server_name: "localhost",
                        root_certificate: &certificate,
                        schema_revision: 1,
                        character_id: CharacterId::decode(&v7(4, 4))?,
                        admission_material: b"route-wire-test",
                        client_build_id: "route-wire-test",
                        deadline: Duration::from_secs(5),
                    })
                    .await?;
                assert!(!client.selected_capabilities().contains(&15));
                walk_use(&mut client, world, at, ordinal).await?;
                assert_eq!(tile(client.world_spatial().actor_position)?, expected);
                drop(client);
                server.await??;
                Ok::<_, Box<dyn std::error::Error>>(())
            })
            .await
            .map_err(|_| "route USE TLS deadline")?
        }))
    })
}

// These peers exercise the production hunt sender and observer. They deliberately
// refuse loot USE after proving a kill, so no replay can qualify a gameplay row.
#[derive(Clone, Copy)]
enum HuntReplay {
    Relocation,
    MovingCorpse,
    Pack,
}
async fn hunt_replay(scenario: HuntReplay) -> TestResult {
    use wire::{attack, cyclopedia_character as stats, world_spatial_entities as entities};
    let (world, _, _) = compiled_town()?;
    assert_eq!(native_creature_name("oteryn:creature.rat")?, "Rat");
    let selected = [4, 6, 17, 18, 199, 286];
    let fields = entities::EntityFields::selected(&selected);
    let origin = ActorPosition {
        x: 32369,
        y: 32238,
        floor: 7,
    };
    let actor = |id, name, kind, position| -> TestResult<entities::WorldSpatialEntity> {
        Ok(entities::WorldSpatialEntity {
            entity: entities::EntityRef {
                identity: v7(id, 1),
                generation: 1,
            },
            display_name: Some(
                wire::entity_names::EntityDisplayName::new(name)
                    .map_err(|e| format!("name: {e:?}"))?,
            ),
            kind,
            position,
            detail: entities::EntityDetail::Actor {
                direction: StepDirection::North,
                appearance_ref: 1,
                health_percent: 100,
            },
        })
    };
    let own = actor(1, "Player", entities::EntityKind::Player, origin)?;
    let rat = actor(
        10,
        "Rat",
        entities::EntityKind::Creature,
        ActorPosition {
            x: origin.x + 1,
            ..origin
        },
    )?;
    let mut visible = vec![own, rat];
    if matches!(scenario, HuntReplay::Relocation | HuntReplay::Pack) {
        visible.insert(
            1,
            actor(
                9,
                "Poison Spider",
                entities::EntityKind::Creature,
                ActorPosition {
                    x: origin.x
                        + if matches!(scenario, HuntReplay::Pack) {
                            1
                        } else {
                            6
                        },
                    y: origin.y + 1,
                    ..origin
                },
            )?,
        );
    }
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let certificate = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(vec![certificate.clone()], key.into())?;
    config.alpn_protocols = vec![wire::ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::task::spawn_local(async move {
        let (tcp, _) = listener.accept().await?;
        let mut peer = acceptor.accept(tcp).await?;
        let bootstrap = read(&mut peer).await?;
        wire::decode_wire_envelope(&bootstrap)?.client_bootstrap()?;
        send(
            &mut peer,
            &wire::encode_server_accepted(&wire::ServerAcceptedValue {
                game_session_id: wire::GameSessionId::decode(&v7(1, 1))?,
                world_id: wire::WorldId::decode(&v7(2, 2))?,
                channel_id: wire::ChannelId::decode(&v7(3, 3))?,
                connection_generation: 1,
                current_server_sequence: 40,
                next_command_id: 7,
                schema_revision: 1,
                selected_capabilities: &selected,
            })?,
        )
        .await?;
        let payload = entities::encode_world_spatial_entities_snapshot_with_fields(
            &entities::WorldSpatialEntitiesSnapshot {
                content_generation: [0x11; 32],
                actor_position: origin,
                own_identity: own.entity.identity,
                entities: visible,
            },
            fields,
            &Default::default(),
        )
        .map_err(|e| format!("snapshot: {e:?}"))?;
        let map =
            wire::world_map::encode_world_map_snapshot(&wire::world_map::WorldMapViewSnapshot {
                header: wire::world_map::MapViewHeader {
                    content_generation: [0x11; 32],
                    bundle_digest: world.base().digest(),
                    reset_epoch: 0,
                    origin: ActorPosition {
                        floor: -origin.floor,
                        ..origin
                    },
                    first_visible_floor: 0,
                },
                tiles: vec![],
            })
            .map_err(|error| format!("map: {error:?}"))?;
        let vitals = wire::actor_spell::encode_actor_vitals(&wire::actor_spell::ActorVitals {
            health: 150,
            max_health: 150,
            mana: 55,
            max_mana: 55,
            soul: 100,
            harmony: 0,
            serene: false,
        })
        .map_err(|error| format!("vitals: {error:?}"))?;
        let combat = attack::encode_actor_combat_state(&attack::ActorCombatState {
            target: None,
            modes: attack::FightModes::DEFAULT,
            in_fight: false,
        })
        .map_err(|error| format!("combat: {error:?}"))?;
        for frame in wire::encode_single_chunk_snapshot(
            1,
            1,
            40,
            &[
                DomainSnapshot {
                    domain_id: 1,
                    revision: 0,
                    snapshot_type: entities::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                    payload: &payload,
                },
                DomainSnapshot {
                    domain_id: wire::actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    revision: 0,
                    snapshot_type: wire::actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                    payload: &vitals,
                },
                DomainSnapshot {
                    domain_id: attack::STATE_DOMAIN_ACTOR_COMBAT_STATE,
                    revision: 0,
                    snapshot_type: attack::SNAPSHOT_TYPE_ACTOR_COMBAT_STATE_V1,
                    payload: &combat,
                },
                DomainSnapshot {
                    domain_id: wire::world_map::STATE_DOMAIN_WORLD_MAP_VIEW,
                    revision: 0,
                    snapshot_type: wire::world_map::SNAPSHOT_TYPE_WORLD_MAP_VIEW_SNAPSHOT_V1,
                    payload: &map,
                },
            ],
        )? {
            send(&mut peer, &frame).await?;
        }
        let mut sequence = 40;
        let mut chasing = false;
        let mut attacked = false;
        loop {
            let bytes = match read(&mut peer).await {
                Ok(bytes) => bytes,
                Err(_) => return Ok::<_, Box<dyn std::error::Error>>(()),
            };
            let command = wire::decode_wire_envelope(&bytes)?.client_command(1)?;
            let begins_attack = command.command_type == attack::COMMAND_TYPE_ATTACK_TARGET_INTENT
                && command.payload
                    != attack::encode_attack_target_intent(None)
                        .map_err(|e| format!("clear: {e:?}"))?;
            let mut status = CommandStatus::Accepted;
            let payload = match command.command_type {
                stats::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB => {
                    let mut data = vec![stats::CharacterTab::GeneralStats as u8];
                    data.extend_from_slice(&u64::from(attacked).to_le_bytes());
                    data
                }
                attack::COMMAND_TYPE_FIGHT_MODES_INTENT => {
                    let modes = attack::decode_fight_modes_intent(command.payload)
                        .map_err(|e| format!("modes: {e:?}"))?;
                    assert_eq!(modes.chase, attack::ChaseMode::Chase);
                    chasing = true;
                    attack::encode_attack_intent_result(attack::AttackIntentDisposition::Ok)
                }
                attack::COMMAND_TYPE_ATTACK_TARGET_INTENT => {
                    if begins_attack {
                        assert!(
                            !matches!(scenario, HuntReplay::Pack),
                            "unsafe pack must not receive ATTACK"
                        );
                        if matches!(scenario, HuntReplay::Relocation) {
                            assert!(chasing, "hunt must enable native chase before attacking");
                        }
                        assert_eq!(
                            command.payload,
                            attack::encode_attack_target_intent(Some(&rat.entity))
                                .map_err(|e| format!("target: {e:?}"))?
                        );
                        attacked = true;
                    }
                    attack::encode_attack_intent_result(attack::AttackIntentDisposition::Ok)
                }
                object::COMMAND_TYPE_USE_INTENT => {
                    status = CommandStatus::Rejected;
                    object::encode_use_result(object::UseDisposition::Rejected)
                        .map_err(|e| format!("use: {e:?}"))?
                }
                other => return Err(format!("unexpected hunt command {other}").into()),
            };
            sequence += 1;
            send(
                &mut peer,
                &wire::encode_command_result(1, sequence, command.command_id, status, &payload)?,
            )
            .await?;
            if begins_attack {
                let (at, enter, update) = match scenario {
                    HuntReplay::Relocation | HuntReplay::Pack => {
                        let at = ActorPosition {
                            x: 32536,
                            y: 32514,
                            ..origin
                        };
                        (
                            at,
                            vec![],
                            vec![entities::WorldSpatialEntity {
                                position: at,
                                ..own
                            }],
                        )
                    }
                    HuntReplay::MovingCorpse => (
                        origin,
                        vec![entities::WorldSpatialEntity {
                            entity: entities::EntityRef {
                                identity: v7(12, 1),
                                generation: 0,
                            },
                            display_name: None,
                            kind: entities::EntityKind::Corpse,
                            position: ActorPosition {
                                y: rat.position.y + 1,
                                ..rat.position
                            },
                            detail: entities::EntityDetail::Object {
                                item_definition_ref: 1,
                                quantity: 1,
                                item_handle: std::num::NonZeroU64::new(12),
                            },
                        }],
                        vec![],
                    ),
                };
                let mut leave = vec![rat.entity];
                if matches!(scenario, HuntReplay::Relocation | HuntReplay::Pack) {
                    leave.push(entities::EntityRef {
                        identity: v7(9, 1),
                        generation: 1,
                    });
                }
                leave.sort();
                let payload = entities::encode_world_spatial_entities_delta_with_fields(
                    &entities::WorldSpatialEntitiesDelta {
                        content_generation: [0x11; 32],
                        actor_position: at,
                        enter,
                        update,
                        leave,
                    },
                    fields,
                    &Default::default(),
                )
                .map_err(|e| format!("delta: {e:?}"))?;
                sequence += 1;
                send(
                    &mut peer,
                    &wire::encode_state_delta(
                        1,
                        sequence,
                        1,
                        0,
                        1,
                        entities::DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                        &payload,
                    )?,
                )
                .await?;
            }
        }
    });
    let client = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id: CharacterId::decode(&v7(4, 4))?,
        admission_material: b"hunt-replay",
        client_build_id: "hunt-replay",
        deadline: Duration::from_secs(2),
    })
    .await;
    let mut client = match client {
        Ok(client) => client,
        Err(error) => {
            return Err(format!("hunt client join: {error}; peer: {:?}", server.await).into());
        }
    };
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        client_hunt::hunt_rat(&mut client, world),
    )
    .await;
    drop(client);
    server.await??;
    let error = result
        .map_err(|_| "hunt replay stalled after a native movement/removal frame")?
        .err()
        .ok_or("replay must not qualify loot")?
        .to_string();
    match scenario {
        HuntReplay::Relocation => assert!(error.contains("relocat"), "{error}"),
        HuntReplay::MovingCorpse => {
            for field in [
                "hunt corpse USE refused",
                "status=",
                "disposition=",
                "handle=",
                "type=",
            ] {
                assert!(error.contains(field), "missing {field}: {error}");
            }
        }
        HuntReplay::Pack => assert!(error.contains("safe engagement"), "{error}"),
    }
    Ok(())
}
fn run_hunt_replay(scenario: HuntReplay) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            (|| -> TestResult {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(tokio::task::LocalSet::new().run_until(hunt_replay(scenario)))
            })()
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "hunt replay worker panicked")?
        .map_err(Into::into)
}
#[test]
fn qualification_nbroute_hunt_chases_rat_and_stops_on_relocation() -> TestResult {
    run_hunt_replay(HuntReplay::Relocation)
}
#[test]
fn qualification_nbroute_hunt_proves_moving_one_hit_corpse() -> TestResult {
    run_hunt_replay(HuntReplay::MovingCorpse)
}

#[test]
fn qualification_nbroute_care_refuses_pack_on_wire() -> TestResult {
    run_hunt_replay(HuntReplay::Pack)
}
