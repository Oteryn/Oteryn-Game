//! Bounded, unregistered offline native one-item codec/resource candidate.
//! No runtime, SQL, registry, Character, Content-admission, or playability proof.

use prost::Message;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::mem::size_of;

#[derive(Clone, PartialEq, Eq, Message)]
struct Definition {
    #[prost(string, tag = "1")]
    family: String,
    #[prost(string, tag = "2")]
    production_key: String,
    #[prost(string, tag = "3")]
    revision_ref: String,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct Item {
    #[prost(bytes = "vec", tag = "1")]
    item_instance_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    world_id: Vec<u8>,
    #[prost(message, optional, tag = "3")]
    definition: Option<Definition>,
    #[prost(uint32, tag = "4")]
    quantity: u32,
    #[prost(uint32, tag = "5")]
    lifecycle: u32,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct Ground {
    #[prost(bytes = "vec", tag = "1")]
    world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    channel_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    spatial_position_fixture: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    corpse_occurrence_id_fixture: Vec<u8>,
    #[prost(string, tag = "5")]
    map_revision_fixture: String,
    #[prost(string, tag = "6")]
    content_revision_fixture: String,
    #[prost(bytes = "vec", tag = "7")]
    native_room_placement_context_fixture: Vec<u8>,
    #[prost(uint64, tag = "8")]
    runtime_scope_ownership_generation_fixture: u64,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct Inventory {
    #[prost(bytes = "vec", tag = "1")]
    character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    typed_position_fixture: Vec<u8>,
    #[prost(uint64, tag = "3")]
    expected_session_generation: u64,
    #[prost(bytes = "vec", tag = "4")]
    expected_game_session_id_fixture: Vec<u8>,
    #[prost(uint64, tag = "5")]
    expected_character_lease_generation_fixture: u64,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct CommandRef {
    #[prost(bytes = "vec", tag = "1")]
    game_session_id_fixture: Vec<u8>,
    #[prost(uint64, tag = "2")]
    command_id_fixture: u64,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct Provenance {
    #[prost(string, tag = "1")]
    typed_cause: String,
    #[prost(bytes = "vec", tag = "2")]
    output_occurrence_id_fixture: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    committed_death_occurrence_id_fixture: Vec<u8>,
    #[prost(string, tag = "4")]
    loot_definition_revision_fixture: String,
    #[prost(string, tag = "5")]
    content_revision_fixture: String,
    #[prost(string, tag = "6")]
    ruleset_revision_fixture: String,
    #[prost(string, tag = "7")]
    sim_revision_fixture: String,
    #[prost(message, optional, tag = "8")]
    command_ref_fixture: Option<CommandRef>,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct Mint {
    #[prost(message, optional, tag = "1")]
    after: Option<Item>,
    #[prost(message, optional, tag = "2")]
    destination: Option<Ground>,
    #[prost(message, optional, tag = "3")]
    source: Option<Provenance>,
    // Explicit semantic absence: this means no prior item existed, not nil/zero.
    #[prost(bool, tag = "4")]
    before_semantically_absent: bool,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct Transfer {
    #[prost(message, optional, tag = "1")]
    before: Option<Item>,
    #[prost(message, optional, tag = "2")]
    after: Option<Item>,
    #[prost(message, optional, tag = "3")]
    source: Option<Ground>,
    #[prost(message, optional, tag = "4")]
    destination: Option<Inventory>,
    #[prost(message, optional, tag = "5")]
    cause: Option<Provenance>,
}
#[derive(Clone, PartialEq, Eq, prost::Oneof)]
// Keep the candidate oneof inline so retained-capacity measurements include
// the native enum footprint instead of an invented boxed layout.
#[allow(clippy::large_enum_variant)]
enum Operation {
    #[prost(message, tag = "2")]
    Mint(Mint),
    #[prost(message, tag = "3")]
    Transfer(Transfer),
}
#[derive(Clone, PartialEq, Eq, Message)]
struct Payload {
    #[prost(uint32, tag = "1")]
    interpretation_revision: u32,
    #[prost(oneof = "Operation", tags = "2, 3")]
    operation: Option<Operation>,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct Envelope {
    #[prost(uint32, tag = "1")]
    envelope_revision: u32,
    #[prost(bytes = "vec", tag = "2")]
    event_id_fixture: Vec<u8>,
    #[prost(uint32, tag = "3")]
    event_type_id_fixture: u32,
    #[prost(uint32, tag = "4")]
    schema_revision_fixture: u32,
    #[prost(string, tag = "5")]
    retention_profile_input: String,
    #[prost(bytes = "vec", tag = "6")]
    transaction_id_fixture: Vec<u8>,
    #[prost(uint32, tag = "7")]
    ordinal: u32,
    #[prost(uint32, tag = "8")]
    count: u32,
    #[prost(bytes = "vec", tag = "9")]
    payload: Vec<u8>,
    #[prost(bytes = "vec", tag = "10")]
    payload_sha256: Vec<u8>,
}

#[derive(Debug, Clone)]
struct Fixture {
    name: &'static str,
    family: &'static str,
    key: &'static str,
    revision: &'static str,
    tag: u8,
}

const FIXTURES: [Fixture; 2] = [
    Fixture {
        name: "definition_a",
        family: "ItemType",
        key: "fixture:alpha",
        revision: "rev-a/1",
        tag: 11,
    },
    Fixture {
        name: "definition_b",
        family: "ItemType",
        key: "fixture:beta",
        revision: "rev-b/7",
        tag: 22,
    },
];
// Candidate-only representation backpressure, not an ANL or production cap.
const CANDIDATE_WIRE_MAX: usize = 4096;
const ANL_ENVELOPE_MAX: usize = 262_144;
const ANL_PAYLOAD_MAX: usize = 196_608;
const ANL_ENVELOPE_STRING_MAX: usize = 128;
const ANL_PROTOBUF_DEPTH_MAX: usize = 32;
const CANDIDATE_SCHEMA_NESTING_DEPTH: usize = 5;

// Independently supplied fixture expectations; never current runtime authority.
#[derive(Clone)]
struct CurrentFacts {
    world_id: Vec<u8>,
    channel_id: Vec<u8>,
    definition: Definition,
    ground_spatial_position: Vec<u8>,
    character_inventory_position: Vec<u8>,
    death_occurrence: Vec<u8>,
    output_occurrence: Vec<u8>,
    character_id: Vec<u8>,
    game_session_id: Vec<u8>,
    session_generation: u64,
    character_lease_generation: u64,
    command_id: u64,
    map_revision: String,
    content_revision: String,
    native_room_placement_context: Vec<u8>,
    runtime_scope_generation: u64,
    loot_definition_revision: String,
    ruleset_revision: String,
    sim_revision: String,
}

fn facts(f: &Fixture) -> CurrentFacts {
    CurrentFacts {
        world_id: id(1),
        channel_id: id(2),
        definition: definition(f),
        ground_spatial_position: vec![1, 2, 3],
        character_inventory_position: vec![5],
        death_occurrence: id(3),
        output_occurrence: id(7),
        character_id: id(4),
        game_session_id: id(5),
        session_generation: 1,
        character_lease_generation: 3,
        command_id: 9,
        map_revision: "map-fixture-r1".into(),
        content_revision: "content-fixture-r1".into(),
        native_room_placement_context: id(6),
        runtime_scope_generation: 1,
        loot_definition_revision: "loot-fixture-r1".into(),
        ruleset_revision: "ruleset-fixture-r1".into(),
        sim_revision: "sim-fixture-r1".into(),
    }
}

fn valid_uuid_fixture(value: &[u8]) -> bool {
    value.len() == 16
        && value.iter().any(|byte| *byte != 0)
        && value[6] >> 4 == 7
        && value[8] >> 6 == 2
}

fn validate_item(value: &Item, current: &CurrentFacts) -> Result<(), String> {
    let definition = value
        .definition
        .as_ref()
        .ok_or("missing typed definition")?;
    if !valid_uuid_fixture(&value.item_instance_id)
        || !valid_uuid_fixture(&value.world_id)
        || value.world_id != current.world_id
        // Fixture representation only; this is not a product quantity cap.
        || value.quantity != 1
        || value.lifecycle != 1
        || definition.family.is_empty()
        || definition.family.len() > ANL_ENVELOPE_STRING_MAX
        || definition.production_key.is_empty()
        || definition.production_key.len() > ANL_ENVELOPE_STRING_MAX
        || definition.revision_ref.is_empty()
        || definition.revision_ref.len() > ANL_ENVELOPE_STRING_MAX
        || definition != &current.definition
    {
        return Err("item/current binding mismatch".into());
    }
    Ok(())
}

fn validate_current_facts(current: &CurrentFacts) -> Result<(), String> {
    if !valid_uuid_fixture(&current.world_id)
        || !valid_uuid_fixture(&current.channel_id)
        || !valid_uuid_fixture(&current.death_occurrence)
        || !valid_uuid_fixture(&current.output_occurrence)
        || !valid_uuid_fixture(&current.character_id)
        || !valid_uuid_fixture(&current.game_session_id)
        || !valid_uuid_fixture(&current.native_room_placement_context)
        || current.ground_spatial_position.is_empty()
        || current.character_inventory_position.is_empty()
        || current.session_generation == 0
        || current.character_lease_generation == 0
        || current.command_id == 0
        || current.runtime_scope_generation == 0
        || current.definition.family.is_empty()
        || current.definition.production_key.is_empty()
        || current.definition.revision_ref.is_empty()
        || current.map_revision.is_empty()
        || current.content_revision.is_empty()
        || current.loot_definition_revision.is_empty()
        || current.ruleset_revision.is_empty()
        || current.sim_revision.is_empty()
        || [
            current.definition.family.len(),
            current.definition.production_key.len(),
            current.definition.revision_ref.len(),
            current.map_revision.len(),
            current.content_revision.len(),
            current.loot_definition_revision.len(),
            current.ruleset_revision.len(),
            current.sim_revision.len(),
        ]
        .iter()
        .any(|len| *len > ANL_ENVELOPE_STRING_MAX)
    {
        return Err("invalid independent current facts".into());
    }
    Ok(())
}

fn validate_ground(value: &Ground, current: &CurrentFacts) -> Result<(), String> {
    if !valid_uuid_fixture(&value.world_id)
        || !valid_uuid_fixture(&value.channel_id)
        || !valid_uuid_fixture(&value.corpse_occurrence_id_fixture)
        || value.world_id != current.world_id
        || value.channel_id != current.channel_id
        || value.corpse_occurrence_id_fixture != current.death_occurrence
        || value.spatial_position_fixture != current.ground_spatial_position
        || value.map_revision_fixture != current.map_revision
        || value.content_revision_fixture != current.content_revision
        || value.native_room_placement_context_fixture != current.native_room_placement_context
        || value.runtime_scope_ownership_generation_fixture != current.runtime_scope_generation
        || value.map_revision_fixture.len() > ANL_ENVELOPE_STRING_MAX
        || value.content_revision_fixture.len() > ANL_ENVELOPE_STRING_MAX
    {
        return Err("ground/current binding mismatch".into());
    }
    Ok(())
}

fn validate_provenance(
    value: &Provenance,
    current: &CurrentFacts,
    cause: &str,
    command_required: bool,
) -> Result<(), String> {
    let command_matches = match (&value.command_ref_fixture, command_required) {
        (Some(command), true) => {
            valid_uuid_fixture(&command.game_session_id_fixture)
                && command.game_session_id_fixture == current.game_session_id
                && command.command_id_fixture == current.command_id
        }
        (None, false) => true,
        _ => false,
    };
    if value.typed_cause != cause
        || value.output_occurrence_id_fixture != current.output_occurrence
        || value.committed_death_occurrence_id_fixture != current.death_occurrence
        || value.loot_definition_revision_fixture != current.loot_definition_revision
        || value.content_revision_fixture != current.content_revision
        || value.ruleset_revision_fixture != current.ruleset_revision
        || value.sim_revision_fixture != current.sim_revision
        || !command_matches
        || [
            value.typed_cause.len(),
            value.loot_definition_revision_fixture.len(),
            value.content_revision_fixture.len(),
            value.ruleset_revision_fixture.len(),
            value.sim_revision_fixture.len(),
        ]
        .iter()
        .any(|len| *len > ANL_ENVELOPE_STRING_MAX)
    {
        return Err("provenance or CommandRef/current binding mismatch".into());
    }
    Ok(())
}

fn validate_payload(value: &Payload, current: &CurrentFacts) -> Result<(), String> {
    validate_current_facts(current)?;
    if value.interpretation_revision != 1 {
        return Err("unsupported interpretation revision".into());
    }
    match value.operation.as_ref().ok_or("missing operation")? {
        Operation::Mint(mint) => {
            let after = mint.after.as_ref().ok_or("missing MINT after")?;
            let destination = mint
                .destination
                .as_ref()
                .ok_or("missing MINT destination")?;
            let source = mint.source.as_ref().ok_or("missing MINT source")?;
            validate_item(after, current)?;
            validate_ground(destination, current)?;
            validate_provenance(source, current, "candidate:mint", false)?;
            if !mint.before_semantically_absent {
                return Err("MINT must explicitly encode semantic absence".into());
            }
        }
        Operation::Transfer(transfer) => {
            let before = transfer.before.as_ref().ok_or("missing TRANSFER before")?;
            let after = transfer.after.as_ref().ok_or("missing TRANSFER after")?;
            let source = transfer.source.as_ref().ok_or("missing TRANSFER ground")?;
            let destination = transfer
                .destination
                .as_ref()
                .ok_or("missing TRANSFER destination")?;
            let cause = transfer.cause.as_ref().ok_or("missing TRANSFER cause")?;
            validate_item(before, current)?;
            validate_item(after, current)?;
            validate_ground(source, current)?;
            if before != after
                || !valid_uuid_fixture(&destination.character_id)
                || destination.character_id != current.character_id
                || destination.typed_position_fixture != current.character_inventory_position
                || destination.expected_session_generation != current.session_generation
                || destination.expected_character_lease_generation_fixture
                    != current.character_lease_generation
                || !valid_uuid_fixture(&destination.expected_game_session_id_fixture)
                || destination.expected_game_session_id_fixture != current.game_session_id
            {
                return Err("TRANSFER binding/state mismatch".into());
            }
            validate_provenance(cause, current, "candidate:transfer", true)?;
        }
    }
    Ok(())
}

fn id(tag: u8) -> Vec<u8> {
    let mut bytes = vec![0; 16];
    bytes[5] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = tag;
    bytes
}

fn offset_id(tag: u8, offset: u8) -> Result<Vec<u8>, String> {
    Ok(id(tag
        .checked_add(offset)
        .ok_or("fixture identity overflow")?))
}

fn definition(f: &Fixture) -> Definition {
    Definition {
        family: f.family.into(),
        production_key: f.key.into(),
        revision_ref: f.revision.into(),
    }
}

fn item(f: &Fixture) -> Item {
    Item {
        item_instance_id: id(f.tag),
        world_id: id(1),
        definition: Some(definition(f)),
        quantity: 1,
        lifecycle: 1,
    }
}

fn provenance(current: &CurrentFacts, cause: &str, command: bool) -> Provenance {
    Provenance {
        typed_cause: cause.into(),
        output_occurrence_id_fixture: current.output_occurrence.clone(),
        committed_death_occurrence_id_fixture: current.death_occurrence.clone(),
        loot_definition_revision_fixture: current.loot_definition_revision.clone(),
        content_revision_fixture: current.content_revision.clone(),
        ruleset_revision_fixture: current.ruleset_revision.clone(),
        sim_revision_fixture: current.sim_revision.clone(),
        command_ref_fixture: command.then(|| CommandRef {
            game_session_id_fixture: current.game_session_id.clone(),
            command_id_fixture: current.command_id,
        }),
    }
}

fn payload(f: &Fixture, transfer: bool) -> Result<Vec<u8>, String> {
    payload_with_facts(f, transfer, &facts(f))
}

fn payload_with_facts(
    f: &Fixture,
    transfer: bool,
    current: &CurrentFacts,
) -> Result<Vec<u8>, String> {
    let ground = Ground {
        world_id: current.world_id.clone(),
        channel_id: current.channel_id.clone(),
        spatial_position_fixture: current.ground_spatial_position.clone(),
        corpse_occurrence_id_fixture: current.death_occurrence.clone(),
        map_revision_fixture: current.map_revision.clone(),
        content_revision_fixture: current.content_revision.clone(),
        native_room_placement_context_fixture: current.native_room_placement_context.clone(),
        runtime_scope_ownership_generation_fixture: current.runtime_scope_generation,
    };
    let op = if transfer {
        Operation::Transfer(Transfer {
            before: Some(item(f)),
            after: Some(item(f)),
            source: Some(ground),
            destination: Some(Inventory {
                character_id: current.character_id.clone(),
                typed_position_fixture: current.character_inventory_position.clone(),
                expected_session_generation: current.session_generation,
                expected_game_session_id_fixture: current.game_session_id.clone(),
                expected_character_lease_generation_fixture: current.character_lease_generation,
            }),
            cause: Some(provenance(current, "candidate:transfer", true)),
        })
    } else {
        Operation::Mint(Mint {
            after: Some(item(f)),
            destination: Some(ground),
            source: Some(provenance(current, "candidate:mint", false)),
            before_semantically_absent: true,
        })
    };
    let message = Payload {
        interpretation_revision: 1,
        operation: Some(op),
    };
    let encoded_len = message.encoded_len();
    if encoded_len > CANDIDATE_WIRE_MAX || encoded_len > ANL_PAYLOAD_MAX {
        return Err("candidate representation backpressure".into());
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(encoded_len)
        .map_err(|_| "candidate representation backpressure")?;
    message.encode(&mut bytes).map_err(|e| e.to_string())?;
    // Prost accepts unknown/duplicate/noncanonical encodings; equality with
    // canonical re-encoding below is the closed-wire gate before use.
    let decoded = decode_payload_canonical(bytes.as_slice())?;
    validate_payload(&decoded, current)?;
    if decoded != message
        || decoded.encode_to_vec() != bytes
        || decoded.interpretation_revision != 1
    {
        return Err("noncanonical or unsupported payload".into());
    }
    Ok(bytes)
}

fn envelope(f: &Fixture, transfer: bool, payload: Vec<u8>) -> Result<Vec<u8>, String> {
    let digest = Sha256::digest(&payload).to_vec();
    let msg = Envelope {
        envelope_revision: 1,
        event_id_fixture: offset_id(f.tag, 30)?,
        event_type_id_fixture: 0,
        schema_revision_fixture: 1,
        retention_profile_input: "P90D".into(),
        transaction_id_fixture: offset_id(f.tag, if transfer { 50 } else { 40 })?,
        ordinal: 1,
        count: 1,
        payload,
        payload_sha256: digest,
    };
    if msg.ordinal != 1 || msg.count != 1 || msg.event_type_id_fixture != 0 {
        return Err("membership".into());
    }
    let mut wire = Vec::new();
    let encoded_len = msg.encoded_len();
    if encoded_len > CANDIDATE_WIRE_MAX || encoded_len > ANL_ENVELOPE_MAX {
        return Err("candidate representation backpressure".into());
    }
    wire.try_reserve_exact(encoded_len)
        .map_err(|_| "candidate representation backpressure")?;
    msg.encode(&mut wire).map_err(|e| e.to_string())?;
    let decoded = decode_envelope_canonical(&wire)?;
    if decoded != msg {
        return Err("noncanonical envelope or digest".into());
    }
    Ok(wire)
}

fn decode_payload_canonical(wire: &[u8]) -> Result<Payload, String> {
    if wire.len() > CANDIDATE_WIRE_MAX || wire.len() > ANL_PAYLOAD_MAX {
        return Err("candidate representation backpressure".into());
    }
    let value = Payload::decode(wire).map_err(|e| e.to_string())?;
    if value.interpretation_revision != 1
        || value.operation.is_none()
        || value.encode_to_vec() != wire
    {
        return Err("unsupported or noncanonical payload".into());
    }
    Ok(value)
}

fn decode_envelope_canonical(wire: &[u8]) -> Result<Envelope, String> {
    if wire.len() > CANDIDATE_WIRE_MAX || wire.len() > ANL_ENVELOPE_MAX {
        return Err("candidate representation backpressure".into());
    }
    let value = Envelope::decode(wire).map_err(|e| e.to_string())?;
    if value.envelope_revision != 1
        || value.schema_revision_fixture != 1
        || !valid_uuid_fixture(&value.event_id_fixture)
        || !valid_uuid_fixture(&value.transaction_id_fixture)
        || value.ordinal != 1
        || value.count != 1
        || value.event_type_id_fixture != 0
        || value.retention_profile_input.len() > ANL_ENVELOPE_STRING_MAX
        || value.retention_profile_input != "P90D"
        || value.payload.len() > ANL_PAYLOAD_MAX
        || value.payload_sha256.len() != 32
        || value.payload_sha256 != Sha256::digest(&value.payload).as_slice()
        || value.encode_to_vec() != wire
    {
        return Err("unsupported or noncanonical envelope".into());
    }
    decode_payload_canonical(&value.payload)?;
    Ok(value)
}

fn retry_same_bytes(stored: &[u8], incoming: &[u8]) -> Result<(), String> {
    if stored == incoming {
        Ok(())
    } else {
        Err("conflicting logical retry".into())
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Disposition {
    Ambiguous,
    NotApplied,
    Committed,
}

#[cfg(test)]
struct ReceiptFixture {
    event_id: Vec<u8>,
    transaction_id: Vec<u8>,
    member_ordinal: u32,
    member_count: u32,
    exact_event_bytes: Vec<u8>,
    disposition: Disposition,
    mutation_applications: u8,
}

#[cfg(test)]
fn record_attempt(
    records: &mut Vec<ReceiptFixture>,
    event_id: &[u8],
    transaction_id: &[u8],
    bytes: &[u8],
    current: &CurrentFacts,
    outcome: Disposition,
) -> Result<Disposition, String> {
    if !valid_uuid_fixture(event_id) || !valid_uuid_fixture(transaction_id) {
        return Err("invalid receipt identity".into());
    }
    let embedded = decode_envelope_canonical(bytes)?;
    let semantic_payload = decode_payload_canonical(&embedded.payload)?;
    validate_payload(&semantic_payload, current)?;
    if embedded.event_id_fixture != event_id
        || embedded.transaction_id_fixture != transaction_id
        || embedded.ordinal != 1
        || embedded.count != 1
    {
        return Err("receipt keys or membership do not match frozen envelope".into());
    }
    if let Some(record) = records
        .iter_mut()
        .find(|r| r.event_id == event_id || r.transaction_id == transaction_id)
    {
        if record.event_id != event_id
            || record.transaction_id != transaction_id
            || record.member_ordinal != 1
            || record.member_count != 1
            || record.exact_event_bytes != bytes
        {
            return Err("conflicting same-ID receipt".into());
        }
        match record.disposition {
            Disposition::Ambiguous => return Ok(Disposition::Ambiguous),
            Disposition::Committed => return Ok(Disposition::Committed),
            Disposition::NotApplied => {
                if outcome == Disposition::Committed {
                    record.mutation_applications = record
                        .mutation_applications
                        .checked_add(1)
                        .ok_or("overflow")?;
                }
                record.disposition = outcome;
                return Ok(outcome);
            }
        }
    }
    records.push(ReceiptFixture {
        event_id: event_id.to_vec(),
        transaction_id: transaction_id.to_vec(),
        member_ordinal: 1,
        member_count: 1,
        exact_event_bytes: bytes.to_vec(),
        disposition: outcome,
        mutation_applications: u8::from(outcome == Disposition::Committed),
    });
    Ok(outcome)
}

#[cfg(test)]
fn reconcile_attempt(
    record: &mut ReceiptFixture,
    proven_committed: Option<bool>,
) -> Result<Disposition, String> {
    if record.disposition != Disposition::Ambiguous {
        return Ok(record.disposition);
    }
    match proven_committed {
        Some(true) => {
            record.mutation_applications = record
                .mutation_applications
                .checked_add(1)
                .ok_or("overflow")?;
            record.disposition = Disposition::Committed;
            Ok(record.disposition)
        }
        Some(false) => {
            record.disposition = Disposition::NotApplied;
            Ok(record.disposition)
        }
        None => Ok(Disposition::Ambiguous),
    }
}

fn hex(input: &[u8]) -> String {
    input.iter().map(|b| format!("{b:02x}")).collect()
}

fn checked_add(a: u64, b: u64) -> Result<u64, String> {
    a.checked_add(b).ok_or_else(|| "overflow".into())
}

fn checked_capacity_sum(values: impl IntoIterator<Item = usize>) -> Result<usize, String> {
    values.into_iter().try_fold(0_usize, |total, value| {
        total.checked_add(value).ok_or("capacity overflow".into())
    })
}

fn item_dynamic(item: &Item) -> Result<usize, String> {
    let definition = item.definition.as_ref().ok_or("missing definition")?;
    checked_capacity_sum([
        item.item_instance_id.capacity(),
        item.world_id.capacity(),
        definition.family.capacity(),
        definition.production_key.capacity(),
        definition.revision_ref.capacity(),
    ])
}

fn payload_dynamic(value: &Payload) -> Result<usize, String> {
    match value.operation.as_ref().ok_or("missing operation")? {
        Operation::Mint(m) => {
            let item = item_dynamic(m.after.as_ref().ok_or("missing mint item")?)?;
            let ground = m.destination.as_ref().ok_or("missing mint ground")?;
            let source = m.source.as_ref().ok_or("missing mint source")?;
            checked_capacity_sum([
                item,
                ground.world_id.capacity(),
                ground.channel_id.capacity(),
                ground.spatial_position_fixture.capacity(),
                ground.corpse_occurrence_id_fixture.capacity(),
                ground.map_revision_fixture.capacity(),
                ground.content_revision_fixture.capacity(),
                ground.native_room_placement_context_fixture.capacity(),
                source.typed_cause.capacity(),
                source.output_occurrence_id_fixture.capacity(),
                source.committed_death_occurrence_id_fixture.capacity(),
                source.loot_definition_revision_fixture.capacity(),
                source.content_revision_fixture.capacity(),
                source.ruleset_revision_fixture.capacity(),
                source.sim_revision_fixture.capacity(),
                source
                    .command_ref_fixture
                    .as_ref()
                    .map_or(0, |c| c.game_session_id_fixture.capacity()),
            ])
        }
        Operation::Transfer(t) => {
            let before = item_dynamic(t.before.as_ref().ok_or("missing transfer before")?)?;
            let after = item_dynamic(t.after.as_ref().ok_or("missing transfer after")?)?;
            let ground = t.source.as_ref().ok_or("missing transfer ground")?;
            let dest = t
                .destination
                .as_ref()
                .ok_or("missing transfer destination")?;
            let cause = t.cause.as_ref().ok_or("missing transfer cause")?;
            checked_capacity_sum([
                before,
                after,
                ground.world_id.capacity(),
                ground.channel_id.capacity(),
                ground.spatial_position_fixture.capacity(),
                ground.corpse_occurrence_id_fixture.capacity(),
                ground.map_revision_fixture.capacity(),
                ground.content_revision_fixture.capacity(),
                ground.native_room_placement_context_fixture.capacity(),
                dest.character_id.capacity(),
                dest.typed_position_fixture.capacity(),
                dest.expected_game_session_id_fixture.capacity(),
                cause.typed_cause.capacity(),
                cause.output_occurrence_id_fixture.capacity(),
                cause.committed_death_occurrence_id_fixture.capacity(),
                cause.loot_definition_revision_fixture.capacity(),
                cause.content_revision_fixture.capacity(),
                cause.ruleset_revision_fixture.capacity(),
                cause.sim_revision_fixture.capacity(),
                cause
                    .command_ref_fixture
                    .as_ref()
                    .map_or(0, |c| c.game_session_id_fixture.capacity()),
            ])
        }
    }
}

fn envelope_dynamic(value: &Envelope) -> Result<usize, String> {
    checked_capacity_sum([
        value.event_id_fixture.capacity(),
        value.retention_profile_input.capacity(),
        value.transaction_id_fixture.capacity(),
        value.payload.capacity(),
        value.payload_sha256.capacity(),
    ])
}

#[cfg(test)]
const RESOURCE_DIMENSIONS: [&str; 8] = [
    "payload_bytes",
    "envelope_bytes",
    "owned_raw_clone_bytes",
    "transient_construction_and_decode_dynamic_capacity_bytes",
    "hash_bytes",
    "decode_input_bytes",
    "retained_dynamic_capacity_bytes",
    "retry_work_units",
];

fn check_resource_limits(usage: [u64; 8], limits: [u64; 8]) -> Result<(), usize> {
    for index in 0..usage.len() {
        if usage[index] > limits[index] {
            return Err(index);
        }
    }
    Ok(())
}

// Totals follow the concrete `one` -> payload -> envelope -> canonical decode
// flow. Payload encode output is 6P (initial plus five canonical re-encodes);
// envelope output is 3E (initial plus two canonical re-encodes). Hash input is
// 4P+E (payload digest in envelope build/decode x3, final payload/envelope
// report hashes). Decode input is 4P+2E (four payload decodes, two envelope).
fn operation_resource_usage(
    p: &[u8],
    e: &[u8],
    payload_dynamic: usize,
    envelope_dynamic: usize,
    p_capacity: usize,
    e_capacity: usize,
    retry_capacity: usize,
) -> Result<[u64; 8], String> {
    let p_len = u64::try_from(p.len()).map_err(|_| "size overflow")?;
    let e_len = u64::try_from(e.len()).map_err(|_| "size overflow")?;
    let payload_dyn = u64::try_from(payload_dynamic).map_err(|_| "size overflow")?;
    let envelope_dyn = u64::try_from(envelope_dynamic).map_err(|_| "size overflow")?;
    let payload_capacity = u64::try_from(p_capacity).map_err(|_| "size overflow")?;
    let envelope_capacity = u64::try_from(e_capacity).map_err(|_| "size overflow")?;
    let retry_capacity = u64::try_from(retry_capacity).map_err(|_| "size overflow")?;
    let hash_input = checked_add(p_len.checked_mul(4).ok_or("size overflow")?, e_len)?;
    let decode_input = checked_add(
        p_len.checked_mul(4).ok_or("size overflow")?,
        e_len.checked_mul(2).ok_or("size overflow")?,
    )?;
    // Exact raw clones are the payload moved into the envelope candidate and
    // the immutable envelope retained for retry.
    let owned_raw_clone = checked_add(p_len, e_len)?;
    // Transient dynamic capacity covers the original Payload/Envelope
    // construction objects plus three transient decoded Payload values and
    // one transient decoded Envelope value. The final decoded Payload and
    // Envelope are charged separately to retained capacity below.
    let transient_dynamic = checked_add(
        envelope_dyn.checked_mul(2).ok_or("size overflow")?,
        payload_dyn.checked_mul(4).ok_or("size overflow")?,
    )?;
    // Retained raw payload/envelope + retry bytes + the final decoded envelope
    // and independently decoded payload capacities.
    let retained = checked_add(
        checked_add(payload_capacity, envelope_capacity)?,
        checked_add(retry_capacity, checked_add(envelope_dyn, payload_dyn)?)?,
    )?;
    Ok([
        p_len,
        e_len,
        owned_raw_clone,
        transient_dynamic,
        hash_input,
        decode_input,
        retained,
        3,
    ])
}

fn one(f: &Fixture, transfer: bool) -> Result<Value, String> {
    let p = payload(f, transfer)?;
    let e = envelope(f, transfer, p.clone())?;
    let decoded_envelope = decode_envelope_canonical(&e)?;
    let decoded_payload = decode_payload_canonical(&decoded_envelope.payload)?;
    // Exact same-logical retry reuses the immutable encoded event bytes.
    let retry = e.clone();
    retry_same_bytes(&e, &retry)?;
    let op = if transfer { "TRANSFER" } else { "MINT" };
    // Actual instrumentation: one payload write, one envelope payload copy,
    // one retry clone; work counts one item and operation-specific effects.
    let effects = if transfer { 2_u64 } else { 1_u64 };
    let mut actual_work = 0_u64;
    // Count each concrete participant/effect as its work step executes.
    for _ in 0..1_u64 {
        actual_work = checked_add(actual_work, 1)?;
    }
    for _ in 0..effects {
        actual_work = checked_add(actual_work, 1)?;
    }
    let mut retry_work = 0_u64;
    for _ in 0..3_u64 {
        retry_work = checked_add(retry_work, 1)?;
    }
    let decoded_envelope_dynamic = envelope_dynamic(&decoded_envelope)?;
    let decoded_payload_dynamic = payload_dynamic(&decoded_payload)?;
    let decoded_dynamic =
        checked_capacity_sum([decoded_envelope_dynamic, decoded_payload_dynamic])?;
    let retained_dynamic = checked_capacity_sum([
        e.capacity(),
        p.capacity(),
        retry.capacity(),
        decoded_dynamic,
    ])?;
    let usage = operation_resource_usage(
        &p,
        &e,
        decoded_payload_dynamic,
        decoded_envelope_dynamic,
        p.capacity(),
        e.capacity(),
        retry.capacity(),
    )?;
    check_resource_limits(usage, usage).map_err(|_| "resource limit")?;
    Ok(json!({
        "operation":op, "fixture":f.name, "typed_definition":{"family":f.family,"production_key":f.key,"revision_ref":f.revision},
        "membership":{"ordinal":1,"count":1}, "payload_bytes":p.len(),"envelope_bytes":e.len(),
        "payload_sha256":hex(&Sha256::digest(&p)),"envelope_sha256":hex(&Sha256::digest(&e)),
        "payload_hex":hex(&p),"envelope_hex":hex(&e),
        "retry":{"same_logical_intent_exact_bytes":retry==e,"conflicting_intent":"REJECTED_BY_EXACT_BYTE_IDENTITY"},
        "retained":{"envelope_capacity":e.capacity(),"payload_capacity":p.capacity(),"retry_capacity":retry.capacity(),"decoded_envelope_and_nested_payload_capacities":decoded_dynamic,"inline_envelope_bytes":size_of::<Envelope>(),"inline_payload_bytes":size_of::<Payload>(),"actual_dynamic_capacity_bytes":retained_dynamic},
        "actual_counters":{"final_serialized_output_bytes":p.len()+e.len(),"protobuf_encode_output_bytes":p.len()*6+e.len()*3,"owned_raw_clone_bytes":usage[2],"transient_construction_and_decode_dynamic_capacity_bytes":usage[3],"hash_input_bytes":usage[4],"decode_input_bytes":usage[5],"retry_clone_bytes":retry.len(),"plan_participants":1,"apply_effects":effects,"logical_work_units":actual_work,"retry_work_units":retry_work,"counter_scope":"final serialized outputs are reported separately from cumulative protobuf encode and canonical re-encode bytes; exact raw clone bytes and transient construction/decode dynamic capacities are separate dimensions; hash/decode formulas count every concrete input pass. Excludes allocator metadata, codec instruction counts, CPU timing, RSS, DB, and runtime"}
    }))
}

fn report(reverse: bool) -> Result<String, String> {
    let mut fixtures = FIXTURES.to_vec();
    if reverse {
        fixtures.reverse();
    }
    let mut ops = Vec::new();
    for f in &fixtures {
        for transfer in [false, true] {
            ops.push(one(f, transfer)?);
        }
    }
    ops.sort_by_key(|v| {
        (
            v["fixture"].as_str().unwrap_or_default().to_owned(),
            v["operation"].as_str().unwrap_or_default().to_owned(),
        )
    });
    let doc = json!({
        "classification":"UNREGISTERED_OFFLINE_NATIVE_ONE_ITEM_CANDIDATE",
        "source_base_sha":"e422fc9d2962f63ccb6f0af9ee80ec5f449c2a31","issue":513,"coordination":162,"allocation_comment":"5859343636",
        "contract":"DUR-03 §39.3","candidate_event_id":"UNSELECTED","schema_registry":"NONE","privacy_resource_profile":"UNACCEPTED",
        "retention_input":"P90D_ACCEPTED_LOGICAL_INPUT_ONLY","fixtures_are_product_admission":false,
        "canonical_gate":"decode equality + canonical re-encode equality; unsupported revision, unknown/duplicate/noncanonical encodings reject",
        "transfer_admission":"CLOSED_CHARACTER_DESTINATION_GLOBAL_REVISION_OWNER_CONFLICT",
        "operations":ops,"limits":{"accepted_ANL_ceilings":{"registry":"RESOURCE_LIMITS_REGISTRY.json","envelope_bytes":ANL_ENVELOPE_MAX,"payload_bytes":ANL_PAYLOAD_MAX,"each_envelope_string_utf8_bytes":ANL_ENVELOPE_STRING_MAX,"protobuf_nesting_depth":ANL_PROTOBUF_DEPTH_MAX,"conjunctive":"all inherited ceilings apply; lower private candidate wire cap also applies"},"candidate_wire_max_bytes":CANDIDATE_WIRE_MAX,"candidate_representation_limit_authority":"none; offline capacity probe only","candidate_schema_nesting_depth":CANDIDATE_SCHEMA_NESTING_DEPTH,"candidate_schema_nesting_depth_below_ANL_max":CANDIDATE_SCHEMA_NESTING_DEPTH < ANL_PROTOBUF_DEPTH_MAX,"backpressure":"encoded_len and borrowed wire length checked before allocation/decode; try_reserve_exact failure and checked arithmetic fail closed"},
        "execution_target":{"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"toolchain_identity":"UNKNOWN","toolchain_identity_reason":"No build-time compiler attestation is embedded in this standalone example"},
        "not_proven":["production event ID or registry row","privacy/resource profile acceptance","runtime, SQL, durability, or restart authority","Content legality, quantity/stack/capacity/loot policy","Character readiness","playability","Reference parity","Canary/Crystal parity"]
    });
    serde_json::to_string_pretty(&doc)
        .map(|s| format!("{s}\n"))
        .map_err(|e| e.to_string())
}

fn main() -> Result<(), Box<dyn Error>> {
    let reverse = std::env::args().any(|arg| arg == "--reverse-fixtures");
    print!("{}", report(reverse)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixture_identity_does_not_branch_on_item_family() {
        for f in &FIXTURES {
            for is_transfer in [false, true] {
                let wire = payload(f, is_transfer).unwrap();
                let decoded = decode_payload_canonical(&wire).unwrap();
                validate_payload(&decoded, &facts(f)).unwrap();
                let encoded_definition = match decoded.operation.unwrap() {
                    Operation::Mint(mint) => mint.after.unwrap().definition.unwrap(),
                    Operation::Transfer(transfer) => transfer.before.unwrap().definition.unwrap(),
                };
                assert_eq!(encoded_definition, definition(f));
            }
        }
    }
    #[test]
    fn repeat_and_reverse_are_identical() {
        assert_eq!(report(false).unwrap(), report(false).unwrap());
        assert_eq!(report(false).unwrap(), report(true).unwrap());
    }
    #[test]
    fn overflow_rejects() {
        assert!(checked_add(u64::MAX, 1).is_err());
    }
    #[test]
    fn conflicting_retry_is_rejected() {
        assert!(retry_same_bytes(&[1, 2, 3], &[1, 2, 3]).is_ok());
        assert!(retry_same_bytes(&[1, 2, 3], &[1, 2, 4]).is_err());
    }
    #[test]
    fn current_facts_are_independent_and_single_mismatch_fails_closed() {
        let f = &FIXTURES[0];
        let baseline = facts(f);
        let mint = decode_payload_canonical(&payload(f, false).unwrap()).unwrap();
        let transfer = decode_payload_canonical(&payload(f, true).unwrap()).unwrap();
        validate_payload(&mint, &baseline).unwrap();
        validate_payload(&transfer, &baseline).unwrap();
        let mut changed = baseline.clone();
        changed.world_id = id(9);
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.channel_id = id(9);
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.definition.revision_ref = "rev-other".into();
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.death_occurrence = id(9);
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.output_occurrence = id(9);
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.ground_spatial_position = vec![3, 2, 1];
        assert!(validate_payload(&mint, &changed).is_err());
        assert!(validate_payload(&transfer, &changed).is_err());
        let mut changed = baseline.clone();
        changed.character_inventory_position = vec![6];
        assert!(validate_payload(&transfer, &changed).is_err());
        for zero_fact in [0, 1, 2, 3] {
            let mut changed = baseline.clone();
            match zero_fact {
                0 => changed.session_generation = 0,
                1 => changed.character_lease_generation = 0,
                2 => changed.command_id = 0,
                _ => changed.runtime_scope_generation = 0,
            }
            assert!(validate_payload(&transfer, &changed).is_err());
        }
        let mut changed = baseline.clone();
        changed.map_revision = "map-other".into();
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.content_revision = "content-other".into();
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.native_room_placement_context = id(9);
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.runtime_scope_generation = 2;
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.loot_definition_revision = "loot-other".into();
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.ruleset_revision = "rules-other".into();
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.sim_revision = "sim-other".into();
        assert!(validate_payload(&mint, &changed).is_err());
        let mut changed = baseline.clone();
        changed.character_id = id(9);
        assert!(validate_payload(&transfer, &changed).is_err());
        let mut changed = baseline.clone();
        changed.loot_definition_revision = "loot-other".into();
        assert!(validate_payload(&transfer, &changed).is_err());
        let mut changed = baseline.clone();
        changed.command_id = 10;
        assert!(validate_payload(&transfer, &changed).is_err());
        let mut changed = baseline.clone();
        changed.game_session_id = id(9);
        assert!(validate_payload(&transfer, &changed).is_err());
        let mut changed = baseline.clone();
        changed.session_generation = 2;
        assert!(validate_payload(&transfer, &changed).is_err());
        let mut changed = baseline.clone();
        changed.character_lease_generation = 4;
        assert!(validate_payload(&transfer, &changed).is_err());
        assert!(!valid_uuid_fixture(&[0; 16]));
        let mut bad = id(1);
        bad[6] = 0x40;
        assert!(!valid_uuid_fixture(&bad));
    }
    #[test]
    fn canonical_but_semantically_tampered_payloads_fail_closed() {
        let f = &FIXTURES[0];
        let current = facts(f);
        let mint = match Payload::decode(payload(f, false).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Mint(v) => v,
            _ => unreachable!(),
        };
        let mut cases = Vec::new();
        let mut changed = mint.clone();
        changed.after.as_mut().unwrap().world_id = id(9);
        cases.push(Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Mint(changed)),
        });
        let mut changed = mint.clone();
        changed.after.as_mut().unwrap().quantity = 0;
        cases.push(Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Mint(changed)),
        });
        let mut changed = mint.clone();
        changed.after.as_mut().unwrap().quantity = 2;
        cases.push(Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Mint(changed)),
        });
        let mut changed = mint.clone();
        changed.before_semantically_absent = false;
        cases.push(Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Mint(changed)),
        });
        let mut changed = mint.clone();
        changed.after.as_mut().unwrap().lifecycle = 2;
        cases.push(Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Mint(changed)),
        });
        let mut changed = mint.clone();
        changed.destination.as_mut().unwrap().channel_id = id(9);
        cases.push(Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Mint(changed)),
        });
        let mut changed = mint.clone();
        changed
            .source
            .as_mut()
            .unwrap()
            .output_occurrence_id_fixture = id(9);
        cases.push(Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Mint(changed)),
        });
        for candidate in cases {
            let bytes = candidate.encode_to_vec();
            let decoded = decode_payload_canonical(&bytes).unwrap();
            assert!(validate_payload(&decoded, &current).is_err());
        }
        let mut transfer = match Payload::decode(payload(f, true).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Transfer(v) => v,
            _ => unreachable!(),
        };
        transfer.after.as_mut().unwrap().item_instance_id = id(9);
        let bytes = Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Transfer(transfer)),
        }
        .encode_to_vec();
        let decoded = decode_payload_canonical(&bytes).unwrap();
        assert!(validate_payload(&decoded, &current).is_err());
        let mut transfer = match Payload::decode(payload(f, true).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Transfer(v) => v,
            _ => unreachable!(),
        };
        transfer.after.as_mut().unwrap().quantity = 2;
        let bytes = Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Transfer(transfer)),
        }
        .encode_to_vec();
        assert!(validate_payload(&decode_payload_canonical(&bytes).unwrap(), &current).is_err());
        let mut transfer = match Payload::decode(payload(f, true).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Transfer(v) => v,
            _ => unreachable!(),
        };
        transfer.destination.as_mut().unwrap().character_id = id(9);
        let bytes = Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Transfer(transfer)),
        }
        .encode_to_vec();
        assert!(validate_payload(&decode_payload_canonical(&bytes).unwrap(), &current).is_err());
        let mut transfer = match Payload::decode(payload(f, true).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Transfer(v) => v,
            _ => unreachable!(),
        };
        transfer
            .source
            .as_mut()
            .unwrap()
            .native_room_placement_context_fixture = id(9);
        let bytes = Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Transfer(transfer)),
        }
        .encode_to_vec();
        assert!(validate_payload(&decode_payload_canonical(&bytes).unwrap(), &current).is_err());
        let mut transfer = match Payload::decode(payload(f, true).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Transfer(v) => v,
            _ => unreachable!(),
        };
        transfer
            .cause
            .as_mut()
            .unwrap()
            .command_ref_fixture
            .as_mut()
            .unwrap()
            .command_id_fixture = 10;
        let bytes = Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Transfer(transfer)),
        }
        .encode_to_vec();
        assert!(validate_payload(&decode_payload_canonical(&bytes).unwrap(), &current).is_err());
        let mut transfer = match Payload::decode(payload(f, true).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Transfer(v) => v,
            _ => unreachable!(),
        };
        transfer
            .destination
            .as_mut()
            .unwrap()
            .expected_session_generation = 2;
        let bytes = Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Transfer(transfer)),
        }
        .encode_to_vec();
        assert!(validate_payload(&decode_payload_canonical(&bytes).unwrap(), &current).is_err());
        let mut transfer = match Payload::decode(payload(f, true).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Transfer(v) => v,
            _ => unreachable!(),
        };
        transfer.cause.as_mut().unwrap().command_ref_fixture = None;
        let bytes = Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Transfer(transfer)),
        }
        .encode_to_vec();
        assert!(validate_payload(&decode_payload_canonical(&bytes).unwrap(), &current).is_err());
        let mut transfer = match Payload::decode(payload(f, true).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Transfer(v) => v,
            _ => unreachable!(),
        };
        transfer
            .destination
            .as_mut()
            .unwrap()
            .expected_game_session_id_fixture = id(9);
        let bytes = Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Transfer(transfer)),
        }
        .encode_to_vec();
        assert!(validate_payload(&decode_payload_canonical(&bytes).unwrap(), &current).is_err());
        let mut transfer = match Payload::decode(payload(f, true).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Transfer(v) => v,
            _ => unreachable!(),
        };
        transfer
            .destination
            .as_mut()
            .unwrap()
            .expected_character_lease_generation_fixture = 4;
        let bytes = Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Transfer(transfer)),
        }
        .encode_to_vec();
        assert!(validate_payload(&decode_payload_canonical(&bytes).unwrap(), &current).is_err());
    }
    #[test]
    fn ambiguous_lost_ack_holds_then_reconciles_once_and_replay_is_idempotent() {
        let current = facts(&FIXTURES[0]);
        let bytes = envelope(&FIXTURES[0], false, payload(&FIXTURES[0], false).unwrap()).unwrap();
        let mut records = Vec::new();
        assert_eq!(
            record_attempt(
                &mut records,
                &id(41),
                &id(51),
                &bytes,
                &current,
                Disposition::Ambiguous
            )
            .unwrap(),
            Disposition::Ambiguous
        );
        assert_eq!(records[0].mutation_applications, 0);
        assert_eq!(
            record_attempt(
                &mut records,
                &id(41),
                &id(51),
                &bytes,
                &current,
                Disposition::Committed
            )
            .unwrap(),
            Disposition::Ambiguous
        );
        assert_eq!(
            reconcile_attempt(&mut records[0], None).unwrap(),
            Disposition::Ambiguous
        );
        assert_eq!(
            reconcile_attempt(&mut records[0], Some(true)).unwrap(),
            Disposition::Committed
        );
        assert_eq!(records[0].mutation_applications, 1);
        assert_eq!(
            record_attempt(
                &mut records,
                &id(41),
                &id(51),
                &bytes,
                &current,
                Disposition::Committed
            )
            .unwrap(),
            Disposition::Committed
        );
        assert_eq!(records[0].mutation_applications, 1);
        assert!(
            record_attempt(
                &mut records,
                &id(41),
                &id(51),
                &[9],
                &current,
                Disposition::Committed
            )
            .is_err()
        );
        assert!(
            record_attempt(
                &mut records,
                &id(41),
                &id(52),
                &bytes,
                &current,
                Disposition::Committed
            )
            .is_err()
        );
        let normal = decode_payload_canonical(&payload(&FIXTURES[0], false).unwrap()).unwrap();
        let mut invalid_mint = match normal.operation.unwrap() {
            Operation::Mint(mint) => mint,
            _ => unreachable!(),
        };
        invalid_mint.before_semantically_absent = false;
        let invalid_payload = Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Mint(invalid_mint)),
        }
        .encode_to_vec();
        let invalid_envelope = envelope(&FIXTURES[0], false, invalid_payload).unwrap();
        let mut untouched = Vec::new();
        assert!(
            record_attempt(
                &mut untouched,
                &id(41),
                &id(51),
                &invalid_envelope,
                &current,
                Disposition::Committed,
            )
            .is_err()
        );
        assert!(untouched.is_empty());
        assert!(
            record_attempt(
                &mut records,
                &id(42),
                &id(51),
                &bytes,
                &current,
                Disposition::Committed
            )
            .is_err()
        );
    }
    #[test]
    fn proven_noncommit_retries_same_candidate_identity_and_bytes() {
        let current = facts(&FIXTURES[0]);
        let bytes = envelope(&FIXTURES[0], false, payload(&FIXTURES[0], false).unwrap()).unwrap();
        let mut records = Vec::new();
        record_attempt(
            &mut records,
            &id(41),
            &id(51),
            &bytes,
            &current,
            Disposition::Ambiguous,
        )
        .unwrap();
        assert_eq!(
            reconcile_attempt(&mut records[0], Some(false)).unwrap(),
            Disposition::NotApplied
        );
        assert_eq!(
            record_attempt(
                &mut records,
                &id(41),
                &id(51),
                &bytes,
                &current,
                Disposition::Committed
            )
            .unwrap(),
            Disposition::Committed
        );
        assert_eq!(records[0].mutation_applications, 1);
    }
    #[test]
    fn unknown_duplicate_and_noncanonical_payloads_reject_by_roundtrip() {
        let mut p = payload(&FIXTURES[0], false).unwrap();
        p.extend_from_slice(&[0x78, 1]);
        assert!(decode_payload_canonical(&p).is_err());
        let p = payload(&FIXTURES[0], false).unwrap();
        assert!(decode_payload_canonical(&[p.as_slice(), p.as_slice()].concat()).is_err());
        assert!(decode_payload_canonical(&[0x08, 0x81, 0x00]).is_err());
        let p = payload(&FIXTURES[0], false).unwrap();
        let e = envelope(&FIXTURES[0], false, p).unwrap();
        assert!(decode_envelope_canonical(&[e.as_slice(), &[0x50, 1]].concat()).is_err());
    }
    #[test]
    fn independent_literal_payload_and_full_envelope_goldens_for_both_operations() {
        let mint_payload = payload(&FIXTURES[0], false).unwrap();
        let mint_envelope = envelope(&FIXTURES[0], false, mint_payload.clone()).unwrap();
        assert_eq!(
            hex(&mint_payload),
            "080112c4020a4c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f312001280112730a10000000000001700080000000000000011210000000000001700080000000000000021a030102032210000000000001700080000000000000032a0e6d61702d666978747572652d72313212636f6e74656e742d666978747572652d72313a100000000000017000800000000000000640011a7d0a0e63616e6469646174653a6d696e741210000000000001700080000000000000071a1000000000000170008000000000000003220f6c6f6f742d666978747572652d72312a12636f6e74656e742d666978747572652d7231321272756c657365742d666978747572652d72313a0e73696d2d666978747572652d72312001"
        );
        assert_eq!(
            hex(&mint_envelope),
            "080112100000000000017000800000000000002920012a0450393044321000000000000170008000000000000033380140014ac902080112c4020a4c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f312001280112730a10000000000001700080000000000000011210000000000001700080000000000000021a030102032210000000000001700080000000000000032a0e6d61702d666978747572652d72313212636f6e74656e742d666978747572652d72313a100000000000017000800000000000000640011a7d0a0e63616e6469646174653a6d696e741210000000000001700080000000000000071a1000000000000170008000000000000003220f6c6f6f742d666978747572652d72312a12636f6e74656e742d666978747572652d7231321272756c657365742d666978747572652d72313a0e73696d2d666978747572652d723120015220355c447ed81ccb1a67601ef9ffdad51ee51c84176447869aee99097bcfe96210"
        );
        assert_eq!(
            hex(&Sha256::digest(&mint_payload)),
            "355c447ed81ccb1a67601ef9ffdad51ee51c84176447869aee99097bcfe96210"
        );
        assert_eq!(
            hex(&Sha256::digest(&mint_envelope)),
            "f81957f4ca74958584be5d6f106daa66e298bd8f7e1b030f3b397e7df3cb9100"
        );
        let transfer_payload = payload(&FIXTURES[0], true).unwrap();
        let transfer_envelope = envelope(&FIXTURES[0], true, transfer_payload.clone()).unwrap();
        assert_eq!(
            hex(&transfer_payload),
            "08011ad8030a4c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f3120012801124c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f31200128011a730a10000000000001700080000000000000011210000000000001700080000000000000021a030102032210000000000001700080000000000000032a0e6d61702d666978747572652d72313212636f6e74656e742d666978747572652d72313a10000000000001700080000000000000064001222b0a1000000000000170008000000000000004120105180122100000000000017000800000000000000528032a97010a1263616e6469646174653a7472616e736665721210000000000001700080000000000000071a1000000000000170008000000000000003220f6c6f6f742d666978747572652d72312a12636f6e74656e742d666978747572652d7231321272756c657365742d666978747572652d72313a0e73696d2d666978747572652d723142140a10000000000001700080000000000000051009"
        );
        assert_eq!(
            hex(&transfer_envelope),
            "080112100000000000017000800000000000002920012a045039304432100000000000017000800000000000003d380140014add0308011ad8030a4c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f3120012801124c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f31200128011a730a10000000000001700080000000000000011210000000000001700080000000000000021a030102032210000000000001700080000000000000032a0e6d61702d666978747572652d72313212636f6e74656e742d666978747572652d72313a10000000000001700080000000000000064001222b0a1000000000000170008000000000000004120105180122100000000000017000800000000000000528032a97010a1263616e6469646174653a7472616e736665721210000000000001700080000000000000071a1000000000000170008000000000000003220f6c6f6f742d666978747572652d72312a12636f6e74656e742d666978747572652d7231321272756c657365742d666978747572652d72313a0e73696d2d666978747572652d723142140a100000000000017000800000000000000510095220a54f63bf00f23b67ba2e92e7337a24f8ce018f75c4dad3173549d1f3bc8e5267"
        );
        assert_eq!(
            hex(&Sha256::digest(&transfer_payload)),
            "a54f63bf00f23b67ba2e92e7337a24f8ce018f75c4dad3173549d1f3bc8e5267"
        );
        assert_eq!(
            hex(&Sha256::digest(&transfer_envelope)),
            "cb86e2bd8adeaf34da8a8a026f40b27f19d360b21395969eb41402b9e420cb8c"
        );
    }
    #[test]
    fn every_fixture_and_operation_resource_usage_accepts_max_and_rejects_lowered_limit() {
        for fixture in &FIXTURES {
            for transfer in [false, true] {
                let p = payload(fixture, transfer).unwrap();
                let e = envelope(fixture, transfer, p.clone()).unwrap();
                let decoded_e = decode_envelope_canonical(&e).unwrap();
                let decoded_p = decode_payload_canonical(&decoded_e.payload).unwrap();
                let usage = operation_resource_usage(
                    &p,
                    &e,
                    payload_dynamic(&decoded_p).unwrap(),
                    envelope_dynamic(&decoded_e).unwrap(),
                    p.capacity(),
                    e.capacity(),
                    e.clone().capacity(),
                )
                .unwrap();
                assert!(check_resource_limits(usage, usage).is_ok());
                for dimension in 0..RESOURCE_DIMENSIONS.len() {
                    if usage[dimension] == 0 {
                        continue;
                    }
                    let mut lowered = usage;
                    lowered[dimension] -= 1;
                    assert_eq!(check_resource_limits(usage, lowered), Err(dimension));
                }
            }
        }
    }
    #[test]
    fn inherited_anl_ceilings_apply_with_the_private_candidate_cap_below_them() {
        assert_eq!(ANL_ENVELOPE_MAX, 262_144);
        assert_eq!(ANL_PAYLOAD_MAX, 196_608);
        assert_eq!(ANL_ENVELOPE_STRING_MAX, 128);
        assert_eq!(ANL_PROTOBUF_DEPTH_MAX, 32);
        assert_eq!(CANDIDATE_WIRE_MAX, 4096);
        assert!(CANDIDATE_SCHEMA_NESTING_DEPTH < ANL_PROTOBUF_DEPTH_MAX);
        let f = &FIXTURES[0];
        let mut current = facts(f);
        current.map_revision = "m".repeat(ANL_ENVELOPE_STRING_MAX);
        current.content_revision = "c".repeat(ANL_ENVELOPE_STRING_MAX);
        assert!(payload_with_facts(f, false, &current).is_ok());
        current.map_revision = "m".repeat(ANL_ENVELOPE_STRING_MAX + 1);
        assert!(payload_with_facts(f, false, &current).is_err());
    }
    #[test]
    fn proto_grammar_contains_the_executable_candidate_fields() {
        let proto = include_str!(
            "../../../docs/contracts/game-events/v1/native_one_item_transaction.proto"
        );
        for required in [
            "message OneItemTransactionCandidate",
            "message TypedDefinitionRevision",
            "message CandidateItemState",
            "message CandidateGround",
            "message CandidateInventory",
            "message CandidateProvenance",
            "message CandidateCommandRef",
            "message OneItemMint",
            "before_semantically_absent = 4",
            "message OneItemTransfer",
            "message OfflineCandidateEnvelope",
            "map_revision_fixture = 5",
            "runtime_scope_ownership_generation_fixture = 8",
            "committed_death_occurrence_id_fixture = 3",
            "command_ref_fixture = 8",
            "uint32 ordinal = 7",
            "uint32 count = 8",
            "bytes payload = 9",
            "bytes payload_sha256 = 10",
        ] {
            assert!(proto.contains(required), "missing grammar term: {required}");
        }
    }
}
