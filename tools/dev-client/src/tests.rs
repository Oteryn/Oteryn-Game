use super::*;
// The fake servers below frame and decode with the same codecs the session crate uses; these
// were `super::*` items before the session extraction, so only this import list changed.
use oteryn_protocol_oteryn::world_object::{
    SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
    WorldObjectOverlayEntry as WireOverlayEntry, encode_world_object_overlay_snapshot,
};
use oteryn_protocol_oteryn::world_spatial::{
    ActorPosition, SNAPSHOT_TYPE_WORLD_SPATIAL_V1, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
    WorldSpatialObservation as WireSpatialObservation, encode_world_spatial,
};
use oteryn_protocol_oteryn::{
    ALPN_OTERYN_GAME_V1, CommandStatus, FrameLength, GameSessionId, decode_snapshot_chunk_framing,
    decode_wire_envelope,
};
use oteryn_protocol_oteryn::{
    ChannelId, DomainSnapshot, MessageType, ServerAcceptedValue, WorldId, encode_server_accepted,
    encode_single_chunk_snapshot,
};
use oteryn_session::{read_frame, write_frame};
use rustls::pki_types::PrivatePkcs8KeyDer;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use world_object::{UseDisposition, WorldObjectTarget};
use world_spatial::StepDisposition;

const DOOR_PLACEMENT: &str = "oteryn:cell/entry-door";
const DOOR_STATE: &str = "oteryn:reference.state.closed";
const CLIENT_BUILD_ID: &str = "oteryn-dev-client-test";
/// Generous enough that it never fires in the happy-path/negative tests below (all
/// same-process, no real network latency); the stall test uses its own short deadline.
const TEST_DEADLINE: Duration = Duration::from_secs(5);

fn test_uuid_v7(marker: u8) -> [u8; 16] {
    [
        0x01, 0x93, 0x4f, 0x10, 0x7c, 0x00, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}

/// One committed-players-free fake server: accept one TLS connection, decode the incoming
/// `ClientBootstrap` back through `oteryn-protocol-oteryn`'s own server-direction decode
/// (proving the encoder `connect_and_join` drives is exactly what a real server ingests),
/// then reply with `ServerAccepted` and a join snapshot carrying both state domains — the
/// native entry door overlay (`oteryn:cell/entry-door`, closed, revision 0) among them.
async fn run_fake_server(
    listener: TcpListener,
    acceptor: tokio_rustls::TlsAcceptor,
) -> Result<(), Box<dyn StdError + Send + Sync>> {
    let (tcp, _) = listener.accept().await?;
    let mut stream = acceptor.accept(tcp).await?;

    let mut prefix = [0_u8; 4];
    stream.read_exact(&mut prefix).await?;
    let length = FrameLength::from_prefix(&prefix)?;
    let mut body = vec![0_u8; length.get() as usize];
    stream.read_exact(&mut body).await?;
    let envelope = decode_wire_envelope(&body)?;
    let bootstrap = envelope.client_bootstrap()?;
    assert_eq!(bootstrap.client_build_id, CLIENT_BUILD_ID);
    assert_eq!(bootstrap.admission_material, b"fixture-grant");

    let accepted = encode_server_accepted(&ServerAcceptedValue {
        game_session_id: GameSessionId::decode(&test_uuid_v7(1))?,
        world_id: WorldId::decode(&test_uuid_v7(2))?,
        channel_id: ChannelId::decode(&test_uuid_v7(3))?,
        connection_generation: 1,
        current_server_sequence: 0,
        next_command_id: 1,
        schema_revision: 1,
        selected_capabilities: &[],
    })?;
    write_frame(&mut stream, &accepted).await?;

    let content_generation = [0x11_u8; 32];
    let spatial_payload = encode_world_spatial(&WireSpatialObservation {
        content_generation,
        actor_position: ActorPosition {
            x: 0,
            y: 0,
            floor: 0,
        },
    });
    let overlay_payload = encode_world_object_overlay_snapshot(&[WireOverlayEntry {
        content_generation,
        placement: DOOR_PLACEMENT.as_bytes().to_vec(),
        state: DOOR_STATE.as_bytes().to_vec(),
        revision: 0,
    }])
    .map_err(|error| format!("encode_world_object_overlay_snapshot: {error:?}"))?;
    let snapshot = encode_single_chunk_snapshot(
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
                payload: &overlay_payload,
            },
        ],
    )?;
    for frame in snapshot {
        write_frame(&mut stream, &frame).await?;
    }
    stream.flush().await?;
    Ok(())
}

/// Builds a manual multi-thread runtime rather than relying on `#[tokio::test]`, which needs
/// tokio's `macros` feature; this workspace pins `tokio` to a vendored, offline-built path
/// dependency (root `Cargo.toml` `[patch.crates-io]`) that does not carry `tokio-macros`.
/// `apps/game-server`'s own qualification tests build their runtime the same way.
fn block_on<F: std::future::Future>(
    future: F,
) -> Result<F::Output, Box<dyn StdError + Send + Sync>> {
    Ok(tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(future))
}

// Include topic files in this module to preserve test paths and private fixture access.
include!("client_tests_admission.rs");
include!("client_tests_snapshots.rs");
include!("client_tests_command_fixtures.rs");
include!("client_tests_commands.rs");
include!("client_tests_command_errors.rs");
include!("client_tests_liveness.rs");
