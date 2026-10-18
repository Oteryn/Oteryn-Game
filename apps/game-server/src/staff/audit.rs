//! D1343 STAFF_ACTION payload contract. Event numbers come from the CP registry;
//! registration alone grants no runtime mutation or collection authority.
use super::signature::{self, Error, Outcome, VerifiedRequest};
use crate::durability::item_mint_audit::{EventEnvelopeV1, TransactionEventRefV1};
use oteryn_protocol_oteryn::{
    FoundationProtocolError, bounded_length_delimited, read_varint, skip_field,
};
use prost::Message;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const RETENTION_PROFILE: &str = "STAFF_ACTION_AUDIT_RETENTION_V1";
/// Finite ordinary ceiling: 365 elapsed days. Retained domain evidence,
/// receipts, roster revisions and their links live as long as relying records.
pub const RETENTION_MS: i64 = 31_536_000_000;
pub const MAX_PAYLOAD_BYTES: usize = 49_152;
pub const MAX_ENVELOPE_BYTES: usize = 50_176;

/// Untrusted SQL mirrors are checked as well as the immutable event bytes.
#[derive(Clone)]
pub struct StoredEvent {
    pub environment: String,
    pub operation_id: String,
    pub envelope: Vec<u8>,
    pub event_id: String,
    pub transaction_id: String,
    pub event_type: i64,
    pub schema_revision: i64,
    pub retention_profile: String,
    pub occurred_at: i64,
    pub expires_at: i64,
    pub envelope_sha256: String,
    pub publication_state: i16,
}

/// Retained operation mirrors must verify even after its finite audit expires.
#[derive(Clone)]
pub struct StoredOperation {
    pub environment: String,
    pub operation_id: String,
    pub request_sha256: String,
    pub request_bytes: Vec<u8>,
    pub outcome_bytes: Vec<u8>,
    pub outcome_sha256: String,
    pub principal: String,
    pub fingerprint: String,
    pub action_id: String,
    pub transaction_id: String,
    pub control_seq: u64,
    pub requires_receipt: bool,
    pub occurred_at: i64,
}
pub fn verify_stored_operation(
    request: &VerifiedRequest,
    row: &StoredOperation,
) -> Result<Outcome, Error> {
    let intent = request.request();
    let outcome: Outcome =
        oteryn_release_manifest::jcs::decode(&row.outcome_bytes, signature::MAX_OUTCOME_BYTES)
            .map_err(|_| Error::Invalid)?;
    outcome.canonical_bytes()?;
    signature::check_execution_stamp(intent, outcome.occurred_at)?;
    if (outcome.disposition == signature::Disposition::Committed)
        != (outcome.result == ResultCode::Ok)
        || row.environment != intent.environment
        || row.operation_id != intent.operation_id
        || row.request_sha256 != request.digest()
        || row.request_bytes != request.signed().bytes
        || row.outcome_sha256 != signature::sha256(&row.outcome_bytes)
        || row.principal != request.signed().principal
        || row.fingerprint != request.fingerprint()
        || row.action_id != outcome.action_id
        || row.transaction_id != outcome.transaction_id
        || row.control_seq != intent.control_seq.0
        || row.requires_receipt == intent.action.kind().is_read()
        || u64::try_from(row.occurred_at).ok() != Some(outcome.occurred_at.0)
    {
        return Err(Error::Binding);
    }
    Ok(outcome)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ActionKind {
    Inspect,
    ReportsList,
    EconomyTrace,
    Kick,
    Mute,
    Unmute,
    ReportClose,
    ReportClearText,
    RosterGrant,
    RosterRevoke,
    RosterMarkRestore,
    RestoreResolve,
    RosterActivate,
    MaintenanceOpen,
    MaintenanceEnd,
    Teleport,
    EconFenceRaise,
    EconFenceLift,
    EconCaseHoldPlace,
    EconCaseHoldRelease,
    EconRemediationPropose,
    EconRemediationApprove,
    EconRemediationResign,
    EconRemediationStep,
    EconRemediationAbort,
    PrivacyExport,
    PrivacyHoldPlace,
    PrivacyHoldExtend,
    PrivacyHoldRelease,
}
impl ActionKind {
    pub fn is_read(self) -> bool {
        matches!(
            self,
            Self::Inspect | Self::ReportsList | Self::EconomyTrace | Self::PrivacyExport
        )
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Reason {
    Bot,
    Harassment,
    Name,
    Cheating,
    BugAbuse,
    Support,
    PlatformSecurity,
    Roster,
    Restore,
    Maintenance,
    LegalHold,
    PrivacyDsr,
    DetectorSignal,
    IntegrityEcode,
    SupportReport,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ResultCode {
    Ok,
    RoleRevoked,
    TierInsufficient,
    TargetNotFound,
    TargetNotInChannel,
    Stale,
    AlreadyReleased,
    RateLimited,
}

#[derive(Debug, Clone, Copy)]
pub struct Contract {
    event_type: u32,
    revision: u32,
}
impl Contract {
    pub fn registered() -> Result<Self, Error> {
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json"
        ))
        .map_err(|_| Error::Invalid)?;
        let event = registry["event_types"]
            .as_array()
            .and_then(|rows| rows.iter().find(|row| row["name"] == "STAFF_ACTION"))
            .ok_or(Error::MissingMaterial)?;
        if event["durability_class"] != "DURABLE_AUDIT"
            || event["current_schema_revision"] != 1
            || event["payload_schema"] != "docs/contracts/game-events/v1/staff_action.proto"
            || event["payload_message"] != "oteryn.events.v1.StaffActionV1"
            || event["privacy_class_floor"] != "RESTRICTED_PLAYER_LINKED"
            || event["retention_profile_id"] != RETENTION_PROFILE
            || event["atomic_mutation_evidence"] != true
            || !registry["retention_profiles"]
                .as_array()
                .is_some_and(|rows| rows.iter().any(|r| r["id"] == RETENTION_PROFILE))
        {
            return Err(Error::Binding);
        }
        let event_type = event["id"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .filter(|n| *n > 0)
            .ok_or(Error::Invalid)?;
        let revision = event["current_schema_revision"]
            .as_u64()
            .and_then(|n| u32::try_from(n).ok())
            .filter(|n| *n > 0)
            .ok_or(Error::Invalid)?;
        Ok(Self {
            event_type,
            revision,
        })
    }
    pub fn event_type(self) -> u32 {
        self.event_type
    }
    pub fn revision(self) -> u32 {
        self.revision
    }
    #[cfg(test)]
    pub fn fixture() -> Self {
        Self {
            event_type: 4,
            revision: 1,
        }
    }
}

/// Registered game-events/v1/staff_action.proto. IDs 1–17 are payload
/// field numbers, not an allocation of the registry's event type number.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct StaffActionV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub action_id: Vec<u8>,
    #[prost(string, tag = "2")]
    pub actor_kind: String,
    #[prost(string, tag = "3")]
    pub action_kind: String,
    #[prost(bytes = "vec", tag = "4")]
    pub target_jcs: Vec<u8>,
    #[prost(string, tag = "5")]
    pub reason: String,
    #[prost(string, tag = "6")]
    pub result: String,
    #[prost(bytes = "vec", tag = "7")]
    pub signed_request: Vec<u8>,
    #[prost(string, tag = "8")]
    pub principal: String,
    #[prost(string, tag = "9")]
    pub key_fingerprint: String,
    #[prost(string, tag = "10")]
    pub signature: String,
    #[prost(string, tag = "11")]
    pub roster_sha256: String,
    #[prost(string, tag = "12")]
    pub request_sha256: String,
    #[prost(bytes = "vec", tag = "13")]
    pub parameters_jcs: Vec<u8>,
    #[prost(string, tag = "14")]
    pub trace_id: String,
    #[prost(bytes = "vec", tag = "15")]
    pub basis_refs_jcs: Vec<u8>,
    #[prost(bytes = "vec", tag = "16")]
    pub outcome_jcs: Vec<u8>,
    #[prost(string, tag = "17")]
    pub environment: String,
}

fn name<T: Serialize>(value: &T) -> Result<String, Error> {
    serde_json::to_value(value)
        .ok()
        .and_then(|v| v.as_str().map(str::to_owned))
        .ok_or(Error::Invalid)
}

fn encode_bounded(message: &impl Message, maximum: usize) -> Result<Vec<u8>, Error> {
    if message.encoded_len() > maximum {
        return Err(Error::Bounds);
    }
    Ok(message.encode_to_vec())
}

/// The bounded server build identity: 1..=128 visible ASCII bytes. The
/// default `oteryn-game-server/<version>` carries a `/`, so the signed-field
/// `token` alphabet does not apply to it.
fn build_id(text: &str) -> bool {
    !text.is_empty() && text.len() <= 128 && text.bytes().all(|b| b.is_ascii_graphic())
}

pub fn encode(
    contract: Contract,
    request: &VerifiedRequest,
    outcome: &Outcome,
    occurred_at: i64,
    server_build: &str,
) -> Result<Vec<u8>, Error> {
    if occurred_at <= 0
        || u64::try_from(occurred_at).ok() != Some(outcome.occurred_at.0)
        || !build_id(server_build)
    {
        return Err(Error::Invalid);
    }
    let action = request.request();
    signature::check_execution_stamp(action, outcome.occurred_at)?;
    let event_id = crate::node::uuid_bytes(&outcome.action_id).ok_or(Error::Invalid)?;
    let transaction = crate::node::uuid_bytes(&outcome.transaction_id).ok_or(Error::Invalid)?;
    let operation = crate::node::uuid_bytes(&action.operation_id).ok_or(Error::Invalid)?;
    let payload = StaffActionV1 {
        action_id: event_id.to_vec(),
        actor_kind: "OPERATOR".into(),
        action_kind: name(&action.action.kind())?,
        target_jcs: oteryn_release_manifest::jcs::encode(
            &action.action.target(),
            signature::MAX_REQUEST_BYTES,
        )
        .map_err(|_| Error::Invalid)?,
        reason: name(&action.reason)?,
        result: name(&outcome.result)?,
        signed_request: request.signed().bytes.clone(),
        principal: request.signed().principal.clone(),
        key_fingerprint: request.fingerprint().into(),
        signature: request.signed().signature.clone(),
        roster_sha256: action.roster_sha256.clone(),
        request_sha256: request.digest().into(),
        parameters_jcs: oteryn_release_manifest::jcs::encode(
            &action.action,
            signature::MAX_REQUEST_BYTES,
        )
        .map_err(|_| Error::Invalid)?,
        trace_id: outcome.transaction_id.clone(),
        basis_refs_jcs: b"[]".to_vec(),
        outcome_jcs: outcome.canonical_bytes()?,
        environment: action.environment.clone(),
    };
    let payload = encode_bounded(&payload, MAX_PAYLOAD_BYTES)?;
    let scope = action.scope.as_ref();
    let envelope = EventEnvelopeV1 {
        envelope_revision: 1,
        event_id: event_id.to_vec(),
        event_type_id: contract.event_type,
        event_schema_revision: contract.revision,
        durability_class: 2,
        privacy_class: 3,
        retention_profile_id: RETENTION_PROFILE.into(),
        occurred_at_unix_ms: occurred_at,
        world_id: scope
            .and_then(|s| crate::node::uuid_bytes(&s.world_id))
            .map(Vec::from),
        channel_id: scope
            .and_then(|s| crate::node::uuid_bytes(&s.channel_id))
            .map(Vec::from),
        operation_id: Some(operation.to_vec()),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: transaction.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        server_build_id: server_build.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..EventEnvelopeV1::default()
    };
    encode_bounded(&envelope, MAX_ENVELOPE_BYTES)
}

#[derive(Clone, Copy)]
enum AuditWireMessage {
    Envelope,
    Transaction,
}

// Prost discards unknown fields and merges duplicate singular fields. Refuse
// both before decoding, without imposing a canonical field serialization order.
fn validate_audit_wire(bytes: &[u8], message: AuditWireMessage) -> Result<(), Error> {
    let mut cursor = 0;
    let mut seen = 0_u32;
    while cursor < bytes.len() {
        let key = read_varint(bytes, &mut cursor).map_err(|_| Error::Invalid)?;
        let field = key >> 3;
        let wire = (key & 7) as u8;
        let expected_wire = match (message, field) {
            (AuditWireMessage::Envelope, 1 | 3..=6 | 8 | 14 | 16 | 22)
            | (AuditWireMessage::Transaction, 2 | 3) => 0,
            (AuditWireMessage::Envelope, 2 | 7 | 9..=13 | 15 | 17..=21 | 23..=27)
            | (AuditWireMessage::Transaction, 1) => 2,
            _ => return Err(Error::Invalid),
        };
        let bit = 1_u32 << field;
        if wire != expected_wire || seen & bit != 0 {
            return Err(Error::Invalid);
        }
        seen |= bit;
        if matches!((message, field), (AuditWireMessage::Envelope, 18)) {
            let transaction = bounded_length_delimited(
                bytes,
                &mut cursor,
                MAX_ENVELOPE_BYTES,
                FoundationProtocolError::MalformedEnvelope,
            )
            .map_err(|_| Error::Invalid)?;
            validate_audit_wire(transaction, AuditWireMessage::Transaction)?;
        } else if matches!((message, field), (AuditWireMessage::Envelope, 26)) {
            let payload = bounded_length_delimited(
                bytes,
                &mut cursor,
                MAX_ENVELOPE_BYTES,
                FoundationProtocolError::MalformedEnvelope,
            )
            .map_err(|_| Error::Invalid)?;
            if payload.len() > MAX_PAYLOAD_BYTES {
                return Err(Error::Bounds);
            }
        } else {
            // Other nested messages are absent from a STAFF_ACTION envelope;
            // the regenerated value comparison below rejects their presence.
            skip_field(bytes, &mut cursor, wire).map_err(|_| Error::Invalid)?;
        }
    }
    Ok(())
}

/// A signature alone never proves execution. Compare the whole retained event
/// with a regenerated event, not a subset of convenient signed fields.
pub fn verify_event(
    contract: Contract,
    request: &VerifiedRequest,
    outcome: &Outcome,
    bytes: &[u8],
) -> Result<(), Error> {
    if bytes.len() > MAX_ENVELOPE_BYTES {
        return Err(Error::Bounds);
    }
    validate_audit_wire(bytes, AuditWireMessage::Envelope)?;
    let envelope = EventEnvelopeV1::decode(bytes).map_err(|_| Error::Invalid)?;
    let expected = encode(
        contract,
        request,
        outcome,
        envelope.occurred_at_unix_ms,
        &envelope.server_build_id,
    )?;
    // ANL-01 does not assume canonical raw envelope serialization. Payload
    // bytes, their digest and every immutable decoded envelope value still match.
    if EventEnvelopeV1::decode(expected.as_slice()).map_err(|_| Error::Invalid)? != envelope {
        return Err(Error::Binding);
    }
    Ok(())
}

/// A mutation requires a verifying COMMITTED receipt. Optional read receipts
/// give the same exact timestamp proof. Without one, use the signed request's
/// latest possible execution time for a conservative expiry exception: an
/// unsigned DB timestamp never excuses premature loss of an audit row.
pub fn evidence_expiry(
    request: &VerifiedRequest,
    outcome: &Outcome,
    receipt: Option<&signature::VerifiedReceipt>,
) -> Result<u64, Error> {
    signature::check_execution_stamp(request.request(), outcome.occurred_at)?;
    let base = if let Some(receipt) = receipt {
        let value = receipt.receipt();
        if value.operation_id != request.request().operation_id
            || value.request_sha256 != request.digest()
            || value.outcome_sha256 != signature::sha256(&outcome.canonical_bytes()?)
            || value.disposition != signature::Disposition::Committed
            || outcome.disposition != signature::Disposition::Committed
            || receipt.signed().principal != request.signed().principal
            || receipt.fingerprint() != request.fingerprint()
        {
            return Err(Error::Receipt);
        }
        outcome.occurred_at.0
    } else if request.request().action.kind().is_read() {
        request
            .request()
            .issued_at
            .0
            .checked_add(signature::EXECUTION_WINDOW_MS)
            .ok_or(Error::Bounds)?
    } else {
        return Err(Error::Receipt);
    };
    base.checked_add(RETENTION_MS as u64).ok_or(Error::Bounds)
}

pub fn verify_stored_event(
    contract: Contract,
    request: &VerifiedRequest,
    outcome: &Outcome,
    row: &StoredEvent,
) -> Result<(), Error> {
    verify_event(contract, request, outcome, &row.envelope)?;
    if row.environment != request.request().environment
        || row.operation_id != request.request().operation_id
        || row.event_id != outcome.action_id
        || row.transaction_id != outcome.transaction_id
        || row.event_type != i64::from(contract.event_type())
        || row.schema_revision != i64::from(contract.revision())
        || row.retention_profile != RETENTION_PROFILE
        || u64::try_from(row.occurred_at).ok() != Some(outcome.occurred_at.0)
        || row.occurred_at.checked_add(RETENTION_MS) != Some(row.expires_at)
        || row.envelope_sha256 != signature::sha256(&row.envelope)
        || !matches!(row.publication_state, 1 | 2)
    {
        return Err(Error::Binding);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]
    use super::super::signature::{Disposition, REQUEST_NAMESPACE, tests::Signer};
    use super::*;

    fn verified_event_fixture() -> (VerifiedRequest, Outcome) {
        let signer = Signer::new();
        let trust = signer.trust("alpha-test", &"a".repeat(64));
        let request = trust
            .verify_request(
                signer.sign(
                    signer
                        .request(2, 2, &"b".repeat(64))
                        .canonical_bytes()
                        .expect("JCS"),
                    REQUEST_NAMESPACE,
                ),
            )
            .expect("verified scoped intent");
        let outcome = Outcome {
            disposition: Disposition::Committed,
            result: ResultCode::Ok,
            action_id: "00000000-0000-7000-8000-000000000003".into(),
            transaction_id: "00000000-0000-7000-8000-000000000004".into(),
            occurred_at: signature::Decimal(1_000_000),
        };
        (request, outcome)
    }

    #[test]
    fn staff_action_prost_payload_boundary_and_overflow() {
        // This isolates the codec's capacity ceiling. Padding is not a legal
        // signed request; semantic admission is exercised separately below.
        // Tag 7 is one byte and this length varint occupies three bytes.
        let mut payload = StaffActionV1 {
            signed_request: vec![0; MAX_PAYLOAD_BYTES - 4],
            ..StaffActionV1::default()
        };
        assert_eq!(payload.encoded_len(), 49_152);
        let bytes = encode_bounded(&payload, MAX_PAYLOAD_BYTES).expect("payload at ceiling");
        assert_eq!(bytes.len(), 49_152);
        assert_eq!(
            StaffActionV1::decode(bytes.as_slice()).expect("prost payload"),
            payload
        );
        let mut envelope = EventEnvelopeV1 {
            payload: bytes,
            ..EventEnvelopeV1::default()
        };
        assert!(validate_audit_wire(&envelope.encode_to_vec(), AuditWireMessage::Envelope).is_ok());

        payload.signed_request.push(0);
        assert_eq!(payload.encoded_len(), 49_153);
        assert_eq!(
            encode_bounded(&payload, MAX_PAYLOAD_BYTES),
            Err(Error::Bounds)
        );
        envelope.payload = payload.encode_to_vec();
        let overflow = envelope.encode_to_vec();
        assert!(overflow.len() < MAX_ENVELOPE_BYTES);
        assert_eq!(
            validate_audit_wire(&overflow, AuditWireMessage::Envelope),
            Err(Error::Bounds)
        );
    }

    #[test]
    fn staff_action_prost_envelope_boundary_and_overflow() {
        // Capacity-only vector, not an authorized event: the build string is
        // padded to isolate this outer ceiling from all signed request fields.
        // Tag 25 occupies two bytes and the length varint occupies three.
        let mut envelope = EventEnvelopeV1 {
            server_build_id: "x".repeat(MAX_ENVELOPE_BYTES - 5),
            ..EventEnvelopeV1::default()
        };
        assert_eq!(envelope.encoded_len(), 50_176);
        let bytes = encode_bounded(&envelope, MAX_ENVELOPE_BYTES).expect("envelope at ceiling");
        assert_eq!(bytes.len(), 50_176);
        assert_eq!(
            EventEnvelopeV1::decode(bytes.as_slice()).expect("prost envelope"),
            envelope
        );

        envelope.server_build_id.push('x');
        assert_eq!(envelope.encoded_len(), 50_177);
        assert_eq!(
            encode_bounded(&envelope, MAX_ENVELOPE_BYTES),
            Err(Error::Bounds)
        );
        let (request, outcome) = verified_event_fixture();
        let contract = Contract::registered().expect("D1343 registry");
        assert_eq!(
            verify_event(contract, &request, &outcome, &envelope.encode_to_vec()),
            Err(Error::Bounds)
        );
    }

    #[test]
    fn the_running_server_build_id_is_an_accepted_envelope_build() {
        // The node's executor records `oteryn-game-server/<version>`; a `/`
        // must not turn every live staff action into a rejected audit.
        let build = crate::durability::character_authority::SERVER_BUILD_ID;
        let (request, outcome) = verified_event_fixture();
        let contract = Contract::registered().expect("D1343 registry");
        let bytes = encode(contract, &request, &outcome, 1_000_000, build).expect("live build");
        assert!(verify_event(contract, &request, &outcome, &bytes).is_ok());
        for refused in ["", "has space", "tab\t", &"x".repeat(129)] {
            assert_eq!(
                encode(contract, &request, &outcome, 1_000_000, refused),
                Err(Error::Invalid)
            );
        }
    }

    #[test]
    fn registered_scoped_event_binds_contract_and_resource_ceilings() {
        let contract = Contract::registered().expect("D1343 registry");
        assert_eq!(contract.event_type(), 5);
        assert_eq!(contract.revision(), 1);
        assert_eq!(RETENTION_MS, 365 * 24 * 60 * 60 * 1_000);
        let limits: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
        ))
        .expect("resource registry");
        for (id, maximum) in [
            ("GM1-STAFF-ACTION-PAYLOAD-BYTES", MAX_PAYLOAD_BYTES),
            ("GM1-STAFF-ACTION-ENVELOPE-BYTES", MAX_ENVELOPE_BYTES),
        ] {
            let entry = limits["entries"]
                .as_array()
                .expect("resource entries")
                .iter()
                .find(|entry| entry["id"] == id)
                .expect("registered ceiling");
            assert_eq!(entry["hard_maximum"].as_u64(), Some(maximum as u64));
        }
        let (request, outcome) = verified_event_fixture();
        let bytes = encode(contract, &request, &outcome, 1_000_000, "staff-test")
            .expect("registered scoped event");
        let envelope = EventEnvelopeV1::decode(bytes.as_slice()).expect("envelope");
        let scope = request
            .request()
            .scope
            .as_ref()
            .expect("ordinary action scope");
        assert_eq!(
            envelope.world_id,
            Some(
                crate::node::uuid_bytes(&scope.world_id)
                    .expect("WorldId")
                    .to_vec()
            )
        );
        assert_eq!(
            envelope.channel_id,
            Some(
                crate::node::uuid_bytes(&scope.channel_id)
                    .expect("ChannelId")
                    .to_vec()
            )
        );
        assert_ne!(envelope.world_id, envelope.channel_id);
        assert_eq!(envelope.event_type_id, 5);
        assert_eq!(envelope.event_schema_revision, 1);
        assert_eq!(envelope.retention_profile_id, RETENTION_PROFILE);
        assert!(envelope.payload.len() <= MAX_PAYLOAD_BYTES);
        assert!(bytes.len() <= MAX_ENVELOPE_BYTES);
        assert!(verify_event(contract, &request, &outcome, &bytes).is_ok());
    }

    #[test]
    fn unsigned_read_timestamp_cannot_excuse_missing_unexpired_audit() {
        let signer = Signer::new();
        let trust = signer.trust("alpha-test", &"a".repeat(64));
        let request = trust
            .verify_request(
                signer.sign(
                    signer
                        .request(2, 2, &"b".repeat(64))
                        .canonical_bytes()
                        .expect("JCS"),
                    REQUEST_NAMESPACE,
                ),
            )
            .expect("verified intent");
        let mut outcome = Outcome {
            disposition: Disposition::Committed,
            result: ResultCode::Ok,
            action_id: "00000000-0000-7000-8000-000000000003".into(),
            transaction_id: "00000000-0000-7000-8000-000000000004".into(),
            occurred_at: super::super::signature::Decimal(1_000_000),
        };
        assert_eq!(
            evidence_expiry(&request, &outcome, None).expect("signed intent upper time bound"),
            1_600_000 + RETENTION_MS as u64
        );
        outcome.occurred_at = super::super::signature::Decimal(1);
        assert_eq!(
            evidence_expiry(&request, &outcome, None),
            Err(Error::Binding)
        );
    }
    #[test]
    fn event_sql_routing_hash_expiry_and_identity_mirrors_all_bind_to_exact_envelope() {
        let signer = Signer::new();
        let trust = signer.trust("alpha-test", &"a".repeat(64));
        let request = trust
            .verify_request(
                signer.sign(
                    signer
                        .request(2, 2, &"b".repeat(64))
                        .canonical_bytes()
                        .expect("JCS"),
                    REQUEST_NAMESPACE,
                ),
            )
            .expect("verified intent");
        let outcome = Outcome {
            disposition: Disposition::Committed,
            result: ResultCode::Ok,
            action_id: "00000000-0000-7000-8000-000000000003".into(),
            transaction_id: "00000000-0000-7000-8000-000000000004".into(),
            occurred_at: super::super::signature::Decimal(1_000_000),
        };
        let envelope = encode(
            Contract::fixture(),
            &request,
            &outcome,
            1_000_000,
            "staff-test",
        )
        .expect("ANL envelope");
        let row = StoredEvent {
            environment: request.request().environment.clone(),
            operation_id: request.request().operation_id.clone(),
            envelope_sha256: signature::sha256(&envelope),
            envelope,
            event_id: outcome.action_id.clone(),
            transaction_id: outcome.transaction_id.clone(),
            event_type: 4,
            schema_revision: 1,
            retention_profile: RETENTION_PROFILE.into(),
            occurred_at: 1_000_000,
            expires_at: 1_000_000 + RETENTION_MS,
            publication_state: 1,
        };
        assert!(verify_stored_event(Contract::fixture(), &request, &outcome, &row).is_ok());
        for field in 0..11 {
            let mut bad = row.clone();
            match field {
                0 => bad.event_type += 1,
                1 => bad.schema_revision += 1,
                2 => bad.retention_profile = "OTHER".into(),
                3 => bad.occurred_at += 1,
                4 => bad.expires_at -= 1,
                5 => bad.envelope_sha256 = "f".repeat(64),
                6 => bad.publication_state = 3,
                7 => bad.event_id = "00000000-0000-7000-8000-000000000005".into(),
                8 => bad.transaction_id = "00000000-0000-7000-8000-000000000005".into(),
                9 => bad.environment = "other".into(),
                _ => bad.operation_id = "00000000-0000-7000-8000-000000000005".into(),
            }
            assert!(verify_stored_event(Contract::fixture(), &request, &outcome, &bad).is_err());
        }

        // Protobuf field order is not semantic, including inside the transaction
        // reference. The retained SQL hash must describe each actual byte string.
        let decoded = EventEnvelopeV1::decode(row.envelope.as_slice()).expect("envelope");
        let transaction = decoded
            .transaction_event
            .as_ref()
            .expect("transaction reference")
            .encode_to_vec();
        let transaction_field = |bytes: &[u8]| {
            assert!(bytes.len() < 128, "single-byte fixture length");
            let mut field = vec![0x92, 0x01]; // TransactionEventRef, field 18.
            field.push(u8::try_from(bytes.len()).expect("fixture length"));
            field.extend_from_slice(bytes);
            field
        };
        let with_transaction = |bytes: &[u8]| {
            let mut envelope = decoded.clone();
            envelope.transaction_event = None;
            let mut wire = envelope.encode_to_vec();
            wire.extend(transaction_field(bytes));
            wire
        };
        let append = |bytes: &[u8], suffix: &[u8]| [bytes, suffix].concat();
        assert_eq!(&row.envelope[..2], &[0x08, 0x01]);
        assert_eq!(&transaction[..2], &[0x0a, 0x10]);
        for reordered in [
            append(&row.envelope[2..], &row.envelope[..2]),
            with_transaction(&append(&transaction[18..], &transaction[..18])),
        ] {
            let mut control = row.clone();
            control.envelope = reordered;
            control.envelope_sha256 = signature::sha256(&control.envelope);
            assert!(
                verify_stored_event(Contract::fixture(), &request, &outcome, &control).is_ok(),
                "field ordering must not change the event"
            );
        }

        // A database owner can recompute the unsigned SQL mirror. Neither the
        // top-level decoder nor its nested decoder may discard the altered fields.
        let mut duplicate_event_id = vec![0x12, 0x10];
        duplicate_event_id.extend_from_slice(&decoded.event_id);
        let mutations = [
            (
                "unknown envelope varint",
                append(&row.envelope, &[0xe0, 0x01, 0x01]),
            ),
            (
                "unknown envelope bytes",
                append(&row.envelope, &[0xe2, 0x01, 0x00]),
            ),
            (
                "duplicate envelope scalar",
                append(&row.envelope, &[0x08, 0x01]),
            ),
            (
                "duplicate envelope bytes",
                append(&row.envelope, &duplicate_event_id),
            ),
            (
                "duplicate envelope transaction",
                append(&row.envelope, &transaction_field(&transaction)),
            ),
            (
                "unknown transaction field",
                with_transaction(&append(&transaction, &[0x20, 0x01])),
            ),
            (
                "duplicate transaction scalar",
                with_transaction(&append(&transaction, &[0x10, 0x01])),
            ),
            (
                "duplicate transaction bytes",
                with_transaction(&append(&transaction, &transaction[..18])),
            ),
        ];
        let mut accepted = Vec::new();
        for (operator, envelope) in mutations {
            let mut bad = row.clone();
            bad.envelope = envelope;
            bad.envelope_sha256 = signature::sha256(&bad.envelope);
            if verify_stored_event(Contract::fixture(), &request, &outcome, &bad).is_ok() {
                accepted.push(operator);
            }
        }
        assert!(
            accepted.is_empty(),
            "accepted altered protobuf: {accepted:?}"
        );
    }
    #[test]
    fn retained_operation_mirrors_verify_without_any_outbox_row() {
        let signer = Signer::new();
        let trust = signer.trust("alpha-test", &"a".repeat(64));
        let request = trust
            .verify_request(
                signer.sign(
                    signer
                        .request(2, 2, &"b".repeat(64))
                        .canonical_bytes()
                        .expect("JCS"),
                    REQUEST_NAMESPACE,
                ),
            )
            .expect("verified intent");
        let outcome = Outcome {
            disposition: Disposition::Committed,
            result: ResultCode::Ok,
            action_id: "00000000-0000-7000-8000-000000000003".into(),
            transaction_id: "00000000-0000-7000-8000-000000000004".into(),
            occurred_at: signature::Decimal(1_000_000),
        };
        let bytes = outcome.canonical_bytes().expect("outcome JCS");
        let row = StoredOperation {
            environment: request.request().environment.clone(),
            operation_id: request.request().operation_id.clone(),
            request_sha256: request.digest().into(),
            request_bytes: request.signed().bytes.clone(),
            outcome_sha256: signature::sha256(&bytes),
            outcome_bytes: bytes,
            principal: request.signed().principal.clone(),
            fingerprint: request.fingerprint().into(),
            action_id: outcome.action_id.clone(),
            transaction_id: outcome.transaction_id.clone(),
            control_seq: 2,
            requires_receipt: false,
            occurred_at: 1_000_000,
        };
        assert_eq!(verify_stored_operation(&request, &row), Ok(outcome));
        for field in 0..13 {
            let mut bad = row.clone();
            match field {
                0 => bad.environment = "other".into(),
                1 => bad.operation_id = "00000000-0000-7000-8000-000000000005".into(),
                2 => bad.request_sha256 = "f".repeat(64),
                3 => bad.request_bytes.push(b' '),
                4 => bad.outcome_sha256 = "f".repeat(64),
                5 => bad.outcome_bytes.push(b' '),
                6 => bad.principal = "other".into(),
                7 => bad.fingerprint = "other".into(),
                8 => bad.action_id = "00000000-0000-7000-8000-000000000005".into(),
                9 => bad.transaction_id = "00000000-0000-7000-8000-000000000005".into(),
                10 => bad.control_seq += 1,
                11 => bad.requires_receipt = true,
                _ => bad.occurred_at += 1,
            }
            assert!(
                verify_stored_operation(&request, &bad).is_err(),
                "operation mirror {field}"
            );
        }
    }
    #[test]
    fn event_kind_target_reason_parameters_and_missing_bytes_each_fail_binding() {
        let signer = Signer::new();
        let trust = signer.trust("alpha-test", &"a".repeat(64));
        let request = signer.request(2, 2, &"b".repeat(64));
        let request = trust
            .verify_request(signer.sign(request.canonical_bytes().expect("JCS"), REQUEST_NAMESPACE))
            .expect("verified intent");
        let outcome = Outcome {
            disposition: Disposition::Committed,
            result: ResultCode::Ok,
            action_id: "00000000-0000-7000-8000-000000000003".into(),
            transaction_id: "00000000-0000-7000-8000-000000000004".into(),
            occurred_at: super::super::signature::Decimal(1_000_000),
        };
        let bytes = encode(
            Contract::fixture(),
            &request,
            &outcome,
            1_000_000,
            "staff-test",
        )
        .expect("encoded audit");
        assert!(verify_event(Contract::fixture(), &request, &outcome, &bytes).is_ok());
        for field in ["kind", "target", "reason", "parameter", "bytes"] {
            let mut envelope = EventEnvelopeV1::decode(bytes.as_slice()).expect("envelope");
            let mut payload = StaffActionV1::decode(envelope.payload.as_slice()).expect("payload");
            match field {
                "kind" => payload.action_kind = "KICK".into(),
                "target" => payload.target_jcs = b"{}".to_vec(),
                "reason" => payload.reason = "BOT".into(),
                "parameter" => payload.parameters_jcs = b"{}".to_vec(),
                _ => payload.signed_request.clear(),
            }
            envelope.payload = payload.encode_to_vec();
            envelope.payload_sha256 = Sha256::digest(&envelope.payload).to_vec();
            assert!(
                verify_event(
                    Contract::fixture(),
                    &request,
                    &outcome,
                    &envelope.encode_to_vec()
                )
                .is_err()
            );
        }
    }
}
