//! Real production runner against a scripted TLS peer; not a GameNode qualification.
use super::*;
use oteryn_protocol_oteryn::world_object::{
    SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
    encode_world_object_overlay_snapshot,
};
use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, DELTA_TYPE_WORLD_SPATIAL_V1, SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, StepDisposition, WorldSpatialObservation,
    encode_step_result, encode_world_spatial,
};
use oteryn_protocol_oteryn::{
    ALPN_OTERYN_GAME_V1, ChannelId, CommandStatus, DomainSnapshot, FrameLength, GameSessionId,
    ServerAcceptedValue, WorldId, decode_wire_envelope, encode_command_result,
    encode_liveness_probe, encode_server_accepted, encode_single_chunk_snapshot,
    encode_state_delta,
};
use rustls::pki_types::PrivatePkcs8KeyDer;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsAcceptor, server::TlsStream};

type BoxError = Box<dyn Error + Send + Sync>;
type Peer = TlsStream<TcpStream>;

fn uuid(marker: u8) -> [u8; 16] {
    [1, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, marker]
}

async fn write_frame(peer: &mut Peer, body: &[u8]) -> Result<(), BoxError> {
    let length = FrameLength::new(u32::try_from(body.len())?)?;
    peer.write_all(&length.to_prefix()).await?;
    peer.write_all(body).await?;
    peer.flush().await?;
    Ok(())
}

async fn read_frame(peer: &mut Peer) -> Result<Vec<u8>, BoxError> {
    let mut prefix = [0; 4];
    peer.read_exact(&mut prefix).await?;
    let mut body = vec![0; FrameLength::from_prefix(&prefix)?.get() as usize];
    peer.read_exact(&mut body).await?;
    Ok(body)
}

fn spatial(x: i32) -> Vec<u8> {
    encode_world_spatial(&WorldSpatialObservation {
        content_generation: [0x11; 32],
        actor_position: ActorPosition { x, y: 0, floor: 0 },
    })
}

async fn live_run(stall: bool) -> Result<(), BoxError> {
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".into()])?;
    let certificate = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let mut tls = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])?
    .with_no_client_auth()
    .with_single_cert(vec![certificate.clone()], key.into())?;
    tls.alpn_protocols = vec![ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    let acceptor = TlsAcceptor::from(Arc::new(tls));
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let (command_seen, ready) = tokio::sync::oneshot::channel();
    let peer = tokio::spawn(async move {
        let (tcp, _) = listener.accept().await?;
        let mut peer = acceptor.accept(tcp).await?;
        decode_wire_envelope(&read_frame(&mut peer).await?)?.client_bootstrap()?;
        write_frame(
            &mut peer,
            &encode_server_accepted(&ServerAcceptedValue {
                game_session_id: GameSessionId::decode(&uuid(1))?,
                world_id: WorldId::decode(&uuid(2))?,
                channel_id: ChannelId::decode(&uuid(3))?,
                connection_generation: 1,
                current_server_sequence: 0,
                next_command_id: 1,
                schema_revision: 1,
                selected_capabilities: &[],
            })?,
        )
        .await?;
        let spatial_payload = spatial(0);
        let overlay = encode_world_object_overlay_snapshot(&[])
            .map_err(|e| std::io::Error::other(format!("overlay: {e:?}")))?;
        for frame in encode_single_chunk_snapshot(
            1,
            1,
            0,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &spatial_payload,
                },
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 0,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &overlay,
                },
            ],
        )? {
            write_frame(&mut peer, &frame).await?;
        }
        let command = read_frame(&mut peer).await?;
        let id = decode_wire_envelope(&command)?
            .client_command(1)?
            .command_id;
        let _ = command_seen.send(());
        if stall {
            // Liveness traffic keeps the old command wait alive without a result.
            for probe in 1..100 {
                if write_frame(&mut peer, &encode_liveness_probe(1, probe)?)
                    .await
                    .is_err()
                    || read_frame(&mut peer).await.is_err()
                {
                    return Ok::<(), BoxError>(());
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        } else {
            write_frame(
                &mut peer,
                &encode_command_result(
                    1,
                    1,
                    id,
                    CommandStatus::Accepted,
                    &encode_step_result(StepDisposition::Blocked),
                )?,
            )
            .await?;
            tokio::time::sleep(Duration::from_millis(20)).await;
            write_frame(
                &mut peer,
                &encode_state_delta(
                    1,
                    2,
                    STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    1,
                    2,
                    DELTA_TYPE_WORLD_SPATIAL_V1,
                    &spatial(1),
                )?,
            )
            .await?;
            let mut byte = [0];
            let _ = tokio::time::timeout(Duration::from_secs(2), peer.read(&mut byte)).await?;
        }
        Ok::<(), BoxError>(())
    });
    let spec = BotSpec {
        bot_id: 0,
        profile: "tls-regression".into(),
        behavior_seed: [3; 32],
        address,
        server_name: "localhost".into(),
        root_certificate: certificate,
        schema_revision: 1,
        character_id: CharacterId::decode(&uuid(4))?,
        admission_material: b"fixture-grant".to_vec(),
        client_build_id: "player-bots-test".into(),
        deadline: Duration::from_secs(5),
    };
    let config = BotRunConfig::new(
        NonZeroUsize::new(1).ok_or_else(|| std::io::Error::other("nonzero"))?,
        Duration::from_millis(100),
    )?;
    let run = BotSupervisor::live().start(
        vec![spec],
        BotScenario {
            actions: vec![BotAction::Step(StepDirection::East)],
        },
        config,
    )?;
    if tokio::time::timeout(Duration::from_secs(2), ready)
        .await?
        .is_err()
    {
        peer.await??;
        return Err(std::io::Error::other("peer exited before receiving a command").into());
    }
    let reports = if stall {
        tokio::time::timeout(Duration::from_secs(1), run.shutdown()).await?
    } else {
        tokio::time::timeout(Duration::from_secs(2), run.finish()).await?
    };
    assert_eq!(reports.len(), 1);
    assert!(reports[0].metrics.admitted);
    assert_eq!(
        reports[0].terminal,
        if stall {
            BotTerminal::ShutdownRequested
        } else {
            BotTerminal::Completed
        }
    );
    if !stall {
        assert_eq!(reports[0].metrics.events_drained, 1);
        assert_eq!(reports[0].metrics.liveness_cycles, 1);
    }
    tokio::time::timeout(Duration::from_secs(2), peer).await???;
    Ok(())
}

fn runtime() -> Result<tokio::runtime::Runtime, std::io::Error> {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
}

#[test]
fn final_command_services_a_delayed_state_delta() -> Result<(), BoxError> {
    runtime()?.block_on(live_run(false))
}

#[test]
fn shutdown_cancels_a_command_while_the_peer_sends_probes() -> Result<(), BoxError> {
    runtime()?.block_on(live_run(true))
}
