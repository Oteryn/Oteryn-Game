//! Dev/qualification-only native Oteryn Game client (ADR-0011 §6): connects to a real game
//! server over rustls TLS 1.3 with ALPN `oteryn-game/1`, admits with a caller-supplied grant
//! (the same `ClientBootstrap` mechanism the shipped server accepts), and decodes the join
//! snapshot's state domains.
//!
//! Every wire codec used here is `oteryn-protocol-oteryn`'s own (`encode_client_bootstrap`,
//! `decode_wire_envelope`/`decode_framed_envelope`, `decode_server_accepted`,
//! `decode_snapshot_chunk`, and the `world_spatial`/`world_object` domain codecs): this crate
//! holds no codec of its own, only the TLS transport and the glue that ties one admission to its
//! join-snapshot decode.
//!
//! Not a production client entry. The shipped native client stays fail-closed behind
//! `PreNativeProtocol` (ADR-0011 §3/§5); this dev harness is the explicit exception ADR-0011 §6
//! allows, and it is kept out of both production closures (`oteryn-client`, `oteryn-game-server`)
//! by `workspace-boundaries.toml`.

use oteryn_protocol_oteryn::world_object::{self, WorldObjectOverlayEntry};
use oteryn_protocol_oteryn::world_spatial::{self, WorldSpatialObservation};
use oteryn_protocol_oteryn::{
    ALPN_OTERYN_GAME_V1, CharacterId, ClientBootstrapValue, FoundationProtocolError, FrameLength,
    GameSessionId, MessageType, decode_server_accepted, decode_snapshot_chunk,
    decode_wire_envelope, encode_client_bootstrap,
};
use rustls::pki_types::{CertificateDer, ServerName};
use std::error::Error as StdError;
use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_rustls::client::TlsStream;

/// Everything `connect_and_join` needs to admit with one grant and read its join snapshot.
#[derive(Debug, Clone, Copy)]
pub struct JoinRequest<'a> {
    /// The server's real loopback (or routable) TCP address.
    pub address: SocketAddr,
    /// The TLS server name to verify the presented certificate against.
    pub server_name: &'a str,
    /// The single trust root the connection's certificate must chain to.
    pub root_certificate: &'a CertificateDer<'static>,
    /// FND-02 `ClientBootstrap.schema_revision`. Any nonzero value the server accepts.
    pub schema_revision: u32,
    pub character_id: CharacterId,
    /// The signed admission grant (e.g. a WP5 fixture grant JWT), as the wire's
    /// `admission_material` bytes.
    pub admission_material: &'a [u8],
    pub client_build_id: &'a str,
}

/// The join snapshot: every domain the server sent right after `ServerAccepted`, decoded through
/// `oteryn-protocol-oteryn`'s own domain codecs (state domains 1 `WORLD_SPATIAL` and 2
/// `WORLD_OBJECT_OVERLAY`).
#[derive(Debug, Clone)]
pub struct JoinSnapshot {
    pub game_session_id: GameSessionId,
    pub world_spatial: WorldSpatialObservation,
    pub world_object_overlay: Vec<WorldObjectOverlayEntry>,
}

#[derive(Debug)]
pub enum DevClientError {
    Tls(rustls::Error),
    Io(io::Error),
    InvalidServerName,
    Protocol(FoundationProtocolError),
    WorldSpatial(world_spatial::WorldSpatialError),
    WorldObject(world_object::WorldObjectError),
    /// The server closed, or replied with something other than `ServerAccepted`, before
    /// admission completed.
    NotAdmitted(MessageType),
    /// A later frame's message type did not match what the join sequence expects next.
    UnexpectedMessage {
        expected: MessageType,
        actual: MessageType,
    },
    /// The join snapshot did not carry a domain this client needed.
    MissingDomain(u32),
}

impl fmt::Display for DevClientError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tls(error) => write!(formatter, "TLS setup failed: {error}"),
            Self::Io(error) => write!(formatter, "transport I/O failed: {error}"),
            Self::InvalidServerName => write!(formatter, "invalid TLS server name"),
            Self::Protocol(error) => write!(formatter, "FND-02 protocol error: {error}"),
            Self::WorldSpatial(error) => {
                write!(formatter, "WORLD_SPATIAL decode failed: {error:?}")
            }
            Self::WorldObject(error) => {
                write!(formatter, "WORLD_OBJECT_OVERLAY decode failed: {error:?}")
            }
            Self::NotAdmitted(message_type) => {
                write!(formatter, "admission refused: server sent {message_type:?}")
            }
            Self::UnexpectedMessage { expected, actual } => write!(
                formatter,
                "expected {expected:?} next, server sent {actual:?}"
            ),
            Self::MissingDomain(domain_id) => {
                write!(formatter, "join snapshot did not carry domain {domain_id}")
            }
        }
    }
}

impl StdError for DevClientError {}

impl From<io::Error> for DevClientError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<FoundationProtocolError> for DevClientError {
    fn from(error: FoundationProtocolError) -> Self {
        Self::Protocol(error)
    }
}

impl From<world_spatial::WorldSpatialError> for DevClientError {
    fn from(error: world_spatial::WorldSpatialError) -> Self {
        Self::WorldSpatial(error)
    }
}

impl From<world_object::WorldObjectError> for DevClientError {
    fn from(error: world_object::WorldObjectError) -> Self {
        Self::WorldObject(error)
    }
}

/// Connects to `request.address` over rustls TLS 1.3 with ALPN `oteryn-game/1`, sends a
/// `ClientBootstrap` built from `request`, and decodes the join snapshot the server sends right
/// after `ServerAccepted` (`SnapshotBegin`, `SnapshotChunk`, `SnapshotCommit` — FND-02 §16).
pub async fn connect_and_join(request: JoinRequest<'_>) -> Result<JoinSnapshot, DevClientError> {
    let connector = tls_connector(request.root_certificate)?;
    let tcp = TcpStream::connect(request.address).await?;
    let server_name = ServerName::try_from(request.server_name.to_owned())
        .map_err(|_error| DevClientError::InvalidServerName)?;
    let mut stream = connector.connect(server_name, tcp).await?;

    let bootstrap = encode_client_bootstrap(&ClientBootstrapValue {
        schema_revision: request.schema_revision,
        character_id: request.character_id,
        admission_material: request.admission_material,
        client_build_id: request.client_build_id,
        supported_capabilities: &[],
    })?;
    write_frame(&mut stream, &bootstrap).await?;

    let accepted = read_frame(&mut stream).await?;
    let accepted_envelope = decode_wire_envelope(&accepted)?;
    if accepted_envelope.message_type() != MessageType::ServerAccepted {
        return Err(DevClientError::NotAdmitted(
            accepted_envelope.message_type(),
        ));
    }
    let accepted_fields = decode_server_accepted(accepted_envelope.payload())?;

    expect_message(&mut stream, MessageType::SnapshotBegin).await?;
    let chunk_frame = read_frame(&mut stream).await?;
    let chunk_envelope = decode_wire_envelope(&chunk_frame)?;
    if chunk_envelope.message_type() != MessageType::SnapshotChunk {
        return Err(DevClientError::UnexpectedMessage {
            expected: MessageType::SnapshotChunk,
            actual: chunk_envelope.message_type(),
        });
    }
    let (_snapshot_id, domains) = decode_snapshot_chunk(chunk_envelope.payload())?;
    expect_message(&mut stream, MessageType::SnapshotCommit).await?;

    let mut world_spatial_observation = None;
    let mut world_object_overlay = None;
    for domain in &domains {
        match domain.domain_id {
            world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY => {
                world_spatial_observation =
                    Some(world_spatial::decode_world_spatial(domain.payload)?);
            }
            world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY => {
                world_object_overlay = Some(world_object::decode_world_object_overlay_snapshot(
                    domain.payload,
                )?);
            }
            _ => {}
        }
    }

    Ok(JoinSnapshot {
        game_session_id: accepted_fields.game_session_id,
        world_spatial: world_spatial_observation.ok_or(DevClientError::MissingDomain(
            world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        ))?,
        world_object_overlay: world_object_overlay.ok_or(DevClientError::MissingDomain(
            world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
        ))?,
    })
}

async fn expect_message(
    stream: &mut TlsStream<TcpStream>,
    expected: MessageType,
) -> Result<(), DevClientError> {
    let frame = read_frame(stream).await?;
    let envelope = decode_wire_envelope(&frame)?;
    if envelope.message_type() != expected {
        return Err(DevClientError::UnexpectedMessage {
            expected,
            actual: envelope.message_type(),
        });
    }
    Ok(())
}

/// Generic over the stream type so the test module's fake *server* (`tokio_rustls::server::
/// TlsStream`) can frame with the exact same code the real client (`tokio_rustls::client::
/// TlsStream`, via `connect_and_join`) uses.
async fn write_frame<S: tokio::io::AsyncWrite + Unpin>(
    stream: &mut S,
    body: &[u8],
) -> Result<(), DevClientError> {
    let length = FrameLength::new(
        u32::try_from(body.len()).map_err(|_error| FoundationProtocolError::FrameTooLarge)?,
    )?;
    stream.write_all(&length.to_prefix()).await?;
    stream.write_all(body).await?;
    stream.flush().await?;
    Ok(())
}

async fn read_frame<S: tokio::io::AsyncRead + Unpin>(
    stream: &mut S,
) -> Result<Vec<u8>, DevClientError> {
    let mut prefix = [0_u8; 4];
    stream.read_exact(&mut prefix).await?;
    let length = FrameLength::from_prefix(&prefix)?;
    let mut body = vec![0_u8; length.get() as usize];
    stream.read_exact(&mut body).await?;
    Ok(body)
}

fn tls_connector(root: &CertificateDer<'static>) -> Result<TlsConnector, DevClientError> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(root.clone()).map_err(DevClientError::Tls)?;
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .map_err(DevClientError::Tls)?
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = vec![ALPN_OTERYN_GAME_V1.as_bytes().to_vec()];
    Ok(TlsConnector::from(Arc::new(config)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_protocol_oteryn::world_object::{
        SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1, STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
        WorldObjectOverlayEntry as WireOverlayEntry, encode_world_object_overlay_snapshot,
    };
    use oteryn_protocol_oteryn::world_spatial::{
        ActorPosition, SNAPSHOT_TYPE_WORLD_SPATIAL_V1, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
        WorldSpatialObservation as WireSpatialObservation, encode_world_spatial,
    };
    use oteryn_protocol_oteryn::{
        ChannelId, DomainSnapshot, ServerAcceptedValue, WorldId, encode_server_accepted,
        encode_single_chunk_snapshot,
    };
    use rustls::pki_types::PrivatePkcs8KeyDer;
    use tokio::net::TcpListener;

    const DOOR_PLACEMENT: &str = "oteryn:cell/entry-door";
    const DOOR_STATE: &str = "oteryn:reference.state.closed";
    const CLIENT_BUILD_ID: &str = "oteryn-dev-client-test";

    fn test_uuid_v7(marker: u8) -> [u8; 16] {
        [
            0x01, 0x93, 0x4f, 0x10, 0x7c, 0x00, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33,
            0x44, marker,
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
        Ok(())
    }

    #[test]
    fn connect_and_join_reports_refusal_when_server_closes_before_admission()
    -> Result<(), Box<dyn StdError + Send + Sync>> {
        block_on(run_server_closes_before_admission_case())?
    }

    async fn run_server_closes_before_admission_case() -> Result<(), Box<dyn StdError + Send + Sync>>
    {
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
        })
        .await;
        assert!(result.is_err());
        Ok(())
    }
}
