//! `CHARACTER_AUTHENTICATED_BOOTSTRAP_INTENT_V1` consumer data.
//!
//! The Platform Account authority owns and authenticates the intent; Game only
//! decodes the exact bounded producer wire and binds it to the Character
//! operation receipt. Only the `OPERATOR_CONTROL_PLANE_BOOTSTRAP` variant is
//! enabled. Outside this crate a [`CharacterBootstrapIntentV1`] is obtained only
//! from [`read_authenticated_intent`], which couples the strict decoder to the
//! purpose-separated TLS 1.3 mTLS reconciliation read and the configured issuer
//! trust; there is no public constructor from caller-supplied bytes or fields.

use oteryn_game_server::domain::{AccountId, WorldId};
use oteryn_game_server::native_admission_source::{
    QueuePermit, SourceError, descriptor::ProducerDescriptor, read_character_bootstrap_intent,
};
use serde::Deserialize;

pub const CONTRACT_VERSION: u8 = 1;
pub const VARIANT: &str = "OPERATOR_CONTROL_PLANE_BOOTSTRAP";
pub const ISSUER_AUTHORITY: &str = "OTERYN_PLATFORM_CHARACTER_AUTHORITY";
pub const OPERATION: &str = "INITIAL_CHARACTER_BOOTSTRAP";
pub const AUDIENCE: &str = "OTERYN_GAME_CHARACTER_AUTHORITY";
pub const MAX_RESPONSE_BYTES: usize = 4096;
/// Technical transport/reconciliation bound of the producer contract.
pub const MAX_TTL_SECONDS: i64 = 300;
const MAX_REVISION_BYTES: usize = 128;
/// Binding format tag: V1 operator variant of the Platform Character authority
/// for the Game Character audience. The constant fields are implied by the tag.
const BINDING_TAG_V1_OPERATOR: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidIntent;

/// One authenticated, immutable Platform bootstrap decision.
#[derive(Clone, PartialEq, Eq)]
pub struct CharacterBootstrapIntentV1 {
    issuer_decision_id: [u8; 16],
    source_revision: i64,
    operation_id: [u8; 16],
    account_id: AccountId,
    target_world_id: WorldId,
    profile_revision: String,
    ruleset_revision: String,
    content_revision: String,
    starter_template_revision: String,
    issued_at_source: i64,
    expires_at_source: i64,
}

// Player-linked identities are never written to diagnostics.
impl std::fmt::Debug for CharacterBootstrapIntentV1 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CharacterBootstrapIntentV1")
            .field("source_revision", &self.source_revision)
            .finish_non_exhaustive()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Wire {
    contract_version: u8,
    variant: String,
    issuer_authority: String,
    issuer_decision_id: String,
    source_revision: String,
    operation_id: String,
    operation: String,
    account_id: String,
    target_world_id: String,
    interpretation_context: WireContext,
    issued_at_source: String,
    expires_at_source: String,
    audience: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireContext {
    profile_revision: String,
    ruleset_revision: String,
    content_revision: String,
    starter_template_revision: String,
}

impl CharacterBootstrapIntentV1 {
    #[must_use]
    pub const fn issuer_decision_id(&self) -> [u8; 16] {
        self.issuer_decision_id
    }
    #[must_use]
    pub const fn source_revision(&self) -> i64 {
        self.source_revision
    }
    #[must_use]
    pub const fn operation_id(&self) -> [u8; 16] {
        self.operation_id
    }
    #[must_use]
    pub const fn account_id(&self) -> AccountId {
        self.account_id
    }
    #[must_use]
    pub const fn target_world_id(&self) -> WorldId {
        self.target_world_id
    }
    #[must_use]
    pub fn interpretation(&self) -> [&str; 4] {
        [
            &self.profile_revision,
            &self.ruleset_revision,
            &self.content_revision,
            &self.starter_template_revision,
        ]
    }
    #[must_use]
    pub const fn issued_at_source(&self) -> i64 {
        self.issued_at_source
    }
    #[must_use]
    pub const fn expires_at_source(&self) -> i64 {
        self.expires_at_source
    }

    /// Complete canonical semantic binding retained with the operation receipt.
    /// Equal operation identity with any other binding is conflicting reuse.
    #[must_use]
    pub fn binding(&self) -> Vec<u8> {
        let mut value = Vec::with_capacity(1 + 16 * 4 + 8 * 3 + 4 * (2 + MAX_REVISION_BYTES));
        value.push(BINDING_TAG_V1_OPERATOR);
        value.extend_from_slice(&self.issuer_decision_id);
        value.extend_from_slice(&self.source_revision.to_be_bytes());
        value.extend_from_slice(&self.operation_id);
        value.extend_from_slice(self.account_id.as_bytes());
        value.extend_from_slice(self.target_world_id.as_bytes());
        value.extend_from_slice(&self.issued_at_source.to_be_bytes());
        value.extend_from_slice(&self.expires_at_source.to_be_bytes());
        for part in self.interpretation() {
            // Revisions are at most 128 bytes, so the length always fits.
            value.extend_from_slice(&(part.len() as u16).to_be_bytes());
            value.extend_from_slice(part.as_bytes());
        }
        value
    }
}

/// Canonical lower-case UUID text of any RFC version 1-8.
fn uuid(value: &str) -> Result<[u8; 16], InvalidIntent> {
    let bytes = value.as_bytes();
    if bytes.len() != 36
        || !matches!(bytes[14], b'1'..=b'8')
        || !matches!(bytes[19], b'8' | b'9' | b'a' | b'b')
    {
        return Err(InvalidIntent);
    }
    let mut out = [0u8; 16];
    let mut nibbles = bytes
        .iter()
        .enumerate()
        .filter(|(index, _)| !matches!(index, 8 | 13 | 18 | 23));
    for byte in &mut out {
        let mut next = || -> Result<u8, InvalidIntent> {
            match nibbles.next() {
                Some((_, digit @ b'0'..=b'9')) => Ok(digit - b'0'),
                Some((_, digit @ b'a'..=b'f')) => Ok(digit - b'a' + 10),
                _ => Err(InvalidIntent),
            }
        };
        *byte = (next()? << 4) | next()?;
    }
    if [8, 13, 18, 23].iter().any(|&index| bytes[index] != b'-') {
        return Err(InvalidIntent);
    }
    Ok(out)
}

/// Canonical decimal (`0|[1-9][0-9]*`) within `minimum..=i64::MAX`.
fn decimal(value: &str, minimum: i64) -> Result<i64, InvalidIntent> {
    if value.is_empty()
        || value.len() > 19
        || !value.bytes().all(|b| b.is_ascii_digit())
        || (value.len() > 1 && value.starts_with('0'))
    {
        return Err(InvalidIntent);
    }
    let parsed = value.parse::<i64>().map_err(|_| InvalidIntent)?;
    if parsed < minimum {
        return Err(InvalidIntent);
    }
    Ok(parsed)
}

fn revision(value: String) -> Result<String, InvalidIntent> {
    let bytes = value.as_bytes();
    if bytes.is_empty()
        || bytes.len() > MAX_REVISION_BYTES
        || !bytes[0].is_ascii_alphanumeric()
        || !bytes
            .iter()
            .all(|b| b.is_ascii_alphanumeric() || b"._:-".contains(b))
    {
        return Err(InvalidIntent);
    }
    Ok(value)
}

/// Strictly decode one producer response for the exact requested operation.
/// Unknown, duplicate or missing members, any other variant, issuer, operation
/// or audience, non-canonical identities or numbers and an out-of-bound source
/// validity window are rejected.
pub(crate) fn decode_producer_response(
    raw: &[u8],
    expected_operation_id: [u8; 16],
) -> Result<CharacterBootstrapIntentV1, InvalidIntent> {
    if raw.is_empty() || raw.len() > MAX_RESPONSE_BYTES {
        return Err(InvalidIntent);
    }
    let wire: Wire = serde_json::from_slice(raw).map_err(|_| InvalidIntent)?;
    if wire.contract_version != CONTRACT_VERSION
        || wire.variant != VARIANT
        || wire.issuer_authority != ISSUER_AUTHORITY
        || wire.operation != OPERATION
        || wire.audience != AUDIENCE
    {
        return Err(InvalidIntent);
    }
    let operation_id = uuid(&wire.operation_id)?;
    // Game operation identities are UUIDv7 (version and RFC variant).
    if operation_id != expected_operation_id
        || operation_id[6] >> 4 != 7
        || operation_id[8] & 0xc0 != 0x80
    {
        return Err(InvalidIntent);
    }
    let issued_at_source = decimal(&wire.issued_at_source, 0)?;
    let expires_at_source = decimal(&wire.expires_at_source, 0)?;
    if expires_at_source <= issued_at_source
        || expires_at_source - issued_at_source > MAX_TTL_SECONDS
    {
        return Err(InvalidIntent);
    }
    Ok(CharacterBootstrapIntentV1 {
        issuer_decision_id: uuid(&wire.issuer_decision_id)?,
        source_revision: decimal(&wire.source_revision, 1)?,
        operation_id,
        account_id: AccountId::from_bytes(uuid(&wire.account_id)?).map_err(|_| InvalidIntent)?,
        target_world_id: WorldId::from_bytes(uuid(&wire.target_world_id)?)
            .map_err(|_| InvalidIntent)?,
        profile_revision: revision(wire.interpretation_context.profile_revision)?,
        ruleset_revision: revision(wire.interpretation_context.ruleset_revision)?,
        content_revision: revision(wire.interpretation_context.content_revision)?,
        starter_template_revision: revision(wire.interpretation_context.starter_template_revision)?,
        issued_at_source,
        expires_at_source,
    })
}

/// Why an authenticated intent read produced no intent.
#[derive(Debug)]
pub enum IntentReadError {
    /// Transport, peer authentication or producer outcome (bounded unavailable).
    Source(SourceError),
    /// The authenticated producer body is not an exact V1 decision.
    Invalid(InvalidIntent),
}

/// Read one operation's current intent from the configured Platform issuer over
/// the authenticated mTLS route and strictly decode that exact body. This is
/// the only public way to obtain a [`CharacterBootstrapIntentV1`].
pub async fn read_authenticated_intent(
    descriptor: &ProducerDescriptor,
    operation_id: [u8; 16],
    permit: &mut QueuePermit<'_>,
) -> Result<CharacterBootstrapIntentV1, IntentReadError> {
    let raw =
        read_character_bootstrap_intent(descriptor, &encode_read_request(operation_id), permit)
            .await
            .map_err(IntentReadError::Source)?;
    decode_producer_response(&raw, operation_id).map_err(IntentReadError::Invalid)
}

/// Game-owned Character interpretation context. The current value is durable
/// operator configuration (`configure_character_interpretation`) that bootstrap
/// resolves inside its own transaction. Intent revisions are requested context,
/// never authority: bootstrap commits only when they equal it exactly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharacterInterpretationV1 {
    revisions: [String; 4],
}

impl CharacterInterpretationV1 {
    pub fn new(
        profile_revision: &str,
        ruleset_revision: &str,
        content_revision: &str,
        starter_template_revision: &str,
    ) -> Result<Self, InvalidIntent> {
        Ok(Self {
            revisions: [
                revision(profile_revision.to_owned())?,
                revision(ruleset_revision.to_owned())?,
                revision(content_revision.to_owned())?,
                revision(starter_template_revision.to_owned())?,
            ],
        })
    }

    #[must_use]
    pub fn revisions(&self) -> [&str; 4] {
        [
            &self.revisions[0],
            &self.revisions[1],
            &self.revisions[2],
            &self.revisions[3],
        ]
    }

    #[must_use]
    pub fn admits(&self, intent: &CharacterBootstrapIntentV1) -> bool {
        self.revisions
            .iter()
            .map(String::as_str)
            .eq(intent.interpretation())
    }
}

/// Exact bounded reconciliation request body for one operation.
#[must_use]
pub fn encode_read_request(operation_id: [u8; 16]) -> String {
    let hex: String = operation_id.iter().map(|b| format!("{b:02x}")).collect();
    format!(
        "{{\"contract_version\":1,\"operation_id\":\"{}-{}-{}-{}-{}\"}}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPERATION_ID: [u8; 16] = [
        0x01, 0x89, 0x0f, 0x4e, 0x7c, 0x00, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, 1,
    ];

    fn wire(edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
        let mut value = serde_json::json!({
            "contract_version": 1,
            "variant": VARIANT,
            "issuer_authority": ISSUER_AUTHORITY,
            "issuer_decision_id": "3f0c5b7e-1d2a-4c3b-9a8f-0123456789ab",
            "source_revision": "7",
            "operation_id": "01890f4e-7c00-7000-8000-000000000001",
            "operation": OPERATION,
            "account_id": "01934f10-7c00-7000-8000-000000000001",
            "target_world_id": "01934f10-7c00-7000-8000-0000000000aa",
            "interpretation_context": {
                "profile_revision": "profile-1",
                "ruleset_revision": "ruleset-1",
                "content_revision": "content-1",
                "starter_template_revision": "starter-1"
            },
            "issued_at_source": "1000",
            "expires_at_source": "1120",
            "audience": AUDIENCE
        });
        edit(&mut value);
        serde_json::to_vec(&value).unwrap_or_default()
    }

    #[test]
    fn exact_producer_wire_decodes_and_binds_every_field() -> Result<(), InvalidIntent> {
        let intent = decode_producer_response(&wire(|_| {}), OPERATION_ID)?;
        assert_eq!(intent.source_revision(), 7);
        assert_eq!(intent.operation_id(), OPERATION_ID);
        assert_eq!(intent.expires_at_source() - intent.issued_at_source(), 120);
        assert_eq!(intent.binding().len(), 1 + 16 * 4 + 8 * 3 + 4 * 11);
        let changed = decode_producer_response(
            &wire(|v| v["interpretation_context"]["content_revision"] = "content-2".into()),
            OPERATION_ID,
        )?;
        assert_ne!(changed.binding(), intent.binding());
        assert!(!format!("{intent:?}").contains("01934f10"));
        Ok(())
    }

    #[test]
    fn every_non_contract_wire_is_rejected() {
        let rejected: Vec<Vec<u8>> = vec![
            wire(|v| v["variant"] = "PLATFORM_USER_CREATE".into()),
            wire(|v| v["contract_version"] = 2.into()),
            wire(|v| v["issuer_authority"] = "OTHER".into()),
            wire(|v| v["operation"] = "RENAME".into()),
            wire(|v| v["audience"] = "OTHER".into()),
            wire(|v| v["source_revision"] = "0".into()),
            wire(|v| v["source_revision"] = "07".into()),
            wire(|v| v["source_revision"] = "9223372036854775808".into()),
            wire(|v| v["source_revision"] = 7.into()),
            wire(|v| v["operation_id"] = "01890f4e-7c00-7000-8000-000000000002".into()),
            wire(|v| v["account_id"] = "01934F10-7c00-7000-8000-000000000001".into()),
            wire(|v| v["account_id"] = "01934f10-7c00-4000-8000-000000000001".into()),
            wire(|v| v["target_world_id"] = "not-a-uuid".into()),
            wire(|v| v["interpretation_context"]["profile_revision"] = "-leading".into()),
            wire(|v| v["interpretation_context"]["profile_revision"] = "x".repeat(129).into()),
            wire(|v| v["expires_at_source"] = "1000".into()),
            wire(|v| v["expires_at_source"] = "1301".into()),
            wire(|v| v["extra"] = "x".into()),
            wire(|v| v["interpretation_context"]["extra"] = "x".into()),
            wire(|v| {
                if let Some(map) = v.as_object_mut() {
                    map.remove("audience");
                }
            }),
            br#"{"contract_version":1,"contract_version":1}"#.to_vec(),
            vec![b' '; MAX_RESPONSE_BYTES + 1],
            Vec::new(),
        ];
        for raw in rejected {
            assert_eq!(
                decode_producer_response(&raw, OPERATION_ID),
                Err(InvalidIntent)
            );
        }
        let mut duplicated = wire(|_| {});
        duplicated.pop();
        duplicated.extend_from_slice(br#","audience":"OTERYN_GAME_CHARACTER_AUTHORITY"}"#);
        assert_eq!(
            decode_producer_response(&duplicated, OPERATION_ID),
            Err(InvalidIntent)
        );
    }

    #[test]
    fn read_request_is_the_exact_bounded_producer_request() {
        let request = encode_read_request(OPERATION_ID);
        assert_eq!(
            request,
            r#"{"contract_version":1,"operation_id":"01890f4e-7c00-7000-8000-000000000001"}"#
        );
        assert!(request.len() <= 256);
    }
}
