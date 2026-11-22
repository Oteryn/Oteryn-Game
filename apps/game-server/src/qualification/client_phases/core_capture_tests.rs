//! Exercise M04's actual pair/capture helpers through native TLS admission and state dispatch.
use super::*;
use oteryn_dev_client::{EntityDetail, EntityRef, WorldSpatialEntity};
use oteryn_protocol_oteryn::{self as native, world_spatial_entities as spatial};
use spatial::DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2;

const SELECTED: &[u32] = &[4, 6, 13, 42, 82, 134, 286, 372, 386];

#[derive(Clone, Copy)]
enum Case {
    Pair,
    Queued,
    Later,
    WrongEntity,
    WrongLanding,
    IconsOnly,
    StaleQueued,
}

#[test]
fn core_capture_regression_m04_admits_distinct_live_characters() -> TestResult {
    run(Case::Pair)
}

#[test]
fn core_capture_regression_m04_uses_queued_and_later_matching_deltas() -> TestResult {
    for case in [Case::Queued, Case::Later] {
        run(case)?;
    }
    Ok(())
}

#[test]
fn core_capture_regression_m04_rejects_unrelated_or_icons_only_events() -> TestResult {
    for case in [Case::WrongEntity, Case::WrongLanding, Case::IconsOnly, Case::StaleQueued] {
        run(case)?;
    }
    Ok(())
}

fn run(case: Case) -> TestResult {
    let thread = std::thread::Builder::new()
        .name("m04-core-capture-tls".into())
        .stack_size(64 << 20)
        .spawn(move || {
            let result = (|| -> TestResult {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(tokio::task::LocalSet::new().run_until(async {
                    tokio::time::timeout(Duration::from_secs(20), exercise(case))
                        .await
                        .map_err(|_| "M04 capture TLS fixture deadline")?
                }))
            })();
            result.map_err(|error| error.to_string())
        })?;
    match thread.join() {
        Ok(result) => result.map_err(Into::into),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

type ServerStream = tokio_rustls::server::TlsStream<TcpStream>;

async fn read(stream: &mut ServerStream) -> TestResult<Vec<u8>> {
    let size = stream.read_u32().await?;
    if size == 0 || size > native::MAX_WIRE_FRAME_BYTES {
        return Err("M04 native frame size".into());
    }
    let mut bytes = vec![0; usize::try_from(size)?];
    stream.read_exact(&mut bytes).await?;
    Ok(bytes)
}

async fn send(stream: &mut ServerStream, bytes: &[u8]) -> TestResult {
    stream.write_all(&framed(bytes)).await?;
    stream.flush().await?;
    Ok(())
}

fn at(x: i32, y: i32) -> ActorPosition {
    ActorPosition { x, y, floor: 7 }
}

fn actor(identity: [u8; 16], position: ActorPosition) -> TestResult<WorldSpatialEntity> {
    Ok(WorldSpatialEntity {
        display_name: Some(native::entity_names::EntityDisplayName::new("Observed Player")
            .map_err(|error| format!("M04 name: {error:?}"))?),
        kind: EntityKind::Player,
        entity: EntityRef { identity, generation: 1 },
        position,
        detail: EntityDetail::Actor {
            direction: StepDirection::East,
            appearance_ref: 1,
            health_percent: 100,
        },
    })
}

fn creature(count: u16) -> TestResult<WorldSpatialEntity> {
    let mut entity = actor([0x73; 16], at(102, 101))?;
    entity.kind = EntityKind::Creature;
    entity.detail = EntityDetail::ActorWithIcons {
        direction: StepDirection::South,
        appearance_ref: 2,
        health_percent: 88,
        icons: spatial::NativeCreatureIcons::new(&[
            spatial::NativeCreatureIcon { category: 1, id: 6, count },
        ]).map_err(|error| format!("M04 native icon: {error:?}"))?,
    };
    Ok(entity)
}

fn ground(marker: u8, handle: u64) -> TestResult<WorldSpatialEntity> {
    Ok(WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::GroundItem,
        entity: EntityRef { identity: [marker; 16], generation: 0 },
        position: at(101, 102),
        detail: EntityDetail::ObjectWithAppearance {
            item_definition_ref: 3031,
            quantity: 7,
            item_handle: Some(oteryn_dev_client::ItemHandle::new(handle).ok_or("M04 item handle")?),
            appearance_id: std::num::NonZeroU16::new(3031).ok_or("M04 appearance")?,
        },
    })
}

fn initial_entities(identity: [u8; 16]) -> TestResult<Vec<WorldSpatialEntity>> {
    Ok(vec![
        actor(identity, at(100, 100))?,
        actor([0x71; 16], at(101, 100))?,
        actor([0x72; 16], at(102, 100))?,
        creature(1)?,
        ground(0x74, 9)?,
    ])
}

fn unrelated(identity: [u8; 16]) -> TestResult<spatial::WorldSpatialEntitiesDelta> {
    Ok(spatial::WorldSpatialEntitiesDelta {
        content_generation: [0x11; 32],
        actor_position: at(100, 101),
        enter: vec![],
        update: vec![actor(identity, at(100, 101))?, actor([0x72; 16], at(102, 102))?],
        leave: vec![ground(0x74, 9)?.entity],
    })
}

fn movement(reference: EntityRef, landing: ActorPosition) -> TestResult<spatial::WorldSpatialEntitiesDelta> {
    let mut moved = actor(reference.identity, landing)?;
    moved.entity = reference;
    Ok(spatial::WorldSpatialEntitiesDelta {
        content_generation: [0x11; 32],
        actor_position: at(100, 101),
        enter: vec![ground(0x75, 11)?],
        update: vec![moved, creature(5)?],
        leave: vec![],
    })
}

fn spatial_payload(delta: &spatial::WorldSpatialEntitiesDelta) -> TestResult<Vec<u8>> {
    spatial::encode_world_spatial_entities_delta_with_fields(
        delta,
        spatial::EntityFields::selected(SELECTED),
        &spatial::EntityLights::new(),
    ).map_err(|error| format!("M04 spatial encoder: {error:?}").into())
}

async fn admit(stream: &mut ServerStream, session: u8) -> TestResult<[u8; 16]> {
    let bytes = read(stream).await?;
    let envelope = native::decode_wire_envelope(&bytes)?;
    let bootstrap = envelope.client_bootstrap()?;
    assert!(SELECTED.iter().all(|capability| bootstrap.supported_capabilities.contains(capability)));
    let identity = *bootstrap.character_id.as_bytes();
    send(stream, &native::encode_server_accepted(&native::ServerAcceptedValue {
        game_session_id: native::GameSessionId::decode(&v7(session, 1))?,
        world_id: native::WorldId::decode(&v7(2, 2))?,
        channel_id: native::ChannelId::decode(&v7(3, 3))?,
        connection_generation: 1,
        current_server_sequence: 40,
        next_command_id: 7,
        schema_revision: 1,
        selected_capabilities: SELECTED,
    })?).await?;
    let snapshot = spatial::encode_world_spatial_entities_snapshot_with_fields(
        &spatial::WorldSpatialEntitiesSnapshot {
            content_generation: [0x11; 32],
            actor_position: at(100, 100),
            own_identity: identity,
            entities: initial_entities(identity)?,
        },
        spatial::EntityFields::selected(SELECTED),
        &spatial::EntityLights::new(),
    ).map_err(|error| format!("M04 snapshot: {error:?}"))?;
    let overlay = native::world_object::encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("M04 overlay: {error:?}"))?;
    for frame in native::encode_single_chunk_snapshot(1, 1, 40, &[
        DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision: 27,
            snapshot_type: spatial::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
            payload: &snapshot,
        },
        DomainSnapshot {
            domain_id: native::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            revision: 0,
            snapshot_type: native::world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
            payload: &overlay,
        },
    ])? {
        send(stream, &frame).await?;
    }
    Ok(identity)
}

async fn native_client(
    address: std::net::SocketAddr,
    certificate: &rustls::pki_types::CertificateDer<'static>,
    index: usize,
) -> TestResult<DevClientSession> {
    let characters = [v7(0x21, 1), v7(0x22, 2)];
    let character = characters.get(index).ok_or("M04 fixture character index")?;
    Ok(oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: certificate,
        schema_revision: 1,
        character_id: CharacterId::decode(character)?,
        admission_material: b"m04-capture-fixture",
        client_build_id: "m04-capture-tls-test",
        deadline: Duration::from_secs(5),
    }).await?)
}

async fn cast_reply(mut stream: ServerStream) -> TestResult {
    let bytes = read(&mut stream).await?;
    let command = native::decode_wire_envelope(&bytes)?.client_command(1)?;
    assert_eq!(command.command_id, 7);
    assert_eq!(command.command_type, native::actor_spell::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_INTENT);
    native::actor_spell::decode_spell_cast_intent(command.payload)
        .map_err(|error| format!("M04 live-session command: {error:?}"))?;
    send(&mut stream, &native::encode_command_result(
        1, 41, 7, CommandStatus::Rejected,
        &native::actor_spell::encode_spell_cast_result(native::actor_spell::SpellCastDisposition::Rejected),
    )?).await?;
    let mut rest = Vec::new();
    let _ = stream.read_to_end(&mut rest).await;
    Ok(())
}

// Keep an admission failure tied to the test peer's actual error. If the client failed while
// the peer is still waiting, cancel that peer instead of adding another timeout to the test.
async fn admission_or_peer_failure<T, U>(
    result: TestResult<T>,
    peer: &mut tokio::task::JoinHandle<TestResult<U>>,
) -> TestResult<T> {
    match result {
        Ok(value) => Ok(value),
        Err(client_error) => {
            if !peer.is_finished() {
                peer.abort();
            }
            let peer_result = match peer.await {
                Ok(Ok(_)) => "completed successfully".to_owned(),
                Ok(Err(error)) => format!("failed: {error}"),
                Err(error) => format!("task failed or cancelled: {error}"),
            };
            Err(format!("M04 admission failed: client={client_error}; peer={peer_result}").into())
        }
    }
}

async fn exercise(case: Case) -> TestResult {
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
    if matches!(case, Case::Pair) {
        let mut server = tokio::task::spawn_local(async move {
            let mut identities = Vec::new();
            let mut peers = Vec::new();
            for session in [1, 2] {
                let (tcp, _) = listener.accept().await?;
                let mut stream = acceptor.accept(tcp).await?;
                identities.push(admit(&mut stream, session).await?);
                peers.push(tokio::task::spawn_local(cast_reply(stream)));
            }
            for peer in peers { peer.await??; }
            Ok::<_, Box<dyn std::error::Error>>(identities)
        });
        let admission = admit_push_pair(|_, index| native_client(address, &certificate, index)).await;
        let (mut pusher, mut target) = admission_or_peer_failure(admission, &mut server).await?;
        let spell = std::num::NonZeroU32::new(1).ok_or("M04 live-session spell")?;
        for client in [&mut pusher, &mut target] {
            let outcome = client.cast_spell(spell, native::actor_spell::SpellTarget::None, false).await?;
            assert_eq!(outcome.status, CommandStatus::Rejected);
            assert_eq!(outcome.disposition, native::actor_spell::SpellCastDisposition::Rejected);
        }
        drop((pusher, target));
        let identities = server.await??;
        assert_eq!(identities.len(), 2);
        assert_ne!(identities[0], identities[1], "M04 requires two independently admitted Characters");
        return Ok(());
    }
    let expected = EntityRef { identity: [0x71; 16], generation: 1 };
    let landing = at(101, 101);
    let mut server = tokio::task::spawn_local(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        let identity = admit(&mut stream, 1).await?;
        let bytes = read(&mut stream).await?;
        let command = native::decode_wire_envelope(&bytes)?.client_command(1)?;
        assert_eq!(command.command_id, 7);
        assert_eq!(command.command_type, COMMAND_TYPE_CREATURE_PUSH_INTENT);
        let intent = decode_creature_push_intent(command.payload)
            .map_err(|error| format!("M04 push request: {error:?}"))?;
        assert_eq!(intent.target, expected);
        assert_eq!(intent.destination, landing);
        let mut sequence = 40;
        if matches!(case, Case::Later) {
            sequence += 1;
            send(&mut stream, &native::encode_command_result(1, sequence, 7, CommandStatus::Accepted, &[8, 1])?).await?;
        }
        let mut emit = Vec::new();
        let empty = spatial::WorldSpatialEntitiesDelta {
            content_generation: [0x11; 32], actor_position: at(100, 100),
            enter: vec![], update: vec![], leave: vec![],
        };
        // The native domain-1 codec reserves field 6 for an icons-only replacement.
        let mut icons = spatial_payload(&empty)?;
        let icon_bytes = native::party_skull::encode_party_icons(&[])
            .map_err(|error| format!("M04 party icons: {error:?}"))?;
        icons.extend_from_slice(&[0x32, u8::try_from(icon_bytes.len())?]);
        icons.extend_from_slice(&icon_bytes);
        emit.push(icons);
        if !matches!(case, Case::IconsOnly) {
            emit.push(spatial_payload(&unrelated(identity)?)?);
        }
        if matches!(case, Case::Queued | Case::Later | Case::StaleQueued) {
            emit.push(spatial_payload(&movement(expected, landing)?)?);
        } else if matches!(case, Case::WrongLanding) {
            let wrong_delta = spatial::WorldSpatialEntitiesDelta {
                update: vec![actor(expected.identity, at(101, 102))?],
                actor_position: at(100, 101),
                ..empty
            };
            emit.push(spatial_payload(&wrong_delta)?);
        }
        for (revision, payload) in (27_u64..).zip(emit) {
            sequence += 1;
            send(&mut stream, &native::encode_state_delta(
                1, sequence, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision, revision + 1, DELTA_TYPE_WORLD_SPATIAL_ENTITIES_V2, &payload,
            )?).await?;
            if revision == 27 {
                // The canonical core stream must preserve this real noncore sequence gap.
                sequence += 1;
                let vitals = native::actor_spell::encode_actor_vitals(&native::actor_spell::ActorVitals {
                    health: 150, max_health: 150, mana: 55, max_mana: 55,
                    ..native::actor_spell::ActorVitals::default()
                }).map_err(|error| format!("M04 noncore fixture: {error:?}"))?;
                send(&mut stream, &native::encode_state_delta(
                    1, sequence, native::actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                    0, 1, native::actor_spell::DELTA_TYPE_ACTOR_VITALS_V1, &vitals,
                )?).await?;
            }
        }
        if !matches!(case, Case::Later) {
            sequence += 1;
            send(&mut stream, &native::encode_command_result(1, sequence, 7, CommandStatus::Accepted, &[8, 1])?).await?;
        }
        let mut rest = Vec::new();
        let _ = stream.read_to_end(&mut rest).await;
        Ok::<_, Box<dyn std::error::Error>>(())
    });
    let admission = native_client(address, &certificate, 0).await;
    let mut client = admission_or_peer_failure(admission, &mut server).await?;
    let baseline = capture_baseline(&client, client.join_snapshot().world_spatial_revision)?;
    let own_identity = *client.world_entities().ok_or("M04 fixture entity view")?.own_identity();
    let payload = encode_creature_push_intent(&CreaturePushIntent { target: expected, destination: landing })
        .map_err(|error| format!("M04 push encode: {error:?}"))?;
    let reply = client.session_mut().gated_command(
        CAPABILITY_CREATURE_PUSH_V1, COMMAND_TYPE_CREATURE_PUSH_INTENT, &payload,
    ).await?;
    assert_eq!(reply.status, CommandStatus::Accepted);
    assert_eq!(reply.payload, [8, 1]);
    let baseline = if matches!(case, Case::StaleQueued) {
        // All queued events predate this actual post-command state; none may satisfy a new push.
        capture_baseline(&client, 30)?
    } else {
        baseline
    };
    let timeout = if matches!(case, Case::Queued | Case::Later) {
        Duration::from_secs(2)
    } else {
        Duration::from_millis(150)
    };
    let result = capture_push_delta(&mut client, baseline, expected, landing, timeout).await;
    if matches!(case, Case::Queued | Case::Later) {
        let captured = result?;
        assert_eq!(captured.baseline_revision, 27);
        assert_eq!(captured.baseline_sequence, 40);
        assert_eq!(captured.baseline.own_identity, own_identity);
        assert_eq!(captured.baseline.actor_position, at(100, 100));
        assert_eq!(captured.baseline.entities, {
            let mut entities = initial_entities(own_identity)?;
            entities.sort_by_key(|entity| entity.entity);
            entities
        });
        assert_eq!(captured.capabilities, [4, 6, 286, 372, 386]);
        assert_eq!(captured.transitions.len(), 3);
        let expected_sequences = if matches!(case, Case::Later) { [42, 44, 45] } else { [41, 43, 44] };
        for (index, transition) in captured.transitions.iter().enumerate() {
            assert_eq!(transition.delta.base_revision, 27 + u64::try_from(index)?);
            assert_eq!(transition.delta.new_revision, 28 + u64::try_from(index)?);
            assert_eq!(transition.delta.server_sequence, expected_sequences[index]);
            assert_eq!(transition.icons_only, index == 0);
            let (decoded, lights) = spatial::decode_world_spatial_entities_delta_with_fields(
                &transition.payload, spatial::EntityFields::selected(&captured.capabilities),
            ).map_err(|error| format!("M04 canonical decode: {error:?}"))?;
            assert!(lights.is_empty());
            assert_eq!(transition.delta.value, decoded);
        }
        assert_eq!(captured.transitions[1].delta.value, unrelated(own_identity)?);
        assert_eq!(captured.transitions[2].delta.value, movement(expected, landing)?);
        let visible = client.world_entities().ok_or("M04 final entity view")?;
        assert_eq!(visible.get(&expected).map(|entity| entity.position), Some(landing));
        assert!(visible.get(&ground(0x74, 9)?.entity).is_none());
        assert_eq!(visible.get(&ground(0x75, 11)?.entity), Some(&ground(0x75, 11)?));
        assert_eq!(visible.get(&creature(5)?.entity), Some(&creature(5)?));
        assert_eq!(visible.get(&actor(own_identity, at(100, 101))?.entity), Some(&actor(own_identity, at(100, 101))?));
    } else {
        let error = result.err().ok_or("M04 accepted a delta without the requested player landing")?;
        assert!(error.to_string().contains("M04 pushed player delta absent"), "{error}");
    }
    drop(client);
    server.await??;
    Ok(())
}
