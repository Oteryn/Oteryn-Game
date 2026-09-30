//! `ReportRuntimeStatusV1` producer (`oteryn-game-native-runtime-status-v1` §3, §4, §8).
//!
//! The node projects its committed Runtime readiness publication to Platform
//! over its own purpose-bound mTLS identity. Delivery never gates boot,
//! serving, admission or shutdown: every failure is only "not delivered".
//! The report never carries an endpoint; the World Registry owns the route.

use super::{
    SourceError, TransientCapacity,
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use std::{
    future::{Future, poll_fn},
    pin::pin,
    sync::Arc,
    task::Poll,
    time::Duration,
};
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

fn leaf_spki(chain: &[CertificateDer<'_>]) -> Result<Vec<u8>, SourceError> {
    let leaf = chain.first().ok_or(SourceError::InvalidDescriptor)?;
    let parsed = rustls::server::ParsedCertificate::try_from(leaf)
        .map_err(|_| SourceError::InvalidDescriptor)?;
    Ok(parsed.subject_public_key_info().as_ref().to_vec())
}

impl RuntimeStatusDescriptor {
    /// `other_purposes` are the certificate chains of every other Platform
    /// identity this host holds. The runtime-status certificate and key must
    /// carry a public key (SPKI) that none of them uses.
    pub fn new(
        connect_endpoint: (String, u16),
        peer_name: String,
        roots: Vec<CertificateDer<'static>>,
        client_chain: Vec<CertificateDer<'static>>,
        client_key: PrivateKeyDer<'static>,
        other_purposes: &[&[CertificateDer<'static>]],
    ) -> Result<Self, SourceError> {
        let own = leaf_spki(&client_chain)?;
        let signer = rustls::crypto::aws_lc_rs::default_provider()
            .key_provider
            .load_private_key(client_key.clone_key())
            .map_err(|_| SourceError::InvalidDescriptor)?;
        let key_spki = signer
            .public_key()
            .ok_or(SourceError::InvalidDescriptor)?
            .as_ref()
            .to_vec();
        if key_spki != own {
            return Err(SourceError::InvalidDescriptor);
        }
        for chain in other_purposes {
            if leaf_spki(chain)? == own {
                return Err(SourceError::InvalidDescriptor);
            }
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

/// Why a report was not delivered. None of these is a statement about the
/// node's readiness (§4); they are distinguished for logs only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotDelivered {
    /// The local publication is outside the contract grammar; nothing sent.
    InvalidReport,
    /// `400`.
    Malformed,
    /// `401`: unauthenticated, wrong purpose or scope not allowed.
    Unauthenticated,
    /// `409`: epoch, generation or ordering conflict with the assignment.
    Conflict,
    /// `429`.
    RateLimited,
    /// A response outside the contract.
    InvalidResponse,
    /// `503`, any other status, or transport failure.
    Unavailable,
}

impl NotDelivered {
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::InvalidReport => "invalid_report",
            Self::Malformed => "malformed",
            Self::Unauthenticated => "unauthenticated",
            Self::Conflict => "conflict",
            Self::RateLimited => "rate_limited",
            Self::InvalidResponse => "invalid_response",
            Self::Unavailable => "unavailable",
        }
    }
}

/// One bounded `ReportRuntimeStatusV1` exchange on the reporter's own
/// capacity (never an admission slot).
pub async fn deliver(
    descriptor: &RuntimeStatusDescriptor,
    capacity: &TransientCapacity,
    publication: &Publication,
    observed_at: i64,
) -> Result<Delivery, NotDelivered> {
    let body = encode(publication, observed_at).map_err(|_| NotDelivered::InvalidReport)?;
    let mut permit = capacity
        .try_queue()
        .map_err(|_| NotDelivered::Unavailable)?;
    permit
        .try_activate()
        .map_err(|_| NotDelivered::Unavailable)?;
    let (status, raw) = http1_mtls::exchange_with_status(
        &descriptor.0,
        Operation::ReportRuntimeStatusV1,
        &body,
        &mut permit,
    )
    .await
    .map_err(|_| NotDelivered::Unavailable)?;
    // Failures are empty bodies (§4).
    let refused = |class| {
        if raw.is_empty() {
            class
        } else {
            NotDelivered::InvalidResponse
        }
    };
    Err(match status {
        200 => return decode_response(&raw).map_err(|_| NotDelivered::InvalidResponse),
        400 => refused(NotDelivered::Malformed),
        401 => refused(NotDelivered::Unauthenticated),
        409 => refused(NotDelivered::Conflict),
        429 => refused(NotDelivered::RateLimited),
        _ => NotDelivered::Unavailable,
    })
}

/// Monotonic and wall time for the report loop; injectable for tests.
pub trait ReportClock: Send + Sync {
    /// Monotonic time since an arbitrary origin.
    fn elapsed(&self) -> Duration;
    /// Unix seconds, the source of `observed_at`.
    fn unix_now(&self) -> i64;
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send;
}

/// Production clock: tokio timers and the system wall clock.
#[derive(Debug, Clone, Copy)]
pub struct SystemClock(tokio::time::Instant);

impl Default for SystemClock {
    fn default() -> Self {
        Self(tokio::time::Instant::now())
    }
}

impl ReportClock for SystemClock {
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
    fn unix_now(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|elapsed| i64::try_from(elapsed.as_secs()).ok())
            .unwrap_or(0)
    }
    fn sleep(&self, duration: Duration) -> impl Future<Output = ()> + Send {
        tokio::time::sleep(duration)
    }
}

/// The heartbeat conditions of §8.2 as seen by the node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gate {
    /// Serving, durability root ready, assignment unchanged.
    Holds,
    /// The check could not run now (the durability connection is in use).
    Busy,
    /// Not serving or the durability root is not ready.
    NotReady,
    /// The scope is no longer assigned to this incarnation at this generation.
    Lost,
}

pub trait HeartbeatGate: Send {
    fn check(&mut self) -> impl Future<Output = Gate> + Send;
}

/// Where reports go: Platform over mTLS, or a test sink.
pub trait ReportSink: Send {
    fn send(
        &mut self,
        publication: &Publication,
        observed_at: i64,
    ) -> impl Future<Output = Result<Delivery, NotDelivered>> + Send;
}

/// The production sink: one exchange at a time on its own capacity.
pub struct MtlsSink {
    descriptor: Arc<RuntimeStatusDescriptor>,
    capacity: TransientCapacity,
}

impl MtlsSink {
    #[must_use]
    pub fn new(descriptor: Arc<RuntimeStatusDescriptor>) -> Self {
        Self {
            descriptor,
            capacity: TransientCapacity::new(),
        }
    }
}

impl ReportSink for MtlsSink {
    async fn send(
        &mut self,
        publication: &Publication,
        observed_at: i64,
    ) -> Result<Delivery, NotDelivered> {
        deliver(&self.descriptor, &self.capacity, publication, observed_at).await
    }
}

/// First retry delay after a busy check; doubled up to the maximum.
pub const BUSY_RETRY_MIN: Duration = Duration::from_millis(100);
pub const BUSY_RETRY_MAX: Duration = Duration::from_secs(1);

/// `Some(output)` of `primary`, or `None` once `sleep` completes first.
pub(crate) async fn first_of<C, S>(changed: C, sleep: S) -> Option<C::Output>
where
    C: Future,
    S: Future<Output = ()>,
{
    let (mut changed, mut sleep) = (pin!(changed), pin!(sleep));
    poll_fn(|context| {
        if let Poll::Ready(result) = changed.as_mut().poll(context) {
            return Poll::Ready(Some(result));
        }
        sleep.as_mut().poll(context).map(|()| None)
    })
    .await
}

fn log(publication: &Publication, what: &str, elapsed: Duration) {
    // §10: operation, scope, `ready`, result class and timing only.
    eprintln!(
        "oteryn-game-server event=runtime_status operation=ReportRuntimeStatusV1 world_id={} channel_id={} ready={} {what} elapsed_ms={}",
        publication.world_id,
        publication.channel_id,
        publication.ready,
        elapsed.as_millis()
    );
}

/// Report loop (§8). Each new committed publication (including the one
/// present at start) is sent once. At every heartbeat tick a `ready = true`
/// publication is repeated with a new `observed_at` when `gate` holds; a busy
/// check is retried with bounded backoff until the next tick. One report is in
/// flight and the watch keeps only the latest publication. The loop ends when
/// the sender is dropped, after sending a publication it has not yet seen.
pub async fn run<C, G, S>(
    clock: C,
    mut latest: watch::Receiver<Option<Publication>>,
    heartbeat: Duration,
    mut gate: G,
    mut sink: S,
) where
    C: ReportClock,
    G: HeartbeatGate,
    S: ReportSink,
{
    latest.mark_changed();
    let mut due = clock.elapsed() + heartbeat;
    let mut skipped = None;
    loop {
        let wait = due.saturating_sub(clock.elapsed());
        let tick = match first_of(latest.changed(), clock.sleep(wait)).await {
            Some(Ok(())) => false,
            Some(Err(_)) => return,
            None => true,
        };
        let publication = latest.borrow_and_update().clone();
        if tick {
            // Fixed rate from the previous tick, never in the past.
            due = (due + heartbeat).max(clock.elapsed());
        } else {
            due = clock.elapsed() + heartbeat;
        }
        let Some(publication) = publication else {
            continue;
        };
        if tick {
            if !publication.ready {
                continue;
            }
            let mut backoff = BUSY_RETRY_MIN;
            let state = loop {
                let state = gate.check().await;
                if state != Gate::Busy || clock.elapsed() + backoff >= due {
                    break state;
                }
                clock.sleep(backoff).await;
                backoff = (backoff * 2).min(BUSY_RETRY_MAX);
            };
            if state != Gate::Holds {
                if skipped != Some(state) {
                    let reason = match state {
                        Gate::Busy => "busy",
                        Gate::NotReady => "not_ready",
                        _ => "lost",
                    };
                    log(
                        &publication,
                        &format!("heartbeat=stopped reason={reason}"),
                        Duration::ZERO,
                    );
                }
                skipped = Some(state);
                continue;
            }
        }
        skipped = None;
        let started = clock.elapsed();
        let result = sink.send(&publication, clock.unix_now()).await;
        let class = match result {
            Ok(Delivery::Accepted) => "accepted",
            Ok(Delivery::Refreshed) => "refreshed",
            Ok(Delivery::Superseded) => "superseded",
            Err(not_delivered) => not_delivered.class(),
        };
        log(
            &publication,
            &format!("result={class}"),
            clock.elapsed().saturating_sub(started),
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
