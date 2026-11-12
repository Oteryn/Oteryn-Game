//! Combat observer regression replays through its real TLS command and spatial decoder.
use super::*;

fn position(x: i32) -> ActorPosition {
    ActorPosition {
        x,
        y: 100,
        floor: 7,
    }
}

fn creature() -> spatial::WorldSpatialEntity {
    spatial::WorldSpatialEntity {
        display_name: None,
        kind: spatial::EntityKind::Creature,
        entity: spatial::EntityRef {
            identity: v7(10, 1),
            generation: 1,
        },
        position: position(101),
        detail: spatial::EntityDetail::Actor {
            direction: StepDirection::East,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}

fn corpse(marker: u8, x: i32) -> spatial::WorldSpatialEntity {
    spatial::WorldSpatialEntity {
        display_name: None,
        kind: spatial::EntityKind::Corpse,
        entity: spatial::EntityRef {
            identity: v7(marker, 1),
            generation: 0,
        },
        position: position(x),
        detail: spatial::EntityDetail::Object {
            item_definition_ref: 1,
            quantity: 1,
            item_handle: std::num::NonZeroU64::new(u64::from(marker)),
        },
    }
}

#[derive(Clone, Copy)]
enum Replay {
    Ranged,
    EarlierNeighbour,
    BatchedEarlierNeighbour,
    Nearest,
    Ambiguous,
}

type ServerStream = tokio_rustls::server::TlsStream<TcpStream>;

async fn send(stream: &mut ServerStream, frame: &[u8]) -> TestResult {
    stream.write_all(&framed(frame)).await?;
    stream.flush().await?;
    Ok(())
}

async fn delta(
    stream: &mut ServerStream,
    sequence: &mut u64,
    revision: &mut u64,
    enter: Vec<spatial::WorldSpatialEntity>,
    update: Vec<spatial::WorldSpatialEntity>,
    leave: Vec<spatial::EntityRef>,
) -> TestResult {
    let payload = spatial::encode_world_spatial_entities_delta_with_item_handles(
        &spatial::WorldSpatialEntitiesDelta {
            content_generation: [1; 32],
            actor_position: position(100),
            enter,
            update,
            leave,
        },
    )
    .map_err(|e| format!("hunt replay spatial encoding: {e:?}"))?;
    *sequence += 1;
    send(
        stream,
        &native::encode_state_delta(
            1,
            *sequence,
            1,
            *revision,
            *revision + 1,
            spatial::DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            &payload,
        )?,
    )
    .await?;
    *revision += 1;
    Ok(())
}

async fn replay_peer(mut stream: ServerStream, scenario: Replay) -> TestResult {
    let mut sequence = 0;
    let mut revision = 0;
    let mut turns = 0;
    let mut attacked = false;
    loop {
        let Ok(bytes) = light_read(&mut stream).await else {
            return Ok(());
        };
        let frame = native::decode_wire_envelope(&bytes)?;
        let command = frame.client_command(1)?;
        let payload = match command.command_type {
            character::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB => {
                let mut payload = vec![character::CharacterTab::GeneralStats as u8];
                payload.extend_from_slice(&u64::from(turns >= 2).to_le_bytes());
                payload
            }
            attacks::COMMAND_TYPE_ATTACK_TARGET_INTENT => {
                assert_eq!(
                    command.payload,
                    attacks::encode_attack_target_intent(Some(&creature().entity))
                        .map_err(|e| format!("attack encoding: {e:?}"))?
                        .as_slice()
                );
                attacked = true;
                attacks::encode_attack_intent_result(attacks::AttackIntentDisposition::Ok)
            }
            native::target_move::COMMAND_TYPE_TARGET_MOVE_INTENT => {
                assert!(attacked);
                turns += 1;
                match (scenario, turns) {
                    (Replay::Ranged, 1) => {
                        let moved = spatial::WorldSpatialEntity {
                            position: position(103),
                            ..creature()
                        };
                        delta(
                            &mut stream,
                            &mut sequence,
                            &mut revision,
                            vec![],
                            vec![moved],
                            vec![],
                        )
                        .await?;
                    }
                    (Replay::Ranged, 2) => {
                        delta(
                            &mut stream,
                            &mut sequence,
                            &mut revision,
                            vec![corpse(12, 104)],
                            vec![],
                            vec![creature().entity],
                        )
                        .await?;
                    }
                    (Replay::EarlierNeighbour | Replay::BatchedEarlierNeighbour, 1) => {
                        delta(
                            &mut stream,
                            &mut sequence,
                            &mut revision,
                            vec![corpse(12, 101)],
                            vec![],
                            vec![],
                        )
                        .await?;
                        if matches!(scenario, Replay::BatchedEarlierNeighbour) {
                            delta(
                                &mut stream,
                                &mut sequence,
                                &mut revision,
                                vec![],
                                vec![],
                                vec![creature().entity],
                            )
                            .await?;
                        }
                    }
                    (Replay::EarlierNeighbour, 2) => {
                        delta(
                            &mut stream,
                            &mut sequence,
                            &mut revision,
                            vec![],
                            vec![],
                            vec![creature().entity],
                        )
                        .await?;
                    }
                    (Replay::EarlierNeighbour | Replay::BatchedEarlierNeighbour, 3) => {
                        delta(
                            &mut stream,
                            &mut sequence,
                            &mut revision,
                            vec![corpse(13, 101)],
                            vec![],
                            vec![],
                        )
                        .await?;
                    }
                    (Replay::Nearest, 1) => {
                        // Put the farther corpse first: iteration order is not evidence.
                        delta(
                            &mut stream,
                            &mut sequence,
                            &mut revision,
                            vec![corpse(12, 102), corpse(13, 101)],
                            vec![],
                            vec![creature().entity],
                        )
                        .await?;
                    }
                    (Replay::Ambiguous, 1) => {
                        delta(
                            &mut stream,
                            &mut sequence,
                            &mut revision,
                            vec![corpse(12, 100), corpse(13, 102)],
                            vec![],
                            vec![creature().entity],
                        )
                        .await?;
                    }
                    (Replay::Ambiguous, 3) => {
                        delta(
                            &mut stream,
                            &mut sequence,
                            &mut revision,
                            vec![],
                            vec![],
                            vec![corpse(12, 100).entity],
                        )
                        .await?;
                    }
                    _ => {}
                }
                vec![8, 1]
            }
            other => return Err(format!("unexpected hunt command {other}").into()),
        };
        sequence += 1;
        send(
            &mut stream,
            &native::encode_command_result(
                1,
                sequence,
                command.command_id,
                CommandStatus::Accepted,
                &payload,
            )?,
        )
        .await?;
    }
}

async fn replay(scenario: Replay) -> TestResult {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let certificate = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let mut config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(vec![certificate.clone()], key.into())?;
    config.alpn_protocols = vec![native::ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(config));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let peer = tokio::task::spawn_local(async move {
        let (tcp, _) = listener.accept().await?;
        replay_peer(acceptor.accept(tcp).await?, scenario).await
    });
    let tcp = TcpStream::connect(address).await?;
    let stream = connector(&certificate, &EXACT)?
        .connect(ServerName::try_from("localhost")?, tcp)
        .await?;
    let mut client = ForgeClient {
        stream,
        generation: 1,
        session: native::GameSessionId::decode(&v7(1, 1))?,
        sequence: 0,
        command: 1,
        probe: 0,
        selected: FORGE_SUPPORTED_CAPABILITIES.to_vec(),
        revisions: BTreeMap::from([(1, 0)]),
        vitals: casts::ActorVitals {
            health: 100,
            max_health: 100,
            mana: 0,
            max_mana: 0,
            soul: 0,
            harmony: 0,
            serene: false,
        },
        position: position(100),
        content: [1; 32],
        map_header: native::world_map::MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 0,
            origin: position(100),
            first_visible_floor: 7,
        },
        // An old corpse at the death tile must never satisfy the hunt.
        entities: vec![creature(), corpse(11, 104)],
        hunt: None,
        bindings: vec![],
        inventory: Default::default(),
        container: Default::default(),
        released: false,
        tree: Default::default(),
    };
    let result = tokio::time::timeout(
        Duration::from_secs(2),
        forge_attack_corpse(&mut client, creature(), 1),
    )
    .await;
    assert!(
        result.is_ok(),
        "fresh corpse near the last target position must bind while the player stands four tiles away"
    );
    let (handle, xp_before) = result??;
    if matches!(scenario, Replay::Ranged) {
        assert_eq!(handle.get(), 12);
    } else {
        assert_eq!(
            handle.get(),
            13,
            "only the target-correlated, uniquely nearest corpse may bind"
        );
    }
    assert_eq!(xp_before, 0);
    drop(client);
    peer.await??;
    Ok(())
}

#[test]
fn hunt_corpse_replay_accepts_ranged_moving_target_death() -> TestResult {
    run_replay(Replay::Ranged)
}

fn run_replay(scenario: Replay) -> TestResult {
    let thread = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let result = (|| -> TestResult {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(tokio::task::LocalSet::new().run_until(replay(scenario)))
            })();
            result.map_err(|error| error.to_string())
        })?;
    match thread.join() {
        Ok(result) => result.map_err(Into::into),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

#[test]
fn hunt_corpse_replay_rejects_corpse_before_target_removal() -> TestResult {
    run_replay(Replay::EarlierNeighbour)
}

#[test]
fn hunt_corpse_replay_preserves_removal_order_within_one_barrier() -> TestResult {
    run_replay(Replay::BatchedEarlierNeighbour)
}

#[test]
fn hunt_corpse_replay_selects_uniquely_nearest_corpse() -> TestResult {
    run_replay(Replay::Nearest)
}

#[test]
fn hunt_corpse_replay_waits_when_corpses_are_equally_plausible() -> TestResult {
    run_replay(Replay::Ambiguous)
}
