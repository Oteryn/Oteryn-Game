/// End to end against a same-process fake server: TLS 1.3 + ALPN handshake, `ClientBootstrap`
/// admission, and a decoded join snapshot whose `WORLD_OBJECT_OVERLAY` entry is the native
/// entry door — by placement `oteryn:cell/entry-door` and its state key, per #162 comment
/// 5875470550 option 3.
#[test]
fn connect_and_join_decodes_join_snapshot_with_entry_door_overlay()
-> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(run_join_snapshot_case())?
}

async fn run_join_snapshot_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let certificate: CertificateDer<'static> = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());

    let mut server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(vec![certificate.clone()], key.into())?;
    server_config.alpn_protocols = vec![ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let server = tokio::spawn(run_fake_server(listener, acceptor));

    let character_id = CharacterId::decode(&test_uuid_v7(4))?;
    let snapshot = connect_and_join(JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id,
        admission_material: b"fixture-grant",
        client_build_id: CLIENT_BUILD_ID,
        deadline: TEST_DEADLINE,
    })
    .await
    .map_err(|error| format!("connect_and_join: {error}"))?;

    server.await??;

    assert_eq!(
        snapshot.game_session_id,
        GameSessionId::decode(&test_uuid_v7(1))?
    );
    assert_eq!(
        snapshot.world_spatial.actor_position,
        ActorPosition {
            x: 0,
            y: 0,
            floor: 0
        }
    );
    assert_eq!(snapshot.world_object_overlay.len(), 1);
    let door = &snapshot.world_object_overlay[0];
    assert_eq!(door.placement, DOOR_PLACEMENT.as_bytes());
    assert_eq!(door.state, DOOR_STATE.as_bytes());
    assert_eq!(door.revision, 0);
    assert_eq!(snapshot.world_spatial_revision, 1);
    assert_eq!(snapshot.world_object_overlay_revision, 0);
    Ok(())
}

#[test]
fn connect_and_join_reports_refusal_when_server_closes_before_admission()
-> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(run_server_closes_before_admission_case())?
}

async fn run_server_closes_before_admission_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let certificate: CertificateDer<'static> = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let mut server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(vec![certificate.clone()], key.into())?;
    server_config.alpn_protocols = vec![ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    tokio::spawn(async move {
        if let Ok((tcp, _)) = listener.accept().await
            && let Ok(mut stream) = acceptor.accept(tcp).await
        {
            let _ = stream.shutdown().await;
        }
    });

    let character_id = CharacterId::decode(&test_uuid_v7(4))?;
    let result = connect_and_join(JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id,
        admission_material: b"fixture-grant",
        client_build_id: CLIENT_BUILD_ID,
        deadline: TEST_DEADLINE,
    })
    .await;
    assert!(result.is_err());
    Ok(())
}

/// Reads one framed `ClientBootstrap` off `stream` and discards it — every fixture below
/// needs a well-formed request to answer, but none inspects its contents.
async fn read_and_discard_bootstrap(
    stream: &mut tokio_rustls::server::TlsStream<TcpStream>,
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    let mut prefix = [0_u8; 4];
    stream.read_exact(&mut prefix).await?;
    let length = FrameLength::from_prefix(&prefix)?;
    let mut body = vec![0_u8; length.get() as usize];
    stream.read_exact(&mut body).await?;
    decode_wire_envelope(&body)?.client_bootstrap()?;
    Ok(())
}

fn fake_accepted_frame() -> Result<Vec<u8>, Box<dyn StdError + Send + Sync>> {
    Ok(encode_server_accepted(&ServerAcceptedValue {
        game_session_id: GameSessionId::decode(&test_uuid_v7(1))?,
        world_id: WorldId::decode(&test_uuid_v7(2))?,
        channel_id: ChannelId::decode(&test_uuid_v7(3))?,
        connection_generation: 1,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: 1,
        selected_capabilities: &[],
    })?)
}

async fn tls_test_listener() -> Result<
    (
        CertificateDer<'static>,
        tokio_rustls::TlsAcceptor,
        TcpListener,
        SocketAddr,
    ),
    Box<dyn StdError + Send + Sync>,
> {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let certificate: CertificateDer<'static> = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let mut server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(vec![certificate.clone()], key.into())?;
    server_config.alpn_protocols = vec![ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    Ok((certificate, acceptor, listener, address))
}

/// Codex P2 finding 2 (#162 review on `1efc029`): a `SnapshotChunk` whose `snapshot_id`
/// does not match `SnapshotBegin`'s must be rejected, not silently accepted.
#[test]
fn connect_and_join_rejects_a_snapshot_chunk_with_mismatched_snapshot_id()
-> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(run_mismatched_snapshot_id_case())?
}

async fn run_mismatched_snapshot_id_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
    let (certificate, acceptor, listener, address) = tls_test_listener().await?;
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        read_and_discard_bootstrap(&mut stream).await?;
        write_frame(&mut stream, &fake_accepted_frame()?).await?;

        let payload = [0_u8];
        let domain = [DomainSnapshot {
            domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision: 1,
            snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
            payload: &payload,
        }];
        // SnapshotBegin carries snapshot_id 1; SnapshotChunk carries a mismatched 2.
        let begin = encode_single_chunk_snapshot(1, 1, 0, &domain)?;
        let mismatched = encode_single_chunk_snapshot(1, 2, 0, &domain)?;
        write_frame(&mut stream, &begin[0]).await?;
        write_frame(&mut stream, &mismatched[1]).await?;
        stream.flush().await?;
        Ok::<(), Box<dyn StdError + Send + Sync>>(())
    });

    let character_id = CharacterId::decode(&test_uuid_v7(4))?;
    let result = connect_and_join(JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id,
        admission_material: b"fixture-grant",
        client_build_id: CLIENT_BUILD_ID,
        deadline: TEST_DEADLINE,
    })
    .await;
    assert!(matches!(
        result,
        Err(DevClientError::Session(SessionError::SnapshotIdMismatch {
            expected: 1,
            actual: 2
        }))
    ));
    server.await??;
    Ok(())
}

/// Codex P2 finding 3 (#162 review on `1efc029`): a server that admits and then stalls
/// (never sends `SnapshotBegin`) must fail with a bounded diagnostic, not hang.
#[test]
fn connect_and_join_times_out_when_the_server_stalls_after_admission()
-> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(run_stalls_after_admission_case())?
}

async fn run_stalls_after_admission_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
    let (certificate, acceptor, listener, address) = tls_test_listener().await?;
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        read_and_discard_bootstrap(&mut stream).await?;
        write_frame(&mut stream, &fake_accepted_frame()?).await?;
        // Stall: never send SnapshotBegin. Held well past the client's short deadline below.
        tokio::time::sleep(Duration::from_secs(5)).await;
        Ok::<(), Box<dyn StdError + Send + Sync>>(())
    });

    let character_id = CharacterId::decode(&test_uuid_v7(4))?;
    let result = connect_and_join(JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id,
        admission_material: b"fixture-grant",
        client_build_id: CLIENT_BUILD_ID,
        deadline: Duration::from_millis(200),
    })
    .await;
    assert!(matches!(
        result,
        Err(DevClientError::Session(SessionError::Timeout(
            "SnapshotBegin"
        )))
    ));
    server.abort();
    Ok(())
}

/// Codex P2 finding 4 (#162 review on `1efc029`): `PROTOCOL_OTERYN_V1_REGISTRY.json`
/// registers exactly one `snapshot_type` (1) for domains 1 and 2; anything else naming one
/// of those domains must be rejected, not silently treated as an unknown domain.
#[test]
fn connect_and_join_rejects_an_unregistered_snapshot_type_for_a_known_domain()
-> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(run_unregistered_snapshot_type_case())?
}

async fn run_unregistered_snapshot_type_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
    let (certificate, acceptor, listener, address) = tls_test_listener().await?;
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        read_and_discard_bootstrap(&mut stream).await?;
        write_frame(&mut stream, &fake_accepted_frame()?).await?;

        let payload = [0_u8];
        let frames = encode_single_chunk_snapshot(
            1,
            1,
            0,
            &[DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                // Not registered for domain 1 (only snapshot_type 1 is).
                snapshot_type: 2,
                payload: &payload,
            }],
        )?;
        for frame in frames {
            write_frame(&mut stream, &frame).await?;
        }
        stream.flush().await?;
        Ok::<(), Box<dyn StdError + Send + Sync>>(())
    });

    let character_id = CharacterId::decode(&test_uuid_v7(4))?;
    let result = connect_and_join(JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id,
        admission_material: b"fixture-grant",
        client_build_id: CLIENT_BUILD_ID,
        deadline: TEST_DEADLINE,
    })
    .await;
    assert!(matches!(
        result,
        Err(DevClientError::Session(
            SessionError::UnregisteredSnapshotType {
                domain_id: 1,
                snapshot_type: 2
            }
        ))
    ));
    server.await??;
    Ok(())
}

/// Like `tls_test_listener`, but deliberately configures no ALPN protocol: a trusted server
/// (valid, chain-verifiable certificate) whose handshake completes without negotiating one.
async fn tls_test_listener_without_alpn() -> Result<
    (
        CertificateDer<'static>,
        tokio_rustls::TlsAcceptor,
        TcpListener,
        SocketAddr,
    ),
    Box<dyn StdError + Send + Sync>,
> {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let certificate: CertificateDer<'static> = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let server_config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(vec![certificate.clone()], key.into())?;
    let acceptor = tokio_rustls::TlsAcceptor::from(Arc::new(server_config));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    Ok((certificate, acceptor, listener, address))
}

/// Codex P2 finding 1 (#162 review on PR #1147, `f0f27df`): a trusted TLS server that never
/// negotiates ALPN must be rejected before anything is sent, not merely proceed unauthenticated.
#[test]
fn connect_and_join_rejects_a_connection_with_no_negotiated_alpn()
-> Result<(), Box<dyn StdError + Send + Sync>> {
    block_on(run_no_alpn_case())?
}

async fn run_no_alpn_case() -> Result<(), Box<dyn StdError + Send + Sync>> {
    let (certificate, acceptor, listener, address) = tls_test_listener_without_alpn().await?;
    let server = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        let mut stream = acceptor.accept(tcp).await?;
        // The client must detect the ALPN mismatch and send nothing: a bounded read
        // observing EOF (or a reset) rather than any bytes proves it.
        let mut buffer = [0_u8; 1];
        match tokio::time::timeout(Duration::from_millis(300), stream.read(&mut buffer)).await {
            Ok(Ok(0)) | Ok(Err(_)) => {}
            Ok(Ok(_)) => return Err("client sent bytes after an ALPN mismatch".into()),
            Err(_) => return Err("client did not close after an ALPN mismatch".into()),
        }
        Ok::<(), Box<dyn StdError + Send + Sync>>(())
    });

    let character_id = CharacterId::decode(&test_uuid_v7(4))?;
    let result = connect_and_join(JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id,
        admission_material: b"fixture-grant",
        client_build_id: CLIENT_BUILD_ID,
        deadline: TEST_DEADLINE,
    })
    .await;
    assert!(matches!(result, Err(DevClientError::AlpnMismatch)));
    server.await??;
    Ok(())
}
