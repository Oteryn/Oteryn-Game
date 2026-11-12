//! Logout cleanup through the real TLS/session command owner and native codecs.
use super::*;

const FIRST_COMMAND_ID: u64 = 7;
const JOIN_SEQUENCE: u64 = 40;
type ServerStream = tokio_rustls::server::TlsStream<TcpStream>;

fn run_transport<F, Fut>(scenario: F) -> TestResult
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: std::future::Future<Output = TestResult>,
{
    let thread = std::thread::Builder::new()
        .name("logout-cleanup-tls".into())
        .stack_size(64 << 20)
        .spawn(move || {
            let result = (|| -> TestResult {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(tokio::task::LocalSet::new().run_until(async {
                    tokio::time::timeout(Duration::from_secs(20), scenario())
                        .await
                        .map_err(|_| "logout cleanup TLS fixture deadline")?
                }))
            })();
            result.map_err(|error| error.to_string())
        })?;
    match thread.join() {
        Ok(result) => result.map_err(Into::into),
        Err(panic) => std::panic::resume_unwind(panic),
    }
}

async fn send(stream: &mut ServerStream, frame: &[u8]) -> TestResult {
    stream.write_all(&framed(frame)).await?;
    stream.flush().await?;
    Ok(())
}

async fn connect_peer<F, Fut>(
    script: F,
) -> TestResult<(DevClientSession, tokio::task::JoinHandle<TestResult>)>
where
    F: FnOnce(ServerStream) -> Fut + 'static,
    Fut: std::future::Future<Output = TestResult> + 'static,
{
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
    let server = tokio::task::spawn_local(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        let bootstrap = light_read(&mut stream).await?;
        let envelope = native::decode_wire_envelope(&bootstrap)?;
        let bootstrap = envelope.client_bootstrap()?;
        let selected = [6, 13, 21, 64];
        assert!(
            selected
                .iter()
                .all(|capability| bootstrap.supported_capabilities.contains(capability))
        );
        send(
            &mut stream,
            &native::encode_server_accepted(&native::ServerAcceptedValue {
                game_session_id: native::GameSessionId::decode(&v7(1, 1))?,
                world_id: native::WorldId::decode(&v7(2, 2))?,
                channel_id: native::ChannelId::decode(&v7(3, 3))?,
                connection_generation: 1,
                current_server_sequence: JOIN_SEQUENCE,
                next_command_id: FIRST_COMMAND_ID,
                schema_revision: 1,
                selected_capabilities: &selected,
            })?,
        )
        .await?;
        let position = ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        };
        let own = v7(4, 4);
        let entities = spatial::encode_world_spatial_entities_snapshot(
            &spatial::WorldSpatialEntitiesSnapshot {
                content_generation: [0x11; 32],
                actor_position: position,
                own_identity: own,
                entities: vec![spatial::WorldSpatialEntity {
                    display_name: None,
                    kind: spatial::EntityKind::Player,
                    entity: spatial::EntityRef {
                        identity: own,
                        generation: 1,
                    },
                    position,
                    detail: spatial::EntityDetail::Actor {
                        direction: StepDirection::South,
                        appearance_ref: 1,
                        health_percent: 100,
                    },
                }],
            },
        )
        .map_err(|error| format!("cleanup entities snapshot: {error:?}"))?;
        let overlay = native::world_object::encode_world_object_overlay_snapshot(&[])
            .map_err(|error| format!("cleanup overlay snapshot: {error:?}"))?;
        for frame in native::encode_single_chunk_snapshot(
            1,
            1,
            JOIN_SEQUENCE,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 0,
                    snapshot_type: spatial::SNAPSHOT_TYPE_WORLD_SPATIAL_ENTITIES_V2,
                    payload: &entities,
                },
                DomainSnapshot {
                    domain_id: native::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 0,
                    snapshot_type: native::world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
            ],
        )? {
            send(&mut stream, &frame).await?;
        }
        script(stream).await
    });
    let client = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id: CharacterId::decode(&v7(4, 4))?,
        admission_material: b"logout-cleanup-fixture",
        client_build_id: "logout-cleanup-tls-test",
        deadline: Duration::from_secs(5),
    })
    .await?;
    Ok((client, server))
}

async fn expect_command(
    stream: &mut ServerStream,
    command_id: u64,
    kind: u32,
    payload: &[u8],
) -> TestResult {
    let bytes = light_read(stream).await?;
    let command = native::decode_wire_envelope(&bytes)?.client_command(1)?;
    assert_eq!(
        (command.command_id, command.command_type),
        (command_id, kind)
    );
    assert_eq!(command.payload, payload);
    Ok(())
}

async fn reply(
    stream: &mut ServerStream,
    command_id: u64,
    status: CommandStatus,
    payload: &[u8],
) -> TestResult {
    send(
        stream,
        &native::encode_command_result(
            1,
            JOIN_SEQUENCE + command_id - FIRST_COMMAND_ID + 1,
            command_id,
            status,
            payload,
        )?,
    )
    .await
}

async fn busy_logout(stream: &mut ServerStream) -> TestResult {
    expect_command(stream, 7, native::logout::COMMAND_TYPE_LOGOUT_INTENT, &[]).await?;
    let payload = native::logout::encode_logout_result(native::logout::LogoutResult::Busy)
        .map_err(|error| format!("cleanup logout payload: {error:?}"))?;
    reply(stream, 7, CommandStatus::Rejected, &payload).await
}

async fn receive_cleanup_turn(stream: &mut ServerStream) -> TestResult {
    busy_logout(stream).await?;
    let payload = native::target_move::encode_target_move_intent(
        native::target_move::TargetMoveIntent::Turn(StepDirection::North),
    )
    .map_err(|error| format!("cleanup turn payload: {error:?}"))?;
    expect_command(
        stream,
        8,
        native::target_move::COMMAND_TYPE_TARGET_MOVE_INTENT,
        &payload,
    )
    .await
}

async fn expect_close(stream: &mut ServerStream) -> TestResult {
    let mut byte = [0];
    match stream.read(&mut byte).await {
        Ok(0) => Ok(()),
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::UnexpectedEof | std::io::ErrorKind::ConnectionReset
            ) =>
        {
            Ok(())
        }
        other => Err(format!("failed cleanup sent another command: {other:?}").into()),
    }
}

#[test]
fn logout_cleanup_retries_after_an_accepted_or_rejected_turn_over_tls() -> TestResult {
    run_transport(|| async {
        for (status, payload) in [
            (CommandStatus::Accepted, [8, 1]),
            (CommandStatus::Rejected, [8, 3]),
        ] {
            let (mut client, peer) = connect_peer(move |mut stream| async move {
                receive_cleanup_turn(&mut stream).await?;
                reply(&mut stream, 8, status, &payload).await?;
                expect_command(
                    &mut stream,
                    9,
                    native::logout::COMMAND_TYPE_LOGOUT_INTENT,
                    &[],
                )
                .await?;
                let payload =
                    native::logout::encode_logout_result(native::logout::LogoutResult::Accepted)
                        .map_err(|error| format!("cleanup accepted payload: {error:?}"))?;
                reply(&mut stream, 9, CommandStatus::Accepted, &payload).await
            })
            .await?;
            release_when_unlocked(&mut client, Duration::from_secs(5)).await?;
            assert_eq!(client.next_command_id(), 10);
            assert_eq!(client.last_server_sequence(), 43);
            drop(client);
            peer.await??;
        }
        Ok(())
    })
}

#[test]
fn logout_cleanup_rejects_malformed_turn_reply_pairs_over_tls() -> TestResult {
    run_transport(|| async {
        let cases: [(CommandStatus, &'static [u8]); 5] = [
            (CommandStatus::Accepted, &[8, 3]),
            (CommandStatus::Rejected, &[8, 1]),
            (CommandStatus::Accepted, &[8, 2]),
            (CommandStatus::Rejected, &[]),
            (CommandStatus::Rejected, &[8, 3, 0]),
        ];
        for (status, payload) in cases {
            let (mut client, peer) = connect_peer(move |mut stream| async move {
                receive_cleanup_turn(&mut stream).await?;
                reply(&mut stream, 8, status, payload).await?;
                expect_close(&mut stream).await
            })
            .await?;
            let error = release_when_unlocked(&mut client, Duration::from_secs(5))
                .await
                .err()
                .ok_or("malformed cleanup TURN unexpectedly succeeded")?;
            assert!(error.to_string().contains("ordinary activity turn refused"));
            assert_eq!(client.next_command_id(), 9);
            drop(client);
            peer.await??;
        }
        Ok(())
    })
}

#[test]
fn logout_cleanup_still_requires_the_final_accepted_logout_reply() -> TestResult {
    run_transport(|| async {
        let (mut client, peer) = connect_peer(|mut stream| async move {
            receive_cleanup_turn(&mut stream).await?;
            reply(&mut stream, 8, CommandStatus::Rejected, &[8, 3]).await?;
            expect_command(
                &mut stream,
                9,
                native::logout::COMMAND_TYPE_LOGOUT_INTENT,
                &[],
            )
            .await?;
            reply(&mut stream, 9, CommandStatus::Rejected, &[]).await?;
            expect_close(&mut stream).await
        })
        .await?;
        let error = release_when_unlocked(&mut client, Duration::from_secs(5))
            .await
            .err()
            .ok_or("refused final logout unexpectedly succeeded")?;
        assert!(
            error
                .to_string()
                .contains("ordinary logout did not release")
        );
        assert_eq!(client.next_command_id(), 10);
        drop(client);
        peer.await??;
        Ok(())
    })
}

#[test]
fn logout_cleanup_rejects_eof_before_turn_or_final_logout_result() -> TestResult {
    run_transport(|| async {
        for after_turn in [false, true] {
            let (mut client, peer) = connect_peer(move |mut stream| async move {
                receive_cleanup_turn(&mut stream).await?;
                if after_turn {
                    reply(&mut stream, 8, CommandStatus::Rejected, &[8, 3]).await?;
                    expect_command(
                        &mut stream,
                        9,
                        native::logout::COMMAND_TYPE_LOGOUT_INTENT,
                        &[],
                    )
                    .await?;
                }
                // The next awaited result never arrives; a TURN refusal is not release.
                Ok(())
            })
            .await?;
            assert!(
                release_when_unlocked(&mut client, Duration::from_secs(5))
                    .await
                    .is_err()
            );
            assert_eq!(client.next_command_id(), if after_turn { 10 } else { 9 });
            drop(client);
            peer.await??;
        }
        Ok(())
    })
}

#[test]
fn logout_cleanup_expired_budget_rejects_busy_without_sending_a_turn() -> TestResult {
    run_transport(|| async {
        let (mut client, peer) = connect_peer(|mut stream| async move {
            busy_logout(&mut stream).await?;
            expect_close(&mut stream).await
        })
        .await?;
        let error = release_when_unlocked(&mut client, Duration::ZERO)
            .await
            .err()
            .ok_or("expired cleanup budget unexpectedly succeeded")?;
        assert!(
            error
                .to_string()
                .contains("ordinary logout did not release")
        );
        assert_eq!(client.next_command_id(), 8);
        drop(client);
        peer.await??;
        Ok(())
    })
}
