//! Closed audit encoding for the pending DAILY wildcard sink (PREY-0 §10).
//! D1338 (#2131:6233263821) admits the dedicated P30D profile and schema1.
//! D1341 assigns PREY_WILDCARD_CHANGED type 6. The exact tuple and byte
//! limits must be registered with migration 0115 before the writer is installed.

use super::DurabilityError;
use super::item_mint_audit::{
    AuditError, DURABLE_AUDIT, ENVELOPE_REVISION, EventEnvelopeV1, RESTRICTED_PLAYER_LINKED,
    TransactionEventRefV1, check_technical_text, check_uuid_v7, encode_bounded,
};
use crate::domain::{AccountId, CharacterId};
use crate::foundation::{ChannelId, WorldId};
use prost::Message;
use sha2::{Digest, Sha256};
use sqlx::PgConnection;

// CP D1341: a source-bound event identity, never a caller-selected option.
const EVENT_TYPE_ID: u32 = 6;
const RETENTION_MILLIS: i64 = 2_592_000_000;
pub(super) const RETENTION_PROFILE_ID: &str = "PREY_WILDCARD_LEDGER_RETENTION_V1";
const EVENT_SCHEMA_REVISION: u32 = 1;
const INTERPRETATION_REVISION: u32 = 1;
const WILDCARDS_MAX: u32 = 52;

// Six UUIDs (108), interpretation (2), scope generation (11), fixed kind
// (14), three cap-bounded amounts (6), line i32 (6), two i64 amounts (20),
// and two u64 Character revisions with two-byte tags (24).
const PAYLOAD_BYTES_MAX: usize = 191;
// Payload191 + bounded ANL metadata; the event type reserves a full u32
// varint even if CP's eventual assigned identity encodes in fewer bytes.
const ENVELOPE_BYTES_MAX: usize = 500;

/// The first closed payload admits DAILY_REWARD only. Later earning/spend or
/// unlock shapes require their own reviewed schema evolution; unknown fields
/// and alternate kinds never pass the canonical decoder below.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct PreyWildcardChangedV1 {
    #[prost(uint32, tag = "1")]
    pub interpretation_revision: u32,
    #[prost(bytes = "vec", tag = "2")]
    pub acting_character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub account_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "5")]
    pub channel_id: Vec<u8>,
    #[prost(uint64, tag = "6")]
    pub runtime_scope_ownership_generation: u64,
    #[prost(bytes = "vec", tag = "7")]
    pub entry_id: Vec<u8>,
    #[prost(string, tag = "8")]
    pub kind: String,
    #[prost(uint32, tag = "9")]
    pub amount: u32,
    #[prost(uint32, tag = "10")]
    pub balance_before: u32,
    #[prost(uint32, tag = "11")]
    pub balance_after: u32,
    #[prost(bytes = "vec", tag = "12")]
    pub daily_claim_transaction_id: Vec<u8>,
    #[prost(uint32, tag = "13")]
    pub daily_line: u32,
    #[prost(uint64, tag = "14")]
    pub quantity: u64,
    #[prost(uint64, tag = "15")]
    pub uncredited_excess: u64,
    #[prost(uint64, tag = "16")]
    pub original_character_revision: u64,
    #[prost(uint64, tag = "17")]
    pub committed_character_revision: u64,
}

fn check_payload(value: &PreyWildcardChangedV1) -> Result<(), AuditError> {
    for id in [
        &value.acting_character_id,
        &value.account_id,
        &value.world_id,
        &value.channel_id,
        &value.entry_id,
        &value.daily_claim_transaction_id,
    ] {
        check_uuid_v7(id)?;
    }
    if value.interpretation_revision != INTERPRETATION_REVISION
        || value.kind != "DAILY_REWARD"
        || value.runtime_scope_ownership_generation == 0
        || value.amount == 0
        || value.amount > WILDCARDS_MAX
        || value.balance_before > WILDCARDS_MAX
        || value.balance_after > WILDCARDS_MAX
        || value.balance_before.checked_add(value.amount) != Some(value.balance_after)
        || value.quantity == 0
        || i64::try_from(value.quantity).is_err()
        || value.quantity.checked_sub(u64::from(value.amount)) != Some(value.uncredited_excess)
        || u64::from(value.amount)
            != value
                .quantity
                .min(u64::from(WILDCARDS_MAX - value.balance_before))
        || i32::try_from(value.daily_line).is_err()
        || value.original_character_revision == 0
        || value.original_character_revision.checked_add(1)
            != Some(value.committed_character_revision)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

fn decode_payload(wire: &[u8]) -> Result<PreyWildcardChangedV1, AuditError> {
    if wire.len() > PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let value = PreyWildcardChangedV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if value.encode_to_vec() != wire {
        return Err(AuditError::InvalidInput);
    }
    check_payload(&value)?;
    Ok(value)
}

pub fn decode_wildcard_envelope(
    wire: &[u8],
) -> Result<(EventEnvelopeV1, PreyWildcardChangedV1), AuditError> {
    let (event_type, retention) = (EVENT_TYPE_ID, RETENTION_MILLIS);
    if wire.len() > ENVELOPE_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let envelope = EventEnvelopeV1::decode(wire).map_err(|_| AuditError::InvalidInput)?;
    if envelope.encode_to_vec() != wire {
        return Err(AuditError::InvalidInput);
    }
    let member = envelope
        .transaction_event
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    check_uuid_v7(&envelope.event_id)?;
    check_uuid_v7(&member.transaction_id)?;
    check_technical_text(&envelope.server_build_id)?;
    if envelope.envelope_revision != ENVELOPE_REVISION
        || envelope.event_type_id != event_type
        || envelope.event_schema_revision != EVENT_SCHEMA_REVISION
        || envelope.durability_class != DURABLE_AUDIT
        || envelope.privacy_class != RESTRICTED_PLAYER_LINKED
        || envelope.retention_profile_id != RETENTION_PROFILE_ID
        || envelope.occurred_at_unix_ms <= 0
        || envelope
            .occurred_at_unix_ms
            .checked_add(retention)
            .is_none()
        || member.ordinal != 1
        || member.count != 1
        || envelope.event_id == member.transaction_id
        || envelope.payload_sha256 != Sha256::digest(&envelope.payload).as_slice()
    {
        return Err(AuditError::InvalidInput);
    }
    let value = decode_payload(&envelope.payload)?;
    if envelope.world_id.as_deref() != Some(value.world_id.as_slice())
        || envelope.channel_id.as_deref() != Some(value.channel_id.as_slice())
        || envelope.instance_id.is_some()
        || envelope.node_id.is_some()
        || envelope.game_session_id.is_some()
        || envelope.connection_generation.is_some()
        || envelope.runtime_order.is_some()
        || envelope.command_id.is_some()
        || envelope.operation_id.is_some()
        || envelope.correlation_id.is_some()
        || envelope.causation.is_some()
        || envelope.analytics_actor.is_some()
        || envelope.protocol_major.is_some()
        || envelope.ruleset_revision.is_some()
        || envelope.content_revision.is_some()
        || member.transaction_id == value.daily_claim_transaction_id
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((envelope, value))
}

#[derive(Clone, Copy)]
pub(super) struct WildcardEventIdentity<'a> {
    pub event_id: [u8; 16],
    pub transaction_id: [u8; 16],
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub scope_ownership_generation: u64,
    pub occurred_at: i64,
    pub server_build_id: &'a str,
}

pub(super) struct DailyWildcardEvent {
    pub claim_transaction_id: [u8; 16],
    pub line: u32,
    pub character_id: CharacterId,
    pub account_id: AccountId,
    pub entry_id: [u8; 16],
    pub quantity: u64,
    pub credited: u8,
    pub uncredited_excess: u64,
    pub balance_before: u8,
    pub balance_after: u8,
    pub original_character_revision: u64,
    pub committed_character_revision: u64,
}

pub(super) struct EncodedWildcardEvent {
    pub envelope_sha256: [u8; 32],
    envelope: Vec<u8>,
    event_id: [u8; 16],
    transaction_id: [u8; 16],
    world_id: WorldId,
    occurred_at: i64,
    expires_at: i64,
}

pub(super) fn encode_daily_wildcard_event(
    identity: WildcardEventIdentity<'_>,
    event: DailyWildcardEvent,
) -> Result<EncodedWildcardEvent, AuditError> {
    let (event_type, retention) = (EVENT_TYPE_ID, RETENTION_MILLIS);
    check_technical_text(identity.server_build_id)?;
    let expires_at = identity
        .occurred_at
        .checked_add(retention)
        .ok_or(AuditError::InvalidInput)?;
    let value = PreyWildcardChangedV1 {
        interpretation_revision: INTERPRETATION_REVISION,
        acting_character_id: event.character_id.as_bytes().to_vec(),
        account_id: event.account_id.as_bytes().to_vec(),
        world_id: identity.world_id.as_bytes().to_vec(),
        channel_id: identity.channel_id.as_bytes().to_vec(),
        runtime_scope_ownership_generation: identity.scope_ownership_generation,
        entry_id: event.entry_id.to_vec(),
        kind: "DAILY_REWARD".into(),
        amount: u32::from(event.credited),
        balance_before: u32::from(event.balance_before),
        balance_after: u32::from(event.balance_after),
        daily_claim_transaction_id: event.claim_transaction_id.to_vec(),
        daily_line: event.line,
        quantity: event.quantity,
        uncredited_excess: event.uncredited_excess,
        original_character_revision: event.original_character_revision,
        committed_character_revision: event.committed_character_revision,
    };
    check_payload(&value)?;
    let payload = encode_bounded(&value, PAYLOAD_BYTES_MAX)?;
    let envelope = EventEnvelopeV1 {
        envelope_revision: ENVELOPE_REVISION,
        event_id: identity.event_id.to_vec(),
        event_type_id: event_type,
        event_schema_revision: EVENT_SCHEMA_REVISION,
        durability_class: DURABLE_AUDIT,
        privacy_class: RESTRICTED_PLAYER_LINKED,
        retention_profile_id: RETENTION_PROFILE_ID.into(),
        occurred_at_unix_ms: identity.occurred_at,
        world_id: Some(value.world_id),
        channel_id: Some(value.channel_id),
        transaction_event: Some(TransactionEventRefV1 {
            transaction_id: identity.transaction_id.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        server_build_id: identity.server_build_id.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..EventEnvelopeV1::default()
    };
    let wire = encode_bounded(&envelope, ENVELOPE_BYTES_MAX)?;
    let (decoded, _) = decode_wildcard_envelope(&wire)?;
    if decoded != envelope {
        return Err(AuditError::InvalidInput);
    }
    Ok(EncodedWildcardEvent {
        envelope_sha256: Sha256::digest(&wire).into(),
        envelope: wire,
        event_id: identity.event_id,
        transaction_id: identity.transaction_id,
        world_id: identity.world_id,
        occurred_at: identity.occurred_at,
        expires_at,
    })
}

pub(super) async fn insert_event(
    connection: &mut PgConnection,
    event: &EncodedWildcardEvent,
) -> Result<(), DurabilityError> {
    let event_type = EVENT_TYPE_ID;
    sqlx::query("INSERT INTO game_account_prey_wildcard_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,world_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,$3,1,$4,encode($5,'hex')::uuid,$6,$7,$8,$9,1)")
        .bind(event.event_id.as_slice()).bind(event.transaction_id.as_slice()).bind(i64::from(event_type))
        .bind(RETENTION_PROFILE_ID).bind(event.world_id.as_bytes().as_slice()).bind(event.occurred_at)
        .bind(event.expires_at).bind(&event.envelope).bind(event.envelope_sha256.as_slice())
        .execute(connection).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(seed: u8) -> Vec<u8> {
        let mut v = vec![seed; 16];
        v[6] = 0x70;
        v[8] = 0x80;
        v
    }
    fn maximum() -> PreyWildcardChangedV1 {
        PreyWildcardChangedV1 {
            interpretation_revision: 1,
            acting_character_id: id(1),
            account_id: id(2),
            world_id: id(3),
            channel_id: id(4),
            runtime_scope_ownership_generation: u64::MAX,
            entry_id: id(5),
            kind: "DAILY_REWARD".into(),
            amount: 1,
            balance_before: 51,
            balance_after: 52,
            daily_claim_transaction_id: id(6),
            daily_line: i32::MAX as u32,
            quantity: i64::MAX as u64,
            uncredited_excess: i64::MAX as u64 - 1,
            original_character_revision: u64::MAX - 1,
            committed_character_revision: u64::MAX,
        }
    }

    #[test]
    fn prey_wildcard_payload_bound_is_reachable_and_canonical() {
        let value = maximum();
        assert!(check_payload(&value).is_ok());
        assert_eq!(value.encoded_len(), PAYLOAD_BYTES_MAX);
        assert_eq!(decode_payload(&value.encode_to_vec()), Ok(value));
        assert_eq!(
            decode_payload(&[0; PAYLOAD_BYTES_MAX + 1]),
            Err(AuditError::CapacityExceeded)
        );
    }

    #[test]
    fn prey_wildcard_payload_rejects_overcredit_unknown_kind_and_noncanonical_bytes() {
        let value = maximum();
        let mut excessive = value.clone();
        excessive.balance_after = 53;
        let mut under = value.clone();
        under.balance_before = 50;
        under.balance_after = 51;
        let mut kind = value.clone();
        kind.kind = "STORE_CLAIM".into();
        let mut revision = value.clone();
        revision.committed_character_revision = revision.original_character_revision;
        let mut zero = value.clone();
        zero.amount = 0;
        zero.balance_after = 51;
        zero.uncredited_excess = zero.quantity;
        let mut amount = value.clone();
        amount.quantity = u64::MAX;
        amount.uncredited_excess = u64::MAX - 1;
        for invalid in [excessive, under, kind, revision, zero, amount] {
            assert!(check_payload(&invalid).is_err());
        }
        // Duplicate known field is semantically identical after decoding but
        // fails the actual canonical wire gate; unknown fields fail likewise.
        let mut smaller = value;
        smaller.daily_line = 0;
        smaller.quantity = 1;
        smaller.uncredited_excess = 0;
        let mut duplicate = smaller.encode_to_vec();
        duplicate.extend_from_slice(&[8, 1]);
        assert!(decode_payload(&duplicate).is_err());
        let mut unknown = smaller.encode_to_vec();
        unknown.extend_from_slice(&[0x90, 0x01, 1]);
        assert!(decode_payload(&unknown).is_err());
    }

    fn encode_maximum() -> Result<EncodedWildcardEvent, AuditError> {
        let value = maximum();
        let bytes = |value: Vec<u8>| value.try_into().map_err(|_| AuditError::InvalidInput);
        let build = "b".repeat(128);
        encode_daily_wildcard_event(
            WildcardEventIdentity {
                event_id: bytes(id(7))?,
                transaction_id: bytes(id(8))?,
                world_id: WorldId::decode(&value.world_id).map_err(|_| AuditError::InvalidInput)?,
                channel_id: ChannelId::decode(&value.channel_id)
                    .map_err(|_| AuditError::InvalidInput)?,
                scope_ownership_generation: u64::MAX,
                occurred_at: i64::MAX - RETENTION_MILLIS,
                server_build_id: &build,
            },
            DailyWildcardEvent {
                claim_transaction_id: bytes(value.daily_claim_transaction_id)?,
                line: value.daily_line,
                character_id: CharacterId::from_bytes(bytes(value.acting_character_id)?)
                    .map_err(|_| AuditError::InvalidInput)?,
                account_id: AccountId::from_bytes(bytes(value.account_id)?)
                    .map_err(|_| AuditError::InvalidInput)?,
                entry_id: bytes(value.entry_id)?,
                quantity: value.quantity,
                credited: 1,
                uncredited_excess: value.uncredited_excess,
                balance_before: 51,
                balance_after: 52,
                original_character_revision: value.original_character_revision,
                committed_character_revision: value.committed_character_revision,
            },
        )
    }

    #[test]
    fn prey_wildcard_real_encoder_fits_registered_bound_and_retains_original_expiry()
    -> Result<(), AuditError> {
        let event = encode_maximum()?;
        let (mut envelope, payload) = decode_wildcard_envelope(&event.envelope)?;
        assert_eq!(payload, maximum());
        assert_eq!(event.expires_at, i64::MAX);
        assert_eq!(event.expires_at - event.occurred_at, RETENTION_MILLIS);
        assert_eq!(envelope.retention_profile_id, RETENTION_PROFILE_ID);
        assert_eq!(envelope.event_type_id, EVENT_TYPE_ID);
        assert_eq!(
            event.envelope_sha256.as_slice(),
            Sha256::digest(&event.envelope).as_slice()
        );
        assert!(event.envelope.len() <= ENVELOPE_BYTES_MAX);
        // The ceiling reserves the entire registered uint32 identity range;
        // this measures the wire upper bound, not an alternate admitted type.
        envelope.event_type_id = u32::MAX;
        assert_eq!(envelope.encoded_len(), ENVELOPE_BYTES_MAX);
        assert_eq!(
            decode_wildcard_envelope(&vec![0; ENVELOPE_BYTES_MAX + 1]),
            Err(AuditError::CapacityExceeded)
        );
        Ok(())
    }

    #[test]
    fn prey_wildcard_envelope_rejects_stale_policy_extra_authority_and_malformed_hash()
    -> Result<(), AuditError> {
        let event = encode_maximum()?;
        let (envelope, _) = decode_wildcard_envelope(&event.envelope)?;
        let mut profile = envelope.clone();
        profile.retention_profile_id = "ECONOMY_LEDGER_RETENTION_V1".into();
        let mut schema = envelope.clone();
        schema.event_schema_revision = 2;
        let mut unknown = envelope.clone();
        unknown.event_type_id = 0;
        let mut world = envelope.clone();
        world.world_id = Some(id(9));
        let mut session = envelope.clone();
        session.game_session_id = Some(id(10));
        let mut overflow = envelope.clone();
        overflow.occurred_at_unix_ms = i64::MAX;
        let mut hash = envelope.clone();
        hash.payload_sha256[0] ^= 1;
        for invalid in [profile, schema, unknown, world, session, overflow, hash] {
            assert!(decode_wildcard_envelope(&invalid.encode_to_vec()).is_err());
        }
        let mut shorter = envelope;
        shorter.server_build_id = "short".into();
        let mut duplicate = shorter.encode_to_vec();
        duplicate.extend_from_slice(&[8, 1]);
        assert!(decode_wildcard_envelope(&duplicate).is_err());
        let mut extra = shorter.encode_to_vec();
        extra.extend_from_slice(&[0xe0, 0x01, 1]);
        assert!(decode_wildcard_envelope(&extra).is_err());
        Ok(())
    }
}
