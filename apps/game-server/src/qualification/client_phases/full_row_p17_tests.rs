//! Exercise both P17 journeys' shared edit through TLS and the native session owner.
use super::*;
use oteryn_protocol_oteryn::{self as native, exiva};

#[test]
fn p17_named_allow_list_enables_its_mode_before_adding_the_player() -> TestResult {
    run_transport(false)
}

#[test]
fn p17_refused_mode_change_does_not_add_a_player() -> TestResult {
    run_transport(true)
}

fn run_transport(refuse_modes: bool) -> TestResult {
    let thread = std::thread::Builder::new()
        .name("p17-settings-tls".into())
        .stack_size(64 << 20)
        .spawn(move || {
            let result = (|| -> TestResult {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()?;
                runtime.block_on(tokio::task::LocalSet::new().run_until(async {
                    tokio::time::timeout(Duration::from_secs(20), exercise(refuse_modes))
                        .await
                        .map_err(|_| "P17 settings TLS fixture deadline")?
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
        return Err("P17 frame outside native size bound".into());
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

async fn exercise(refuse_modes: bool) -> TestResult {
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
        let bytes = read(&mut stream).await?;
        let envelope = native::decode_wire_envelope(&bytes)?;
        assert!(envelope.client_bootstrap()?.supported_capabilities.contains(&147));
        send(&mut stream, &native::encode_server_accepted(&native::ServerAcceptedValue {
            game_session_id: native::GameSessionId::decode(&v7(1, 1))?,
            world_id: native::WorldId::decode(&v7(2, 2))?,
            channel_id: native::ChannelId::decode(&v7(3, 3))?,
            connection_generation: 1,
            current_server_sequence: 40,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[147],
        })?).await?;
        let spatial = encode_world_spatial(&WorldSpatialObservation {
            content_generation: [0x11; 32],
            actor_position: ActorPosition { x: 0, y: 0, floor: 0 },
        });
        let overlay = native::world_object::encode_world_object_overlay_snapshot(&[])
            .map_err(|error| format!("P17 overlay: {error:?}"))?;
        for frame in native::encode_single_chunk_snapshot(1, 1, 40, &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 0,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial,
            },
            DomainSnapshot {
                domain_id: native::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 0,
                snapshot_type: native::world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &overlay,
            },
        ])? {
            send(&mut stream, &frame).await?;
        }
        let mut state = PlayerList::default();
        let mut next_id = 7;
        loop {
            let bytes = read(&mut stream).await?;
            let command = native::decode_wire_envelope(&bytes)?.client_command(1)?;
            assert_eq!(command.command_id, next_id);
            assert_eq!(command.command_type, COMMAND_TYPE_EXIVA_SETTINGS);
            let request = exiva::Request::decode(command.payload).ok_or("P17 request undecodable")?;
            let mut status = CommandStatus::Accepted;
            let payload = match request.operation {
                exiva::Operation::SetModes if refuse_modes => {
                    status = CommandStatus::Rejected;
                    vec![0]
                }
                exiva::Operation::SetModes => {
                    state.modes = request.modes;
                    vec![1]
                }
                exiva::Operation::AddName => {
                    state.players.push((v7(8, 8), request.name.ok_or("P17 name absent")?));
                    vec![1]
                }
                exiva::Operation::List => state.encode().ok_or("P17 List encode")?,
                _ => return Err("unexpected P17 fixture operation".into()),
            };
            send(&mut stream, &native::encode_command_result(
                1, 41 + next_id - 7, next_id, status, &payload,
            )?).await?;
            next_id += 1;
            if request.operation == exiva::Operation::List {
                return Ok::<_, Box<dyn std::error::Error>>(state);
            }
        }
    });
    let mut client = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address,
        server_name: "localhost",
        root_certificate: &certificate,
        schema_revision: 1,
        character_id: CharacterId::decode(&v7(4, 4))?,
        admission_material: b"p17-settings-fixture",
        client_build_id: "p17-settings-tls-test",
        deadline: Duration::from_secs(5),
    }).await?;
    let result = allow_named_player(&mut client, "Packet Two").await;
    let observed = list(&mut client).await?;
    assert_eq!(server.await??, observed);
    if refuse_modes {
        assert!(result.is_err(), "a refused mode change must stop the allow-list edit");
        assert_eq!(observed, PlayerList::default());
    } else {
        result?;
        assert_eq!(observed, PlayerList {
            modes: 2,
            players: vec![(v7(8, 8), "Packet Two".into())],
        }, "AddName alone must not be mistaken for enabling the player allow list");
    }
    Ok(())
}
