//! Strict historical fresh-operation storage. No decoded value is a live capability.
//! Budgets are explicit caller allocations; this module selects no production ceiling.
use super::DurabilityError;
use super::admission_authority_guards::*;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use oteryn_game_server::foundation::admission_authority_publication::*;
use oteryn_game_server::foundation::fnd04_verifier::*;
use oteryn_game_server::foundation::fresh_admission_durability::*;
use oteryn_game_server::foundation::*;
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, DurabilityError>;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope<'a> {
    version: u8,
    payload: &'a str,
}

fn write_current(w: &mut Writer, f: &FreshCurrentEvidence) -> Result<()> {
    w.text(&f.account_id)?;
    w.bytes(f.character_id.as_bytes())?;
    w.bytes(f.world_id.as_bytes())?;
    w.bytes(f.channel_id.as_bytes())?;
    w.u64(f.character_lease_generation)?;
    w.text(&f.route_revision)?;
    w.text(&f.runtime_observation_revision)?;
    w.u64(f.scope_ownership_generation)?;
    for value in [
        &f.ruleset_revision,
        &f.content_revision,
        &f.map_revision,
        &f.world_policy_revision,
        &f.offer_revision,
    ] {
        w.text(value)?;
    }
    Ok(())
}
fn read_current(r: &mut Reader<'_>) -> Result<FreshCurrentEvidence> {
    Ok(FreshCurrentEvidence {
        account_id: r.text()?,
        character_id: checked(CharacterId::decode(&r.bytes::<16>()?))?,
        world_id: checked(WorldId::decode(&r.bytes::<16>()?))?,
        channel_id: checked(ChannelId::decode(&r.bytes::<16>()?))?,
        character_lease_generation: r.u64()?,
        route_revision: r.text()?,
        runtime_observation_revision: r.text()?,
        scope_ownership_generation: r.u64()?,
        ruleset_revision: r.text()?,
        content_revision: r.text()?,
        map_revision: r.text()?,
        world_policy_revision: r.text()?,
        offer_revision: r.text()?,
    })
}
fn write_operation(w: &mut Writer, operation: &FreshAdmissionOperationV1) -> Result<()> {
    let b = &operation.authorization;
    let initial = checked(b.initial_commit())?;
    w.tag(b.version)?;
    w.text(&b.account_id)?;
    w.bytes(&b.facts.replay_key().to_bytes())?;
    w.bytes(initial.character_id().as_bytes())?;
    w.bytes(initial.world_id().as_bytes())?;
    w.bytes(initial.channel_id().as_bytes())?;
    w.u64(initial.character_lease_generation())?;
    w.u64(initial.scope_ownership_generation())?;
    w.bytes(b.candidate_session.as_bytes())?;
    w.bytes(&b.transport.to_bytes())?;
    w.u64(b.connection_generation)?;
    write_current(w, &b.current_facts)?;
    w.u64(b.protocol_major)?;
    w.u64(b.transport_profile)?;
    w.u64(b.signed_security_generation)?;
    w.text(&b.signing.key_id)?;
    w.bytes(&b.signing.public_key)?;
    w.boolean(b.signing.trusted)?;
    write_provenance(w, &b.signing.provenance)?;
    write_security(w, &b.security)?;
    w.i64(b.credential_times.0)?;
    w.i64(b.credential_times.1)?;
    w.i64(b.credential_times.2)?;
    w.i64(b.verified_at)?;
    w.i64(b.accepted_deadline)?;
    write_changes(w, &b.expected_guards)?;
    write_changes(w, &operation.transition.predecessors)?;
    write_changes(w, &operation.transition.successors)?;
    w.i64(operation.transition.prepared_at)
}
fn read_operation(r: &mut Reader<'_>) -> Result<FreshAdmissionOperationV1> {
    let version = r.tag()?;
    let account_id = r.text()?;
    let replay = checked(FreshAdmissionReplayKey::decode(&r.bytes::<33>()?))?.to_bytes();
    let character = checked(CharacterId::decode(&r.bytes::<16>()?))?;
    let world = checked(WorldId::decode(&r.bytes::<16>()?))?;
    let channel = checked(ChannelId::decode(&r.bytes::<16>()?))?;
    let facts = checked(FreshAdmissionFacts::new(
        checked(replay[1..].try_into())?,
        character,
        world,
        channel,
        r.u64()?,
        r.u64()?,
    ))?;
    let candidate_session = checked(GameSessionId::decode(&r.bytes::<16>()?))?;
    let transport = checked(AuthenticatedTransportRefV1::decode(&r.bytes::<16>()?))?;
    let connection_generation = r.u64()?;
    let current_facts = read_current(r)?;
    let protocol_major = r.u64()?;
    let transport_profile = r.u64()?;
    let signed_security_generation = r.u64()?;
    let signing = FreshSigningTrustObservationV1 {
        key_id: r.text()?,
        public_key: r.bytes()?,
        trusted: r.boolean()?,
        provenance: read_provenance(r)?,
    };
    let security = read_security(r)?;
    let credential_times = (r.i64()?, r.i64()?, r.i64()?);
    let verified_at = r.i64()?;
    let accepted_deadline = r.i64()?;
    let expected_guards = read_changes(r)?;
    let authorization = FreshAdmissionAuditBindingV1 {
        version,
        account_id,
        facts,
        candidate_session,
        transport,
        connection_generation,
        current_facts,
        protocol_major,
        transport_profile,
        signed_security_generation,
        signing,
        security,
        credential_times,
        verified_at,
        accepted_deadline,
        expected_guards,
    };
    let transition = AdmissionClaimTransitionEvidenceV1 {
        predecessors: read_changes(r)?,
        successors: read_changes(r)?,
        prepared_at: r.i64()?,
    };
    Ok(FreshAdmissionOperationV1 {
        authorization,
        transition,
    })
}

/// The full immutable operation, including independently authored claim effects.
/// Each allocation is checked against the caller's finite budget before copying.
pub fn encode_operation(
    operation: &FreshAdmissionOperationV1,
    maximum_bytes: usize,
) -> Result<String> {
    let mut writer = Writer::new(maximum_bytes);
    write_operation(&mut writer, operation)?;
    // The historical predicate clones guard evidence internally: first establish
    // that the complete retained operation fits the explicit allocation budget.
    checked(operation.validate_historical(operation.transition.prepared_at))?;
    encode_envelope(&writer.bytes, maximum_bytes)
}
pub(super) fn encode_envelope(bytes: &[u8], maximum_bytes: usize) -> Result<String> {
    let groups = bytes
        .len()
        .checked_div(3)
        .and_then(|groups| groups.checked_mul(4));
    let tail = match bytes.len() % 3 {
        0 => 0,
        1 => 2,
        _ => 3,
    };
    let required = groups
        .and_then(|size| size.checked_add(tail))
        .and_then(|size| size.checked_add("{\"version\":1,\"payload\":\"\"}".len()));
    if required.is_none_or(|required| required > maximum_bytes) {
        return Err(DurabilityError::InvalidStoredState);
    }
    let payload = URL_SAFE_NO_PAD.encode(bytes);
    checked(serde_json::to_string(&Envelope {
        version: 1,
        payload: &payload,
    }))
}
pub(super) fn decode_envelope(encoded: &str, maximum_bytes: usize) -> Result<Vec<u8>> {
    if encoded.len() > maximum_bytes {
        return Err(DurabilityError::InvalidStoredState);
    }
    // Borrowed payload parsing performs no peer-sized string copy. Unknown,
    // duplicate, escaped/noncanonical members and unsupported versions reject.
    let envelope: Envelope<'_> = checked(serde_json::from_str(encoded))?;
    if envelope.version != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    const PREFIX: &str = "{\"version\":1,\"payload\":\"";
    const SUFFIX: &str = "\"}";
    if !encoded.starts_with(PREFIX)
        || !encoded.ends_with(SUFFIX)
        || encoded.len() != PREFIX.len() + envelope.payload.len() + SUFFIX.len()
        || &encoded[PREFIX.len()..encoded.len() - SUFFIX.len()] != envelope.payload
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    // The configured engine rejects padding and nonzero unused trailing bits.
    let bytes = checked(URL_SAFE_NO_PAD.decode(envelope.payload))?;
    Ok(bytes)
}
/// Restore historical data only. Receipt restoration additionally validates the
/// original durable decided_at; neither operation creates a current source.
pub fn decode_operation(encoded: &str, maximum_bytes: usize) -> Result<FreshAdmissionOperationV1> {
    let bytes = decode_envelope(encoded, maximum_bytes)?;
    let mut reader = Reader::new(&bytes);
    let operation = read_operation(&mut reader)?;
    reader.finish()?;
    checked(operation.validate_historical(operation.transition.prepared_at))?;
    Ok(operation)
}
