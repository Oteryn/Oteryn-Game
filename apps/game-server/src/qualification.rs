//! Server Seam physical qualification. Runs only inside the disposable WP5
//! topology (`tools/qualification/wp5_s3b`, seam target): the exact protected
//! Platform producer behind TLS 1.3 mTLS plus PostgreSQL 18.6. The owners are
//! composed exactly as in S3-B, then every client case traverses the shipped
//! `serve_gameplay` entry over real loopback TCP + TLS 1.3 + FND-02 framing.
//! The fresh grant is signed with an ephemeral key whose public half the
//! topology published into the Platform trust registry.
//!
//! This is Server Seam integration evidence, not ADR-0007 QA Tier 1/Tier 2.

use crate::admission_evidence::{self, Facts, Request, Response, decode_response, encode_request};
use crate::character_bootstrap_intent::read_authenticated_intent;
use crate::character_recovery_fence::CharacterRecoveryStore;
use crate::durability::DurabilityRoot;
use crate::durability::admission_authority_guards::GuardPublicationDisposition;
use crate::durability::native_admission_source::{
    DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
    SourceObservation,
};
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, BootstrapSecret, ControlActor,
    LaunchBinding, NodeIncarnationProof, OperationKey, RuntimeScopeAssignmentWriter,
};
use crate::foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    AdmissionAuthorityOwningPublisherV1, AdmissionAuthorityPublicationChangeV1,
    AdmissionAuthorityPublicationErrorV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
    AdmissionPublicationSourceV1,
};
use crate::foundation::{
    ChannelId, CharacterId, CommandStatus, DomainSnapshot, FoundationProtocolError, MessageType,
    NodeId, RuntimeScopeRefV1, WorldId, decode_wire_envelope, encode_command_protocol_error,
    encode_command_result, encode_protocol_error, encode_single_chunk_snapshot, encode_state_delta,
};
use crate::gameplay_transport::world_spatial::{
    ActorPosition, COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, DELTA_TYPE_WORLD_SPATIAL_V1,
    SNAPSHOT_TYPE_WORLD_SPATIAL_V1, STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY, StepDirection,
    StepDisposition, WorldSpatialObservation, encode_step_intent, encode_step_result,
    encode_world_spatial,
};
use crate::native_admission_source::{self, TransientCapacity, descriptor::ProducerDescriptor};
use crate::{GameplayListenerConfig, GameplaySeamOwners, serve_gameplay};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use ed25519_dalek::{Signer, SigningKey};
use oteryn_foundation::CancellationToken;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use sqlx::{Connection, Executor};
use std::future::{Future, poll_fn};
use std::net::SocketAddr;
use std::pin::pin;
use std::sync::Arc;
use std::task::Poll;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use std::{env, fs, io};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{TlsConnector, rustls};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[path = "qualification_spell_character.rs"]
mod spell_character;

#[path = "qualification_wild_spawn.rs"]
mod wild_spawn;

#[cfg(test)]
#[path = "spell_book_sweep_tests.rs"]
mod spell_book_sweep_tests;

const SOURCE_AUTHORITY: &str = "platform";
const PLATFORM_SOURCE: &str = "5d4883acf7079e26fd51e03f460166730de1ada0";
/// Interpretation requested by the Platform intents that `run.sh` issues.
const INTERPRETATION: [&str; 4] = [
    "s3b-profile-1",
    "s3b-ruleset-1",
    "s3b-content-1",
    "s3b-starter-1",
];
const RUNTIME_AUTHORITY: &str = "game:wp5-seam-runtime";
/// First readiness publication of the served Channel.
const RUNTIME_OBSERVATION: &str = "runtime-1";

fn required(name: &str) -> Result<String, io::Error> {
    env::var(name).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, name))
}

fn evidence(line: &str) {
    println!("SEAM_EVIDENCE {line}");
}

fn pem_der(path: &str, label: &str) -> TestResult<Vec<u8>> {
    let text = fs::read_to_string(path)?;
    let begin = format!("-----BEGIN {label}-----");
    let end = format!("-----END {label}-----");
    let body = text
        .split_once(&begin)
        .and_then(|(_, tail)| tail.split_once(&end).map(|(body, _)| body))
        .ok_or("invalid PEM boundary")?;
    Ok(STANDARD.decode(body.lines().map(str::trim).collect::<String>())?)
}

fn producer_descriptor(authority: &str, cert: &str, key: &str) -> TestResult<ProducerDescriptor> {
    let port = required("WP5_S3A_PORT")?.parse::<u16>()?;
    let ca = pem_der(&required("WP5_S3A_CA_CERT")?, "CERTIFICATE")?;
    let client = pem_der(&required(cert)?, "CERTIFICATE")?;
    let key = pem_der(&required(key)?, "PRIVATE KEY")?;
    Ok(ProducerDescriptor::new(
        authority.into(),
        ("127.0.0.1".into(), port),
        "source.test".into(),
        "source.test".into(),
        vec![CertificateDer::from(ca)],
        vec![CertificateDer::from(client)],
        PrivateKeyDer::try_from(key)?,
    )?)
}

fn hex32(text: &str) -> TestResult<[u8; 32]> {
    let text = text.trim();
    if text.len() != 64 {
        return Err("expected 32 hex bytes".into());
    }
    let mut out = [0_u8; 32];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16)?;
    }
    Ok(out)
}

fn v7(tag: u8, lane: u8) -> [u8; 16] {
    [
        0x01, 0x93, 0x4f, 0x10, 0x7c, lane, 0x70, tag, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        tag,
    ]
}

fn now_seconds() -> TestResult<i64> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    )?)
}

async fn fetch_real(
    descriptor: &ProducerDescriptor,
    request: Request<'_>,
) -> TestResult<(admission_evidence::Observation, Vec<u8>)> {
    use native_admission_source::descriptor::Operation;
    let capacity = TransientCapacity::new();
    let mut permit = capacity.try_queue()?;
    permit.try_activate()?;
    let operation = match request {
        Request::Account {
            recovery: false, ..
        } => Operation::ReadAccountSecurityV1,
        Request::Trust {
            recovery: false, ..
        } => Operation::ReadFreshSigningTrustV1,
        _ => return Err("recovery operations are outside fresh admission".into()),
    };
    let encoded = encode_request(&request).map_err(|_| "request encoding")?;
    let raw =
        native_admission_source::http1_mtls::exchange(descriptor, operation, &encoded, &mut permit)
            .await
            .map_err(|error| format!("S1 exchange: {error:?}"))?;
    let Ok(Response::Observed(observation)) = decode_response(&request, SOURCE_AUTHORITY, &raw)
    else {
        return Err("Platform did not return an authoritative observation".into());
    };
    Ok((observation, raw))
}

/// The holder's S2 refresh: exact Platform observations accepted under custody.
async fn ingest(
    root: &DurabilityRoot,
    custody: &NodeIncarnationProof,
    descriptor: &ProducerDescriptor,
    account_id: &str,
    key_id: &str,
) -> TestResult<u64> {
    let (account, account_raw) = fetch_real(
        descriptor,
        Request::Account {
            recovery: false,
            account_id,
            purpose: "platform_security",
            scope: "fresh_admission",
        },
    )
    .await?;
    let Facts::Account {
        allowed: true,
        minimum_valid_generation,
    } = account.facts
    else {
        return Err("Platform account is not allowed".into());
    };
    let (trust, trust_raw) = fetch_real(
        descriptor,
        Request::Trust {
            recovery: false,
            key_id,
            key_purpose: "fresh_admission",
        },
    )
    .await?;
    for (operation, subject, observation, raw) in [
        (
            NativeSourceOperation::ReadAccountSecurityV1,
            NativeSourceSubject::account_security(account_id)?,
            account,
            account_raw,
        ),
        (
            NativeSourceOperation::ReadFreshSigningTrustV1,
            NativeSourceSubject::signing_trust(
                "urn:oteryn:platform:game-admission",
                "oteryn-pre-admission-v1",
                "fresh_admission",
                key_id,
            )?,
            trust,
            trust_raw,
        ),
    ] {
        root.accept_native_source_observation(
            custody,
            SourceObservation {
                source_authority: SOURCE_AUTHORITY.into(),
                operation,
                subject,
                source_revision: observation.source_revision,
                decision_identity: observation.decision_identity.as_str().into(),
                observed_at: observation.source_observed_at,
                semantic_facts: raw,
            },
        )
        .await?;
    }
    Ok(minimum_valid_generation)
}

/// The account's current security generation, read from Platform without
/// accepting anything into S2: grants are built from it while S2 stays fed
/// only by the admission's own on-demand fetch (OPS-NODE-BOOT-01 D4).
async fn platform_generation(descriptor: &ProducerDescriptor, account_id: &str) -> TestResult<u64> {
    let (account, _) = fetch_real(
        descriptor,
        Request::Account {
            recovery: false,
            account_id,
            purpose: "platform_security",
            scope: "fresh_admission",
        },
    )
    .await?;
    let Facts::Account {
        allowed: true,
        minimum_valid_generation,
    } = account.facts
    else {
        return Err("Platform account is not allowed".into());
    };
    Ok(minimum_valid_generation)
}

struct RuntimeReadiness(AdmissionAuthorityPublicationChangeV1);
impl crate::foundation::fnd04_verifier::fresh_source_sealed::Sealed for RuntimeReadiness {}
impl AdmissionAuthorityOwningPublisherV1 for RuntimeReadiness {
    fn resolve_publication(
        &self,
        _now: i64,
    ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
    {
        Ok(vec![self.0.clone()])
    }
}

/// First readiness publication for the served Channel.
async fn publish_readiness(
    root: &DurabilityRoot,
    holder: &NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    generation: u64,
    now: i64,
) -> TestResult {
    let change = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Runtime(scope),
        source: AdmissionPublicationSourceV1 {
            authority: RUNTIME_AUTHORITY.into(),
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision: 1,
            decision_identity: "runtime-ready:1".into(),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
        },
        precondition: AdmissionPublicationPreconditionV1::Bootstrap {
            restored_publication_high_water: Some(0),
        },
        publication_revision: 1,
        state: AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation: generation,
            ready: true,
            route_revision: "route-s3b-1".into(),
            runtime_observation_revision: RUNTIME_OBSERVATION.into(),
            protocol_major: 1,
            transport_profile: 1,
            ruleset_revision: "rules-s3b-1".into(),
            content_revision: "content-s3b-1".into(),
            map_revision: "map-s3b-1".into(),
            world_policy_revision: "policy-s3b-1".into(),
            offer_revision: "offer-s3b-1".into(),
        },
    };
    let publication = AdmissionAuthorityPublicationV1::prepare(&RuntimeReadiness(change), now)
        .map_err(|error| format!("{error:?}"))?;
    let disposition = root
        .publish_runtime_readiness(holder, &publication)
        .await
        .map_err(|error| format!("readiness: {error:?}"))?;
    if disposition != GuardPublicationDisposition::Applied {
        return Err(format!("readiness not applied: {disposition:?}").into());
    }
    Ok(())
}

/// Grant claims the Platform issuer would sign.
struct Grant<'a> {
    signing: &'a SigningKey,
    key_id: &'a str,
    account_id: &'a str,
    character_id: [u8; 16],
    world_id: [u8; 16],
    channel_id: [u8; 16],
    security_generation: u64,
    scope_generation: u64,
    nonce: [u8; 32],
    attempt: [u8; 16],
}

fn uuid_text(bytes: &[u8; 16]) -> String {
    crate::node::uuid_text(bytes)
}

fn sign_grant(grant: &Grant<'_>, issued_at: i64) -> String {
    let header = format!(
        r#"{{"alg":"Ed25519","kid":"{}","typ":"oteryn-admission+jwt"}}"#,
        grant.key_id
    );
    let payload = serde_json::json!({
        "iss": "urn:oteryn:platform:game-admission",
        "aud": "urn:oteryn:game:admission",
        "iat": issued_at,
        "nbf": issued_at,
        "exp": issued_at + 10,
        "jti": URL_SAFE_NO_PAD.encode(grant.nonce),
        "profile": "oteryn-pre-admission-v1",
        "purpose": "fresh_entry",
        "attempt_ref": uuid_text(&grant.attempt),
        "account_id": grant.account_id,
        "character_id": uuid_text(&grant.character_id),
        "world_id": uuid_text(&grant.world_id),
        "channel_id": uuid_text(&grant.channel_id),
        "account_security_generation": grant.security_generation.to_string(),
        "route_revision": "route-s3b-1",
        "runtime_observation_revision": RUNTIME_OBSERVATION,
        "scope_ownership_generation": grant.scope_generation.to_string(),
        "protocol_major": 1,
        "transport_profile": 1,
        "ruleset_revision": "rules-s3b-1",
        "content_revision": "content-s3b-1",
        "map_revision": "map-s3b-1",
        "world_policy_revision": "policy-s3b-1",
        "offer_revision": "offer-s3b-1",
    });
    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(payload.to_string())
    );
    let signature = URL_SAFE_NO_PAD.encode(grant.signing.sign(signing_input.as_bytes()).to_bytes());
    format!("{signing_input}.{signature}")
}

async fn database() -> TestResult<(String, String)> {
    let admin_url = required("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.ends_with("@127.0.0.1:5432/postgres")
    {
        return Err("unsafe PostgreSQL test admin URL".into());
    }
    let name = format!(
        "seam_{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let mut admin = sqlx::PgConnection::connect(&admin_url).await?;
    admin
        .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
            "CREATE DATABASE {name}"
        ))))
        .await?;
    admin.close().await?;
    let url = format!(
        "{}/{name}",
        admin_url
            .strip_suffix("/postgres")
            .ok_or("invalid admin URL")?
    );
    let mut connection = sqlx::PgConnection::connect(&url).await?;
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&mut connection)
        .await?;
    if version != oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM {
        return Err(
            format!("the seam qualification requires PostgreSQL 18.6, found {version}").into(),
        );
    }
    sqlx::migrate!("./migrations").run(&mut connection).await?;
    connection.close().await?;
    Ok((url, version))
}

async fn committed_admissions(url: &str) -> TestResult<i64> {
    let mut connection = sqlx::PgConnection::connect(url).await?;
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_durability_fresh_admission_receipts")
            .fetch_one(&mut connection)
            .await?;
    connection.close().await?;
    Ok(count)
}

/// One LOGIN member of a migration group role (OPS-NODE-BOOT-01 D2): the
/// serving node holds only the runtime credential, operator actions only the
/// control-plane credential.
async fn group_login(url: &str, group: &str) -> TestResult<(String, String)> {
    let role = format!(
        "{group}_{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
    );
    let password = format!("{role}-disposable");
    let mut owner = sqlx::PgConnection::connect(url).await?;
    owner
        .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
            "CREATE ROLE {role} LOGIN PASSWORD '{password}' IN ROLE {group}"
        ))))
        .await?;
    // Deployment grants CONNECT to this database's logins only (migration 0006).
    let database = url.rsplit_once('/').ok_or("database URL has no name")?.1;
    owner
        .execute(sqlx::query(sqlx::AssertSqlSafe(format!(
            "GRANT CONNECT ON DATABASE {database} TO {role}"
        ))))
        .await?;
    owner.close().await?;
    let (_, address) = url.split_once('@').ok_or("database URL has no authority")?;
    let login = format!("postgresql://{role}:{password}@{address}");
    Ok((role, login))
}

async fn register(
    control: &DurabilityRoot,
    root: &DurabilityRoot,
    tag: u8,
) -> TestResult<NodeIncarnationProof> {
    let secret = BootstrapSecret::from_bytes([tag; 32]);
    let launch = LaunchBinding::new(&format!("seam-launch-{tag}")).map_err(|e| format!("{e:?}"))?;
    control
        .issue_node_bootstrap_authorization(&secret, &launch, None)
        .await
        .map_err(|e| format!("bootstrap authorization {tag}: {e:?}"))?;
    Ok(root
        .register_node_incarnation(&secret, &launch, NodeId::decode(&v7(tag, 0x01))?)
        .await
        .map_err(|e| format!("register {tag}: {e:?}"))?)
}

/// Client TLS profile for one case.
struct ClientProfile {
    tls12: bool,
    alpn: Option<&'static [u8]>,
}

const EXACT: ClientProfile = ClientProfile {
    tls12: false,
    alpn: Some(b"oteryn-game/1"),
};

fn connector(
    root_cert: &CertificateDer<'static>,
    profile: &ClientProfile,
) -> TestResult<TlsConnector> {
    let mut roots = rustls::RootCertStore::empty();
    roots.add(root_cert.clone())?;
    let version = if profile.tls12 {
        &rustls::version::TLS12
    } else {
        &rustls::version::TLS13
    };
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::aws_lc_rs::default_provider(),
    ))
    .with_protocol_versions(&[version])?
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = profile.alpn.map(<[u8]>::to_vec).into_iter().collect();
    Ok(TlsConnector::from(Arc::new(config)))
}

fn varint(output: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        output.push((value as u8 & 0x7f) | 0x80);
        value >>= 7;
    }
    output.push(value as u8);
}

fn scalar(output: &mut Vec<u8>, field: u64, value: u64) {
    varint(output, field << 3);
    varint(output, value);
}

fn bytes_field(output: &mut Vec<u8>, field: u64, value: &[u8]) {
    varint(output, (field << 3) | 2);
    varint(output, value.len() as u64);
    output.extend_from_slice(value);
}

fn envelope(message_type: u64, generation: u64, payload: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    scalar(&mut output, 1, message_type);
    if generation != 0 {
        scalar(&mut output, 2, generation);
    }
    bytes_field(&mut output, 4, payload);
    output
}

/// ADMIT-CAP-1: the entry-room seam Channel requires capability 21 `LOGOUT_V1` (offered by both
/// the in-process seam and the node-boot node, requires nothing, gates only command type 23,
/// which no seam stage sends). Every seam bootstrap and resume declares support for it, so each
/// admission and resume passes the owner's required-set check; one bootstrap and one resume
/// omit it and must be refused (ADMIT0 §4).
const SEAM_REQUIRED_CAPABILITIES: &[u32] = &[oteryn_protocol_oteryn::logout::CAPABILITY_LOGOUT_V1];

fn bootstrap(major: u64, profile: u64, character: &[u8; 16], material: &str) -> Vec<u8> {
    bootstrap_supporting(
        major,
        profile,
        character,
        material,
        SEAM_REQUIRED_CAPABILITIES,
    )
}

fn bootstrap_supporting(
    major: u64,
    profile: u64,
    character: &[u8; 16],
    material: &str,
    supported: &[u32],
) -> Vec<u8> {
    let mut payload = Vec::new();
    scalar(&mut payload, 1, major);
    scalar(&mut payload, 2, profile);
    scalar(&mut payload, 3, 1);
    for capability in supported {
        scalar(&mut payload, 4, u64::from(*capability));
    }
    bytes_field(&mut payload, 5, material.as_bytes());
    bytes_field(&mut payload, 6, character);
    bytes_field(&mut payload, 7, b"seam-qualification");
    envelope(1, 0, &payload)
}

/// Structurally valid FND-02 resume request. The qualification supplies only
/// unissued reconnect material; this transport must not mint resume authority.
fn resume_payload(session: &[u8; 16], material: &[u8]) -> Vec<u8> {
    resume_payload_supporting(session, material, SEAM_REQUIRED_CAPABILITIES)
}

fn resume_payload_supporting(session: &[u8; 16], material: &[u8], supported: &[u32]) -> Vec<u8> {
    let mut payload = Vec::new();
    bytes_field(&mut payload, 1, session);
    bytes_field(&mut payload, 2, material);
    scalar(&mut payload, 4, 1);
    scalar(&mut payload, 5, 1);
    scalar(&mut payload, 6, 1);
    for capability in supported {
        scalar(&mut payload, 7, u64::from(*capability));
    }
    bytes_field(&mut payload, 8, b"seam-qualification");
    payload
}

fn resume(session: &[u8; 16], material: &[u8]) -> Vec<u8> {
    envelope(3, 0, &resume_payload(session, material))
}

fn framed(body: &[u8]) -> Vec<u8> {
    let mut output = u32::try_from(body.len())
        .unwrap_or(u32::MAX)
        .to_be_bytes()
        .to_vec();
    output.extend_from_slice(body);
    output
}

/// What the server returned to one client case.
#[derive(Debug, PartialEq, Eq)]
enum Reply {
    /// TLS negotiation failed; nothing reached the frame layer.
    TlsRefused,
    /// Closed without any frame.
    Closed,
    /// One or more frames, then close.
    Frames(Vec<Vec<u8>>),
}

async fn exchange(address: SocketAddr, connector: &TlsConnector, raw: &[u8]) -> TestResult<Reply> {
    let tcp = TcpStream::connect(address).await?;
    let connect = connector.connect(ServerName::try_from("localhost")?, tcp);
    let Ok(mut stream) = tokio::time::timeout(Duration::from_secs(30), connect)
        .await
        .map_err(|_| "TLS connect did not complete")?
    else {
        return Ok(Reply::TlsRefused);
    };
    let _ = stream.write_all(raw).await;
    let _ = stream.flush().await;
    let mut output = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(20), stream.read_to_end(&mut output)).await;
    Ok(frames(&output))
}

/// For continuity cases, require the serving node to actually end the
/// connection within the bounded deadline. `exchange` deliberately tolerates
/// a still-open admitted socket for other cases.
async fn exchange_must_close(
    address: SocketAddr,
    connector: &TlsConnector,
    raw: &[u8],
) -> TestResult<Reply> {
    exchange_must_close_paced(address, connector, &[(0, raw.to_vec())]).await
}

/// SPEED-1: more than a player step duration (550 ms at level 1 on default ground), so a step
/// sent this long after the previous step's result is never early.
const STEP_PACE: Duration = Duration::from_millis(700);

/// The complete frames at the start of `output`.
fn complete_frames(output: &[u8]) -> usize {
    let (mut count, mut cursor) = (0, 0);
    while let Some(length) = output
        .get(cursor..cursor + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .map(|bytes| u32::from_be_bytes(bytes) as usize)
    {
        if output.len() < cursor + 4 + length {
            break;
        }
        count += 1;
        cursor += 4 + length;
    }
    count
}

/// `exchange_must_close` for a paced client (SPEED-1, CONDITIONS-0 §4.3): each `(after,
/// segment)` is written once `after` complete server frames have arrived and [`STEP_PACE`] has
/// passed, so no step waits in the server's buffer or is refused as early. A connection that
/// ends sooner gets no further segment.
async fn exchange_must_close_paced(
    address: SocketAddr,
    connector: &TlsConnector,
    segments: &[(usize, Vec<u8>)],
) -> TestResult<Reply> {
    let tcp = TcpStream::connect(address).await?;
    let mut stream = tokio::time::timeout(
        Duration::from_secs(30),
        connector.connect(ServerName::try_from("localhost")?, tcp),
    )
    .await
    .map_err(|_| "TLS connect did not complete")??;
    let mut output = Vec::new();
    for (after, segment) in segments {
        if *after > 0 {
            let mut chunk = [0_u8; 4096];
            while complete_frames(&output) < *after {
                let read = tokio::time::timeout(Duration::from_secs(20), stream.read(&mut chunk))
                    .await
                    .map_err(|_| "server did not answer a paced segment")??;
                if read == 0 {
                    break;
                }
                output.extend_from_slice(&chunk[..read]);
            }
            if complete_frames(&output) < *after {
                break;
            }
            tokio::time::sleep(STEP_PACE).await;
        }
        stream.write_all(segment).await?;
        stream.flush().await?;
    }
    match tokio::time::timeout(Duration::from_secs(20), stream.read_to_end(&mut output)).await {
        Ok(Ok(_)) => {}
        Ok(Err(error))
            if matches!(
                error.kind(),
                io::ErrorKind::UnexpectedEof
                    | io::ErrorKind::ConnectionReset
                    | io::ErrorKind::BrokenPipe
            ) => {}
        Ok(Err(error)) => return Err(error.into()),
        Err(_) => return Err("server did not close the connection".into()),
    }
    let reply = frames(&output);
    let complete_bytes = match &reply {
        Reply::Closed => 0,
        Reply::Frames(frames) => frames.iter().map(|frame| frame.len() + 4).sum(),
        Reply::TlsRefused => return Err("TLS refusal after a completed handshake".into()),
    };
    if complete_bytes != output.len() {
        return Err("partial or trailing server frame".into());
    }
    Ok(reply)
}

fn frames(output: &[u8]) -> Reply {
    let mut frames = Vec::new();
    let mut cursor = 0;
    while cursor + 4 <= output.len() {
        let length = u32::from_be_bytes([
            output[cursor],
            output[cursor + 1],
            output[cursor + 2],
            output[cursor + 3],
        ]) as usize;
        let Some(body) = output.get(cursor + 4..cursor + 4 + length) else {
            break;
        };
        frames.push(body.to_vec());
        cursor += 4 + length;
    }
    if frames.is_empty() {
        Reply::Closed
    } else {
        Reply::Frames(frames)
    }
}

fn client_command(id: u64, command_type: u64, payload: &[u8]) -> Vec<u8> {
    client_command_at(1, id, command_type, payload)
}

fn client_command_at(generation: u64, id: u64, command_type: u64, payload: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    scalar(&mut body, 1, id);
    scalar(&mut body, 2, command_type);
    bytes_field(&mut body, 4, payload);
    envelope(7, generation, &body)
}

/// The exact frames after `ServerAccepted` for the first-control scenario, from the committed
/// room of `world`: start (0,0,0), east (1,0,0) walkable, north blocked, south absent. The join
/// snapshot also carries the door's current (closed, revision 0) `WORLD_OBJECT_OVERLAY`
/// (USE-WIRE-V1, #162 5868482467). No Character has cast facts yet (spell cast §4, SPELL-D4), so
/// the snapshot carries no `ACTOR_VITALS` and a spell cast intent is `REJECTED` with no effect.
fn first_control_frames(world: WorldId) -> TestResult<Vec<Vec<u8>>> {
    let room = crate::content::qualify_native_entry_room(world)
        .map_err(|e| format!("native entry room: {e}"))?;
    let at = |x| {
        encode_world_spatial(&WorldSpatialObservation {
            content_generation: room.compiled().client_digest(),
            actor_position: ActorPosition { x, y: 0, floor: 0 },
        })
    };
    let result = |sequence, id, status, disposition| {
        encode_command_result(1, sequence, id, status, &encode_step_result(disposition))
    };
    let delta = |sequence, from, x| {
        encode_state_delta(
            1,
            sequence,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            from,
            from + 1,
            DELTA_TYPE_WORLD_SPATIAL_V1,
            &at(x),
        )
    };
    let door_overlay = oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry {
        content_generation: room.compiled().client_digest(),
        placement: crate::content::accepted::DOOR_CELL.0.as_bytes().to_vec(),
        state: crate::content::accepted::DOOR_CLOSED_STATE
            .as_bytes()
            .to_vec(),
        revision: 0,
    };
    let vitals_payload = vocationless_vitals_payload()?;
    let mut frames: Vec<Vec<u8>> = encode_single_chunk_snapshot(
        1,
        1,
        0,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &at(0),
            },
            DomainSnapshot {
                domain_id: oteryn_protocol_oteryn::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 0,
                snapshot_type:
                    oteryn_protocol_oteryn::world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload:
                    &oteryn_protocol_oteryn::world_object::encode_world_object_overlay_snapshot(&[
                        door_overlay,
                    ])
                    .map_err(|_| "encode door overlay snapshot")?,
            },
            DomainSnapshot {
                domain_id: oteryn_protocol_oteryn::actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                revision: 1,
                snapshot_type: oteryn_protocol_oteryn::actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                payload: &vitals_payload,
            },
        ],
    )?
    .into();
    frames.extend([
        result(1, 1, CommandStatus::Accepted, StepDisposition::Moved)?,
        delta(2, 1, 1)?,
        result(3, 2, CommandStatus::Accepted, StepDisposition::Moved)?,
        delta(4, 2, 0)?,
        result(5, 3, CommandStatus::Accepted, StepDisposition::Blocked)?,
        result(6, 4, CommandStatus::Accepted, StepDisposition::Blocked)?,
        // A spell cast of an actor without Character cast facts: gated, REJECTED.
        encode_command_result(
            1,
            7,
            5,
            CommandStatus::Rejected,
            &crate::gameplay_transport::actor_spell::encode_spell_cast_result(
                crate::gameplay_transport::actor_spell::SpellCastDisposition::Rejected,
            ),
        )?,
        // An unregistered command type: REJECTED with no type-owned payload.
        encode_command_result(1, 8, 6, CommandStatus::Rejected, &[])?,
        // Command 8 after 6: a gap naming the offending and the expected ID.
        encode_command_protocol_error(FoundationProtocolError::CommandSequenceGap, 1, 8, 7)?,
    ]);
    Ok(frames)
}

/// TUTORIAL-1: a vocationless level-1 Character joins with full vitals at revision 1, so every
/// join snapshot of the seam carries this `ACTOR_VITALS` domain after the door overlay.
fn vocationless_vitals_payload() -> TestResult<Vec<u8>> {
    use oteryn_protocol_oteryn::actor_spell::{ActorVitals, encode_actor_vitals};
    Ok(encode_actor_vitals(&ActorVitals {
        health: 150,
        max_health: 150,
        mana: 55,
        max_mana: 55,
        soul: 100,
        harmony: 0,
        serene: false,
    })
    .map_err(|_| "encode vocationless vitals snapshot")?)
}

/// The exact frames after `ServerAccepted` for the USE-WIRE-V1 scenario (#162 5868482467,
/// M2b): the door starts closed at revision 0 (join snapshot). Adjacent USE opens it
/// (COMMITTED, revision 1); stepping north now moves through the open doorway; USE while
/// standing in the doorway is OCCUPIED (no mutation); stepping south vacates it; USE now
/// COMMITs the close (revision 2); stepping north again is Blocked by the closed door; a
/// stale-revision USE is STALE_STATE; an unknown placement is NOTHING_TO_USE; and replaying the
/// already-consumed CommandId 6 expires and closes the connection — the same FND-02 CommandId
/// discipline a replayed STEP CommandId gets, so it can never make a second transition. Every
/// accepted cell in the qualified room is within Chebyshev distance 1 of the door (#935/#937),
/// so a genuine TOO_FAR case cannot be exercised through real movement here; it is covered by
/// `gameplay_transport::tests::use_object_reachable_is_chebyshev_one_same_floor_only` instead.
fn use_wire_frames(world: WorldId) -> TestResult<Vec<Vec<u8>>> {
    use oteryn_protocol_oteryn::world_object::{
        DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1, SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
        STATE_DOMAIN_WORLD_OBJECT_OVERLAY, UseDisposition, WorldObjectOverlayEntry,
        encode_use_result, encode_world_object_overlay_delta, encode_world_object_overlay_snapshot,
    };
    let room = crate::content::qualify_native_entry_room(world)
        .map_err(|e| format!("native entry room: {e}"))?;
    let content_generation = room.compiled().client_digest();
    let door_key = crate::content::accepted::DOOR_CELL.0.as_bytes();
    let spatial_at = |x, y| {
        encode_world_spatial(&WorldSpatialObservation {
            content_generation,
            actor_position: ActorPosition { x, y, floor: 0 },
        })
    };
    let overlay_entry = |state: &str, revision: u64| WorldObjectOverlayEntry {
        content_generation,
        placement: door_key.to_vec(),
        state: state.as_bytes().to_vec(),
        revision,
    };
    let step_result = |sequence, id, disposition| {
        encode_command_result(
            1,
            sequence,
            id,
            CommandStatus::Accepted,
            &encode_step_result(disposition),
        )
    };
    let spatial_delta = |sequence, from, x, y| {
        encode_state_delta(
            1,
            sequence,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            from,
            from + 1,
            DELTA_TYPE_WORLD_SPATIAL_V1,
            &spatial_at(x, y),
        )
    };
    let use_result = |sequence, id, disposition| {
        encode_command_result(
            1,
            sequence,
            id,
            CommandStatus::Accepted,
            &encode_use_result(disposition).unwrap_or_default(),
        )
    };
    let overlay_delta = |sequence, from, state: &str, to| -> TestResult<Vec<u8>> {
        let payload = encode_world_object_overlay_delta(&overlay_entry(state, to))
            .map_err(|_| "encode overlay delta")?;
        Ok(encode_state_delta(
            1,
            sequence,
            STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
            from,
            to,
            DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
            &payload,
        )?)
    };
    let vitals_payload = vocationless_vitals_payload()?;
    let mut frames: Vec<Vec<u8>> = encode_single_chunk_snapshot(
        1,
        1,
        0,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision: 1,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &spatial_at(0, 0),
            },
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 0,
                snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload: &encode_world_object_overlay_snapshot(&[overlay_entry(
                    crate::content::accepted::DOOR_CLOSED_STATE,
                    0,
                )])
                .map_err(|_| "encode overlay snapshot")?,
            },
            DomainSnapshot {
                domain_id: oteryn_protocol_oteryn::actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                revision: 1,
                snapshot_type: oteryn_protocol_oteryn::actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                payload: &vitals_payload,
            },
        ],
    )?
    .into();
    frames.extend([
        step_result(1, 1, StepDisposition::Moved)?, // cmd1: east (0,0)->(1,0)
        spatial_delta(2, 1, 1, 0)?,
        use_result(3, 2, UseDisposition::Committed)?, // cmd2: use open, adjacent
        overlay_delta(4, 0, crate::content::accepted::DOOR_OPEN_STATE, 1)?,
        step_result(5, 3, StepDisposition::Moved)?, // cmd3: north (1,0)->(1,-1), door open
        spatial_delta(6, 2, 1, -1)?,
        use_result(7, 4, UseDisposition::Occupied)?, // cmd4: use close, standing in the doorway
        step_result(8, 5, StepDisposition::Moved)?,  // cmd5: south (1,-1)->(1,0), vacates
        spatial_delta(9, 3, 1, 0)?,
        use_result(10, 6, UseDisposition::Committed)?, // cmd6: use close, now unoccupied
        overlay_delta(11, 1, crate::content::accepted::DOOR_CLOSED_STATE, 2)?,
        step_result(12, 7, StepDisposition::Blocked)?, // cmd7: north again, door now closed
        use_result(13, 8, UseDisposition::StaleState)?, // cmd8: use open, expected_revision=0 (stale)
        use_result(14, 9, UseDisposition::NothingToUse)?, // cmd9: use, unknown placement
        // cmd6 replayed: already consumed, expires and closes (same FND-02 discipline as STEP).
        encode_command_protocol_error(FoundationProtocolError::CommandOutcomeExpired, 1, 6, 0)?,
    ]);
    Ok(frames)
}

/// #162 C1b: moved onto `oteryn_protocol_oteryn::decode_server_accepted` (added for the dev
/// client, C1b) rather than this file continuing to hand-parse `ServerAccepted`'s fixed field-1
/// offset itself.
fn accepted_session(reply: &Reply) -> Option<[u8; 16]> {
    let Reply::Frames(frames) = reply else {
        return None;
    };
    let envelope = decode_wire_envelope(frames.first()?).ok()?;
    if envelope.message_type() != MessageType::ServerAccepted {
        return None;
    }
    Some(
        *oteryn_protocol_oteryn::decode_server_accepted(envelope.payload())
            .ok()?
            .game_session_id
            .as_bytes(),
    )
}

async fn join<A, B>(a: impl Future<Output = A>, b: impl Future<Output = B>) -> (A, B) {
    let mut a = pin!(a);
    let mut b = pin!(b);
    let (mut left, mut right) = (None, None);
    poll_fn(|context| {
        if left.is_none()
            && let Poll::Ready(value) = a.as_mut().poll(context)
        {
            left = Some(value);
        }
        if right.is_none()
            && let Poll::Ready(value) = b.as_mut().poll(context)
        {
            right = Some(value);
        }
        match (left.take(), right.take()) {
            (Some(l), Some(r)) => Poll::Ready((l, r)),
            (l, r) => {
                left = l;
                right = r;
                Poll::Pending
            }
        }
    })
    .await
}

/// OPS-NODE-BOOT-01 §4: the shipped `oteryn-game-server serve` process,
/// booted and assigned by `oteryn-game-ops`, reproduces every #823 stage
/// against its own bound port. Characters were bootstrapped through the node
/// control socket; this harness only reads them and the assignment back.
#[test]
#[ignore = "requires the disposable node-boot spell topology and Character-owned cast facts"]
fn node_boot_spells_against_running_node() -> TestResult {
    // Reuse the node's issued manifest and the normal client admission. Neither the
    // grant nor this test supplies vocation, progression, resources or targets.
    let manifest = required("NODE_BOOT_SPELL_MANIFEST")?;
    let world = crate::node::uuid_bytes(&required("NODE_BOOT_WORLD_ID")?)
        .ok_or("invalid NODE_BOOT_WORLD_ID")?;
    let channel = crate::node::uuid_bytes(&required("NODE_BOOT_CHANNEL_ID")?)
        .ok_or("invalid NODE_BOOT_CHANNEL_ID")?;
    let room = crate::content::qualify_native_entry_room_from_gameplay_manifest(
        WorldId::decode(&world)?,
        std::path::Path::new(&manifest),
    )
    .map_err(|error| format!("spell manifest qualification: {error}"))?;
    let input = crate::content::native_gameplay::NativeGameplayInput::from_manifest(
        std::path::Path::new(&manifest),
    )?;
    let catalog =
        crate::spell::executable_catalog::compile(&input.catalog.bytes, &input.catalog.sha256)?;
    let selections = crate::spell::executable_catalog::compile_source_selection(
        &input.source_selection.bytes,
        &input.source_selection.sha256,
        &catalog,
    )?;
    let book = catalog.into_spell_book_with_selections(&selections)?;
    let signing = SigningKey::from_bytes(&hex32(&required("WP5_S3B_FRESH_KEY_SEED")?)?);
    let key_id = required("WP5_S3B_FRESH_KEY_ID")?;
    let account = required("WP5_S3A_ACCOUNT_ID")?;
    let certificate = CertificateDer::from(pem_der(
        &required("NODE_BOOT_GAMEPLAY_CERT")?,
        "CERTIFICATE",
    )?);
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let url = required("NODE_BOOT_DATABASE_URL")?;
            let mut database = sqlx::PgConnection::connect(&url).await?;
            let character: Vec<u8> = sqlx::query_scalar(
                "SELECT uuid_send(character_id) FROM game_character_roots WHERE account_id = $1::uuid",
            )
            .bind(&account)
            .fetch_one(&mut database)
            .await?;
            let character = <[u8; 16]>::try_from(character.as_slice())?;
            let generation: String = sqlx::query_scalar(
                "SELECT ownership_generation::text FROM game_runtime_scope_assignments \
                 WHERE world_id = encode($1, 'hex')::uuid AND channel_id = encode($2, 'hex')::uuid AND state = 1",
            )
            .bind(world.as_slice())
            .bind(channel.as_slice())
            .fetch_one(&mut database)
            .await?;
            database.close().await?;
            let descriptor = producer_descriptor(
                SOURCE_AUTHORITY,
                "WP5_S3A_CLIENT_CERT",
                "WP5_S3A_CLIENT_KEY",
            )?;
            let security_generation = platform_generation(&descriptor, &account).await?;
            let token = sign_grant(
                &Grant {
                    signing: &signing,
                    key_id: &key_id,
                    account_id: &account,
                    character_id: character,
                    world_id: world,
                    channel_id: channel,
                    security_generation,
                    scope_generation: generation.parse()?,
                    nonce: [0x61; 32],
                    attempt: v7(61, 3),
                },
                now_seconds()?,
            );
            let mut client = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
                address: required("NODE_BOOT_ADDRESS")?.parse()?,
                server_name: "localhost",
                root_certificate: &certificate,
                schema_revision: 1,
                character_id: CharacterId::decode(&character)?,
                admission_material: token.as_bytes(),
                client_build_id: "oteryn-dev-client/spell-qualification",
                deadline: Duration::from_secs(20),
            })
            .await?;
            if client.world_spatial().content_generation != room.compiled().client_digest() {
                return Err("running node did not admit the issued spell content generation".into());
            }
            let owned_vitals = client.actor_vitals().is_some();
            let before_unknown = client.actor_vitals().copied();
            let unknown = client
                .cast_spell(
                    std::num::NonZeroU32::new(u32::MAX).ok_or("invalid probe index")?,
                    crate::gameplay_transport::actor_spell::SpellTarget::None,
                    false,
                )
                .await?;
            if unknown.disposition != crate::gameplay_transport::actor_spell::SpellCastDisposition::Rejected
                || unknown.actor_vitals_delta.is_some()
                || client.actor_vitals().copied() != before_unknown
            {
                return Err("unknown spell index mutated resources or was not rejected".into());
            }
            evidence("spell_probe unknown_index=rejected resources=unchanged");
            let mut cast = 0;
            for offset in 0..book.source_len() {
                let index = std::num::NonZeroU32::new(u32::try_from(offset + 1)?)
                    .ok_or("invalid spell index")?;
                let (spell, selected) = book.source_indexed(index).ok_or("missing source index")?;
                let outcome = client
                    .cast_spell(index, crate::gameplay_transport::actor_spell::SpellTarget::None, false)
                    .await?;
                if outcome.disposition == crate::gameplay_transport::actor_spell::SpellCastDisposition::Cast {
                    cast += 1;
                    if spell.cooldown_micros >= 1_000_000 {
                        let before_retry = client.actor_vitals().copied();
                        let retry = client
                            .cast_spell(index, crate::gameplay_transport::actor_spell::SpellTarget::None, false)
                            .await?;
                        if retry.disposition != crate::gameplay_transport::actor_spell::SpellCastDisposition::CoolingDown
                            || retry.actor_vitals_delta.is_some()
                            || client.actor_vitals().copied() != before_retry
                        {
                            return Err(format!(
                                "spell {} immediate retry did not refuse without resource payment: {retry:?}",
                                spell.key,
                            )
                            .into());
                        }
                        evidence(&format!("spell_probe index={index} immediate_retry=cooling_down resources=unchanged"));
                    }
                }
                evidence(&format!(
                    "spell_probe index={index} key={} selected={selected} target=none disposition={:?} vitals_delta={}",
                    spell.key,
                    outcome.disposition,
                    outcome.actor_vitals_delta.is_some(),
                ));
            }
            evidence(&format!(
                "spell_probe_summary definitions={} attempted={} cast={cast} character_owned_vitals={owned_vitals} scope=untargeted_catalog_probe gameplay_complete=false",
                book.source_len(),
                book.source_len(),
            ));
            if !owned_vitals {
                return Err("spell execution blocked: bootstrap produced no Character-owned vocation/progression cast facts; transport refusals are not successful spell execution".into());
            }
            if cast == 0 {
                return Err("no spell executed: inspect Character-owned level, vocation, resources and spell_probe dispositions".into());
            }
            Ok(())
        })
}

#[test]
#[ignore = "requires a running node in the disposable WP5 node-boot topology"]
fn node_boot_seam_against_running_node() -> TestResult {
    let key_id = required("WP5_S3B_FRESH_KEY_ID")?;
    let signing = SigningKey::from_bytes(&hex32(&required("WP5_S3B_FRESH_KEY_SEED")?)?);
    let accounts = [
        required("WP5_S3A_ACCOUNT_ID")?,
        required("WP5_S3B_SECOND_ACCOUNT_ID")?,
    ];
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let url = required("NODE_BOOT_DATABASE_URL")?;
            let world = crate::node::uuid_bytes(&required("NODE_BOOT_WORLD_ID")?)
                .ok_or("invalid NODE_BOOT_WORLD_ID")?;
            let channel = crate::node::uuid_bytes(&required("NODE_BOOT_CHANNEL_ID")?)
                .ok_or("invalid NODE_BOOT_CHANNEL_ID")?;
            let certificate = CertificateDer::from(pem_der(
                &required("NODE_BOOT_GAMEPLAY_CERT")?,
                "CERTIFICATE",
            )?);
            let mut database = sqlx::PgConnection::connect(&url).await?;
            let mut characters = Vec::new();
            for account in &accounts {
                let character: Vec<u8> = sqlx::query_scalar(
                    "SELECT uuid_send(character_id) FROM game_character_roots WHERE account_id = $1::uuid",
                )
                .bind(account)
                .fetch_one(&mut database)
                .await?;
                characters.push(<[u8; 16]>::try_from(character.as_slice())?);
            }
            let scope_generation: String = sqlx::query_scalar(
                "SELECT ownership_generation::text FROM game_runtime_scope_assignments \
                 WHERE world_id = encode($1, 'hex')::uuid AND channel_id = encode($2, 'hex')::uuid AND state = 1",
            )
            .bind(world.as_slice())
            .bind(channel.as_slice())
            .fetch_one(&mut database)
            .await?;
            database.close().await?;
            evidence("node_boot characters=2 source=control_socket assignment=operator");
            seam_clients(SeamClients {
                address: required("NODE_BOOT_ADDRESS")?.parse()?,
                certificate: &certificate,
                signing: &signing,
                key_id: &key_id,
                accounts: &accounts,
                characters: &characters,
                world,
                channel,
                scope_generation: scope_generation.parse()?,
                descriptor: &producer_descriptor(
                    SOURCE_AUTHORITY,
                    "WP5_S3A_CLIENT_CERT",
                    "WP5_S3A_CLIENT_KEY",
                )?,
                url: &url,
                runtime: None,
                recovery: &recovery_key()?,
            })
            .await
        })
}

#[test]
#[ignore = "requires the disposable WP5 Platform + PostgreSQL 18.6 topology"]
fn server_seam_real_owners_over_tcp_tls() -> TestResult {
    let key_id = required("WP5_S3B_FRESH_KEY_ID")?;
    let signing = SigningKey::from_bytes(&hex32(&required("WP5_S3B_FRESH_KEY_SEED")?)?);
    if signing.verifying_key().to_bytes() != hex32(&required("WP5_S3B_FRESH_PUBLIC_KEY")?)? {
        return Err("seed does not match the published fresh key".into());
    }
    let accounts = [
        required("WP5_S3A_ACCOUNT_ID")?,
        required("WP5_S3B_SECOND_ACCOUNT_ID")?,
    ];
    // Debug builds poll the composed seam scenario through a call chain deeper than the
    // default 2 MiB test and worker stacks (it overflowed in the admission stage even with
    // the scenario future boxed). Poll it on a dedicated thread, and run the runtime's
    // workers, with an explicit 8 MiB stack; the scenario itself is unchanged.
    const SEAM_STACK_BYTES: usize = 8 << 20;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(SEAM_STACK_BYTES)
        .build()?;
    std::thread::Builder::new()
        .name("server-seam-scenario".into())
        .stack_size(SEAM_STACK_BYTES)
        .spawn(move || {
            runtime
                .block_on(Box::pin(seam_flow(&accounts, &key_id, &signing)))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "server seam scenario thread panicked")?
        .map_err(Into::into)
}

async fn seam_flow(accounts: &[String; 2], key_id: &str, signing: &SigningKey) -> TestResult {
    let spell_input = env::var_os("OTERYN_SEAM_SPELL_MANIFEST")
        .map(|path| {
            crate::content::native_gameplay::NativeGameplayInput::from_manifest(
                std::path::Path::new(&path),
            )
        })
        .transpose()?;
    let descriptor = producer_descriptor(
        SOURCE_AUTHORITY,
        "WP5_S3A_CLIENT_CERT",
        "WP5_S3A_CLIENT_KEY",
    )?;
    let (url, version) = database().await?;
    evidence(&format!(
        "pins postgres_server_version_num={version} platform={PLATFORM_SOURCE}"
    ));
    let (control_role, control_url) = group_login(&url, "oteryn_game_control").await?;
    let (_, runtime_url) = group_login(&url, "oteryn_game_runtime").await?;
    let control = DurabilityRoot::connect_test_runtime(&control_url)?;
    let root = DurabilityRoot::connect_test_runtime(&runtime_url)?;
    if !control.maintain_ready_once().await? || !root.maintain_ready_once().await? {
        return Err("durability roots not ready".into());
    }
    let world = WorldId::decode(&v7(1, 0x02))?;
    let channel = ChannelId::decode(&v7(1, 0x03))?;
    let scope = RuntimeScopeRefV1::channel(world, channel);

    // Owners, composed exactly as S3-B: #415 holder, S2 custody, assignment,
    // readiness; #414 sealed authority and operator-issued Characters.
    let holder = register(&control, &root, 0xa1).await?;
    let now = now_seconds()?;
    let provenance = FreshStoreProvenance {
        namespace: "wp5-seam".into(),
        authorization: "seam-disposable-topology".into(),
        source_authority: SOURCE_AUTHORITY.into(),
        initialized_at: now,
    };
    let descriptor_registration = DescriptorRegistration {
        revision: 1,
        facts: format!("platform@{PLATFORM_SOURCE};source.test;tls1.3-mtls").into_bytes(),
        installed_at: now,
    };
    if !control
        .record_native_source_descriptor_issuance(
            SOURCE_AUTHORITY,
            descriptor_registration.clone(),
            Some(provenance.clone()),
        )
        .await?
    {
        return Err("S2 descriptor issuance refused".into());
    }
    root.initialize_native_admission_source(&holder, provenance, descriptor_registration)
        .await?;
    // Deployment administration: the owner grants the control login this scope.
    let mut owner = sqlx::PgConnection::connect(&url).await?;
    sqlx::query(
        "INSERT INTO game_control_scope_grants (control_role, world_id, channel_id, operation) \
         SELECT $1, encode($2, 'hex')::uuid, encode($3, 'hex')::uuid, operation \
         FROM generate_series(1, 3) AS operation",
    )
    .bind(&control_role)
    .bind(v7(1, 0x02).as_slice())
    .bind(v7(1, 0x03).as_slice())
    .execute(&mut owner)
    .await?;
    if spell_input.is_some() {
        // The native item/Wheel owners independently read the current durable
        // activation. Grant only this disposable control login this exact scope.
        sqlx::query(
            "INSERT INTO game_control_scope_grants (control_role, world_id, channel_id, operation) \
             VALUES ($1, encode($2, 'hex')::uuid, encode($3, 'hex')::uuid, 4)",
        )
        .bind(&control_role)
        .bind(world.as_bytes().as_slice())
        .bind(channel.as_bytes().as_slice())
        .execute(&mut owner)
        .await?;
    }
    owner.close().await?;
    let writer = RuntimeScopeAssignmentWriter::open(control.clone(), "seam-writer")
        .await
        .map_err(|e| format!("{e:?}"))?;
    let request = AssignmentRequest {
        operation_key: OperationKey::from_bytes([1; 32]),
        actor: ControlActor::new(&control_role).map_err(|e| format!("{e:?}"))?,
        command: AssignmentCommand::Assign {
            scope,
            target: holder.fact(),
        },
    };
    let assigned = match writer.submit(&request).await {
        Ok(AssignmentOutcome::Committed(receipt)) => receipt,
        other => return Err(format!("assign: {other:?}").into()),
    };
    let scope_generation = assigned.assignment.ownership_generation;
    // #935: activate the genuine committed entry room for this World, as `serve` does, so the
    // Channel pin and the Movement cells come from the same qualified generation.
    let room = match &spell_input {
        Some(input) => crate::content::qualify_selected_native_gameplay_room(world, input),
        None => crate::content::qualify_native_entry_room(world),
    }
    .map_err(|e| format!("native entry room: {e}"))?;
    let issuance = crate::content::NativeEntryActivationIssuance {
        world_id: world,
        activation_sequence: 1,
        server_artifact_digest: room.compiled().server_digest(),
        client_artifact_digest: room.compiled().client_digest(),
        frame_binding_digest: room.frame_binding().digest(),
    };
    if spell_input.is_some() {
        let request = crate::durability::content_activation::ContentActivationRequest {
            world_id: world,
            channel_id: channel,
            activation_sequence: issuance.activation_sequence,
            previous_sequence: None,
            server_artifact_digest: issuance.server_artifact_digest,
            client_artifact_digest: issuance.client_artifact_digest,
            frame_binding_digest: issuance.frame_binding_digest,
        };
        if !control.record_content_activation(&request).await? {
            return Err("spell content activation owner refused exact scoped issuance".into());
        }
        let recorded = root
            .read_current_content_activation(world, channel)
            .await?
            .ok_or("spell content activation issuance missing after owner write")?;
        if recorded.activation_sequence != issuance.activation_sequence
            || recorded.server_artifact_digest != issuance.server_artifact_digest
            || recorded.client_artifact_digest != issuance.client_artifact_digest
            || recorded.frame_binding_digest != issuance.frame_binding_digest
        {
            return Err("spell content activation readback differs from qualified issuance".into());
        }
        evidence(
            "spell_content activation=durably_issued scoped_grant=content_activate readback=exact",
        );
    }
    let mut content_controller = crate::content::ContentActivationController::new();
    let quiescence = crate::content::NodeBootQuiescence::before_channel_runtime();
    let pin = match &spell_input {
        Some(input) => crate::content::activate_native_entry_room_with_gameplay(
            &mut content_controller,
            &quiescence,
            world,
            &issuance,
            input,
        ),
        None => crate::content::activate_native_entry_room(
            &mut content_controller,
            &quiescence,
            world,
            &issuance,
        ),
    };
    let (channel_pin, movement_cells, door_content, spawn_source) = pin
        .map_err(|e| format!("native entry activation: {e}"))?
        .into_channel_parts();
    // #162 5868482467 (M2b): the same door-binding the production boot sequence performs, from
    // the same committed scope and ownership generation this Channel runtime is composed with
    // below.
    let door_scope_generation =
        crate::foundation::ScopeOwnershipGeneration::new(assigned.assignment.ownership_generation)
            .map_err(|e| format!("native entry door scope generation: {e:?}"))?;
    let door_content_generation =
        crate::world_runtime::ReferenceContentGeneration::from_content(&door_content)
            .map_err(|e| format!("native entry door content generation: {e:?}"))?;
    let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_activation(
        scope,
        door_scope_generation,
        door_content_generation,
    );
    let door = tokio::sync::Mutex::new(
        crate::world_runtime::bind_native_entry_door(
            &door_content,
            &door_fence,
            scope,
            door_scope_generation,
        )
        .map_err(|e| format!("native entry door runtime: {e:?}"))?,
    );
    // C2: the same chest injection the production boot sequence performs.
    let chest = crate::interaction_chest_use::with_entry_chest(&door_content)
        .map_err(|e| format!("native entry chest content: {e:?}"))?;
    let spells = match content_controller
        .active()
        .and_then(|active| active.native_gameplay())
    {
        Some(native) => native.spell_book().clone(),
        None => crate::spell::cast::v1_spell_book()?,
    };
    let achievements = crate::achievement_catalogue::AchievementCatalogue::embedded()
        .map_err(|e| format!("achievement catalogue: {e:?}"))?;
    let imported_charms = crate::content::charm_source::CanonicalCharmCatalogue::embedded()?;
    // KAN-26: the Channel runtime is composed from this exact committed
    // assignment before readiness, as `serve` does.
    let runtime = tokio::sync::Mutex::new(
        crate::foundation::ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            holder.fact().node_id(),
            holder.fact().registration_revision(),
            assigned.assignment.ownership_generation,
            assigned.assignment.source_revision,
            &assigned.assignment.decision_identity,
            usize::try_from(crate::node::config::PREPRODUCTION_FIRST_SLICE_ACTOR_CAPACITY)?,
            channel_pin,
        )
        .map_err(|e| format!("channel runtime: {e:?}"))?,
    );
    // SPAWN-1a: the activation spawn realization `serve` performs, before readiness.
    let spawned_monsters = runtime
        .lock()
        .await
        .realize_activation_spawns(
            spawn_source
                .map(|source| source.activation_facts())
                .into_iter()
                .collect(),
        )
        .map_err(|e| format!("creature spawn realization: {e:?}"))?;
    evidence(&format!(
        "creature_spawns state=realized spawned_monsters={spawned_monsters}"
    ));
    if spell_input.is_some()
        && let Some(source_world) = room.source_world()
    {
        // Refuse invalid placement producers before client admission and the
        // ordinary reconnect grace. This is the same constructor cast executes;
        // qualification does not initialize SQL state or mark the map ready.
        let native = content_controller
            .active()
            .and_then(|active| active.native_gameplay())
            .ok_or("spell map preflight has no active native content")?;
        let runtime_guard = runtime.lock().await;
        let pending = source_world.pending_mutable_placements().len();
        let proof = crate::spell::source_map_initialization::CurrentNativeMapInitialization::from_current_owner(
            &runtime_guard,
            &room,
            native,
        )
        .map_err(|reason| {
            evidence(&format!(
                "spell_map_preflight pending_placements={pending} disposition=refused reason={reason}"
            ));
            format!("spell map placement preflight: {reason}")
        })?;
        use crate::durability::native_map_items_abi::NativeMapInitializationProof;
        evidence(&format!(
            "spell_map_preflight pending_placements={pending} qualified_placements={} disposition=qualified runtime_initialized=false",
            proof.placements().len(),
        ));
    }
    publish_readiness(&root, &holder, scope, scope_generation, now_seconds()?).await?;
    let retained = std::path::PathBuf::from(required("WP5_S3B_CHARACTER_FENCE_DIR")?);
    let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
        .map_err(|e| format!("recovery store: {e:?}"))?;
    {
        let fresh = recovery
            .authorize_fresh_store(v7(1, 0x07), u64::try_from(now_seconds()?)?)
            .map_err(|e| format!("fresh recovery: {e:?}"))?;
        control
            .admit_fresh_character_recovery(&fresh)
            .await
            .map_err(|e| format!("fresh admission: {e:?}"))?;
    }
    let fence = recovery
        .seal_current()
        .map_err(|e| format!("seal: {e:?}"))?;
    let authority = root
        .open_character_authority(&fence)
        .await
        .map_err(|e| format!("open authority: {e:?}"))?;
    let mut operator = sqlx::PgConnection::connect(&control_url).await?;
    sqlx::query_scalar::<_, i64>("SELECT game_character_configure_interpretation($1, $2, $3, $4)")
        .bind(INTERPRETATION[0])
        .bind(INTERPRETATION[1])
        .bind(INTERPRETATION[2])
        .bind(INTERPRETATION[3])
        .fetch_one(&mut operator)
        .await?;
    operator.close().await?;
    let intents = producer_descriptor(
        native_admission_source::CHARACTER_BOOTSTRAP_INTENT_ISSUER,
        "WP5_S3B_INTENT_CLIENT_CERT",
        "WP5_S3B_INTENT_CLIENT_KEY",
    )?;
    let capacity = TransientCapacity::new();
    let mut characters = Vec::new();
    for (index, account) in accounts.iter().enumerate() {
        let tag = u8::try_from(index)? + 1;
        ingest(&root, &holder, &descriptor, account, key_id).await?;
        let mut permit = capacity.try_queue()?;
        permit.try_activate()?;
        let intent = read_authenticated_intent(&intents, v7(tag, 0x04), &mut permit)
            .await
            .map_err(|e| format!("intent read {tag}: {e:?}"))?;
        drop(permit);
        let record = root
            .bootstrap_character(&authority, &holder, &intent)
            .await
            .map_err(|e| format!("bootstrap character {tag}: {e:?}"))?;
        characters.push(*record.character_id.as_bytes());
    }
    evidence("owners=composed holder=assigned characters=2 source=platform_operator_intents");

    // D4: let every pre-seeded observation age past the five-second bound, so
    // an admission can only succeed on evidence fetched for that attempt.
    let fresh_evidence = crate::FreshEvidenceSource::new(producer_descriptor(
        SOURCE_AUTHORITY,
        "WP5_S3A_CLIENT_CERT",
        "WP5_S3A_CLIENT_KEY",
    )?);
    tokio::time::sleep(Duration::from_secs(6)).await;
    evidence("evidence=on_demand pre_seeded_age_seconds>5");

    // The shipped entry, with explicit non-shipping TLS material.
    let generated = rcgen::generate_simple_self_signed(vec!["localhost".to_owned()])?;
    let certificate: CertificateDer<'static> = generated.cert.der().clone();
    let key = PrivatePkcs8KeyDer::from(generated.signing_key.serialize_der());
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let shutdown = CancellationToken::new();
    let (quest_catalogue, quest_log_content, _) = crate::node::serve::load_quest_catalogue(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state,
        "content-s3b-1",
    )
    .map_err(|error| format!("quest catalogue: {error:?}"))?;
    let serve = serve_gameplay(
        &listener,
        GameplayListenerConfig {
            certificates: vec![certificate.clone()],
            private_key: key.into(),
            max_connections: 256,
            max_handshake_units: 64,
            entry_deadline: Duration::from_secs(20),
        },
        GameplaySeamOwners {
            emit: crate::node::serve::emit,
            root: &root,
            character: &authority,
            holder: &holder,
            evidence: &fresh_evidence,
            world_id: world,
            channel_id: channel,
            // ADMIT0: the same binding boot derives. The entry-room seam declares
            // `SEAM_REQUIRED_CAPABILITIES` (ADMIT-CAP-1); the spell seam declares none.
            required_capabilities:
                crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                    if spell_input.is_some() {
                        &[]
                    } else {
                        SEAM_REQUIRED_CAPABILITIES
                    },
                    scope,
                    scope_generation,
                    "policy-s3b-1",
                    crate::gameplay_transport::world_required_capabilities::offered_for_scope(
                        false,
                        spell_input
                            .as_ref()
                            .and_then(|_| content_controller.active()),
                    ),
                )
                .map_err(|error| format!("required gameplay capabilities: {error:?}"))?,
            runtime: &runtime,
            movement_cells: &movement_cells,
            door: &door,
            chest: &chest,
            spells: &spells,
            active_generation: spell_input
                .as_ref()
                .and_then(|_| content_controller.active()),
            premium_coordinator: None,
            rooms: crate::gameplay_transport::SeamRooms::new(
                spell_input.as_ref().map(|_| &room),
                spell_input.as_ref().map(|_| &room),
            ),
            achievements: &achievements,
            npc_talk: None,
            imported_charms: &imported_charms,
            quest_catalogue: &quest_catalogue,
            quest_log_content: &quest_log_content,
            last_position_flush: None,
        },
        &shutdown,
        None,
    );

    let recovery_signing = recovery_key()?;
    let client_config = SeamClients {
        address,
        certificate: &certificate,
        signing,
        key_id,
        accounts,
        characters: &characters,
        world: v7(1, 0x02),
        channel: v7(1, 0x03),
        scope_generation,
        descriptor: &descriptor,
        url: &url,
        runtime: Some(&runtime),
        recovery: &recovery_signing,
    };
    let clients = async {
        if spell_input.is_some() {
            seam_spell_clients(
                client_config,
                &root,
                &authority,
                &holder,
                &spells,
                content_controller
                    .active()
                    .and_then(|active| active.native_gameplay())
                    .ok_or("spell scenario has no active qualified content")?,
                &room,
                &runtime_url,
            )
            .await
        } else {
            seam_clients(client_config).await
        }
    };
    // The listener must outlive every client case: an early listener exit is a
    // failure, never a hang on an unaccepted connection.
    let mut serve = pin!(serve);
    let mut clients = pin!(clients);
    let mut client_result = None;
    let served = poll_fn(|context| {
        if client_result.is_none()
            && let Poll::Ready(result) = clients.as_mut().poll(context)
        {
            // Success or failure, the client cases are over: drain the listener
            // so a client error surfaces instead of waiting forever.
            shutdown.cancel();
            client_result = Some(result);
        }
        match serve.as_mut().poll(context) {
            Poll::Ready(result) => Poll::Ready(result),
            Poll::Pending => Poll::Pending,
        }
    })
    .await;
    match client_result {
        Some(result) => result?,
        None => return Err(format!("listener ended before the client cases: {served:?}").into()),
    }
    served?;
    if spell_input.is_some() {
        evidence("spell_seam shutdown=drained scope=real_owner_tcp_tls gameplay_complete=false");
        return Ok(());
    }
    // KAN-26: every committed fresh admission holds exactly one player actor;
    // refused, replayed and losing attempts leave no reservation behind; and
    // ordinary disconnect (every client has closed by now) removes nothing.
    // #822: four lost actors were removed only by a durable terminal release: the silent
    // (concurrent[0]) one after grace, the resumed (session) one when its recovered connection
    // ended again, the use-wire (#162 5868482467) one after its own grace, and the dev-client
    // (#162 C1b) one after its own grace. The re-admitted character[0] holds the one remaining
    // actor, and a plain disconnect removed nothing (its loss window was cut by shutdown).
    // `committed_admissions` is a durable receipt count that never decreases, so it reflects all
    // five admissions made (fresh, concurrent-winner, use-wire re-admission, dev-client
    // re-admission, grace-expiry re-admission), not the one actor still committed.
    let (committed_players, pending) = runtime.lock().await.player_slot_counts();
    if committed_players != 1 || pending != 0 || committed_admissions(&url).await? != 5 {
        return Err(
            format!("channel runtime committed={committed_players} pending={pending}").into(),
        );
    }
    evidence(
        "channel_runtime committed_players=1 pending_reservations=0 released_after_grace=4 disconnect_removed=0",
    );
    // #935: the remaining committed actor was positioned once, by the Channel
    // owner, at the pinned generation's start cell under the pinned context,
    // and never stepped.
    let revisions = runtime.lock().await.entry_start_player_revisions();
    if revisions != [1] {
        return Err(format!("entry-start player revisions={revisions:?}").into());
    }
    evidence("first_entry positioned_players=1 start=entry-start context=pinned revisions=1");
    // Released actors leave no control-loss mark behind.
    let marks = runtime.lock().await.player_control_loss_epochs();
    if !marks.is_empty() {
        return Err(format!("runtime control-loss marks={marks:?}").into());
    }
    evidence("grace_expiry runtime_released_actors=2 marks_left=0");
    evidence("shutdown=drained FORMAL_ADR0007_QA_TIER1_TIER2=NOT_EVALUATED");
    Ok(())
}

/// Full-content smoke through actual Character writers and the shipped TLS listener.
#[allow(clippy::too_many_arguments)]
async fn seam_spell_clients(
    config: SeamClients<'_>,
    root: &DurabilityRoot,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    holder: &NodeIncarnationProof,
    book: &crate::spell::SpellBook,
    native: &crate::content::native_gameplay::NativeGameplayState,
    room: &crate::content::QualifiedNativeEntryRoom,
    runtime_url: &str,
) -> TestResult {
    let character = config.characters[1];
    let account = &config.accounts[1];
    let generation = platform_generation(config.descriptor, account).await?;
    let token = |tag| -> TestResult<String> {
        Ok(sign_grant(
            &Grant {
                signing: config.signing,
                key_id: config.key_id,
                account_id: account,
                character_id: character,
                world_id: config.world,
                channel_id: config.channel,
                security_generation: generation,
                scope_generation: config.scope_generation,
                nonce: [tag; 32],
                attempt: v7(tag, 0x31),
            },
            now_seconds()?,
        ))
    };
    let first_token = token(70)?;
    evidence("spell_seam stage=initial_join");
    let first = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address: config.address,
        server_name: "localhost",
        root_certificate: config.certificate,
        schema_revision: 1,
        character_id: CharacterId::decode(&character)?,
        admission_material: first_token.as_bytes(),
        client_build_id: "oteryn-dev-client/spell-owner-setup",
        deadline: Duration::from_secs(20),
    })
    .await?;
    evidence("spell_seam stage=initial_join_complete");
    let session = first.join_snapshot().game_session_id;
    let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(root.clone())
        .current_session(session)
        .await
        .map_err(|error| format!("spell setup session: {error:?}"))?;
    spell_character::prepare_free_sorcerer(
        root,
        authority,
        holder,
        &current,
        crate::domain::progression::ProgressionRevisionContext {
            profile: INTERPRETATION[0].into(),
            ruleset: INTERPRETATION[1].into(),
            content: INTERPRETATION[2].into(),
            simulation: "spell-qualification-simulation-1".into(),
            evidence: "spell-qualification-real-owners-1".into(),
            declaration: "spell-qualification-finite-1".into(),
        },
        v7(71, 0x31),
        v7(72, 0x31),
    )
    .await?;
    evidence("spell_seam stage=character_writers_complete");
    drop(first);
    // Let the ordinary control-loss and grace-release owners remove the old actor.
    // Re-admission, rather than overwriting runtime resources, installs new cast facts.
    let deadline = tokio::time::Instant::now() + Duration::from_secs(170);
    while !matches!(
        session_loss_row(config.url, *session.as_bytes()).await?,
        Some((3, _, _))
    ) {
        if tokio::time::Instant::now() >= deadline {
            return Err("spell setup session did not release through ordinary grace".into());
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let second_token = token(73)?;
    evidence("spell_seam stage=classed_reentry");
    let mut client = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address: config.address,
        server_name: "localhost",
        root_certificate: config.certificate,
        schema_revision: 1,
        character_id: CharacterId::decode(&character)?,
        admission_material: second_token.as_bytes(),
        client_build_id: "oteryn-dev-client/spell-seam",
        deadline: Duration::from_secs(20),
    })
    .await?;
    let before = client
        .actor_vitals()
        .copied()
        .ok_or("classed re-admission supplied no vitals")?;
    evidence(&format!(
        "spell_seam stage=classed_reentry_complete health={} max_health={} mana={} max_mana={} soul={}",
        before.health, before.max_health, before.mana, before.max_mana, before.soul
    ));
    let heal_key = &book
        .spoken("exura")
        .ok_or("full catalog has no exura")?
        .spell
        .key;
    let index = (1..=u32::try_from(book.source_len())?)
        .filter_map(std::num::NonZeroU32::new)
        .find(|index| {
            book.indexed(*index)
                .is_some_and(|spell| &spell.key == heal_key)
        })
        .ok_or("exura has no active index")?;
    evidence(&format!(
        "spell_seam stage=exura_command_send index={index}"
    ));
    let healed = client
        .cast_spell(
            index,
            crate::gameplay_transport::actor_spell::SpellTarget::None,
            false,
        )
        .await?;
    evidence(&format!(
        "spell_seam stage=exura_command_result disposition={:?}",
        healed.disposition
    ));
    let after = client.actor_vitals().copied().ok_or("cast lost vitals")?;
    if healed.disposition != crate::gameplay_transport::actor_spell::SpellCastDisposition::Cast
        || healed.actor_vitals_delta.is_none()
        || after.mana >= before.mana
        || after.health > after.max_health
        || after.health < before.health
    {
        return Err(format!(
            "real exura failed: outcome={healed:?} before={before:?} after={after:?}"
        )
        .into());
    }
    let retry = client
        .cast_spell(
            index,
            crate::gameplay_transport::actor_spell::SpellTarget::None,
            false,
        )
        .await?;
    if retry.disposition
        != crate::gameplay_transport::actor_spell::SpellCastDisposition::CoolingDown
        || retry.actor_vitals_delta.is_some()
        || client.actor_vitals().copied() != Some(after)
    {
        return Err(format!("real exura cooldown failed: {retry:?}").into());
    }
    evidence(&format!(
        "spell_seam definitions={} caster=free_sorcerer_level8 setup=fenced_character_writers reentry=ordinary_grace exura=cast mana_before={} mana_after={} immediate_retry=cooling_down health_bounded=true wounded_heal_magnitude=not_evaluated",
        book.source_len(),
        before.mana,
        after.mana
    ));
    if native.magnitude_policy() != crate::spell::magnitude_owner::MagnitudePolicy::BaselineTest {
        evidence("spell_seam magnitude_policy=strict high_level_training_scenarios=not_selected");
        for offset in 0..book.source_len() {
            let index = std::num::NonZeroU32::new(u32::try_from(offset + 1)?)
                .ok_or("invalid spell catalog index")?;
            let (spell, selected) = book
                .source_indexed(index)
                .ok_or("missing spell catalog index")?;
            let outcome = client
                .cast_spell(
                    index,
                    crate::gameplay_transport::actor_spell::SpellTarget::None,
                    false,
                )
                .await?;
            evidence(&format!(
                "spell_probe index={index} key={} selected={selected} caster=free_sorcerer_level8 target=none disposition={:?} vitals_delta={} scope=untargeted_catalog_probe gameplay_complete=false",
                spell.key,
                outcome.disposition,
                outcome.actor_vitals_delta.is_some()
            ));
        }
        return Ok(());
    }
    // Award XP through the same Character owner. Existing actor resources are
    // left alone until normal grace release and fresh client admission.
    let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(root.clone())
        .current_session(client.join_snapshot().game_session_id)
        .await
        .map_err(|error| format!("spell upgrade session: {error:?}"))?;
    spell_character::advance_source_level(root, authority, holder, &current, 1000, v7(74, 0x31))
        .await?;
    let previous_session = client.join_snapshot().game_session_id;
    drop(client);
    wait_spell_fixture_release(config.url, previous_session).await?;
    let character_owner = crate::domain::CharacterId::from_bytes(character)
        .map_err(|error| format!("spell character: {error:?}"))?;
    let saved_before = root
        .read_character_build_state(authority, character_owner)
        .await
        .map_err(|error| format!("spell initial save: {error:?}"))?;
    let high_token = token(75)?;
    let mut client = oteryn_dev_client::connect_session(oteryn_dev_client::JoinRequest {
        address: config.address,
        server_name: "localhost",
        root_certificate: config.certificate,
        schema_revision: 1,
        character_id: CharacterId::decode(&character)?,
        admission_material: high_token.as_bytes(),
        client_build_id: "oteryn-dev-client/spell-high-level-seam",
        deadline: Duration::from_secs(20),
    })
    .await?;
    let high_vitals = client
        .actor_vitals()
        .copied()
        .ok_or("high-level re-admission supplied no vitals")?;
    if high_vitals.max_mana != 29850 || high_vitals.max_health != 5145 {
        return Err(format!("source level1000 maxima mismatch: {high_vitals:?}").into());
    }
    evidence(
        "spell_seam caster=free_sorcerer_level1000 setup=xp_writer reentry=ordinary_grace premium=false resources=normal_admission",
    );
    let mut observed_mana_paid = 0_u64;
    let mut wild_rat = None;
    // These exercise self heal/light/shield/invisibility and a position damage
    // recipe. The native test spawn qualifies physical melee and target damage;
    // client condition visibility and corpse/loot/XP remain separate lanes.
    for (words, position_target) in [
        ("utevo lux", false),
        ("exori vis", true),
        ("exura", false),
        ("utamo vita", false),
        ("utana vid", false),
        ("utana vid", false),
        ("utana vid", false),
        ("utana vid", false),
    ] {
        let source = book
            .spoken(words)
            .ok_or_else(|| format!("full catalog missing representative spell {words}"))?
            .spell;
        let source_key = &source.key;
        let source_index = (1..=u32::try_from(book.source_len())?)
            .filter_map(std::num::NonZeroU32::new)
            .find(|index| {
                book.indexed(*index)
                    .is_some_and(|spell| &spell.key == source_key)
            })
            .ok_or_else(|| format!("representative spell has no index: {words}"))?;
        let cooldown = source
            .groups
            .iter()
            .map(|group| group.cooldown_micros)
            .chain(std::iter::once(source.cooldown_micros))
            .max()
            .unwrap_or(0);
        client
            .service_liveness(Duration::from_micros(cooldown.saturating_add(100_000)))
            .await?;
        if position_target {
            // Source target-or-direction spells require an actual facing owner.
            // Two normal steps on the qualified entry cells return the caster to
            // its original tile and establish West through the movement owner.
            for direction in [StepDirection::East, StepDirection::West] {
                client.service_liveness(Duration::from_secs(1)).await?;
                let movement = client.step(direction).await?;
                if movement.disposition != StepDisposition::Moved {
                    return Err(format!(
                        "position recipe could not establish real facing: {movement:?}"
                    )
                    .into());
                }
            }
            evidence("spell_seam facing_owner=normal_movement path=temple_east_return");
            let protected_position = client.world_spatial().actor_position;
            if (
                protected_position.x,
                protected_position.y,
                protected_position.floor,
            ) != (0, 0, -5)
            {
                return Err(
                    "Thalom protection-zone route requires its qualified temple start".into(),
                );
            }
            let protected_target = crate::gameplay_transport::actor_spell::SpellTarget::Position(
                oteryn_protocol_oteryn::actor_spell::SpellTargetPosition {
                    x: 1,
                    y: 0,
                    floor: -5,
                },
            );
            let protected_vitals = client
                .actor_vitals()
                .copied()
                .ok_or("protected probe missing vitals")?;
            let refused = client
                .cast_spell(source_index, protected_target, false)
                .await?;
            if refused.disposition
                != crate::gameplay_transport::actor_spell::SpellCastDisposition::Rejected
                || refused.actor_vitals_delta.is_some()
                || client.actor_vitals().copied() != Some(protected_vitals)
            {
                return Err(format!(
                    "protection-zone aggression did not refuse without payment: {refused:?}"
                )
                .into());
            }
            evidence("spell_scenario words=exori_vis temple_protection=refused mana=unchanged");
            // Pinned Thalom source stairs: entering (0,5,-5) descends to
            // (0,6,-6), then entering (0,10,-6) descends to (0,11,-7).
            // All movement and floor changes use the ordinary source-step owner.
            for (x, y, floor) in [
                (0, 1, -5),
                (0, 2, -5),
                (0, 3, -5),
                (0, 4, -5),
                (0, 6, -6),
                (0, 7, -6),
                (0, 8, -6),
                (0, 9, -6),
                (0, 11, -7),
            ] {
                client.service_liveness(Duration::from_secs(1)).await?;
                let movement = client.step(StepDirection::South).await?;
                let observed = client.world_spatial().actor_position;
                if movement.disposition != StepDisposition::Moved
                    || (observed.x, observed.y, observed.floor) != (x, y, floor)
                {
                    return Err(format!("source Thalom staircase route failed expected=({x},{y},{floor}) observed={observed:?} result={movement:?}").into());
                }
                evidence(&format!(
                    "spell_seam source_step=south observed=({x},{y},{floor})"
                ));
            }
            evidence(
                "spell_seam aggressive_caster_tile=(0,11,-7) target_tile=(1,11,-7) protection=false proof=pinned_thalom_source normal_movement=true",
            );
            let live_runtime = config
                .runtime
                .ok_or("wild spawn has no current Channel owner")?;
            let mut sql = sqlx::PgConnection::connect(runtime_url).await?;
            let runtime_reader: bool = sqlx::query_scalar("SELECT pg_has_role(current_user,'oteryn_game_runtime','member') AND NOT (SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname=current_user)")
                .fetch_one(&mut sql).await?;
            if !runtime_reader {
                return Err("wild spawn collision proof requires actual runtime role".into());
            }
            let mut tx = sql.begin().await?;
            let mut current = live_runtime.lock().await;
            let rat = wild_spawn::realize_rat_in_transaction(
                &mut tx,
                root,
                authority,
                holder,
                room,
                native,
                &mut current,
            )
            .await?;
            if rat.state.master.is_some()
                || rat.state.policy.is_familiar
                || rat.position != wild_spawn::POSITION
                || rat.health != rat.maximum_health
            {
                return Err("native qualification spawn did not produce an actual wild Rat".into());
            }
            // The spawn is ephemeral runtime state, not a SQL mutation. A failed
            // read-only COMMIT aborts this disposable fixture before any PASS claim;
            // fixture shutdown discards its scope, rather than claiming SQL rollback
            // undoes an already-realized in-memory actor.
            tx.commit().await?;
            drop(current);
            evidence(&format!(
                "spell_seam native_test_spawn={} public_source_spawn=false policy={} revision={} actual_health={} position=(1,11,-7) master=none authority=current_channel_assignment dynamic_collision=actual_item_owner",
                wild_spawn::DECLARATION,
                rat.state.policy.definition_key,
                rat.state.policy.definition_revision,
                rat.health,
            ));
            let initial = client
                .actor_vitals()
                .copied()
                .ok_or("Rat AI probe has no vitals")?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
            loop {
                client.service_liveness(Duration::from_secs(1)).await?;
                let observed = client
                    .actor_vitals()
                    .copied()
                    .ok_or("Rat AI probe lost vitals")?;
                if observed.health < initial.health {
                    evidence(&format!(
                        "spell_seam monster_melee=actual_ai_cycle target=ordinary_player hp_before={} hp_after={} shield=false invisibility=false death_loot_xp=not_evaluated",
                        initial.health, observed.health
                    ));
                    break;
                }
                if tokio::time::Instant::now() >= deadline {
                    return Err(
                        "native spawned Rat AI did not produce actual player health loss".into(),
                    );
                }
            }
            wild_rat = Some(rat);
        }
        let position = client.world_spatial().actor_position;
        let target = if position_target {
            crate::gameplay_transport::actor_spell::SpellTarget::Position(
                oteryn_protocol_oteryn::actor_spell::SpellTargetPosition {
                    x: position
                        .x
                        .checked_add(1)
                        .ok_or("position target coordinate overflow")?,
                    y: position.y,
                    floor: position.floor,
                },
            )
        } else {
            crate::gameplay_transport::actor_spell::SpellTarget::None
        };
        let before = client
            .actor_vitals()
            .copied()
            .ok_or("representative cast missing vitals")?;
        let outcome = client.cast_spell(source_index, target, false).await?;
        let after = client
            .actor_vitals()
            .copied()
            .ok_or("representative cast lost vitals")?;
        if outcome.disposition != crate::gameplay_transport::actor_spell::SpellCastDisposition::Cast
            || outcome.actor_vitals_delta.is_none()
            || after.mana >= before.mana
            || after.health > after.max_health
        {
            return Err(format!("representative real cast failed words={words} result={outcome:?} before={before:?} after={after:?}").into());
        }
        if words == "exura" {
            // The prior real wild-creature attack supplies the wound. Neither
            // fixture HP writes nor synthetic damage are used to qualify healing.
            if wild_rat.is_none()
                || before.health >= before.max_health
                || after.health <= before.health
            {
                return Err(format!(
                    "Light Healing did not heal an actual Rat wound before={before:?} after={after:?}"
                ).into());
            }
            evidence(&format!(
                "spell_seam words=exura wound=actual_monster_melee hp_before={} hp_after={} healing_magnitude=actual_owner optional_modifiers=baseline_omission",
                before.health, after.health
            ));
        }
        if position_target {
            let before_rat = wild_rat
                .as_ref()
                .ok_or("damage probe has no real wild Rat")?;
            let current = config
                .runtime
                .ok_or("damage probe has no Channel owner")?
                .lock()
                .await;
            let damaged = current
                .companion_snapshot_including_dead(before_rat.actor)
                .map_err(|error| format!("actual target health read: {error:?}"))?;
            if damaged.health >= before_rat.health
                || damaged.state.policy != before_rat.state.policy
            {
                return Err(format!(
                    "Energy Strike produced no real creature damage before={} after={}",
                    before_rat.health, damaged.health
                )
                .into());
            }
            evidence(&format!(
                "spell_seam words=exori_vis target=actual_native_test_rat hp_before={} hp_after={} damage_magnitude=actual_owner baseline_optional_modifiers=omitted corpse_loot_xp=not_evaluated",
                before_rat.health, damaged.health
            ));
        }
        observed_mana_paid += u64::from(before.mana - after.mana);
        evidence(&format!(
            "spell_scenario words={words:?} disposition=cast target={target:?} mana_paid={} condition_visibility=not_evaluated creature_damage_magnitude={}",
            before.mana - after.mana,
            if position_target {
                "actual_owner"
            } else {
                "not_evaluated"
            }
        ));
    }
    if observed_mana_paid < 1600 {
        return Err(format!("training stress paid only {observed_mana_paid} mana").into());
    }
    evidence(&format!(
        "spell_seam training_stress observed_mana_paid={observed_mana_paid} checkpoint_threshold=1600"
    ));
    for offset in 0..book.source_len() {
        let index = std::num::NonZeroU32::new(u32::try_from(offset + 1)?)
            .ok_or("invalid spell catalog index")?;
        let (spell, selected) = book
            .source_indexed(index)
            .ok_or("missing spell catalog index")?;
        let outcome = client
            .cast_spell(
                index,
                crate::gameplay_transport::actor_spell::SpellTarget::None,
                false,
            )
            .await?;
        evidence(&format!(
            "spell_probe index={index} key={} selected={selected} caster=free_sorcerer_level1000 target=none disposition={:?} vitals_delta={} scope=untargeted_catalog_probe gameplay_complete=false",
            spell.key,
            outcome.disposition,
            outcome.actor_vitals_delta.is_some()
        ));
    }
    let final_session = client.join_snapshot().game_session_id;
    drop(client);
    wait_spell_fixture_release(config.url, final_session).await?;
    let saved_after = root
        .read_character_build_state(authority, character_owner)
        .await
        .map_err(|error| format!("spell final save: {error:?}"))?;
    let formula = crate::spell::mana_training::CompiledTrainingFormula::from_profile(
        include_bytes!("../../../tools/content-schema/native-gameplay/build-training.json"),
        INTERPRETATION[2],
    )
    .map_err(|error| format!("spell training formula: {error:?}"))?;
    let cumulative = |build: &crate::durability::character_build::DurableBuildState| {
        let (level, progress) = build.magic();
        crate::durability::character_build::cumulative_progress(level, progress, 0, |next| {
            crate::durability::character_build::BuildFormula::required(
                &formula,
                build.vocation(),
                0,
                next,
            )
        })
    };
    let trained = cumulative(&saved_after)
        .checked_sub(cumulative(&saved_before))
        .ok_or("saved training moved backwards")?;
    if trained < observed_mana_paid {
        return Err(format!("paid cast training was not durably saved: paid={observed_mana_paid} saved_delta={trained} before={saved_before:?} after={saved_after:?}").into());
    }
    evidence(&format!(
        "spell_seam training_save=durable ordinary_disconnect=complete mana_paid={observed_mana_paid} saved_training_delta={trained} content_revision={}",
        INTERPRETATION[2]
    ));
    Ok(())
}

async fn wait_spell_fixture_release(
    url: &str,
    session: crate::foundation::GameSessionId,
) -> TestResult {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(170);
    while !matches!(
        session_loss_row(url, *session.as_bytes()).await?,
        Some((3, _, _))
    ) {
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "spell fixture session did not release through ordinary grace and training save"
                    .into(),
            );
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    Ok(())
}

/// Everything the #823 client stages need from a serving node: its address
/// and TLS trust, the grant signer, the Characters and scope generation, and
/// the durable store to count committed admissions.
struct SeamClients<'a> {
    address: SocketAddr,
    certificate: &'a CertificateDer<'static>,
    signing: &'a SigningKey,
    key_id: &'a str,
    accounts: &'a [String; 2],
    characters: &'a [[u8; 16]],
    world: [u8; 16],
    channel: [u8; 16],
    scope_generation: u64,
    descriptor: &'a ProducerDescriptor,
    url: &'a str,
    runtime: Option<&'a tokio::sync::Mutex<crate::foundation::ChannelRuntimeV1>>,
    /// The published Platform recovery key (`oteryn-reauth-recovery-v1`) and its id.
    recovery: &'a (String, SigningKey),
}

/// The Platform recovery signing key published in the topology (#822 resume).
fn recovery_key() -> TestResult<(String, SigningKey)> {
    Ok((
        required("WP5_S3B_RECOVERY_KEY_ID")?,
        SigningKey::from_bytes(&hex32(&required("WP5_S3B_RECOVERY_KEY_SEED")?)?),
    ))
}

/// A Platform `oteryn-reauth-recovery-v1` credential for `character` of `account` in `world`.
fn sign_recovery(
    (key_id, signing): &(String, SigningKey),
    account: &str,
    character: &[u8; 16],
    world: &[u8; 16],
    security_generation: u64,
    issued_at: i64,
    nonce: [u8; 32],
) -> String {
    let header = format!(r#"{{"alg":"Ed25519","kid":"{key_id}","typ":"oteryn-recovery+jwt"}}"#);
    let payload = serde_json::json!({
        "iss": "urn:oteryn:platform:game-recovery",
        "aud": "urn:oteryn:game:recovery",
        "iat": issued_at,
        "nbf": issued_at,
        "exp": issued_at + 30,
        "jti": URL_SAFE_NO_PAD.encode(nonce),
        "profile": "oteryn-reauth-recovery-v1",
        "purpose": "existing_actor_recovery",
        "attempt_ref": uuid_text(&v7(nonce[0], 0x0e)),
        "account_id": account,
        "character_id": uuid_text(character),
        "world_id": uuid_text(world),
        "account_security_generation": security_generation.to_string(),
        "protocol_major": 1,
        "transport_profile": 1,
        "ruleset_revision": "rules-s3b-1",
        "content_revision": "content-s3b-1",
        "map_revision": "map-s3b-1",
        "world_policy_revision": "policy-s3b-1",
    });
    let signing_input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header),
        URL_SAFE_NO_PAD.encode(payload.to_string())
    );
    let signature = URL_SAFE_NO_PAD.encode(signing.sign(signing_input.as_bytes()).to_bytes());
    format!("{signing_input}.{signature}")
}

/// The exact frames after a same-session recovery of the admission-stage session: resumed
/// generation 2 continues CommandId 6 and server_sequence 7 with a baseline snapshot at the
/// committed position (0,0,0) revision 3; command 6 steps east (revision 4); command 8 is a gap.
/// One accepted resume at `generation`: `ServerResumeAccepted` continuing `sequence` and
/// `next` CommandId, the snapshot of the actor at `x` and `revision`, command `next` stepping
/// to `to` at `revision + 1`, then the gap error for command `next + 2`.
struct ResumeFrames {
    generation: u64,
    sequence: u64,
    next: u64,
    revision: u64,
    x: i32,
    to: i32,
}

fn resume_frames(
    world: WorldId,
    session: &[u8; 16],
    resume: ResumeFrames,
) -> TestResult<Vec<Vec<u8>>> {
    let ResumeFrames {
        generation,
        sequence,
        next,
        revision,
        x,
        to,
    } = resume;
    let room = crate::content::qualify_native_entry_room(world)
        .map_err(|e| format!("native entry room: {e}"))?;
    let at = |x| {
        encode_world_spatial(&WorldSpatialObservation {
            content_generation: room.compiled().client_digest(),
            actor_position: ActorPosition { x, y: 0, floor: 0 },
        })
    };
    let mut frames = vec![crate::foundation::encode_server_resume_accepted(
        &crate::foundation::ServerResumeAcceptedValue {
            game_session_id: crate::foundation::GameSessionId::decode(session)?,
            connection_generation: generation,
            current_server_sequence: sequence,
            next_command_id: next,
            schema_revision: crate::gameplay_transport::SERVER_SCHEMA_REVISION,
            selected_capabilities: SEAM_REQUIRED_CAPABILITIES,
        },
    )?];
    // USE-WIRE-V1 (#162 5868482467): the resync join snapshot also carries the door's current
    // `WORLD_OBJECT_OVERLAY`; nothing in this scenario touches the door, so it is still closed
    // at revision 0.
    let door_overlay = oteryn_protocol_oteryn::world_object::WorldObjectOverlayEntry {
        content_generation: room.compiled().client_digest(),
        placement: crate::content::accepted::DOOR_CELL.0.as_bytes().to_vec(),
        state: crate::content::accepted::DOOR_CLOSED_STATE
            .as_bytes()
            .to_vec(),
        revision: 0,
    };
    let vitals_payload = vocationless_vitals_payload()?;
    frames.extend(encode_single_chunk_snapshot(
        generation,
        1,
        sequence,
        &[
            DomainSnapshot {
                domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                revision,
                snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                payload: &at(x),
            },
            DomainSnapshot {
                domain_id: oteryn_protocol_oteryn::world_object::STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                revision: 0,
                snapshot_type:
                    oteryn_protocol_oteryn::world_object::SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                payload:
                    &oteryn_protocol_oteryn::world_object::encode_world_object_overlay_snapshot(&[
                        door_overlay,
                    ])
                    .map_err(|_| "encode door overlay snapshot")?,
            },
            DomainSnapshot {
                domain_id: oteryn_protocol_oteryn::actor_spell::STATE_DOMAIN_ACTOR_VITALS,
                revision: 1,
                snapshot_type: oteryn_protocol_oteryn::actor_spell::SNAPSHOT_TYPE_ACTOR_VITALS_V1,
                payload: &vitals_payload,
            },
        ],
    )?);
    frames.extend([
        encode_command_result(
            generation,
            sequence + 1,
            next,
            CommandStatus::Accepted,
            &encode_step_result(StepDisposition::Moved),
        )?,
        encode_state_delta(
            generation,
            sequence + 2,
            STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
            revision,
            revision + 1,
            DELTA_TYPE_WORLD_SPATIAL_V1,
            &at(to),
        )?,
        encode_command_protocol_error(
            FoundationProtocolError::CommandSequenceGap,
            generation,
            next + 2,
            next + 1,
        )?,
    ]);
    Ok(frames)
}

/// The #823 `SEAM_PASS` client stages against one serving node.
#[path = "qualification/client_phases/negatives.rs"]
mod client_negatives;

#[path = "qualification/client_phases/admission.rs"]
mod client_admission;

#[path = "qualification/client_phases/recovery.rs"]
mod client_recovery;

#[path = "qualification/client_phases/use_wire.rs"]
mod client_use_wire;

#[path = "qualification/client_phases/dev_client.rs"]
mod client_dev_client;

#[path = "qualification/client_phases/readmission.rs"]
mod client_readmission;

async fn seam_clients(clients: SeamClients<'_>) -> TestResult {
    let SeamClients {
        certificate,
        signing,
        key_id,
        world,
        channel,
        scope_generation,
        ..
    } = clients;
    let exact = connector(certificate, &EXACT)?;
    let mut nonce_tag = 0x40u8;
    let mut next_grant = |account: &str, character: [u8; 16], generation: u64| {
        nonce_tag += 1;
        Grant {
            signing,
            key_id,
            account_id: "",
            character_id: character,
            world_id: world,
            channel_id: channel,
            security_generation: generation,
            scope_generation,
            nonce: [nonce_tag; 32],
            attempt: v7(nonce_tag, 0x05),
        }
        .with_account(account)
    };

    // Retain the plaintext socket through every phase, as in the original client flow.
    let (grant, _plaintext) =
        client_negatives::transport(&clients, &exact, &mut next_grant).await?;
    client_negatives::admission_negatives(&clients, &exact, &grant).await?;
    let session = client_admission::admission(&clients, &exact, &grant).await?;
    let concurrent =
        client_admission::concurrent_and_control_loss(&clients, &exact, &mut next_grant, session)
            .await?;
    client_recovery::resume_session(&clients, &exact, session).await?;
    client_recovery::resumed_loss_and_expiry(&clients, &exact, session, concurrent).await?;
    client_use_wire::use_wire(&clients, &exact, &mut next_grant).await?;
    client_dev_client::dev_client(&clients, &mut next_grant).await?;
    client_readmission::readmit_after_release(&clients, &exact, &mut next_grant).await?;
    Ok(())
}

/// One exact-equality expectation of the `dev_client` stage.
fn dev_client_expect<T: PartialEq + std::fmt::Debug + ?Sized>(
    label: &str,
    actual: &T,
    expected: &T,
) -> TestResult {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("dev client {label} diverged: {actual:?} (expected {expected:?})").into())
    }
}

/// Reads the idle session until a pushed delta is queued (bounded, 5s) and returns the queued
/// events: a `COMMITTED` use returns at its result, so the door delta behind it is read here.
/// VIS-3: with capability 6 selected, an own step's entity delta is also queued as an event (the
/// outcome carries only its observation, `crates/session`); it is drained after each step so the
/// door reads see only domain 2. Without capability 6 a step queues nothing.
fn dev_client_step_event(
    session: &mut oteryn_dev_client::DevClientSession,
    label: &str,
    step: &oteryn_dev_client::StepOutcome,
) -> TestResult {
    use oteryn_dev_client::SessionEvent;
    let events = session.take_events();
    let entities = session.selected_capabilities().contains(
        &oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES,
    );
    let own = match (entities, &step.world_spatial_delta, events.as_slice()) {
        (false, _, []) => true,
        (true, Some(observed), [SessionEvent::WorldSpatialEntities(delta)]) => {
            delta.server_sequence == observed.server_sequence
                && delta.base_revision == observed.base_revision
                && delta.new_revision == observed.new_revision
                && delta.value.content_generation == observed.value.content_generation
                && delta.value.actor_position == observed.value.actor_position
        }
        _ => false,
    };
    if own {
        Ok(())
    } else {
        Err(format!(
            "dev client {label} events: {events:?} (expected only the step's own entity delta \
             {:?}, capability 6 selected={entities})",
            step.world_spatial_delta
        )
        .into())
    }
}

async fn dev_client_door_events(
    session: &mut oteryn_dev_client::DevClientSession,
) -> TestResult<Vec<oteryn_dev_client::SessionEvent>> {
    for _ in 0..20 {
        session
            .service_liveness(std::time::Duration::from_millis(250))
            .await
            .map_err(|error| format!("dev client idle read: {error}"))?;
        let events = session.take_events();
        if !events.is_empty() {
            return Ok(events);
        }
    }
    Err("dev client door delta never arrived".into())
}

/// Durable session state, current loss epoch and grace seconds counted from that loss's decision.
async fn session_loss_row(url: &str, session: [u8; 16]) -> TestResult<Option<(i16, String, i64)>> {
    let mut connection = sqlx::PgConnection::connect(url).await?;
    let row: Option<(i16, Option<String>, Option<i64>)> = sqlx::query_as(
        "SELECT s.session_state, s.control_loss_epoch::text, \
         s.original_grace_deadline - r.decided_at \
         FROM game_durability_reconnect_sessions s \
         LEFT JOIN game_durability_admission_lifecycle_receipts r \
           ON r.operation_key = 'owning-loss-v1'::bytea || uuid_send(s.game_session_id) \
              || int8send(s.control_loss_epoch::bigint) \
         WHERE s.game_session_id = encode($1, 'hex')::uuid",
    )
    .bind(session.as_slice())
    .fetch_optional(&mut connection)
    .await?;
    connection.close().await?;
    Ok(row.map(|(state, epoch, grace)| (state, epoch.unwrap_or_default(), grace.unwrap_or(-1))))
}

/// Owned account for a grant built inside a closure.
struct OwnedGrant<'a> {
    inner: Grant<'a>,
    account: String,
}

impl<'a> Grant<'a> {
    fn with_account(self, account: &str) -> OwnedGrant<'a> {
        OwnedGrant {
            inner: self,
            account: account.to_owned(),
        }
    }
}

impl<'a> OwnedGrant<'a> {
    fn borrowed(&self) -> Grant<'_> {
        Grant {
            signing: self.inner.signing,
            key_id: self.inner.key_id,
            account_id: &self.account,
            character_id: self.inner.character_id,
            world_id: self.inner.world_id,
            channel_id: self.inner.channel_id,
            security_generation: self.inner.security_generation,
            scope_generation: self.inner.scope_generation,
            nonce: self.inner.nonce,
            attempt: self.inner.attempt,
        }
    }
}

#[test]
fn seam_qualification_pins_are_closed() {
    assert_eq!(PLATFORM_SOURCE.len(), 40);
    assert!(PLATFORM_SOURCE.bytes().all(|byte| byte.is_ascii_hexdigit()));
    let _ = CharacterId::decode(&v7(1, 1));
}
