//! Server Seam physical qualification. Runs only inside the disposable WP5
//! topology (`tools/qualification/wp5_s3b`, seam target): the exact protected
//! Platform producer behind TLS 1.3 mTLS plus PostgreSQL 17.6. The owners are
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
    ChannelId, CharacterId, FoundationProtocolError, MessageType, NodeId, RuntimeScopeRefV1,
    WorldId, decode_wire_envelope, encode_protocol_error,
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

const SOURCE_AUTHORITY: &str = "platform";
const PLATFORM_SOURCE: &str = "9147bfd3a771762a6646fd87b9172cdb3a6c9a01";
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
    super::canonical_uuid(bytes)
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
    if version != "170006" {
        return Err(
            format!("the seam qualification requires PostgreSQL 17.6, found {version}").into(),
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

fn bootstrap(major: u64, profile: u64, character: &[u8; 16], material: &str) -> Vec<u8> {
    let mut payload = Vec::new();
    scalar(&mut payload, 1, major);
    scalar(&mut payload, 2, profile);
    scalar(&mut payload, 3, 1);
    bytes_field(&mut payload, 5, material.as_bytes());
    bytes_field(&mut payload, 6, character);
    bytes_field(&mut payload, 7, b"seam-qualification");
    envelope(1, 0, &payload)
}

/// Structurally valid FND-02 resume request. The qualification supplies only
/// unissued reconnect material; this transport must not mint resume authority.
fn resume_payload(session: &[u8; 16], material: &[u8]) -> Vec<u8> {
    let mut payload = Vec::new();
    bytes_field(&mut payload, 1, session);
    bytes_field(&mut payload, 2, material);
    scalar(&mut payload, 4, 1);
    scalar(&mut payload, 5, 1);
    scalar(&mut payload, 6, 1);
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
    let tcp = TcpStream::connect(address).await?;
    let mut stream = tokio::time::timeout(
        Duration::from_secs(30),
        connector.connect(ServerName::try_from("localhost")?, tcp),
    )
    .await
    .map_err(|_| "TLS connect did not complete")??;
    stream.write_all(raw).await?;
    stream.flush().await?;
    let mut output = Vec::new();
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

fn accepted_session(reply: &Reply) -> Option<[u8; 16]> {
    let Reply::Frames(frames) = reply else {
        return None;
    };
    let envelope = decode_wire_envelope(frames.first()?).ok()?;
    if envelope.message_type() != MessageType::ServerAccepted {
        return None;
    }
    // Field 1: canonical GameSessionId bytes.
    let payload = envelope.payload();
    (payload.get(..2)? == [0x0a, 16]).then(|| payload[2..18].try_into().ok())?
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
            })
            .await
        })
}

#[test]
#[ignore = "requires the disposable WP5 Platform + PostgreSQL 17.6 topology"]
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
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(seam_flow(&accounts, &key_id, &signing))
}

async fn seam_flow(accounts: &[String; 2], key_id: &str, signing: &SigningKey) -> TestResult {
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
        )
        .map_err(|e| format!("channel runtime: {e:?}"))?,
    );
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
            root: &root,
            character: &authority,
            holder: &holder,
            evidence: &fresh_evidence,
            world_id: world,
            channel_id: channel,
            runtime: &runtime,
        },
        &shutdown,
    );

    let clients = seam_clients(SeamClients {
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
    });
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
    // KAN-26: every committed fresh admission holds exactly one player actor;
    // refused, replayed and losing attempts leave no reservation behind; and
    // ordinary disconnect (every client has closed by now) removes nothing.
    let (committed_players, pending) = runtime.lock().await.player_slot_counts();
    if committed_players != 2 || pending != 0 || committed_admissions(&url).await? != 2 {
        return Err(
            format!("channel runtime committed={committed_players} pending={pending}").into(),
        );
    }
    evidence("channel_runtime committed_players=2 pending_reservations=0 disconnect_removed=0");
    evidence("shutdown=drained FORMAL_ADR0007_QA_TIER1_TIER2=NOT_EVALUATED");
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
}

/// The #823 `SEAM_PASS` client stages against one serving node.
async fn seam_clients(clients: SeamClients<'_>) -> TestResult {
    let SeamClients {
        address,
        certificate,
        signing,
        key_id,
        accounts,
        characters,
        world,
        channel,
        scope_generation,
        descriptor,
        url,
        runtime,
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

    // Positive control first: the exact profile completes TLS, so every
    // refusal below is the node's policy and never a broken trust setup.
    let tcp = TcpStream::connect(address).await?;
    tokio::time::timeout(
        Duration::from_secs(30),
        exact.connect(ServerName::try_from("localhost")?, tcp),
    )
    .await
    .map_err(|_| "exact TLS handshake did not complete")?
    .map_err(|error| format!("exact TLS handshake refused: {error}"))?;
    evidence("transport exact_profile_handshake=completed");

    evidence("stage=transport_negatives");
    // Transport negatives: nothing reaches the frame layer.
    let generation = platform_generation(descriptor, &accounts[0]).await?;
    let grant = next_grant(&accounts[0], characters[0], generation);
    let valid_frame = framed(&bootstrap(
        1,
        1,
        &characters[0],
        &sign_grant(&grant.borrowed(), now_seconds()?),
    ));
    for (label, profile, raw) in [
        (
            "tls12_exact_alpn",
            ClientProfile {
                tls12: true,
                alpn: Some(b"oteryn-game/1"),
            },
            valid_frame.clone(),
        ),
        (
            "wrong_alpn",
            ClientProfile {
                tls12: false,
                alpn: Some(b"wrong"),
            },
            valid_frame.clone(),
        ),
        (
            "missing_alpn",
            ClientProfile {
                tls12: false,
                alpn: None,
            },
            valid_frame.clone(),
        ),
    ] {
        let reply = exchange(address, &connector(certificate, &profile)?, &raw).await?;
        if !matches!(reply, Reply::TlsRefused | Reply::Closed) {
            return Err(format!("{label} reached the frame layer: {reply:?}").into());
        }
    }
    let mut plaintext = TcpStream::connect(address).await?;
    plaintext.write_all(&valid_frame).await?;
    let mut output = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(20), plaintext.read_to_end(&mut output)).await;
    // The TLS layer may answer with an alert record (content type 0x15);
    // nothing else, and never a Foundation frame, may come back.
    if output
        .first()
        .is_some_and(|content_type| *content_type != 0x15)
    {
        return Err(format!("plaintext received a non-alert reply: {output:02x?}").into());
    }
    if committed_admissions(url).await? != 0 {
        return Err("a transport negative committed an admission".into());
    }
    evidence(
        "transport tls12_exact_alpn=refused wrong_alpn=refused missing_alpn=refused plaintext=refused admissions=0",
    );

    evidence("stage=foundation_negatives");
    // Foundation negatives through the real listener, before FND-04.
    let token = sign_grant(&grant.borrowed(), now_seconds()?);
    let malformed_session = v7(1, 0x09);
    let mut malformed_resume = resume_payload(&malformed_session, b"unissued-reconnect-proof");
    // All other fields are canonical; only this singular session identity repeats.
    bytes_field(&mut malformed_resume, 1, &malformed_session);
    let reply =
        exchange_must_close(address, &exact, &framed(&envelope(3, 0, &malformed_resume))).await?;
    if reply
        != Reply::Frames(vec![encode_protocol_error(
            FoundationProtocolError::MalformedEnvelope,
            0,
        )?])
        || committed_admissions(url).await? != 0
    {
        return Err(format!("malformed ClientResume admitted or misreported: {reply:?}").into());
    }
    for (label, raw, error) in [
        (
            "wrong_protocol_major",
            framed(&bootstrap(2, 1, &characters[0], &token)),
            Some(FoundationProtocolError::ProtocolMajorMismatch),
        ),
        (
            "wrong_transport_profile",
            framed(&bootstrap(1, 2, &characters[0], &token)),
            Some(FoundationProtocolError::TransportProfileMismatch),
        ),
        (
            "client_command_before_admission",
            framed(&envelope(7, 0, &[])),
            None,
        ),
        ("oversized_frame", 1_048_577u32.to_be_bytes().to_vec(), None),
        ("truncated_frame", vec![0, 0, 0, 9, 1, 2], None),
    ] {
        let reply = exchange(address, &exact, &raw).await?;
        if let Some(error) = error {
            if reply != Reply::Frames(vec![encode_protocol_error(error, 0)?]) {
                return Err(format!("{label}: {reply:?}").into());
            }
        } else if accepted_session(&reply).is_some() {
            return Err(format!("{label} was admitted").into());
        }
    }
    if committed_admissions(url).await? != 0 {
        return Err("a Foundation negative committed an admission".into());
    }
    evidence(
        "foundation wrong_protocol_major=rejected wrong_transport_profile=rejected malformed_resume=protocol_error phase_invalid=rejected oversized=closed truncated=closed admissions=0",
    );

    evidence("stage=fnd04_negatives");
    // FND-04 negatives: one invariant each, every other fact valid.
    let mut tampered = sign_grant(&grant.borrowed(), now_seconds()?);
    let last = tampered.pop().ok_or("empty token")?;
    tampered.push(if last == 'A' { 'B' } else { 'A' });
    let expired = sign_grant(&grant.borrowed(), now_seconds()? - 60);
    let other_character = framed(&bootstrap(
        1,
        1,
        &characters[1],
        &sign_grant(&grant.borrowed(), now_seconds()?),
    ));
    let mut foreign_key = grant.borrowed();
    let foreign = SigningKey::from_bytes(&[0x7e; 32]);
    foreign_key.signing = &foreign;
    for (label, raw) in [
        (
            "invalid_signature",
            framed(&bootstrap(1, 1, &characters[0], &tampered)),
        ),
        (
            "expired",
            framed(&bootstrap(1, 1, &characters[0], &expired)),
        ),
        ("wrong_character_binding", other_character),
        (
            "untrusted_signer",
            framed(&bootstrap(
                1,
                1,
                &characters[0],
                &sign_grant(&foreign_key, now_seconds()?),
            )),
        ),
    ] {
        let reply = exchange(address, &exact, &raw).await?;
        if reply != Reply::Closed {
            return Err(format!("{label}: {reply:?}").into());
        }
    }
    if committed_admissions(url).await? != 0 {
        return Err("an FND-04 negative committed an admission".into());
    }
    evidence(
        "fnd04 invalid_signature=refused expired=refused wrong_character_binding=refused untrusted_signer=refused admissions=0",
    );

    evidence("stage=admission");
    // Positive: the real owners admit on evidence fetched by this attempt
    // (the pre-seeded S2 observations are older than five seconds);
    // post-admission input fails closed.
    let admitted_token = sign_grant(&grant.borrowed(), now_seconds()?);
    let mut raw = framed(&bootstrap(1, 1, &characters[0], &admitted_token));
    raw.extend_from_slice(&framed(&envelope(7, 1, &[])));
    let reply = exchange_must_close(address, &exact, &raw).await?;
    let session =
        accepted_session(&reply).ok_or_else(|| format!("admission refused: {reply:?}"))?;
    let Reply::Frames(frames) = &reply else {
        return Err("missing frames".into());
    };
    if frames.get(1)
        != Some(&encode_protocol_error(
            FoundationProtocolError::UnknownMessageType,
            1,
        )?)
    {
        return Err(format!("post-admission input not closed: {reply:?}").into());
    }
    if session[6] >> 4 != 7 || committed_admissions(url).await? != 1 {
        return Err("admission did not commit exactly one GameSession".into());
    }
    evidence(
        "admission=committed server_accepted=1 post_admission_command=closed_unknown_message admissions=1",
    );

    // The fresh GameSession is durably committed and its first TLS socket has
    // ended. A new TLS connection's valid ClientResume must fail closed:
    // no ServerResumeAccepted, no replacement, no additional admission.
    let before = if let Some(runtime) = runtime {
        let counts = runtime.lock().await.player_slot_counts();
        if counts != (1, 0) {
            return Err(format!("fresh admission did not retain one actor: {counts:?}").into());
        }
        Some(counts)
    } else {
        None
    };
    let reply = exchange_must_close(
        address,
        &exact,
        &framed(&resume(&session, b"unissued-reconnect-proof")),
    )
    .await?;
    let after = if let Some(runtime) = runtime {
        Some(runtime.lock().await.player_slot_counts())
    } else {
        None
    };
    if reply != Reply::Closed || committed_admissions(url).await? != 1 || after != before {
        return Err(
            format!("valid ClientResume acquired authority: {reply:?} actors={after:?}").into(),
        );
    }
    evidence(
        "resume valid_after_committed_socket_close=refused server_resume_accepted=0 admissions=1",
    );
    if after.is_some() {
        evidence("resume committed_players=1 pending_reservations=0");
    }

    // Replay of the consumed grant on a fresh connection.
    let reply = exchange(
        address,
        &exact,
        &framed(&bootstrap(1, 1, &characters[0], &admitted_token)),
    )
    .await?;
    if reply != Reply::Closed || committed_admissions(url).await? != 1 {
        return Err(format!("replayed grant admitted: {reply:?}").into());
    }
    evidence("replayed_grant=refused admissions=1");

    // Concurrent use of one valid grant: at most one GameSession.
    let generation = platform_generation(descriptor, &accounts[1]).await?;
    let second = next_grant(&accounts[1], characters[1], generation);
    let token = sign_grant(&second.borrowed(), now_seconds()?);
    let raw = framed(&bootstrap(1, 1, &characters[1], &token));
    let (left, right) = join(
        exchange(address, &exact, &raw),
        exchange(address, &exact, &raw),
    )
    .await;
    let accepted = [left?, right?]
        .iter()
        .filter(|reply| accepted_session(reply).is_some())
        .count();
    if accepted != 1 || committed_admissions(url).await? != 2 {
        return Err(format!("concurrent same-grant admission accepted {accepted}").into());
    }
    evidence("concurrent_same_grant accepted=1 admissions=2");
    Ok(())
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
