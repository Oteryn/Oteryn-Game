// WP5 S3-B: sealed composition of the real protected owners.
//
// Runs only inside the disposable S3-B topology (`tools/qualification/wp5_s3b`):
// the exact protected Platform native-evidence producer behind TLS 1.3 mTLS
// (S3-A topology) plus PostgreSQL 17.6 for the Game owners. Every admission
// fact comes from a real owner:
// - Platform account security and fresh signing trust: exact S1 transport
//   responses, stored byte-for-byte in S2 under the #415 custody fence;
// - Character ownership/world: #414 Character Authority, bootstrapped from a
//   real Platform `CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1` decision issued
//   by the Platform operator command and read over the purpose-separated mTLS
//   reconciliation endpoint;
// - Channel ownership/readiness: #415 assignment plus holder-attested readiness;
// - durable admission: the existing FreshAdmissionStore commit.
// The fresh grant is signed with an ephemeral key whose public half the harness
// published into the Platform trust registry; Platform grant issuance itself is
// not part of the pinned Platform source.
#![allow(dead_code)]
extern crate self as oteryn_game_server;
#[path = "../src/admission_evidence.rs"]
pub mod admission_evidence;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_bootstrap_intent.rs"]
pub mod character_bootstrap_intent;
#[allow(dead_code, unused_imports)]
#[path = "../src/character_recovery_fence.rs"]
pub mod character_recovery_fence;
#[allow(dead_code, unused_imports)]
#[path = "../src/domain/mod.rs"]
pub mod domain;
#[allow(dead_code, unused_imports)]
#[path = "../src/durability/mod.rs"]
mod durability;
#[allow(dead_code, unused_imports)]
#[path = "../src/foundation/mod.rs"]
pub mod foundation;
#[path = "../src/native_admission_source/mod.rs"]
pub mod native_admission_source;

use admission_evidence::{Facts, Request, Response, decode_response, encode_request};
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use character_bootstrap_intent::read_authenticated_intent;
use character_recovery_fence::CharacterRecoveryStore;
use durability::DurabilityRoot;
use durability::admission_authority_guards::GuardPublicationDisposition;
use durability::fresh_admission::FreshAdmissionStore;
use durability::fresh_admission_composition::{FreshAdmissionComposition, FreshAdmissionSubject};
use durability::native_admission_source::{
    DescriptorRegistration, FreshStoreProvenance, NativeSourceOperation, NativeSourceSubject,
    SourceObservation,
};
use durability::recovery_evidence_composition::RecoveryEvidenceSubject;
use durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, BootstrapSecret, ControlActor,
    LaunchBinding, NodeIncarnationProof, OperationKey, RuntimeScopeAssignmentWriter,
};
use ed25519_dalek::{Signer, SigningKey};
use foundation::admission_authority_publication::{
    AdmissionAuthorityGuardKeyV1, AdmissionAuthorityGuardStateV1,
    AdmissionAuthorityOwningPublisherV1, AdmissionAuthorityPublicationChangeV1,
    AdmissionAuthorityPublicationErrorV1, AdmissionAuthorityPublicationV1,
    AdmissionPublicationPreconditionV1, AdmissionPublicationPurposeV1,
    AdmissionPublicationSourceV1, FreshAdmissionClaimTransitionV1,
};
use foundation::fnd04_verifier::{
    FreshDurabilityCurrentAuthorityV1, FreshDurabilityTrustContext, RecoveryCurrentEvidence,
    verify_fresh_grant_durability_v1,
};
use foundation::fresh_admission_durability::{
    FreshAdmissionCommitAuthorizationV1, FreshAdmissionCommitRequestV1,
    FreshAdmissionDurabilityFlowV1, FreshAdmissionDurabilityPortV1, FreshAdmissionDurableOutcomeV1,
    FreshAdmissionOperationV1, FreshAdmissionSubmissionV1,
};
use foundation::{
    AuthenticatedTransportRefV1, ChannelId, CharacterId, GameSessionId, NodeId, RuntimeScopeRefV1,
    WorldId,
};
use native_admission_source::{TransientCapacity, descriptor::ProducerDescriptor};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use sqlx::{Connection, Executor};
use std::time::{SystemTime, UNIX_EPOCH};
use std::{env, fs, io};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const SOURCE_AUTHORITY: &str = "platform";
const PLATFORM_SOURCE: &str = "9147bfd3a771762a6646fd87b9172cdb3a6c9a01";
/// Game-owned interpretation configured by the operator procedure; the Platform
/// intents issued by `run.sh` request exactly these revisions.
const INTERPRETATION: [&str; 4] = [
    "s3b-profile-1",
    "s3b-ruleset-1",
    "s3b-content-1",
    "s3b-starter-1",
];
const RUNTIME_AUTHORITY: &str = "game:wp5-s3b-runtime";

fn required(name: &str) -> Result<String, io::Error> {
    env::var(name).map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, name))
}

fn evidence(line: &str) {
    println!("S3B_EVIDENCE {line}");
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

fn descriptor() -> TestResult<ProducerDescriptor> {
    producer_descriptor(
        SOURCE_AUTHORITY,
        "WP5_S3A_CLIENT_CERT",
        "WP5_S3A_CLIENT_KEY",
    )
}

/// Purpose-separated Character bootstrap-intent issuer descriptor.
fn intent_descriptor(cert: &str, key: &str) -> TestResult<ProducerDescriptor> {
    producer_descriptor(
        native_admission_source::CHARACTER_BOOTSTRAP_INTENT_ISSUER,
        cert,
        key,
    )
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

fn uuid_text(bytes: &[u8; 16]) -> String {
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

fn now_seconds() -> TestResult<i64> {
    Ok(i64::try_from(
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
    )?)
}

/// One exact S1 exchange; returns the decoded observation and the exact body.
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
        Request::Account { recovery: true, .. } => Operation::ReadRecoveryAccountSecurityV2,
        Request::Trust { recovery: true, .. } => Operation::ReadRecoverySigningTrustV2,
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

/// Fetch the fresh account and trust observations from the real Platform and
/// accept their exact bodies into S2 under the custody fence.
async fn ingest(
    root: &DurabilityRoot,
    custody: &NodeIncarnationProof,
    descriptor: &ProducerDescriptor,
    account_id: &str,
    key_id: &str,
) -> TestResult<(u64, i64)> {
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
    Ok((
        minimum_valid_generation,
        account.source_observed_at.max(trust.source_observed_at),
    ))
}

/// Real mTLS Recovery V2 -> acknowledged S2 -> locked sealed V2 verifier.
/// The expected credential bindings are inert synthetic fixture context.
async fn qualify_registered_recovery(
    root: &DurabilityRoot,
    custody: &NodeIncarnationProof,
    descriptor: &ProducerDescriptor,
    account_id: &str,
    database_url: &str,
) -> TestResult {
    let before = authority_snapshot(database_url).await?;
    let key_id = required("WP5_S3B_RECOVERY_KEY_ID")?;
    let signing = SigningKey::from_bytes(&hex32(&required("WP5_S3B_RECOVERY_KEY_SEED")?)?);
    let public_key = hex32(&required("WP5_S3B_RECOVERY_PUBLIC_KEY")?)?;
    if signing.verifying_key().to_bytes() != public_key {
        return Err("ephemeral Recovery signing key mismatch".into());
    }
    let (account, account_raw) = fetch_real(
        descriptor,
        Request::Account {
            recovery: true,
            account_id,
            purpose: "platform_security",
            scope: "existing_actor_recovery",
        },
    )
    .await?;
    let (trust, trust_raw) = fetch_real(
        descriptor,
        Request::Trust {
            recovery: true,
            key_id: &key_id,
            key_purpose: "existing_actor_recovery",
        },
    )
    .await?;
    let Facts::Account {
        allowed: true,
        minimum_valid_generation,
    } = account.facts
    else {
        return Err("real Recovery account is not allowed".into());
    };
    if trust.facts
        != (Facts::Trust {
            trusted: true,
            public_key,
        })
    {
        return Err("real Recovery fixed-purpose key mismatch".into());
    }
    for (operation, subject, observation, raw) in [
        (
            NativeSourceOperation::ReadRecoveryAccountSecurityV2,
            NativeSourceSubject::account_security(account_id)?,
            account,
            account_raw,
        ),
        (
            NativeSourceOperation::ReadRecoverySigningTrustV2,
            NativeSourceSubject::signing_trust(
                "urn:oteryn:platform:game-recovery",
                "oteryn-reauth-recovery-v1",
                "existing_actor_recovery",
                &key_id,
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
    let current = RecoveryCurrentEvidence {
        account_id: account_id.into(),
        character_id: CharacterId::decode(&v7(7, 2)).map_err(|e| format!("{e:?}"))?,
        world_id: WorldId::decode(&v7(1, 2)).map_err(|e| format!("{e:?}"))?,
        ruleset_revision: "rules-s3b-1".into(),
        content_revision: "content-s3b-1".into(),
        map_revision: "map-s3b-1".into(),
        world_policy_revision: "policy-s3b-1".into(),
    };
    let now = now_seconds()?;
    let header = serde_json::json!({"alg":"Ed25519","kid":key_id,"typ":"oteryn-recovery+jwt"});
    let payload = serde_json::json!({
        "iss":"urn:oteryn:platform:game-recovery","aud":"urn:oteryn:game:recovery",
        "iat":now,"nbf":now,"exp":now+10,"jti":URL_SAFE_NO_PAD.encode([7;32]),
        "profile":"oteryn-reauth-recovery-v1","purpose":"existing_actor_recovery",
        "attempt_ref":uuid_text(&v7(7,4)),"account_id":account_id,
        "character_id":uuid_text(current.character_id.as_bytes()),"world_id":uuid_text(current.world_id.as_bytes()),
        "account_security_generation":minimum_valid_generation.to_string(),"protocol_major":1,"transport_profile":1,
        "ruleset_revision":"rules-s3b-1","content_revision":"content-s3b-1","map_revision":"map-s3b-1","world_policy_revision":"policy-s3b-1"
    });
    let input = format!(
        "{}.{}",
        URL_SAFE_NO_PAD.encode(header.to_string()),
        URL_SAFE_NO_PAD.encode(payload.to_string())
    );
    let token = format!(
        "{input}.{}",
        URL_SAFE_NO_PAD.encode(signing.sign(input.as_bytes()).to_bytes())
    );
    let subject = RecoveryEvidenceSubject::new(account_id, &key_id)?;
    let verified = root
        .verify_registered_recovery(custody, &subject, &token, &current)
        .await?
        .map_err(|e| format!("Recovery verify: {e:?}"))?;
    if verified.security().provenance.source_revision != account.source_revision
        || verified.signing().provenance.source_revision != trust.source_revision
        || verified.security().provenance.source_observed_at != account.source_observed_at
        || verified.signing().provenance.source_observed_at != trust.source_observed_at
        || verified.security().provenance.publication_revision != account.source_revision
        || verified.signing().provenance.publication_revision != trust.source_revision
    {
        return Err("Recovery source provenance changed at sealed projection".into());
    }
    root.revalidate_registered_recovery(custody, &verified, &current)
        .await?
        .map_err(|e| format!("Recovery revalidate: {e:?}"))?;
    if authority_snapshot(database_url).await? != before {
        return Err(
            "Recovery credential check changed existing authority or publication slot state".into(),
        );
    }
    evidence(
        "recovery_v2=real_mtls_s2_sealed_verify_revalidate source_time=preserved publication=scoped_ordinal_projection authority_mutations=0",
    );
    Ok(())
}

async fn authority_snapshot(database_url: &str) -> TestResult<String> {
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    // Fixed qualification tables: session/controller, account/Character leases,
    // persisted grace/protection and a deliberately occupied source slot.
    let snapshot = sqlx::query_scalar(
        "SELECT jsonb_build_object( \
         'sessions',(SELECT coalesce(jsonb_agg(to_jsonb(s) ORDER BY game_session_id),'[]'::jsonb) FROM game_durability_reconnect_sessions s), \
         'accounts',(SELECT coalesce(jsonb_agg(to_jsonb(a) ORDER BY account_id),'[]'::jsonb) FROM game_durability_admission_account_guards a), \
         'characters',(SELECT coalesce(jsonb_agg(to_jsonb(c) ORDER BY character_id),'[]'::jsonb) FROM game_durability_admission_character_guards c), \
         'continuity',(SELECT coalesce(jsonb_agg(to_jsonb(p) ORDER BY character_id,control_loss_epoch),'[]'::jsonb) FROM game_durability_control_loss_continuity p), \
         'source_slots',(SELECT coalesce(jsonb_agg(to_jsonb(x) ORDER BY slot_id),'[]'::jsonb) FROM game_durability_native_source_publication_slots x))::text",
    ).fetch_one(&mut connection).await?;
    connection.close().await?;
    Ok(snapshot)
}

// Runtime readiness producer for the holder: the ownership generation is the
// #415 assignment generation; the revision facts are the runtime's own.
struct RuntimeReadiness(AdmissionAuthorityPublicationChangeV1);
impl foundation::fnd04_verifier::fresh_source_sealed::Sealed for RuntimeReadiness {}
impl AdmissionAuthorityOwningPublisherV1 for RuntimeReadiness {
    fn resolve_publication(
        &self,
        _now: i64,
    ) -> Result<Vec<AdmissionAuthorityPublicationChangeV1>, AdmissionAuthorityPublicationErrorV1>
    {
        Ok(vec![self.0.clone()])
    }
}

async fn publish_readiness(
    root: &DurabilityRoot,
    holder: &NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    generation: u64,
    now: i64,
) -> TestResult {
    let store =
        durability::admission_authority_guards::AdmissionGuardStore::from_root(root.clone());
    let current = store
        .load(&[AdmissionAuthorityGuardKeyV1::Runtime(scope)])
        .await?
        .pop()
        .flatten();
    let (precondition, publication_revision, source_revision) = match &current {
        None => (
            AdmissionPublicationPreconditionV1::Bootstrap {
                restored_publication_high_water: Some(0),
            },
            1,
            1,
        ),
        Some(prior) => (
            AdmissionPublicationPreconditionV1::CompareAndSet {
                expected_publication_revision: prior.publication_revision,
            },
            prior.publication_revision + 1,
            prior.source.source_revision + 1,
        ),
    };
    let change = AdmissionAuthorityPublicationChangeV1 {
        key: AdmissionAuthorityGuardKeyV1::Runtime(scope),
        source: AdmissionPublicationSourceV1 {
            authority: current.as_ref().map_or(RUNTIME_AUTHORITY.into(), |prior| {
                prior.source.authority.clone()
            }),
            purpose: AdmissionPublicationPurposeV1::RuntimeOwnershipAndReadiness,
            source_revision,
            decision_identity: format!("runtime-ready:{source_revision}"),
            source_observed_at: now,
            clock_uncertainty_seconds: 0,
        },
        precondition,
        publication_revision,
        state: AdmissionAuthorityGuardStateV1::Runtime {
            ownership_generation: generation,
            ready: true,
            route_revision: "route-s3b-1".into(),
            runtime_observation_revision: format!("runtime-{source_revision}"),
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

struct Capture(Option<FreshAdmissionCommitRequestV1>);
impl FreshAdmissionDurabilityPortV1 for Capture {
    fn submit(&mut self, request: &FreshAdmissionCommitRequestV1) -> FreshAdmissionSubmissionV1 {
        if self.0.is_some() {
            return FreshAdmissionSubmissionV1::Unavailable;
        }
        self.0 = Some(request.clone());
        FreshAdmissionSubmissionV1::Accepted
    }
    fn reconcile(&mut self, _: &FreshAdmissionOperationV1) -> FreshAdmissionSubmissionV1 {
        FreshAdmissionSubmissionV1::Unavailable
    }
}

struct Grant<'a> {
    signing: &'a SigningKey,
    key_id: &'a str,
    subject: &'a FreshAdmissionSubject,
    security_generation: u64,
    scope_generation: u64,
    nonce: [u8; 32],
    attempt: [u8; 16],
}

/// Fresh grant as the Platform issuer would sign it (ephemeral key).
fn grant(grant: &Grant<'_>, now: i64) -> String {
    let header = format!(
        r#"{{"alg":"Ed25519","kid":"{}","typ":"oteryn-admission+jwt"}}"#,
        grant.key_id
    );
    let payload = serde_json::json!({
        "iss": "urn:oteryn:platform:game-admission",
        "aud": "urn:oteryn:game:admission",
        "iat": now,
        "nbf": now,
        "exp": now + 10,
        "jti": URL_SAFE_NO_PAD.encode(grant.nonce),
        "profile": "oteryn-pre-admission-v1",
        "purpose": "fresh_entry",
        "attempt_ref": uuid_text(&grant.attempt),
        "account_id": grant.subject.account_id,
        "character_id": uuid_text(grant.subject.character_id.as_bytes()),
        "world_id": uuid_text(grant.subject.world_id.as_bytes()),
        "channel_id": uuid_text(grant.subject.channel_id.as_bytes()),
        "account_security_generation": grant.security_generation.to_string(),
        "route_revision": "route-s3b-1",
        "runtime_observation_revision": "RUNTIME_OBSERVATION",
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

/// Verify the grant against the sealed composition and build the durable commit.
fn admission_request(
    composition: &FreshAdmissionComposition,
    token: &str,
    session: GameSessionId,
    transport: AuthenticatedTransportRefV1,
    now: i64,
) -> TestResult<FreshAdmissionCommitRequestV1> {
    let facts = verify_fresh_grant_durability_v1(
        token,
        now,
        &FreshDurabilityTrustContext::from_owning_source(composition),
        &FreshDurabilityCurrentAuthorityV1::from_owning_source(composition),
    )
    .map_err(|error| format!("grant verification: {error:?}"))?;
    let authorization =
        FreshAdmissionCommitAuthorizationV1::new(&facts, session, transport, composition, now)
            .map_err(|error| format!("authorization: {error:?}"))?;
    let transition = FreshAdmissionClaimTransitionV1::prepare(composition, &authorization, now)
        .map_err(|error| format!("claim: {error:?}"))?;
    let mut flow = FreshAdmissionDurabilityFlowV1::begin(authorization, transition)
        .map_err(|error| format!("flow: {error:?}"))?;
    let mut capture = Capture(None);
    flow.submit(&mut capture)
        .map_err(|error| format!("submit: {error:?}"))?;
    capture.0.ok_or_else(|| "flow did not submit".into())
}

async fn database() -> TestResult<(String, String)> {
    let admin_url = required("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
    if !admin_url.starts_with("postgresql://oteryn_test_admin:")
        || !admin_url.ends_with("@127.0.0.1:5432/postgres")
    {
        return Err("unsafe PostgreSQL test admin URL".into());
    }
    let name = format!(
        "s3b_{}",
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
        return Err(format!("S3-B requires PostgreSQL 17.6, found {version}").into());
    }
    sqlx::migrate!("./migrations").run(&mut connection).await?;
    connection.close().await?;
    Ok((url, version))
}

async fn register(
    root: &DurabilityRoot,
    tag: u8,
    supersedes: Option<NodeId>,
) -> TestResult<NodeIncarnationProof> {
    let secret = BootstrapSecret::from_bytes([tag; 32]);
    let launch = LaunchBinding::new(&format!("s3b-launch-{tag}")).map_err(|e| format!("{e:?}"))?;
    root.issue_node_bootstrap_authorization(&secret, &launch, supersedes)
        .await
        .map_err(|e| format!("bootstrap authorization {tag}: {e:?}"))?;
    Ok(root
        .register_node_incarnation(&secret, &launch, NodeId::decode(&v7(tag, 0x01))?)
        .await
        .map_err(|e| format!("register {tag}: {e:?}"))?)
}

fn assignment(tag: u8, command: AssignmentCommand) -> TestResult<AssignmentRequest> {
    Ok(AssignmentRequest {
        operation_key: OperationKey::from_bytes([tag; 32]),
        // The recorded actor is the authenticated session role (OPS-NODE-BOOT-01 D2).
        actor: ControlActor::new("oteryn_test_admin").map_err(|e| format!("{e:?}"))?,
        command,
    })
}

#[test]
#[ignore = "requires the disposable S3-B Platform + PostgreSQL 17.6 topology"]
fn real_owners_compose_fresh_admission_and_fence_replacement() -> TestResult {
    let descriptor = descriptor()?;
    let accounts = [
        required("WP5_S3A_ACCOUNT_ID")?,
        required("WP5_S3B_SECOND_ACCOUNT_ID")?,
    ];
    let key_id = required("WP5_S3B_FRESH_KEY_ID")?;
    let signing = SigningKey::from_bytes(&hex32(&required("WP5_S3B_FRESH_KEY_SEED")?)?);
    let published = hex32(&required("WP5_S3B_FRESH_PUBLIC_KEY")?)?;
    if signing.verifying_key().to_bytes() != published {
        return Err("seed does not match the published fresh key".into());
    }
    composition_flow(&descriptor, &accounts, &key_id, &signing)
}

fn composition_flow(
    descriptor: &ProducerDescriptor,
    accounts: &[String; 2],
    key_id: &str,
    signing: &SigningKey,
) -> TestResult {
    let key_id = key_id.to_owned();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (url, version) = database().await?;
            evidence(&format!("postgres_server_version_num={version} platform={PLATFORM_SOURCE}"));
            let root = DurabilityRoot::connect_test_runtime(&url)?;
            if !root.maintain_ready_once().await? {
                return Err("durability root not ready".into());
            }
            let world = WorldId::decode(&v7(1, 0x02))?;
            let channel = ChannelId::decode(&v7(1, 0x03))?;
            let scope = RuntimeScopeRefV1::channel(world, channel);

            // #415: GameNode A registers with one-launch bootstrap authorization.
            let node_a = register(&root, 0xa1, None).await?;
            // S2: A takes custody of the native source durable state.
            let now = now_seconds()?;
            let provenance = FreshStoreProvenance {
                namespace: "wp5-s3b".into(),
                authorization: "s3b-disposable-topology".into(),
                source_authority: SOURCE_AUTHORITY.into(),
                initialized_at: now,
            };
            let registration = DescriptorRegistration {
                revision: 1,
                facts: format!("platform@{PLATFORM_SOURCE};source.test;tls1.3-mtls").into_bytes(),
                installed_at: now,
            };
            // Control-plane issuance first (OPS-NODE-BOOT-01 D2).
            if !root
                .record_native_source_descriptor_issuance(SOURCE_AUTHORITY, registration.clone(), Some(provenance.clone()))
                .await?
            {
                return Err("S2 descriptor issuance refused".into());
            }
            root.initialize_native_admission_source(&node_a, provenance, registration)
                .await?;
            // Owner-written exact-scope grant for the test control session.
            let mut owner = sqlx::PgConnection::connect(&url).await?;
            sqlx::query(
                "INSERT INTO game_control_scope_grants (control_role, world_id, channel_id, operation) \
                 SELECT session_user, encode($1, 'hex')::uuid, encode($2, 'hex')::uuid, operation \
                 FROM generate_series(1, 3) AS operation",
            )
            .bind(v7(1, 0x02).as_slice())
            .bind(v7(1, 0x03).as_slice())
            .execute(&mut owner)
            .await?;
            owner.close().await?;
            // #415: Channel assignment to A; A publishes attested readiness.
            let writer = RuntimeScopeAssignmentWriter::open(root.clone(), "s3b-writer")
                .await
                .map_err(|e| format!("{e:?}"))?;
            let first = match writer
                .submit(&assignment(1, AssignmentCommand::Assign { scope, target: node_a.fact() })?)
                .await
            {
                Ok(AssignmentOutcome::Committed(receipt)) => receipt,
                other => return Err(format!("assign: {other:?}").into()),
            };
            publish_readiness(&root, &node_a, scope, first.assignment.ownership_generation, now_seconds()?).await?;
            evidence(&format!(
                "runtime_assignment=assigned holder=A generation={} readiness=attested",
                first.assignment.ownership_generation
            ));

            // #414: sealed recovery fence and reconciled Character authority.
            let retained = std::path::PathBuf::from(required("WP5_S3B_CHARACTER_FENCE_DIR")?);
            let recovery = CharacterRecoveryStore::open(&retained, "character-primary", "game-ops")
                .map_err(|e| format!("recovery store: {e:?}"))?;
            {
                let fresh = recovery
                    .authorize_fresh_store(v7(1, 0x07), u64::try_from(now_seconds()?)?)
                    .map_err(|e| format!("fresh recovery: {e:?}"))?;
                root.admit_fresh_character_recovery(&fresh)
                    .await
                    .map_err(|e| format!("fresh admission: {e:?}"))?;
            }
            let fence = recovery.seal_current().map_err(|e| format!("seal: {e:?}"))?;
            let authority = root
                .open_character_authority(&fence)
                .await
                .map_err(|e| format!("open authority: {e:?}"))?;
            // Game-owned interpretation through the operator-only procedure.
            let mut operator = sqlx::PgConnection::connect(&url).await?;
            let revision: i64 = sqlx::query_scalar(
                "SELECT game_character_configure_interpretation($1, $2, $3, $4)",
            )
            .bind(INTERPRETATION[0])
            .bind(INTERPRETATION[1])
            .bind(INTERPRETATION[2])
            .bind(INTERPRETATION[3])
            .fetch_one(&mut operator)
            .await?;
            operator.close().await?;
            evidence(&format!("character_interpretation=configured revision={revision} path=operator_procedure"));

            // The intent endpoint is purpose-separated: the native-evidence
            // client identity is refused, a native-evidence descriptor cannot
            // read intents and an unknown operation is bounded unavailable.
            let intents = intent_descriptor("WP5_S3B_INTENT_CLIENT_CERT", "WP5_S3B_INTENT_CLIENT_KEY")?;
            let wrong_client = intent_descriptor("WP5_S3A_CLIENT_CERT", "WP5_S3A_CLIENT_KEY")?;
            let capacity = TransientCapacity::new();
            for (label, reader, operation) in [
                ("wrong_client_identity", &wrong_client, v7(1, 0x04)),
                ("native_evidence_descriptor", descriptor, v7(1, 0x04)),
                ("unknown_operation", &intents, v7(9, 0x04)),
            ] {
                let mut permit = capacity.try_queue()?;
                permit.try_activate()?;
                if read_authenticated_intent(reader, operation, &mut permit).await.is_ok() {
                    return Err(format!("intent read accepted: {label}").into());
                }
            }
            evidence("intent_reads wrong_client_identity=refused native_evidence_descriptor=refused unknown_operation=unavailable");

            // #414: one Character per Platform AccountId, each from the real
            // operator-issued Platform intent, with current S2 account security.
            let mut characters = Vec::new();
            for (index, account) in accounts.iter().enumerate() {
                let tag = u8::try_from(index)? + 1;
                ingest(&root, &node_a, descriptor, account, &key_id).await?;
                let mut permit = capacity.try_queue()?;
                permit.try_activate()?;
                let intent = read_authenticated_intent(&intents, v7(tag, 0x04), &mut permit)
                    .await
                    .map_err(|e| format!("intent read {tag}: {e:?}"))?;
                drop(permit);
                if uuid_text(intent.account_id().as_bytes()) != *account {
                    return Err("Platform intent names another AccountId".into());
                }
                let record = root
                    .bootstrap_character(&authority, &node_a, &intent)
                    .await
                    .map_err(|e| format!("bootstrap character {tag}: {e:?}"))?;
                let replay = root
                    .bootstrap_character(&authority, &node_a, &intent)
                    .await
                    .map_err(|e| format!("replay character {tag}: {e:?}"))?;
                if replay != record {
                    return Err("exact intent replay changed the committed Character".into());
                }
                characters.push(CharacterId::decode(record.character_id.as_bytes())?);
            }
            evidence("character_authority=bootstrapped characters=2 intents=platform_operator_issued replay=exact audit=durable_outbox");

            // Admission 1: real Platform facts -> S2 -> guards -> sealed composition -> commit.
            let subject_one = FreshAdmissionSubject {
                account_id: accounts[0].clone(),
                character_id: characters[0],
                world_id: world,
                channel_id: channel,
                signing_key_id: key_id.clone(),
            };
            let (generation, observed) = ingest(&root, &node_a, descriptor, &accounts[0], &key_id).await?;
            let now = now_seconds()?.max(observed);
            root.publish_fresh_admission_sources(&authority, &node_a, &subject_one, now).await?;
            let composition = root.compose_fresh_admission(&authority, &node_a, &subject_one).await?;
            let mut token_grant = Grant {
                signing,
                key_id: &key_id,
                subject: &subject_one,
                security_generation: generation,
                scope_generation: first.assignment.ownership_generation,
                nonce: [0x31; 32],
                attempt: v7(1, 0x05),
            };
            let request = admission_request(
                &composition,
                &runtime_bound_grant(&root, &token_grant, scope, now).await?,
                GameSessionId::decode(&v7(1, 0x06))?,
                AuthenticatedTransportRefV1::decode(&[0x51; 16])?,
                now,
            )?;
            let store = FreshAdmissionStore::from_root(root.clone());
            match root.commit_composed_fresh_admission(&authority, &node_a, &composition, &request).await? {
                FreshAdmissionDurableOutcomeV1::Committed(_) => {}
                other => return Err(format!("admission 1 not committed: {other:?}").into()),
            }
            evidence("fresh_admission=committed account=1 holder=A sources=platform_s2,character_414,runtime_415");
            let slot_binding = vec![0x71];
            let occupied_slot = root.checkpoint_native_source_publication(&node_a,slot_binding.clone(),now_seconds()?).await?;
            qualify_registered_recovery(&root, &node_a, descriptor, &accounts[0], &url).await?;
            root.clear_native_source_publication(&node_a,occupied_slot,slot_binding).await?;

            // Prepare a second admission on A, then replace A before it commits.
            let subject_two = FreshAdmissionSubject {
                account_id: accounts[1].clone(),
                character_id: characters[1],
                ..subject_one.clone()
            };
            let (generation_two, observed) = ingest(&root, &node_a, descriptor, &accounts[1], &key_id).await?;
            let now = now_seconds()?.max(observed);
            root.publish_fresh_admission_sources(&authority, &node_a, &subject_two, now).await?;
            let stale = root.compose_fresh_admission(&authority, &node_a, &subject_two).await?;
            token_grant.subject = &subject_two;
            token_grant.security_generation = generation_two;
            token_grant.nonce = [0x32; 32];
            token_grant.attempt = v7(2, 0x05);
            let stale_request = admission_request(
                &stale,
                &runtime_bound_grant(&root, &token_grant, scope, now).await?,
                GameSessionId::decode(&v7(2, 0x06))?,
                AuthenticatedTransportRefV1::decode(&[0x52; 16])?,
                now,
            )?;

            // #415: B supersedes A; the Channel is replaced atomically with the readiness fence.
            let node_b = register(&root, 0xb2, Some(node_a.fact().node_id())).await?;
            let predecessor = root
                .read_runtime_scope_predecessor(scope)
                .await
                .map_err(|e| format!("{e:?}"))?
                .ok_or("missing predecessor")?;
            let second = match writer
                .submit(&assignment(2, AssignmentCommand::Replace { scope, predecessor, target: node_b.fact() })?)
                .await
            {
                Ok(AssignmentOutcome::Committed(receipt)) => receipt,
                other => return Err(format!("replace: {other:?}").into()),
            };
            match store.commit(&stale_request).await? {
                FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority => {}
                other => return Err(format!("stale composition committed: {other:?}").into()),
            }
            evidence("stale_composition_after_replacement=rejected_stale_authority");
            if root.compose_fresh_admission(&authority, &node_a, &subject_two).await.is_ok() {
                return Err("superseded A composed a fresh admission".into());
            }
            if ingest(&root, &node_a, descriptor, &accounts[1], &key_id).await.is_ok() {
                return Err("superseded A mutated S2 source state".into());
            }
            evidence("superseded_holder=fenced compose=refused s2_custody=refused");

            // B claims S2 custody, publishes readiness for the new generation and admits account 2.
            root.claim_native_admission_source_custody(&node_b).await?;
            publish_readiness(&root, &node_b, scope, second.assignment.ownership_generation, now_seconds()?).await?;
            let (generation_two, observed) = ingest(&root, &node_b, descriptor, &accounts[1], &key_id).await?;
            let now = now_seconds()?.max(observed);
            root.publish_fresh_admission_sources(&authority, &node_b, &subject_two, now).await?;
            let composition = root.compose_fresh_admission(&authority, &node_b, &subject_two).await?;
            token_grant.security_generation = generation_two;
            token_grant.scope_generation = second.assignment.ownership_generation;
            token_grant.nonce = [0x33; 32];
            token_grant.attempt = v7(3, 0x05);
            let request = admission_request(
                &composition,
                &runtime_bound_grant(&root, &token_grant, scope, now).await?,
                GameSessionId::decode(&v7(3, 0x06))?,
                AuthenticatedTransportRefV1::decode(&[0x53; 16])?,
                now,
            )?;
            // An owner change between composition and commit (a newer S2 floor)
            // is caught at the commit boundary: the prepared request is stale.
            ingest(&root, &node_b, descriptor, &accounts[1], &key_id).await?;
            match root.commit_composed_fresh_admission(&authority, &node_b, &composition, &request).await? {
                FreshAdmissionDurableOutcomeV1::RejectedStaleAuthority => {}
                other => return Err(format!("owner change not revalidated at commit: {other:?}").into()),
            }
            evidence("owner_change_between_compose_and_commit=rejected_stale_authority");
            let (generation_two, observed) = ingest(&root, &node_b, descriptor, &accounts[1], &key_id).await?;
            let now = now_seconds()?.max(observed);
            root.publish_fresh_admission_sources(&authority, &node_b, &subject_two, now).await?;
            let composition = root.compose_fresh_admission(&authority, &node_b, &subject_two).await?;
            token_grant.security_generation = generation_two;
            let request = admission_request(
                &composition,
                &runtime_bound_grant(&root, &token_grant, scope, now).await?,
                GameSessionId::decode(&v7(3, 0x06))?,
                AuthenticatedTransportRefV1::decode(&[0x53; 16])?,
                now,
            )?;
            match root.commit_composed_fresh_admission(&authority, &node_b, &composition, &request).await? {
                FreshAdmissionDurableOutcomeV1::Committed(_) => {}
                other => return Err(format!("admission 2 not committed: {other:?}").into()),
            }
            evidence(&format!(
                "fresh_admission=committed account=2 holder=B generation={} custody=claimed",
                second.assignment.ownership_generation
            ));
            evidence("composition=complete");
            Ok(())
        })
}

/// Sign the grant with the Runtime guard's current observation revision: the
/// grant must bind the exact runtime facts the holder published.
async fn runtime_bound_grant(
    root: &DurabilityRoot,
    grant_input: &Grant<'_>,
    scope: RuntimeScopeRefV1,
    now: i64,
) -> TestResult<String> {
    let store =
        durability::admission_authority_guards::AdmissionGuardStore::from_root(root.clone());
    let current = store
        .load(&[AdmissionAuthorityGuardKeyV1::Runtime(scope)])
        .await?
        .pop()
        .flatten()
        .ok_or("missing runtime guard")?;
    let AdmissionAuthorityGuardStateV1::Runtime {
        runtime_observation_revision,
        ..
    } = current.state
    else {
        return Err("invalid runtime guard".into());
    };
    let token = grant(grant_input, now);
    // Re-sign with the exact runtime observation revision.
    let (header, rest) = token.split_once('.').ok_or("token")?;
    let (payload, _) = rest.split_once('.').ok_or("token")?;
    let mut claims: serde_json::Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(payload)?)?;
    claims["runtime_observation_revision"] = runtime_observation_revision.into();
    let signing_input = format!("{header}.{}", URL_SAFE_NO_PAD.encode(claims.to_string()));
    let signature = URL_SAFE_NO_PAD.encode(
        grant_input
            .signing
            .sign(signing_input.as_bytes())
            .to_bytes(),
    );
    Ok(format!("{signing_input}.{signature}"))
}

#[test]
fn s3b_pins_are_closed() {
    assert_eq!(PLATFORM_SOURCE.len(), 40);
    assert_eq!(uuid_text(&v7(1, 1)).len(), 36);
    let _ = CharacterId::decode;
}
