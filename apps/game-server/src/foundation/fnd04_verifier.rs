use base64::Engine;
use ed25519_dalek::{Signature, VerifyingKey};
use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::{ChannelId, CharacterId, FreshAdmissionFacts, WorldId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericDateError {
    Malformed,
    NotYetValid,
    Expired,
}

pub const MAX_COMPACT_JWS_BYTES: usize = 4_096;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fnd04VerificationError {
    Malformed,
    AuthenticationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompactJws {
    protected_header_segment: String,
    payload_segment: String,
    signature_segment: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ProtectedHeader {
    kid: String,
    typ: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixedTrustContext {
    keys: BTreeMap<String, [u8; 32]>,
}

fn parse_protected_header(
    compact_jws: &CompactJws,
) -> Result<ProtectedHeader, Fnd04VerificationError> {
    let protected_header = decode_canonical_base64url(&compact_jws.protected_header_segment, 512)?;
    let value: serde_json::Value =
        serde_json::from_slice(&protected_header).map_err(|_| Fnd04VerificationError::Malformed)?;
    let object = value.as_object().ok_or(Fnd04VerificationError::Malformed)?;
    if object.len() != 3
        || !object.contains_key("alg")
        || !object.contains_key("kid")
        || !object.contains_key("typ")
    {
        return Err(Fnd04VerificationError::Malformed);
    }

    let alg = object
        .get("alg")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.is_ascii())
        .ok_or(Fnd04VerificationError::Malformed)?;
    let kid = object
        .get("kid")
        .and_then(serde_json::Value::as_str)
        .filter(|value| {
            (1..=64).contains(&value.len())
                && value.is_ascii()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        })
        .ok_or(Fnd04VerificationError::Malformed)?;
    let typ = object
        .get("typ")
        .and_then(serde_json::Value::as_str)
        .filter(|value| {
            (1..=64).contains(&value.len())
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_graphic() && !byte.is_ascii_whitespace())
        })
        .ok_or(Fnd04VerificationError::Malformed)?;

    if alg != "Ed25519" {
        return Err(Fnd04VerificationError::AuthenticationFailed);
    }

    Ok(ProtectedHeader {
        kid: kid.to_owned(),
        typ: typ.to_owned(),
    })
}

fn verify_compact_signature(
    compact_jws: &CompactJws,
    protected_header: &ProtectedHeader,
    trust_context: &FixedTrustContext,
) -> Result<(), Fnd04VerificationError> {
    let public_key = trust_context
        .keys
        .get(&protected_header.kid)
        .ok_or(Fnd04VerificationError::AuthenticationFailed)?;
    let verifying_key = VerifyingKey::from_bytes(public_key)
        .map_err(|_| Fnd04VerificationError::AuthenticationFailed)?;
    let signature = decode_canonical_base64url(&compact_jws.signature_segment, 64)?;
    let signature: [u8; 64] = signature
        .try_into()
        .map_err(|_| Fnd04VerificationError::AuthenticationFailed)?;
    let signing_input = format!(
        "{}.{}",
        compact_jws.protected_header_segment, compact_jws.payload_segment
    );
    verifying_key
        .verify_strict(signing_input.as_bytes(), &Signature::from_bytes(&signature))
        .map_err(|_| Fnd04VerificationError::AuthenticationFailed)
}

fn parse_compact_jws(token: &str) -> Result<CompactJws, Fnd04VerificationError> {
    if token.len() > MAX_COMPACT_JWS_BYTES || !token.is_ascii() {
        return Err(Fnd04VerificationError::Malformed);
    }

    let mut segments = token.split('.');
    let Some(protected_header_segment) = segments.next() else {
        return Err(Fnd04VerificationError::Malformed);
    };
    let Some(payload_segment) = segments.next() else {
        return Err(Fnd04VerificationError::Malformed);
    };
    let Some(signature_segment) = segments.next() else {
        return Err(Fnd04VerificationError::Malformed);
    };
    if segments.next().is_some()
        || protected_header_segment.is_empty()
        || payload_segment.is_empty()
        || signature_segment.is_empty()
    {
        return Err(Fnd04VerificationError::Malformed);
    }

    let protected_header = decode_canonical_base64url(protected_header_segment, 512)?;
    let payload = decode_canonical_base64url(payload_segment, 3_072)?;
    decode_canonical_base64url(signature_segment, MAX_COMPACT_JWS_BYTES)?;
    validate_bounded_json_object(&protected_header)?;
    validate_bounded_json_object(&payload)?;

    Ok(CompactJws {
        protected_header_segment: protected_header_segment.to_owned(),
        payload_segment: payload_segment.to_owned(),
        signature_segment: signature_segment.to_owned(),
    })
}

fn decode_canonical_base64url(
    segment: &str,
    maximum_decoded_bytes: usize,
) -> Result<Vec<u8>, Fnd04VerificationError> {
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(segment)
        .map_err(|_| Fnd04VerificationError::Malformed)?;
    if decoded.len() > maximum_decoded_bytes
        || base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&decoded) != segment
    {
        return Err(Fnd04VerificationError::Malformed);
    }
    Ok(decoded)
}

fn validate_bounded_json_object(input: &[u8]) -> Result<(), Fnd04VerificationError> {
    let mut deserializer = serde_json::Deserializer::from_slice(input);
    BoundedJsonObject { depth: 1 }
        .deserialize(&mut deserializer)
        .map_err(|_| Fnd04VerificationError::Malformed)?;
    deserializer
        .end()
        .map_err(|_| Fnd04VerificationError::Malformed)
}

struct BoundedJsonObject {
    depth: u8,
}

impl<'de> DeserializeSeed<'de> for BoundedJsonObject {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(BoundedJsonVisitor { depth: self.depth })
    }
}

struct BoundedJsonValue {
    depth: u8,
}

impl<'de> DeserializeSeed<'de> for BoundedJsonValue {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(BoundedJsonVisitor { depth: self.depth })
    }
}

struct BoundedJsonVisitor {
    depth: u8,
}

impl<'de> Visitor<'de> for BoundedJsonVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded JSON")
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        if self.depth > 2 {
            return Err(de::Error::custom("JSON nesting limit exceeded"));
        }

        let mut members = BTreeSet::new();
        while let Some(member) = map.next_key::<String>()? {
            if !members.insert(member) {
                return Err(de::Error::custom("duplicate JSON member"));
            }
            map.next_value_seed(BoundedJsonValue {
                depth: self.depth.saturating_add(1),
            })?;
        }
        Ok(())
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        if self.depth > 2 {
            return Err(de::Error::custom("JSON nesting limit exceeded"));
        }

        while sequence
            .next_element_seed(BoundedJsonValue {
                depth: self.depth.saturating_add(1),
            })?
            .is_some()
        {}
        Ok(())
    }
}

pub struct NumericDate;

impl NumericDate {
    pub fn validate(now: i64, iat: i64, nbf: i64, exp: i64) -> Result<(), NumericDateError> {
        let nbf_lower_bound = iat.checked_sub(1).ok_or(NumericDateError::Malformed)?;
        let nbf_upper_bound = iat.checked_add(1).ok_or(NumericDateError::Malformed)?;
        if !(nbf_lower_bound..=nbf_upper_bound).contains(&nbf) {
            return Err(NumericDateError::Malformed);
        }

        let lifetime = exp.checked_sub(iat).ok_or(NumericDateError::Malformed)?;
        if exp <= iat || lifetime > 30 {
            return Err(NumericDateError::Malformed);
        }

        let latest_accepted_not_before = now.checked_add(5).ok_or(NumericDateError::Malformed)?;
        if latest_accepted_not_before < nbf {
            return Err(NumericDateError::NotYetValid);
        }

        let earliest_accepted_expiry = now.checked_sub(5).ok_or(NumericDateError::Malformed)?;
        if earliest_accepted_expiry >= exp {
            return Err(NumericDateError::Expired);
        }

        let issue_age = iat
            .checked_sub(now)
            .and_then(i64::checked_abs)
            .ok_or(NumericDateError::Malformed)?;
        if issue_age > 35 {
            return Err(NumericDateError::Malformed);
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fnd04ConsumerError {
    FreshMalformed,
    FreshAuthenticationFailed,
    FreshBindingMismatch,
    FreshRevisionUnsupported,
    FreshNotYetValid,
    FreshExpired,
    FreshSecurityEvidenceStale,
    FreshSecurityStateRevoked,
    FreshAccountCharacterConflict,
    FreshWorldStale,
    FreshRouteStale,
    FreshRuntimeStale,
    RecoveryMalformed,
    RecoveryAuthenticationFailed,
    RecoveryBindingMismatch,
    RecoveryRevisionUnsupported,
    RecoveryNotYetValid,
    RecoveryExpired,
    RecoverySecurityEvidenceStale,
    RecoverySecurityStateRevoked,
    RecoveryWorldStale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GrantKind {
    Fresh,
    Recovery,
}

impl GrantKind {
    const fn malformed(self) -> Fnd04ConsumerError {
        match self {
            Self::Fresh => Fnd04ConsumerError::FreshMalformed,
            Self::Recovery => Fnd04ConsumerError::RecoveryMalformed,
        }
    }
    const fn authentication_failed(self) -> Fnd04ConsumerError {
        match self {
            Self::Fresh => Fnd04ConsumerError::FreshAuthenticationFailed,
            Self::Recovery => Fnd04ConsumerError::RecoveryAuthenticationFailed,
        }
    }
    const fn binding_mismatch(self) -> Fnd04ConsumerError {
        match self {
            Self::Fresh => Fnd04ConsumerError::FreshBindingMismatch,
            Self::Recovery => Fnd04ConsumerError::RecoveryBindingMismatch,
        }
    }
    const fn revision_unsupported(self) -> Fnd04ConsumerError {
        match self {
            Self::Fresh => Fnd04ConsumerError::FreshRevisionUnsupported,
            Self::Recovery => Fnd04ConsumerError::RecoveryRevisionUnsupported,
        }
    }
    const fn not_yet_valid(self) -> Fnd04ConsumerError {
        match self {
            Self::Fresh => Fnd04ConsumerError::FreshNotYetValid,
            Self::Recovery => Fnd04ConsumerError::RecoveryNotYetValid,
        }
    }
    const fn expired(self) -> Fnd04ConsumerError {
        match self {
            Self::Fresh => Fnd04ConsumerError::FreshExpired,
            Self::Recovery => Fnd04ConsumerError::RecoveryExpired,
        }
    }
    const fn evidence_stale(self) -> Fnd04ConsumerError {
        match self {
            Self::Fresh => Fnd04ConsumerError::FreshSecurityEvidenceStale,
            Self::Recovery => Fnd04ConsumerError::RecoverySecurityEvidenceStale,
        }
    }
    const fn security_revoked(self) -> Fnd04ConsumerError {
        match self {
            Self::Fresh => Fnd04ConsumerError::FreshSecurityStateRevoked,
            Self::Recovery => Fnd04ConsumerError::RecoverySecurityStateRevoked,
        }
    }
    const fn world_stale(self) -> Fnd04ConsumerError {
        match self {
            Self::Fresh => Fnd04ConsumerError::FreshWorldStale,
            Self::Recovery => Fnd04ConsumerError::RecoveryWorldStale,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
pub enum Fnd04EvidenceScope {
    FreshAdmission,
    ExistingActorRecovery,
}

impl From<GrantKind> for Fnd04EvidenceScope {
    fn from(value: GrantKind) -> Self {
        match value {
            GrantKind::Fresh => Self::FreshAdmission,
            GrantKind::Recovery => Self::ExistingActorRecovery,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fnd04EvidenceError {
    UnavailableOrStale,
    ExplicitlyDenied,
}

/// Source of durable, scope-keyed FND-04 decisions.
///
/// Implementations must authenticate the Platform source; preserve the highest accepted
/// comparable revision plus a decision identity for each `(scope, account)` and
/// `(scope, kid)`; reject equal-revision contradictions; and restore that state on restart.
/// Returning `UnavailableOrStale` is required whenever those invariants cannot be proven.
pub trait Fnd04EvidenceAuthority {
    fn signing_key(
        &self,
        scope: Fnd04EvidenceScope,
        key_id: &str,
        now: i64,
    ) -> Result<[u8; 32], Fnd04EvidenceError>;

    fn account_minimum_generation(
        &self,
        scope: Fnd04EvidenceScope,
        account_id: &str,
        now: i64,
    ) -> Result<u64, Fnd04EvidenceError>;
}

pub struct FreshTrustContext<'a>(&'a dyn Fnd04EvidenceAuthority);

impl<'a> FreshTrustContext<'a> {
    #[must_use]
    pub const fn new(authority: &'a dyn Fnd04EvidenceAuthority) -> Self {
        Self(authority)
    }
}

pub struct RecoveryTrustContext<'a>(&'a dyn Fnd04EvidenceAuthority);

impl<'a> RecoveryTrustContext<'a> {
    #[must_use]
    pub const fn new(authority: &'a dyn Fnd04EvidenceAuthority) -> Self {
        Self(authority)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreshCurrentEvidence {
    pub account_id: String,
    pub character_id: CharacterId,
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub character_lease_generation: u64,
    pub route_revision: String,
    pub runtime_observation_revision: String,
    pub scope_ownership_generation: u64,
    pub ruleset_revision: String,
    pub content_revision: String,
    pub map_revision: String,
    pub world_policy_revision: String,
    pub offer_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryCurrentEvidence {
    pub account_id: String,
    pub character_id: CharacterId,
    pub world_id: WorldId,
    pub ruleset_revision: String,
    pub content_revision: String,
    pub map_revision: String,
    pub world_policy_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedRecoveryFacts {
    grant_nonce: [u8; 32],
    account_id: String,
    character_id: CharacterId,
    world_id: WorldId,
}

impl VerifiedRecoveryFacts {
    #[must_use]
    pub const fn grant_nonce(&self) -> [u8; 32] {
        self.grant_nonce
    }
    #[must_use]
    pub fn account_id(&self) -> &str {
        &self.account_id
    }
    #[must_use]
    pub const fn character_id(&self) -> CharacterId {
        self.character_id
    }
    #[must_use]
    pub const fn world_id(&self) -> WorldId {
        self.world_id
    }
}

#[derive(Debug, Clone)]
struct Claims {
    issuer: String,
    audience: String,
    profile: String,
    purpose: String,
    nonce: [u8; 32],
    account_id: String,
    character: [u8; 16],
    world: [u8; 16],
    channel: Option<[u8; 16]>,
    account_security_generation: u64,
    scope_ownership_generation: Option<u64>,
    route_revision: Option<String>,
    runtime_observation_revision: Option<String>,
    protocol_major: u64,
    transport_profile: u64,
    ruleset_revision: String,
    content_revision: String,
    map_revision: String,
    world_policy_revision: String,
    offer_revision: Option<String>,
    iat: i64,
    nbf: i64,
    exp: i64,
}

pub fn verify_fresh_grant(
    token: &str,
    now: i64,
    trust: &FreshTrustContext<'_>,
    current: &FreshCurrentEvidence,
) -> Result<FreshAdmissionFacts, Fnd04ConsumerError> {
    let kind = GrantKind::Fresh;
    let (header, payload) = authenticate(token, now, trust.0, kind)?;
    let claims = parse_claims(&payload, kind)?;
    validate_bindings(&header, &claims, kind)?;
    validate_time(&claims, now, kind)?;
    let minimum_generation = account_minimum_generation(trust.0, kind, &claims.account_id, now)?;
    if claims.account_security_generation < minimum_generation {
        return Err(kind.security_revoked());
    }
    if claims.protocol_major != 1 || claims.transport_profile != 1 {
        return Err(kind.revision_unsupported());
    }
    let (Some(channel), Some(scope), Some(route), Some(runtime), Some(offer)) = (
        claims.channel,
        claims.scope_ownership_generation,
        claims.route_revision.as_ref(),
        claims.runtime_observation_revision.as_ref(),
        claims.offer_revision.as_ref(),
    ) else {
        return Err(kind.malformed());
    };
    let character = CharacterId::decode(&claims.character).map_err(|_| kind.malformed())?;
    let world = WorldId::decode(&claims.world).map_err(|_| kind.malformed())?;
    let channel = ChannelId::decode(&channel).map_err(|_| kind.malformed())?;
    if claims.account_id != current.account_id || character != current.character_id {
        return Err(Fnd04ConsumerError::FreshAccountCharacterConflict);
    }
    if world != current.world_id || channel != current.channel_id {
        return Err(kind.world_stale());
    }
    if scope != current.scope_ownership_generation
        || runtime != &current.runtime_observation_revision
    {
        return Err(Fnd04ConsumerError::FreshRuntimeStale);
    }
    if route != &current.route_revision {
        return Err(Fnd04ConsumerError::FreshRouteStale);
    }
    if claims.ruleset_revision != current.ruleset_revision
        || claims.content_revision != current.content_revision
        || claims.map_revision != current.map_revision
        || claims.world_policy_revision != current.world_policy_revision
        || offer != &current.offer_revision
    {
        return Err(kind.revision_unsupported());
    }
    FreshAdmissionFacts::new(
        claims.nonce,
        character,
        world,
        channel,
        current.character_lease_generation,
        scope,
    )
    .map_err(|_| kind.malformed())
}

pub fn verify_recovery_grant(
    token: &str,
    now: i64,
    trust: &RecoveryTrustContext<'_>,
    current: &RecoveryCurrentEvidence,
) -> Result<VerifiedRecoveryFacts, Fnd04ConsumerError> {
    let kind = GrantKind::Recovery;
    let (header, payload) = authenticate(token, now, trust.0, kind)?;
    let claims = parse_claims(&payload, kind)?;
    validate_bindings(&header, &claims, kind)?;
    validate_time(&claims, now, kind)?;
    let minimum_generation = account_minimum_generation(trust.0, kind, &claims.account_id, now)?;
    if claims.account_security_generation < minimum_generation {
        return Err(kind.security_revoked());
    }
    if claims.protocol_major != 1 || claims.transport_profile != 1 {
        return Err(kind.revision_unsupported());
    }
    let character = CharacterId::decode(&claims.character).map_err(|_| kind.malformed())?;
    let world = WorldId::decode(&claims.world).map_err(|_| kind.malformed())?;
    if claims.account_id != current.account_id || character != current.character_id {
        return Err(kind.binding_mismatch());
    }
    if world != current.world_id {
        return Err(kind.world_stale());
    }
    if claims.ruleset_revision != current.ruleset_revision
        || claims.content_revision != current.content_revision
        || claims.map_revision != current.map_revision
        || claims.world_policy_revision != current.world_policy_revision
    {
        return Err(kind.revision_unsupported());
    }
    Ok(VerifiedRecoveryFacts {
        grant_nonce: claims.nonce,
        account_id: claims.account_id,
        character_id: character,
        world_id: world,
    })
}

fn authenticate(
    token: &str,
    now: i64,
    authority: &dyn Fnd04EvidenceAuthority,
    kind: GrantKind,
) -> Result<(ProtectedHeader, Vec<u8>), Fnd04ConsumerError> {
    let compact = parse_compact_jws(token).map_err(|_| kind.malformed())?;
    let header = parse_protected_header(&compact).map_err(|error| match error {
        Fnd04VerificationError::Malformed => kind.malformed(),
        Fnd04VerificationError::AuthenticationFailed => kind.authentication_failed(),
    })?;
    let selected = authority
        .signing_key(kind.into(), &header.kid, now)
        .map_err(|error| map_evidence_error(error, kind, true))?;
    let fixed = FixedTrustContext {
        keys: [(header.kid.clone(), selected)].into_iter().collect(),
    };
    verify_compact_signature(&compact, &header, &fixed).map_err(|error| match error {
        Fnd04VerificationError::Malformed => kind.malformed(),
        Fnd04VerificationError::AuthenticationFailed => kind.authentication_failed(),
    })?;
    decode_canonical_base64url(&compact.payload_segment, 3_072)
        .map_err(|_| kind.malformed())
        .map(|payload| (header, payload))
}

fn account_minimum_generation(
    authority: &dyn Fnd04EvidenceAuthority,
    kind: GrantKind,
    account_id: &str,
    now: i64,
) -> Result<u64, Fnd04ConsumerError> {
    authority
        .account_minimum_generation(kind.into(), account_id, now)
        .map_err(|error| map_evidence_error(error, kind, false))
}

const fn map_evidence_error(
    error: Fnd04EvidenceError,
    kind: GrantKind,
    signing_key: bool,
) -> Fnd04ConsumerError {
    match error {
        Fnd04EvidenceError::UnavailableOrStale => kind.evidence_stale(),
        Fnd04EvidenceError::ExplicitlyDenied if signing_key => kind.authentication_failed(),
        Fnd04EvidenceError::ExplicitlyDenied => kind.security_revoked(),
    }
}

fn parse_claims(payload: &[u8], kind: GrantKind) -> Result<Claims, Fnd04ConsumerError> {
    let object = serde_json::from_slice::<Value>(payload).map_err(|_| kind.malformed())?;
    let object = object.as_object().ok_or_else(|| kind.malformed())?;
    let required: &[&str] = match kind {
        GrantKind::Fresh => &[
            "iss",
            "aud",
            "iat",
            "nbf",
            "exp",
            "jti",
            "profile",
            "purpose",
            "attempt_ref",
            "account_id",
            "character_id",
            "world_id",
            "channel_id",
            "account_security_generation",
            "route_revision",
            "runtime_observation_revision",
            "scope_ownership_generation",
            "protocol_major",
            "transport_profile",
            "ruleset_revision",
            "content_revision",
            "map_revision",
            "world_policy_revision",
            "offer_revision",
        ],
        GrantKind::Recovery => &[
            "iss",
            "aud",
            "iat",
            "nbf",
            "exp",
            "jti",
            "profile",
            "purpose",
            "attempt_ref",
            "account_id",
            "character_id",
            "world_id",
            "account_security_generation",
            "protocol_major",
            "transport_profile",
            "ruleset_revision",
            "content_revision",
            "map_revision",
            "world_policy_revision",
        ],
    };
    if object.len() != required.len() || required.iter().any(|name| !object.contains_key(*name)) {
        return Err(kind.malformed());
    }
    let string = |name: &str, maximum: usize| -> Result<String, Fnd04ConsumerError> {
        let value = object
            .get(name)
            .and_then(Value::as_str)
            .filter(|value| visible_ascii(value, maximum))
            .ok_or_else(|| kind.malformed())?;
        Ok(value.to_owned())
    };
    let numeric = |name: &str| -> Result<i64, Fnd04ConsumerError> {
        object
            .get(name)
            .and_then(Value::as_i64)
            .ok_or_else(|| kind.malformed())
    };
    let nonce_string = string("jti", 43)?;
    let nonce = decode_canonical_base64url(&nonce_string, 32).map_err(|_| kind.malformed())?;
    let nonce: [u8; 32] = nonce.try_into().map_err(|_| kind.malformed())?;
    let account_id = string("account_id", 36)?;
    if canonical_uuid(&account_id, false).is_none() {
        return Err(kind.malformed());
    }
    let attempt = string("attempt_ref", 36)?;
    if canonical_uuid(&attempt, true).is_none() {
        return Err(kind.malformed());
    }
    let character =
        canonical_uuid(&string("character_id", 36)?, true).ok_or_else(|| kind.malformed())?;
    let world = canonical_uuid(&string("world_id", 36)?, true).ok_or_else(|| kind.malformed())?;
    let channel = match kind {
        GrantKind::Fresh => {
            Some(canonical_uuid(&string("channel_id", 36)?, true).ok_or_else(|| kind.malformed())?)
        }
        GrantKind::Recovery => None,
    };
    let generation = parse_generation(&string("account_security_generation", 20)?)
        .ok_or_else(|| kind.malformed())?;
    let scope = match kind {
        GrantKind::Fresh => Some(
            parse_generation(&string("scope_ownership_generation", 20)?)
                .ok_or_else(|| kind.malformed())?,
        ),
        GrantKind::Recovery => None,
    };
    let revision = |name: &str| {
        string(name, 64).and_then(|value| {
            if valid_revision(&value) {
                Ok(value)
            } else {
                Err(kind.malformed())
            }
        })
    };
    Ok(Claims {
        issuer: string("iss", 128)?,
        audience: string("aud", 128)?,
        profile: string("profile", 64)?,
        purpose: string("purpose", 64)?,
        nonce,
        account_id,
        character,
        world,
        channel,
        account_security_generation: generation,
        scope_ownership_generation: scope,
        route_revision: if kind == GrantKind::Fresh {
            Some(revision("route_revision")?)
        } else {
            None
        },
        runtime_observation_revision: if kind == GrantKind::Fresh {
            Some(revision("runtime_observation_revision")?)
        } else {
            None
        },
        protocol_major: object
            .get("protocol_major")
            .and_then(Value::as_u64)
            .ok_or_else(|| kind.malformed())?,
        transport_profile: object
            .get("transport_profile")
            .and_then(Value::as_u64)
            .ok_or_else(|| kind.malformed())?,
        ruleset_revision: revision("ruleset_revision")?,
        content_revision: revision("content_revision")?,
        map_revision: revision("map_revision")?,
        world_policy_revision: revision("world_policy_revision")?,
        offer_revision: if kind == GrantKind::Fresh {
            Some(revision("offer_revision")?)
        } else {
            None
        },
        iat: numeric("iat")?,
        nbf: numeric("nbf")?,
        exp: numeric("exp")?,
    })
}

fn validate_bindings(
    header: &ProtectedHeader,
    claims: &Claims,
    kind: GrantKind,
) -> Result<(), Fnd04ConsumerError> {
    let (issuer, audience, typ, purpose, profile) = match kind {
        GrantKind::Fresh => (
            "urn:oteryn:platform:game-admission",
            "urn:oteryn:game:admission",
            "oteryn-admission+jwt",
            "fresh_entry",
            "oteryn-pre-admission-v1",
        ),
        GrantKind::Recovery => (
            "urn:oteryn:platform:game-recovery",
            "urn:oteryn:game:recovery",
            "oteryn-recovery+jwt",
            "existing_actor_recovery",
            "oteryn-reauth-recovery-v1",
        ),
    };
    if claims.issuer != issuer
        || claims.audience != audience
        || header.typ != typ
        || claims.purpose != purpose
    {
        return Err(kind.binding_mismatch());
    }
    if claims.profile != profile {
        return Err(kind.revision_unsupported());
    }
    Ok(())
}

fn validate_time(claims: &Claims, now: i64, kind: GrantKind) -> Result<(), Fnd04ConsumerError> {
    match NumericDate::validate(now, claims.iat, claims.nbf, claims.exp) {
        Ok(()) => Ok(()),
        Err(NumericDateError::Malformed) => Err(kind.malformed()),
        Err(NumericDateError::NotYetValid) => Err(kind.not_yet_valid()),
        Err(NumericDateError::Expired) => Err(kind.expired()),
    }
}

fn visible_ascii(value: &str, maximum: usize) -> bool {
    !value.is_empty()
        && value.len() <= maximum
        && value
            .bytes()
            .all(|byte| byte.is_ascii_graphic() && !byte.is_ascii_whitespace())
}
fn valid_revision(value: &str) -> bool {
    (1..=64).contains(&value.len())
        && value.is_ascii()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b':' | b'-'))
}
fn parse_generation(value: &str) -> Option<u64> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        None
    } else {
        value.parse().ok().filter(|value: &u64| *value != 0)
    }
}
fn canonical_uuid(value: &str, require_v7: bool) -> Option<[u8; 16]> {
    if value.len() != 36
        || !matches!(value.as_bytes().get(8), Some(b'-'))
        || !matches!(value.as_bytes().get(13), Some(b'-'))
        || !matches!(value.as_bytes().get(18), Some(b'-'))
        || !matches!(value.as_bytes().get(23), Some(b'-'))
    {
        return None;
    }
    let mut bytes = [0; 16];
    let mut input = value.bytes().filter(|byte| *byte != b'-');
    for byte in &mut bytes {
        let high = hex(input.next()?)?;
        let low = hex(input.next()?)?;
        *byte = (high << 4) | low;
    }
    if input.next().is_some()
        || bytes.iter().all(|byte| *byte == 0)
        || (bytes[8] & 0xc0) != 0x80
        || (require_v7 && (bytes[6] >> 4) != 7)
    {
        None
    } else {
        Some(bytes)
    }
}
fn hex(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn numeric_date_extremes_fail_closed_without_panicking() {
        for value in [i64::MIN, i64::MAX] {
            let result =
                std::panic::catch_unwind(|| NumericDate::validate(value, value, value, value));

            assert!(matches!(result, Ok(Err(NumericDateError::Malformed))));
        }
    }

    #[test]
    fn compact_jws_requires_bounded_exactly_three_ascii_segments() {
        for token in [
            String::new(),
            "e30.e30".to_owned(),
            "e30.e30.AA.extra".to_owned(),
            "a".repeat(4_097),
            "e30.é30.AA".to_owned(),
        ] {
            assert_eq!(
                parse_compact_jws(&token),
                Err(Fnd04VerificationError::Malformed)
            );
        }

        assert!(parse_compact_jws("e30.e30.AA").is_ok());
    }

    #[test]
    fn compact_jws_rejects_noncanonical_base64url_segments() {
        for token in ["e30=.e30.AA", "e30.e30.A+", "e30.e30.A"] {
            assert_eq!(
                parse_compact_jws(token),
                Err(Fnd04VerificationError::Malformed)
            );
        }
    }

    #[test]
    fn compact_jws_rejects_invalid_utf8_and_duplicate_json_members() {
        for token in ["__8.e30.AA", "e30.eyJhIjoxLCJhIjoyfQ.AA"] {
            assert_eq!(
                parse_compact_jws(token),
                Err(Fnd04VerificationError::Malformed)
            );
        }
    }

    #[test]
    fn protected_header_enforces_exact_members_and_algorithm_precedence()
    -> Result<(), Fnd04VerificationError> {
        let malformed = compact_token(r#"{"alg":"Ed25519","kid":"fresh"}"#);
        let non_exact_algorithm = compact_token(r#"{"alg":"EdDSA","kid":"fresh","typ":"x"}"#);
        let valid = compact_token(r#"{"alg":"Ed25519","kid":"fresh","typ":"x"}"#);

        let malformed_header = parse_compact_jws(&malformed)?;
        let non_exact_algorithm_header = parse_compact_jws(&non_exact_algorithm)?;
        let valid_header = parse_compact_jws(&valid)?;

        assert_eq!(
            parse_protected_header(&malformed_header),
            Err(Fnd04VerificationError::Malformed)
        );
        assert_eq!(
            parse_protected_header(&non_exact_algorithm_header),
            Err(Fnd04VerificationError::AuthenticationFailed)
        );
        assert!(parse_protected_header(&valid_header).is_ok());
        Ok(())
    }

    #[test]
    fn signature_validation_only_uses_verifier_fixed_trust_keys()
    -> Result<(), Fnd04VerificationError> {
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let header = r#"{"alg":"Ed25519","kid":"fresh","typ":"x"}"#;
        let encoded_header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
        let signing_input = format!("{encoded_header}.e30");
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(signing_key.sign(signing_input.as_bytes()).to_bytes());
        let token = format!("{signing_input}.{signature}");
        let compact_jws = parse_compact_jws(&token)?;
        let protected_header = parse_protected_header(&compact_jws)?;
        let trusted = FixedTrustContext {
            keys: [("fresh".to_owned(), signing_key.verifying_key().to_bytes())]
                .into_iter()
                .collect(),
        };
        let untrusted = FixedTrustContext {
            keys: [("other".to_owned(), signing_key.verifying_key().to_bytes())]
                .into_iter()
                .collect(),
        };

        assert!(verify_compact_signature(&compact_jws, &protected_header, &trusted).is_ok());
        assert_eq!(
            verify_compact_signature(&compact_jws, &protected_header, &untrusted),
            Err(Fnd04VerificationError::AuthenticationFailed)
        );
        Ok(())
    }

    #[test]
    fn fresh_consumer_returns_facts_only_after_fixed_context_and_current_evidence_match()
    -> Result<(), Fnd04ConsumerError> {
        let signing_key = SigningKey::from_bytes(&[9; 32]);
        let authority = fresh_authority("fresh-1", signing_key.verifying_key().to_bytes());
        let trust = FreshTrustContext::new(&authority);
        let current = FreshCurrentEvidence::for_test(100)?;
        let grant = signed_token(
            &signing_key,
            r#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#,
            fresh_payload(),
        );

        let facts = verify_fresh_grant(&grant, 100, &trust, &current)?;

        assert_eq!(facts.replay_key().to_bytes()[1..], [7; 32]);
        Ok(())
    }

    #[test]
    fn fresh_consumer_classifies_explicit_current_key_revocation_as_authentication_failure()
    -> Result<(), Fnd04ConsumerError> {
        let signing_key = SigningKey::from_bytes(&[10; 32]);
        let authority =
            TestAuthority::default().deny_key(Fnd04EvidenceScope::FreshAdmission, "fresh-1");
        let trust = FreshTrustContext::new(&authority);
        let grant = signed_token(
            &signing_key,
            r#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#,
            fresh_payload(),
        );

        assert_eq!(
            verify_fresh_grant(&grant, 100, &trust, &FreshCurrentEvidence::for_test(100)?),
            Err(Fnd04ConsumerError::FreshAuthenticationFailed),
        );
        Ok(())
    }

    #[test]
    fn revoked_kid_is_rejected_while_a_different_current_kid_remains_usable()
    -> Result<(), Fnd04ConsumerError> {
        let active = SigningKey::from_bytes(&[16; 32]);
        let revoked = SigningKey::from_bytes(&[17; 32]);
        let authority = TestAuthority::default()
            .key(
                Fnd04EvidenceScope::FreshAdmission,
                "active-1",
                active.verifying_key().to_bytes(),
            )
            .deny_key(Fnd04EvidenceScope::FreshAdmission, "revoked-1");
        let trust = FreshTrustContext::new(&authority);
        let current = FreshCurrentEvidence::for_test(100)?;
        let revoked_grant = signed_token(
            &revoked,
            r#"{"alg":"Ed25519","kid":"revoked-1","typ":"oteryn-admission+jwt"}"#,
            fresh_payload(),
        );
        assert_eq!(
            verify_fresh_grant(&revoked_grant, 100, &trust, &current),
            Err(Fnd04ConsumerError::FreshAuthenticationFailed),
        );

        let active_grant = signed_token(
            &active,
            r#"{"alg":"Ed25519","kid":"active-1","typ":"oteryn-admission+jwt"}"#,
            fresh_payload(),
        );
        assert!(verify_fresh_grant(&active_grant, 100, &trust, &current).is_ok());
        Ok(())
    }

    #[test]
    fn fresh_signing_key_cannot_be_reused_for_recovery_scope() -> Result<(), Fnd04ConsumerError> {
        let signing_key = SigningKey::from_bytes(&[19; 32]);
        let authority = fresh_authority("shared-1", signing_key.verifying_key().to_bytes());
        let trust = RecoveryTrustContext::new(&authority);
        let grant = signed_token(
            &signing_key,
            r#"{"alg":"Ed25519","kid":"shared-1","typ":"oteryn-recovery+jwt"}"#,
            recovery_payload(),
        );

        assert_eq!(
            verify_recovery_grant(
                &grant,
                100,
                &trust,
                &RecoveryCurrentEvidence::for_test(100)?,
            ),
            Err(Fnd04ConsumerError::RecoveryAuthenticationFailed),
        );
        Ok(())
    }

    #[test]
    fn unavailable_durable_nonrollback_floor_fails_closed() -> Result<(), Fnd04ConsumerError> {
        let signing_key = SigningKey::from_bytes(&[18; 32]);
        let authority = fresh_authority("fresh-1", signing_key.verifying_key().to_bytes())
            .unavailable_account();
        let trust = FreshTrustContext::new(&authority);
        let grant = signed_token(
            &signing_key,
            r#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#,
            fresh_payload(),
        );
        let current = FreshCurrentEvidence::for_test(100)?;

        assert_eq!(
            verify_fresh_grant(&grant, 100, &trust, &current),
            Err(Fnd04ConsumerError::FreshSecurityEvidenceStale),
        );
        Ok(())
    }

    #[test]
    fn recovery_consumer_returns_non_authoritative_facts_from_only_recovery_context()
    -> Result<(), Fnd04ConsumerError> {
        let signing_key = SigningKey::from_bytes(&[11; 32]);
        let authority = recovery_authority("recovery-1", signing_key.verifying_key().to_bytes());
        let trust = RecoveryTrustContext::new(&authority);
        let grant = signed_token(
            &signing_key,
            r#"{"alg":"Ed25519","kid":"recovery-1","typ":"oteryn-recovery+jwt"}"#,
            recovery_payload(),
        );

        let facts = verify_recovery_grant(
            &grant,
            100,
            &trust,
            &RecoveryCurrentEvidence::for_test(100)?,
        )?;

        assert_eq!(facts.grant_nonce(), [8; 32]);
        assert_eq!(facts.account_id(), "00000000-0000-4000-8000-000000000001");
        Ok(())
    }

    #[test]
    fn invalid_signature_masks_authenticated_schema_and_profile_classifications()
    -> Result<(), Fnd04ConsumerError> {
        let trusted_key = SigningKey::from_bytes(&[12; 32]);
        let untrusted_key = SigningKey::from_bytes(&[13; 32]);
        let authority = fresh_authority("fresh-1", trusted_key.verifying_key().to_bytes());
        let trust = FreshTrustContext::new(&authority);
        let malformed_schema = fresh_payload().replace(
            r#",\"offer_revision\":\"offer-1\""#,
            r#",\"unknown\":\"offer-1\""#,
        );
        let unsupported_profile =
            fresh_payload().replace("oteryn-pre-admission-v1", "oteryn-pre-admission-v2");
        for payload in [malformed_schema, unsupported_profile] {
            let grant = signed_token(
                &untrusted_key,
                r#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#,
                payload,
            );
            assert_eq!(
                verify_fresh_grant(&grant, 100, &trust, &FreshCurrentEvidence::for_test(100)?),
                Err(Fnd04ConsumerError::FreshAuthenticationFailed),
            );
        }
        Ok(())
    }

    #[test]
    fn authenticated_unsupported_profile_is_not_reinterpreted_as_fresh_admission()
    -> Result<(), Fnd04ConsumerError> {
        let signing_key = SigningKey::from_bytes(&[14; 32]);
        let authority = fresh_authority("fresh-1", signing_key.verifying_key().to_bytes());
        let trust = FreshTrustContext::new(&authority);
        let grant = signed_token(
            &signing_key,
            r#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#,
            fresh_payload().replace("oteryn-pre-admission-v1", "oteryn-pre-admission-v2"),
        );

        assert_eq!(
            verify_fresh_grant(&grant, 100, &trust, &FreshCurrentEvidence::for_test(100)?),
            Err(Fnd04ConsumerError::FreshRevisionUnsupported),
        );
        Ok(())
    }

    #[test]
    fn unavailable_current_security_evidence_fails_before_any_fresh_facts_are_returned()
    -> Result<(), Fnd04ConsumerError> {
        let signing_key = SigningKey::from_bytes(&[15; 32]);
        let authority = fresh_authority("fresh-1", signing_key.verifying_key().to_bytes())
            .unavailable_account();
        let trust = FreshTrustContext::new(&authority);
        let grant = signed_token(
            &signing_key,
            r#"{"alg":"Ed25519","kid":"fresh-1","typ":"oteryn-admission+jwt"}"#,
            fresh_payload(),
        );
        let current = FreshCurrentEvidence::for_test(100)?;

        assert_eq!(
            verify_fresh_grant(&grant, 100, &trust, &current),
            Err(Fnd04ConsumerError::FreshSecurityEvidenceStale),
        );
        Ok(())
    }

    fn compact_token(header: &str) -> String {
        let encoded_header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
        format!("{encoded_header}.e30.AA")
    }

    #[derive(Default)]
    struct TestAuthority {
        keys: BTreeMap<(Fnd04EvidenceScope, String), [u8; 32]>,
        denied_keys: BTreeSet<(Fnd04EvidenceScope, String)>,
        unavailable_account: bool,
    }

    impl TestAuthority {
        fn key(mut self, scope: Fnd04EvidenceScope, kid: &str, key: [u8; 32]) -> Self {
            self.keys.insert((scope, kid.to_owned()), key);
            self
        }

        fn deny_key(mut self, scope: Fnd04EvidenceScope, kid: &str) -> Self {
            self.denied_keys.insert((scope, kid.to_owned()));
            self
        }

        fn unavailable_account(mut self) -> Self {
            self.unavailable_account = true;
            self
        }
    }

    impl Fnd04EvidenceAuthority for TestAuthority {
        fn signing_key(
            &self,
            scope: Fnd04EvidenceScope,
            key_id: &str,
            _now: i64,
        ) -> Result<[u8; 32], Fnd04EvidenceError> {
            let key = (scope, key_id.to_owned());
            if self.denied_keys.contains(&key) {
                return Err(Fnd04EvidenceError::ExplicitlyDenied);
            }
            self.keys
                .get(&key)
                .copied()
                .ok_or(Fnd04EvidenceError::ExplicitlyDenied)
        }

        fn account_minimum_generation(
            &self,
            _scope: Fnd04EvidenceScope,
            _account_id: &str,
            _now: i64,
        ) -> Result<u64, Fnd04EvidenceError> {
            if self.unavailable_account {
                Err(Fnd04EvidenceError::UnavailableOrStale)
            } else {
                Ok(1)
            }
        }
    }

    fn fresh_authority(kid: &str, key: [u8; 32]) -> TestAuthority {
        TestAuthority::default().key(Fnd04EvidenceScope::FreshAdmission, kid, key)
    }

    fn recovery_authority(kid: &str, key: [u8; 32]) -> TestAuthority {
        TestAuthority::default().key(Fnd04EvidenceScope::ExistingActorRecovery, kid, key)
    }

    fn fresh_payload() -> String {
        let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([7; 32]);
        format!(
            r#"{{"iss":"urn:oteryn:platform:game-admission","aud":"urn:oteryn:game:admission","iat":100,"nbf":100,"exp":110,"jti":"{nonce}","profile":"oteryn-pre-admission-v1","purpose":"fresh_entry","attempt_ref":"00000000-0000-7000-8000-000000000001","account_id":"00000000-0000-4000-8000-000000000001","character_id":"00000000-0000-7000-8000-000000000002","world_id":"00000000-0000-7000-8000-000000000003","channel_id":"00000000-0000-7000-8000-000000000004","account_security_generation":"1","route_revision":"route-1","runtime_observation_revision":"runtime-1","scope_ownership_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"rules-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1","offer_revision":"offer-1"}}"#
        )
    }

    fn signed_token(signing_key: &SigningKey, header: &str, payload: String) -> String {
        let encoded_header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
        let encoded_payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload);
        let signing_input = format!("{encoded_header}.{encoded_payload}");
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(signing_key.sign(signing_input.as_bytes()).to_bytes());
        format!("{signing_input}.{signature}")
    }

    fn recovery_payload() -> String {
        let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([8; 32]);
        format!(
            r#"{{"iss":"urn:oteryn:platform:game-recovery","aud":"urn:oteryn:game:recovery","iat":100,"nbf":100,"exp":110,"jti":"{nonce}","profile":"oteryn-reauth-recovery-v1","purpose":"existing_actor_recovery","attempt_ref":"00000000-0000-7000-8000-000000000001","account_id":"00000000-0000-4000-8000-000000000001","character_id":"00000000-0000-7000-8000-000000000002","world_id":"00000000-0000-7000-8000-000000000003","account_security_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"rules-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1"}}"#
        )
    }

    impl FreshCurrentEvidence {
        fn for_test(_now: i64) -> Result<Self, Fnd04ConsumerError> {
            let id = |last: u8| {
                let mut value = [0u8; 16];
                value[6] = 0x70;
                value[8] = 0x80;
                value[15] = last;
                value
            };
            Ok(Self {
                account_id: "00000000-0000-4000-8000-000000000001".to_owned(),
                character_id: CharacterId::decode(&id(2))
                    .map_err(|_| Fnd04ConsumerError::FreshMalformed)?,
                world_id: WorldId::decode(&id(3))
                    .map_err(|_| Fnd04ConsumerError::FreshMalformed)?,
                channel_id: ChannelId::decode(&id(4))
                    .map_err(|_| Fnd04ConsumerError::FreshMalformed)?,
                character_lease_generation: 1,
                route_revision: "route-1".to_owned(),
                runtime_observation_revision: "runtime-1".to_owned(),
                scope_ownership_generation: 1,
                ruleset_revision: "rules-1".to_owned(),
                content_revision: "content-1".to_owned(),
                map_revision: "map-1".to_owned(),
                world_policy_revision: "policy-1".to_owned(),
                offer_revision: "offer-1".to_owned(),
            })
        }
    }

    impl RecoveryCurrentEvidence {
        fn for_test(now: i64) -> Result<Self, Fnd04ConsumerError> {
            let fresh = FreshCurrentEvidence::for_test(now)?;
            Ok(Self {
                account_id: fresh.account_id,
                character_id: fresh.character_id,
                world_id: fresh.world_id,
                ruleset_revision: fresh.ruleset_revision,
                content_revision: fresh.content_revision,
                map_revision: fresh.map_revision,
                world_policy_revision: fresh.world_policy_revision,
            })
        }
    }
}

#[cfg(test)]
mod durability_evidence_v1_tests {
    use super::*;
    use base64::Engine;
    use ed25519_dalek::{Signer, SigningKey};

    struct RecoveryAuthority {
        key: [u8; 32],
    }

    impl Fnd04EvidenceAuthority for RecoveryAuthority {
        fn signing_key(
            &self,
            scope: Fnd04EvidenceScope,
            key_id: &str,
            _now: i64,
        ) -> Result<[u8; 32], Fnd04EvidenceError> {
            if scope == Fnd04EvidenceScope::ExistingActorRecovery && key_id == "recovery-1" {
                Ok(self.key)
            } else {
                Err(Fnd04EvidenceError::ExplicitlyDenied)
            }
        }

        fn account_minimum_generation(
            &self,
            scope: Fnd04EvidenceScope,
            _account_id: &str,
            _now: i64,
        ) -> Result<u64, Fnd04EvidenceError> {
            if scope == Fnd04EvidenceScope::ExistingActorRecovery {
                Ok(1)
            } else {
                Err(Fnd04EvidenceError::ExplicitlyDenied)
            }
        }
    }

    fn id(last: u8) -> [u8; 16] {
        let mut value = [0u8; 16];
        value[6] = 0x70;
        value[8] = 0x80;
        value[15] = last;
        value
    }

    fn recovery_payload() -> String {
        let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode([8; 32]);
        format!(
            r#"{{"iss":"urn:oteryn:platform:game-recovery","aud":"urn:oteryn:game:recovery","iat":100,"nbf":100,"exp":110,"jti":"{nonce}","profile":"oteryn-reauth-recovery-v1","purpose":"existing_actor_recovery","attempt_ref":"00000000-0000-7000-8000-000000000001","account_id":"00000000-0000-4000-8000-000000000001","character_id":"00000000-0000-7000-8000-000000000002","world_id":"00000000-0000-7000-8000-000000000003","account_security_generation":"1","protocol_major":1,"transport_profile":1,"ruleset_revision":"rules-1","content_revision":"content-1","map_revision":"map-1","world_policy_revision":"policy-1"}}"#
        )
    }

    fn signed_recovery(signing_key: &SigningKey) -> String {
        let header = r#"{"alg":"Ed25519","kid":"recovery-1","typ":"oteryn-recovery+jwt"}"#;
        let encoded_header = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header);
        let encoded_payload =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(recovery_payload());
        let signing_input = format!("{encoded_header}.{encoded_payload}");
        let signature = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(signing_key.sign(signing_input.as_bytes()).to_bytes());
        format!("{signing_input}.{signature}")
    }

    #[test]
    fn recovery_durability_facts_preserve_signed_security_revision_and_expiry()
    -> Result<(), Fnd04ConsumerError> {
        let signing_key = SigningKey::from_bytes(&[23; 32]);
        let authority = RecoveryAuthority {
            key: signing_key.verifying_key().to_bytes(),
        };
        let trust = RecoveryTrustContext::new(&authority);
        let current = RecoveryCurrentEvidence {
            account_id: "00000000-0000-4000-8000-000000000001".to_owned(),
            character_id: CharacterId::decode(&id(2))
                .map_err(|_| Fnd04ConsumerError::RecoveryMalformed)?,
            world_id: WorldId::decode(&id(3))
                .map_err(|_| Fnd04ConsumerError::RecoveryMalformed)?,
            ruleset_revision: "rules-1".to_owned(),
            content_revision: "content-1".to_owned(),
            map_revision: "map-1".to_owned(),
            world_policy_revision: "policy-1".to_owned(),
        };

        let facts = verify_recovery_grant_durability_v1(
            &signed_recovery(&signing_key),
            100,
            &trust,
            &current,
        )?;

        assert_eq!(facts.grant_nonce(), [8; 32]);
        assert_eq!(facts.account_id(), current.account_id);
        assert_eq!(facts.character_id(), current.character_id);
        assert_eq!(facts.world_id(), current.world_id);
        assert_eq!(facts.account_security_generation(), 1);
        assert_eq!(facts.protocol_major(), 1);
        assert_eq!(facts.transport_profile(), 1);
        assert_eq!(facts.ruleset_revision(), "rules-1");
        assert_eq!(facts.content_revision(), "content-1");
        assert_eq!(facts.map_revision(), "map-1");
        assert_eq!(facts.world_policy_revision(), "policy-1");
        assert_eq!(facts.credential_expiration(), 110);
        Ok(())
    }
}
