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
        let (world, _) = compiled_town()?;
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
