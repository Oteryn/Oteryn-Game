//! `ReportScopeAssignmentV1` producer (`oteryn-game-native-runtime-status-v1` §3, §5).
//!
//! The scope ownership authority (`oteryn-game-ops`) reports a committed
//! assignment to Platform over its own ownership-authority mTLS identity. The
//! body is derived from the durable assignment row and the node identity bound
//! to its holder, so a re-send is byte-identical. A failed report never
//! changes the Game assignment, which stays authoritative. The epoch is a
//! declared value here and is never raised by a report (§6, U16).

use super::{
    SourceError, TransientCapacity,
    descriptor::{Operation, ProducerDescriptor},
    http1_mtls,
    runtime_status::{canonical_uuid, purpose_descriptor},
};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, net::SocketAddr, path::PathBuf, time::Duration};

/// Request bound (§3, `NRS-REPORT-BYTES`).
pub const REPORT_BYTES: usize = 2048;
/// Response bound (§3, `NRS-RESPONSE-BYTES`).
pub const RESPONSE_BYTES: usize = 256;
/// Compiled namespace of the ownership-authority descriptor.
pub const PURPOSE: &str = "OTERYN_GAME_SCOPE_OWNERSHIP_AUTHORITY";
/// Compiled `ReportScopeAssignmentV1` path (§3); not configurable.
pub const PATH: &str = "/internal/v1/game-auth/native-scope-assignments";
/// Longest accepted node-host identity.
pub const NODE_IDENTITY_BYTES: usize = 256;
/// Configuration bounds.
pub const CONFIG_BYTES: usize = 16 * 1024;
pub const CONFIG_SCOPES_MAX: usize = 64;
pub const CONFIG_IDENTITIES_MAX: usize = 16;
pub const CONFIG_OTHER_PRODUCERS_MAX: usize = 8;
pub const CONFIG_NODE_CERTIFICATES_MAX: usize = 64;

/// One committed assignment as reported (§5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignment {
    pub assignment_epoch: u64,
    pub world_id: String,
    pub channel_id: String,
    pub ownership_generation: u64,
    pub node_identity: String,
    pub assigned_at: i64,
}

#[derive(Serialize)]
struct Wire<'a> {
    contract_version: u8,
    operation: &'static str,
    assignment_epoch: String,
    world_id: &'a str,
    channel_id: &'a str,
    ownership_generation: String,
    node_identity: &'a str,
    assigned_at: String,
}

/// A node-host identity (certificate subject): printable ASCII without JSON
/// escapes, bounded.
#[must_use]
pub fn valid_node_identity(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= NODE_IDENTITY_BYTES
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b" .,=:_-/@+".contains(&b))
        && !value.starts_with(' ')
        && !value.ends_with(' ')
}

/// Exact §5 wire body. Refuses anything outside the contract grammar.
pub fn encode(a: &Assignment) -> Result<String, SourceError> {
    let valid = a.assignment_epoch != 0
        && canonical_uuid(&a.world_id, Some(b'7'))
        && canonical_uuid(&a.channel_id, Some(b'7'))
        && a.world_id != a.channel_id
        && a.ownership_generation != 0
        && valid_node_identity(&a.node_identity)
        && a.assigned_at >= 0;
    if !valid {
        return Err(SourceError::InvalidInput);
    }
    let body = serde_json::to_string(&Wire {
        contract_version: 1,
        operation: "ReportScopeAssignmentV1",
        assignment_epoch: a.assignment_epoch.to_string(),
        world_id: &a.world_id,
        channel_id: &a.channel_id,
        ownership_generation: a.ownership_generation.to_string(),
        node_identity: &a.node_identity,
        assigned_at: a.assigned_at.to_string(),
    })
    .map_err(|_| SourceError::InvalidInput)?;
    if body.len() > REPORT_BYTES {
        return Err(SourceError::CapacityExceeded);
    }
    Ok(body)
}

/// Platform's definite acknowledgement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Delivery {
    Accepted,
    Superseded,
}

impl Delivery {
    #[must_use]
    pub const fn class(self) -> &'static str {
        match self {
            Self::Accepted => "accepted",
            Self::Superseded => "superseded",
        }
    }
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

/// Why a report was not delivered. Only the definite ones stop the retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotDelivered {
    /// The local assignment is outside the contract grammar; nothing sent.
    InvalidReport,
    /// `400` (definite).
    Malformed,
    /// `401`: unauthenticated, wrong purpose or identity not allowed (definite).
    Unauthenticated,
    /// `409`: conflict with Platform's ordering (definite).
    Conflict,
    /// `429`.
    RateLimited,
    /// A response outside the contract.
    InvalidResponse,
    /// `503`, any other status, or transport failure (including a timeout).
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

    /// A definite outcome: retrying the same content cannot change it.
    #[must_use]
    pub const fn definite(self) -> bool {
        matches!(
            self,
            Self::InvalidReport | Self::Malformed | Self::Unauthenticated | Self::Conflict
        )
    }
}

/// Ownership-authority client identity (§3): its own certificate and key.
pub struct ScopeAssignmentDescriptor(ProducerDescriptor);

impl ScopeAssignmentDescriptor {
    /// `other_purposes` are the certificate chains of every other Platform
    /// identity this host holds; none may share the leaf public key.
    pub fn new(
        connect_endpoint: (String, u16),
        peer_name: String,
        roots: Vec<CertificateDer<'static>>,
        client_chain: Vec<CertificateDer<'static>>,
        client_key: PrivateKeyDer<'static>,
        other_purposes: &[&[CertificateDer<'static>]],
    ) -> Result<Self, SourceError> {
        purpose_descriptor(
            PURPOSE,
            connect_endpoint,
            peer_name,
            roots,
            client_chain,
            client_key,
            other_purposes,
        )
        .map(Self)
    }
}

/// One bounded `ReportScopeAssignmentV1` exchange of an encoded body.
pub async fn deliver(
    descriptor: &ScopeAssignmentDescriptor,
    capacity: &TransientCapacity,
    body: &str,
) -> Result<Delivery, NotDelivered> {
    let mut permit = capacity
        .try_queue()
        .map_err(|_| NotDelivered::Unavailable)?;
    permit
        .try_activate()
        .map_err(|_| NotDelivered::Unavailable)?;
    let (status, raw) = http1_mtls::exchange_with_status(
        &descriptor.0,
        Operation::ReportScopeAssignmentV1,
        body,
        &mut permit,
    )
    .await
    .map_err(|_| NotDelivered::Unavailable)?;
    if status == 200 {
        return decode_response(&raw).map_err(|_| NotDelivered::InvalidResponse);
    }
    let refused = |class| {
        if raw.is_empty() {
            class
        } else {
            NotDelivered::InvalidResponse
        }
    };
    Err(match status {
        400 => refused(NotDelivered::Malformed),
        401 => refused(NotDelivered::Unauthenticated),
        409 => refused(NotDelivered::Conflict),
        429 => refused(NotDelivered::RateLimited),
        _ => NotDelivered::Unavailable,
    })
}

/// Bounded exponential backoff between attempts of one report.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    pub first: Duration,
    pub max: Duration,
    pub attempts: u32,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            first: Duration::from_millis(250),
            max: Duration::from_secs(4),
            attempts: 8,
        }
    }
}

/// Outcome of a report with retries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    pub result: Result<Delivery, NotDelivered>,
    pub attempts: u32,
}

/// Encode once and deliver the identical bytes until a definite result or the
/// attempt bound. The last indefinite outcome is returned when the bound is
/// reached; the caller re-runs the report later with the same content.
pub async fn report(
    descriptor: &ScopeAssignmentDescriptor,
    assignment: &Assignment,
    policy: RetryPolicy,
) -> Report {
    let body = match encode(assignment) {
        Ok(body) => body,
        Err(_) => {
            return Report {
                result: Err(NotDelivered::InvalidReport),
                attempts: 0,
            };
        }
    };
    let capacity = TransientCapacity::new();
    let mut backoff = policy.first;
    let mut attempts = 0;
    loop {
        attempts += 1;
        let result = deliver(descriptor, &capacity, &body).await;
        let stop = match result {
            Ok(_) => true,
            Err(not_delivered) => not_delivered.definite(),
        };
        if stop || attempts >= policy.attempts.max(1) {
            return Report { result, attempts };
        }
        tokio::time::sleep(backoff).await;
        backoff = (backoff * 2).min(policy.max);
    }
}

/// One scope and the node-host identities that may serve it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScopeIdentities {
    pub world_id: String,
    pub channel_id: String,
    pub node_identities: Vec<String>,
}

/// `oteryn-game-ops --report-config <path>`: the ownership-authority channel
/// to Platform. The path is compiled; the epoch is declared until its Game
/// storage exists (U-RS5) and is never raised by the tool.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReportConfig {
    pub endpoint: SocketAddr,
    pub peer_name: String,
    pub trust_roots_file: PathBuf,
    pub client_certificate_file: PathBuf,
    pub client_key_file: PathBuf,
    /// Runtime-status client certificate of every node-host identity named in
    /// `scope`, exactly that set and each one required, so no node host can
    /// share the authority's public key unchecked (§3).
    pub node_certificate_files: BTreeMap<String, PathBuf>,
    /// Client certificates of further producer identities, such as the
    /// account-characters one. The local node configuration's
    /// native-evidence and runtime-status certificates and every
    /// `node_certificate_files` entry are always checked. The authority
    /// identity must not share a public key with any of them (§3).
    #[serde(default)]
    pub other_producer_certificate_files: Vec<PathBuf>,
    pub assignment_epoch: u64,
    pub scope: Vec<ScopeIdentities>,
}

impl ReportConfig {
    /// Parse and validate. Duplicate, unknown and missing keys refuse.
    pub fn parse(document: &[u8]) -> Result<Self, SourceError> {
        if document.len() > CONFIG_BYTES {
            return Err(SourceError::InvalidDescriptor);
        }
        let text = std::str::from_utf8(document).map_err(|_| SourceError::InvalidDescriptor)?;
        let config: Self = toml::from_str(text).map_err(|_| SourceError::InvalidDescriptor)?;
        let mut scopes = std::collections::BTreeSet::new();
        let mut others = std::collections::BTreeSet::new();
        let valid = config.assignment_epoch != 0
            && !config.peer_name.is_empty()
            && config.peer_name.len() <= 128
            && config.endpoint.port() != 0
            && [
                &config.trust_roots_file,
                &config.client_certificate_file,
                &config.client_key_file,
            ]
            .iter()
            .all(|path| path.is_absolute())
            && config.other_producer_certificate_files.len() <= CONFIG_OTHER_PRODUCERS_MAX
            && config.node_certificate_files.len() <= CONFIG_NODE_CERTIFICATES_MAX
            && config
                .other_producer_certificate_files
                .iter()
                .chain(config.node_certificate_files.values())
                .all(|path| {
                    path.is_absolute()
                        && path != &config.client_certificate_file
                        && path != &config.client_key_file
                        && others.insert(path)
                })
            && !config.scope.is_empty()
            && config.scope.len() <= CONFIG_SCOPES_MAX
            && config.scope.iter().all(|scope| {
                let mut identities = std::collections::BTreeSet::new();
                canonical_uuid(&scope.world_id, Some(b'7'))
                    && canonical_uuid(&scope.channel_id, Some(b'7'))
                    && scope.world_id != scope.channel_id
                    && scopes.insert((scope.world_id.clone(), scope.channel_id.clone()))
                    && !scope.node_identities.is_empty()
                    && scope.node_identities.len() <= CONFIG_IDENTITIES_MAX
                    && scope.node_identities.iter().all(|identity| {
                        valid_node_identity(identity) && identities.insert(identity)
                    })
            })
            && config.node_certificate_files.keys().eq(config
                .scope
                .iter()
                .flat_map(|scope| &scope.node_identities)
                .collect::<std::collections::BTreeSet<_>>());
        if valid {
            Ok(config)
        } else {
            Err(SourceError::InvalidDescriptor)
        }
    }

    /// Whether this channel is the node configuration's Platform channel:
    /// the same `[platform]` endpoint and peer name (§3) and its
    /// `platform.runtime_status.assignment_epoch`. Platform matches runtime
    /// status against the reported assignment on that channel and epoch, so
    /// a mismatch could never route. Trust roots are compared by content by
    /// the caller.
    #[must_use]
    pub fn matches_node_channel(
        &self,
        endpoint: SocketAddr,
        peer_name: &str,
        runtime_status_epoch: u64,
    ) -> bool {
        self.endpoint == endpoint
            && self.peer_name == peer_name
            && self.assignment_epoch == runtime_status_epoch
    }

    /// Whether `identity` is configured for the scope (§5).
    #[must_use]
    pub fn allows(&self, world_id: &str, channel_id: &str, identity: &str) -> bool {
        self.scope.iter().any(|scope| {
            scope.world_id == world_id
                && scope.channel_id == channel_id
                && scope.node_identities.iter().any(|known| known == identity)
        })
    }
}

/// Whether the leaf certificate's subject is `identity` (§5: the node
/// identity is the certificate subject of the node host's runtime-status
/// identity). It matches the subject's single common name, or the whole
/// subject rendered as RFC 4514 short names (`CN=node-a,O=Oteryn`). A
/// subject that cannot be rendered without escaping never matches.
#[must_use]
pub fn certificate_has_node_identity(leaf: &CertificateDer<'_>, identity: &str) -> bool {
    let Some(subject) = subject_rdns(leaf.as_ref()) else {
        return false;
    };
    let common_names: Vec<&str> = subject
        .iter()
        .flatten()
        .filter(|(name, _)| *name == "CN")
        .map(|(_, value)| *value)
        .collect();
    let rendered = subject
        .iter()
        .rev()
        .map(|rdn| {
            rdn.iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join("+")
        })
        .collect::<Vec<_>>()
        .join(",");
    valid_node_identity(identity) && (common_names == [identity] || rendered == identity)
}

type Rdn<'a> = Vec<(&'static str, &'a str)>;

/// One DER element: its tag, its content and the rest of the input.
fn der(input: &[u8]) -> Option<(u8, &[u8], &[u8])> {
    let (&tag, rest) = input.split_first()?;
    let (&first, rest) = rest.split_first()?;
    let (length, rest) = if first < 0x80 {
        (usize::from(first), rest)
    } else {
        let count = usize::from(first & 0x7f);
        if count == 0 || count > 4 || rest.len() < count {
            return None;
        }
        let (bytes, rest) = rest.split_at(count);
        let length = bytes
            .iter()
            .fold(0usize, |length, &b| (length << 8) | usize::from(b));
        (length, rest)
    };
    (rest.len() >= length).then(|| (tag, &rest[..length], &rest[length..]))
}

/// The subject RDNs of a certificate, in encoded order. Only attribute types
/// with an RFC 4514 short name and string values that need no escaping are
/// accepted.
fn subject_rdns(certificate: &[u8]) -> Option<Vec<Rdn<'_>>> {
    const SEQUENCE: u8 = 0x30;
    const SET: u8 = 0x31;
    let (SEQUENCE, certificate, _) = der(certificate)? else {
        return None;
    };
    let (SEQUENCE, mut tbs, _) = der(certificate)? else {
        return None;
    };
    if tbs.first() == Some(&0xa0) {
        tbs = der(tbs)?.2;
    }
    // serialNumber, signature, issuer, validity.
    for _ in 0..4 {
        tbs = der(tbs)?.2;
    }
    let (SEQUENCE, mut name, _) = der(tbs)? else {
        return None;
    };
    let mut rdns = Vec::new();
    while !name.is_empty() {
        let (SET, mut set, rest) = der(name)? else {
            return None;
        };
        name = rest;
        let mut rdn = Vec::new();
        while !set.is_empty() {
            let (SEQUENCE, attribute, rest) = der(set)? else {
                return None;
            };
            set = rest;
            let (0x06, oid, value) = der(attribute)? else {
                return None;
            };
            let (tag, value, rest) = der(value)?;
            let short = match oid {
                [0x55, 0x04, 0x03] => "CN",
                [0x55, 0x04, 0x06] => "C",
                [0x55, 0x04, 0x07] => "L",
                [0x55, 0x04, 0x08] => "ST",
                [0x55, 0x04, 0x09] => "STREET",
                [0x55, 0x04, 0x0a] => "O",
                [0x55, 0x04, 0x0b] => "OU",
                [0x09, 0x92, 0x26, 0x89, 0x93, 0xf2, 0x2c, 0x64, 0x01, 0x19] => "DC",
                [0x09, 0x92, 0x26, 0x89, 0x93, 0xf2, 0x2c, 0x64, 0x01, 0x01] => "UID",
                _ => return None,
            };
            // UTF8String, PrintableString, IA5String.
            let value = std::str::from_utf8(value).ok()?;
            let plain = matches!(tag, 0x0c | 0x13 | 0x16)
                && rest.is_empty()
                && !value.is_empty()
                && !value.starts_with([' ', '#'])
                && !value.ends_with(' ')
                && value
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b" .:_-/@".contains(&b));
            if !plain {
                return None;
            }
            rdn.push((short, value));
        }
        if rdn.is_empty() {
            return None;
        }
        rdns.push(rdn);
    }
    (!rdns.is_empty()).then_some(rdns)
}
