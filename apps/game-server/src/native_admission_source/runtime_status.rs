//! `ReportRuntimeStatusV1` producer (`oteryn-game-native-runtime-status-v1` §3, §4, §8).
//!
//! The node projects its committed Runtime readiness publication to Platform
//! over its own purpose-bound mTLS identity. Delivery never gates boot,
//! serving, admission or shutdown: every failure is only "not delivered".
//! The report never carries an endpoint; the World Registry owns the route.

use super::{SourceError, TransientCapacity, descriptor::ProducerDescriptor, http1_mtls};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use std::{future::Future, sync::Arc, time::Duration};
use tokio::sync::watch;

/// `NRS-REPORT-BYTES`.
pub const REPORT_BYTES: usize = 2048;
/// `NRS-RESPONSE-BYTES`.
pub const RESPONSE_BYTES: usize = 256;
/// `NRS-HEARTBEAT` (H, U-RS1).
pub const HEARTBEAT: Duration = Duration::from_secs(5);
/// Compiled namespace of the runtime-status descriptor. It is never a
/// configured source authority, so no other purpose's descriptor matches it.
pub const PURPOSE: &str = "OTERYN_GAME_NATIVE_RUNTIME_STATUS";
/// Compiled `ReportRuntimeStatusV1` path (§3); not configurable.
pub const PATH: &str = "/internal/v1/game-auth/native-runtime-status";

/// The committed publication a report projects (§4); `observed_at` is added
/// per delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Publication {
    pub source_authority: String,
    pub world_id: String,
    pub channel_id: String,
    pub node_id: String,
    pub assignment_epoch: u64,
    pub scope_ownership_generation: u64,
    pub source_revision: u64,
    pub decision_identity: String,
    pub ready: bool,
    pub published_at: i64,
    pub protocol_major: u64,
    pub transport_profile: u64,
    pub route_revision: String,
    pub runtime_observation_revision: String,
    pub ruleset_revision: String,
    pub content_revision: String,
    pub map_revision: String,
    pub world_policy_revision: String,
    pub offer_revision: String,
}

#[derive(Serialize)]
struct Wire<'a> {
    contract_version: u8,
    operation: &'static str,
    source_authority: &'a str,
    world_id: &'a str,
    channel_id: &'a str,
    node_id: &'a str,
    assignment_epoch: String,
    scope_ownership_generation: String,
    source_revision: String,
    decision_identity: &'a str,
    ready: bool,
    published_at: String,
    observed_at: String,
    protocol_major: u64,
    transport_profile: u64,
    route_revision: &'a str,
    runtime_observation_revision: &'a str,
    ruleset_revision: &'a str,
    content_revision: &'a str,
    map_revision: &'a str,
    world_policy_revision: &'a str,
    offer_revision: &'a str,
}

fn charset(value: &str, maximum: usize, extra: &[u8]) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(&b) || extra.contains(&b))
}

fn canonical_uuid(value: &str, version: Option<u8>) -> bool {
    let b = value.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_digit() || (b'a'..=b'f').contains(c),
        })
        && version.is_none_or(|v| b[14] == v && matches!(b[19], b'8' | b'9' | b'a' | b'b'))
}

/// Exact §4 wire body. Refuses anything outside the contract grammar.
pub fn encode(p: &Publication, observed_at: i64) -> Result<String, SourceError> {
    let revisions = [
        &p.route_revision,
        &p.runtime_observation_revision,
        &p.ruleset_revision,
        &p.content_revision,
        &p.map_revision,
        &p.world_policy_revision,
        &p.offer_revision,
    ];
    let valid = charset(&p.source_authority, 128, b"/")
        && canonical_uuid(&p.world_id, Some(b'7'))
        && canonical_uuid(&p.channel_id, Some(b'7'))
        && canonical_uuid(&p.node_id, None)
        && p.assignment_epoch != 0
        && p.scope_ownership_generation != 0
        && p.source_revision != 0
        && charset(&p.decision_identity, 128, b"")
        && p.published_at >= 0
        && observed_at >= p.published_at
        && p.protocol_major == 1
        && p.transport_profile == 1
        && revisions.iter().all(|r| charset(r, 64, b""));
    if !valid {
        return Err(SourceError::InvalidInput);
    }
    let body = serde_json::to_string(&Wire {
        contract_version: 1,
        operation: "ReportRuntimeStatusV1",
        source_authority: &p.source_authority,
        world_id: &p.world_id,
        channel_id: &p.channel_id,
        node_id: &p.node_id,
        assignment_epoch: p.assignment_epoch.to_string(),
        scope_ownership_generation: p.scope_ownership_generation.to_string(),
        source_revision: p.source_revision.to_string(),
        decision_identity: &p.decision_identity,
        ready: p.ready,
        published_at: p.published_at.to_string(),
        observed_at: observed_at.to_string(),
        protocol_major: p.protocol_major,
        transport_profile: p.transport_profile,
        route_revision: &p.route_revision,
        runtime_observation_revision: &p.runtime_observation_revision,
        ruleset_revision: &p.ruleset_revision,
        content_revision: &p.content_revision,
        map_revision: &p.map_revision,
        world_policy_revision: &p.world_policy_revision,
        offer_revision: &p.offer_revision,
    })
    .map_err(|_| SourceError::InvalidInput)?;
    if body.len() > REPORT_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    Ok(body)
}

/// Platform's definite acknowledgement (§4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Delivery {
    Accepted,
    Refreshed,
    Superseded,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Ack {
    contract_version: u8,
    result: Delivery,
}

/// Exact success body; unknown, duplicate, missing or `null` members refuse.
pub fn decode_response(raw: &[u8]) -> Result<Delivery, SourceError> {
    if raw.len() > RESPONSE_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    match serde_json::from_slice::<Ack>(raw) {
        Ok(Ack {
            contract_version: 1,
            result,
        }) => Ok(result),
        _ => Err(SourceError::InvalidInput),
    }
}

/// Runtime-status client identity (§3): its own certificate and key, never
/// those of another purpose.
pub struct RuntimeStatusDescriptor(ProducerDescriptor);

impl RuntimeStatusDescriptor {
    /// `other_purposes` are the chains and keys of every other Platform
    /// identity this host holds; reusing any of them refuses.
    pub fn new(
        connect_endpoint: (String, u16),
        peer_name: String,
        roots: Vec<CertificateDer<'static>>,
        client_chain: Vec<CertificateDer<'static>>,
        client_key: PrivateKeyDer<'static>,
        other_purposes: &[(&[CertificateDer<'static>], &PrivateKeyDer<'static>)],
    ) -> Result<Self, SourceError> {
        let leaf = client_chain.first().ok_or(SourceError::InvalidDescriptor)?;
        if other_purposes.iter().any(|(chain, key)| {
            chain.first() == Some(leaf) || key.secret_der() == client_key.secret_der()
        }) {
            return Err(SourceError::InvalidDescriptor);
        }
        ProducerDescriptor::new(
            PURPOSE.into(),
            connect_endpoint,
            peer_name.clone(),
            peer_name,
            roots,
            client_chain,
            client_key,
        )
        .map(Self)
    }
}

/// One bounded `ReportRuntimeStatusV1` exchange on the reporter's own
/// capacity (never an admission slot). Any non-200 is `Unavailable`.
pub async fn deliver(
    descriptor: &RuntimeStatusDescriptor,
    capacity: &TransientCapacity,
    publication: &Publication,
    observed_at: i64,
) -> Result<Delivery, SourceError> {
    if descriptor.0.source_authority() != PURPOSE {
        return Err(SourceError::InvalidDescriptor);
    }
    let body = encode(publication, observed_at)?;
    let mut permit = capacity.try_queue()?;
    permit.try_activate()?;
    let raw =
        http1_mtls::exchange_at(&descriptor.0, PATH, REPORT_BYTES, &body, &mut permit).await?;
    decode_response(&raw)
}

/// Report loop (§8). Each new committed publication is sent once; while it is
/// `ready = true` and `gate` holds, it is repeated every `heartbeat` with a new
/// `observed_at`. One report is in flight and the watch keeps only the latest
/// publication. The loop ends when the sender is dropped, after sending any
/// publication it has not yet seen.
pub async fn run<G, F>(
    descriptor: Arc<RuntimeStatusDescriptor>,
    mut latest: watch::Receiver<Option<Publication>>,
    heartbeat: Duration,
    clock: fn() -> i64,
    mut gate: G,
) where
    G: FnMut() -> F,
    F: Future<Output = bool>,
{
    let capacity = TransientCapacity::new();
    // The publication present at start is a new commit to report at once.
    latest.mark_changed();
    let mut due = tokio::time::Instant::now() + heartbeat;
    loop {
        let heartbeat_tick = match tokio::time::timeout_at(due, latest.changed()).await {
            Ok(Ok(())) => false,
            Ok(Err(_)) => return,
            Err(_) => true,
        };
        due = tokio::time::Instant::now() + heartbeat;
        let Some(publication) = latest.borrow_and_update().clone() else {
            continue;
        };
        if heartbeat_tick && !(publication.ready && gate().await) {
            continue;
        }
        let started = tokio::time::Instant::now();
        let result = deliver(&descriptor, &capacity, &publication, clock()).await;
        // §10: operation, scope, `ready`, result class and timing only.
        eprintln!(
            "oteryn-game-server event=runtime_status operation=ReportRuntimeStatusV1 world_id={} channel_id={} ready={} result={} elapsed_ms={}",
            publication.world_id,
            publication.channel_id,
            publication.ready,
            match result {
                Ok(Delivery::Accepted) => "accepted",
                Ok(Delivery::Refreshed) => "refreshed",
                Ok(Delivery::Superseded) => "superseded",
                Err(_) => "not_delivered",
            },
            started.elapsed().as_millis()
        );
    }
}

/// The Registry route descriptor (Platform login contract §7.3). The node
/// never reports it; it may only recompute the Registry's value to compare.
pub struct RouteDescriptor<'a> {
    pub world_id: &'a str,
    pub channel_id: &'a str,
    pub host: &'a str,
    pub port: u16,
    pub tls_server_name: &'a str,
    pub alpn: &'a str,
    pub protocol_major: u64,
    pub transport_profile: u64,
}

/// `rt.<registry route version>.<first 32 hex of SHA-256(JCS(descriptor))>`.
/// Every member is a plain string or integer, so JCS is sorted keys with no
/// whitespace.
#[must_use]
pub fn route_revision(registry_version: u64, d: &RouteDescriptor<'_>) -> String {
    use sha2::Digest;
    // Keys in ascending order; the default map also sorts.
    let canonical: serde_json::Map<String, serde_json::Value> = [
        ("alpn", d.alpn.into()),
        ("channel_id", d.channel_id.into()),
        ("host", d.host.into()),
        ("port", d.port.into()),
        ("protocol_major", d.protocol_major.into()),
        ("tls_server_name", d.tls_server_name.into()),
        ("transport_profile", d.transport_profile.into()),
        ("world_id", d.world_id.into()),
    ]
    .into_iter()
    .map(|(k, v): (&str, serde_json::Value)| (k.to_owned(), v))
    .collect();
    let digest = sha2::Sha256::digest(serde_json::Value::Object(canonical).to_string().as_bytes());
    let hex: String = digest[..16].iter().map(|b| format!("{b:02x}")).collect();
    format!("rt.{registry_version}.{hex}")
}
