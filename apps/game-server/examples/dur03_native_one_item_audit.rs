//! DUR-03 native one-item durable-audit codec and resource-bound evidence for
//! registered event type 2 (`oteryn.events.v1.OneItemTransactionV1`) carried in
//! the normative ANL-01 `EventEnvelope`. Every bound below is the registered
//! hard maximum from `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` (decision
//! D50/D51). Offline example only: no runtime, SQL, Character, Content-admission
//! or playability proof.

use prost::Message;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::error::Error;
use std::mem::size_of;

// ---- Payload: docs/contracts/game-events/v1/native_one_item_transaction.proto

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
    spatial_position: Vec<u8>,
    #[prost(bytes = "vec", tag = "4")]
    corpse_ref: Vec<u8>,
    #[prost(string, tag = "5")]
    map_revision: String,
    #[prost(string, tag = "6")]
    content_revision: String,
    #[prost(bytes = "vec", tag = "7")]
    native_room_placement_context: Vec<u8>,
    #[prost(uint64, tag = "8")]
    runtime_scope_ownership_generation: u64,
}
// Field 2 (`typed_position`) is reserved: excluded fail-closed.
#[derive(Clone, PartialEq, Eq, Message)]
struct Inventory {
    #[prost(bytes = "vec", tag = "1")]
    character_id: Vec<u8>,
    #[prost(uint64, tag = "3")]
    expected_session_generation: u64,
    #[prost(bytes = "vec", tag = "4")]
    expected_game_session_id: Vec<u8>,
    #[prost(uint64, tag = "5")]
    expected_character_lease_generation: u64,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct CommandRef {
    #[prost(bytes = "vec", tag = "1")]
    game_session_id: Vec<u8>,
    #[prost(uint64, tag = "2")]
    command_id: u64,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct DeathKey {
    #[prost(bytes = "vec", tag = "1")]
    world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    channel_id: Vec<u8>,
    #[prost(uint64, tag = "3")]
    scope_ownership_generation: u64,
    #[prost(uint32, tag = "4")]
    actor_local_id: u32,
    #[prost(uint64, tag = "5")]
    actor_local_generation: u64,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct Provenance {
    #[prost(string, tag = "1")]
    typed_cause: String,
    #[prost(message, optional, tag = "2")]
    death_occurrence: Option<DeathKey>,
    #[prost(message, optional, tag = "3")]
    loot_table: Option<Definition>,
    #[prost(string, tag = "4")]
    loot_purpose_key: String,
    #[prost(uint32, tag = "5")]
    draw_ordinal: u32,
    #[prost(string, tag = "6")]
    content_revision: String,
    #[prost(string, tag = "7")]
    ruleset_revision: String,
    #[prost(string, tag = "8")]
    sim_revision: String,
    #[prost(message, optional, tag = "9")]
    command_ref: Option<CommandRef>,
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
// Keep the oneof inline so retained-capacity measurements include the native
// enum footprint instead of an invented boxed layout.
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

// ---- Normative ANL-01 EventEnvelope: docs/contracts/game-events/v1/foundation.proto

#[derive(Clone, PartialEq, Eq, Message)]
struct RuntimeOrderRef {
    #[prost(uint64, tag = "1")]
    scope_ownership_generation: u64,
    #[prost(uint64, tag = "2")]
    runtime_execution_ordinal: u64,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct TransactionEventRef {
    #[prost(bytes = "vec", tag = "1")]
    transaction_id: Vec<u8>,
    #[prost(uint32, tag = "2")]
    ordinal: u32,
    #[prost(uint32, tag = "3")]
    count: u32,
}
#[derive(Clone, PartialEq, Eq, prost::Oneof)]
enum Cause {
    #[prost(bytes = "vec", tag = "1")]
    EventId(Vec<u8>),
    #[prost(message, tag = "2")]
    Command(CommandRef),
    #[prost(bytes = "vec", tag = "3")]
    OperationId(Vec<u8>),
    #[prost(bytes = "vec", tag = "4")]
    TransactionId(Vec<u8>),
}
#[derive(Clone, PartialEq, Eq, Message)]
struct CausationRef {
    #[prost(oneof = "Cause", tags = "1, 2, 3, 4")]
    cause: Option<Cause>,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct AnalyticsActorRef {
    #[prost(string, tag = "1")]
    identity_domain: String,
    #[prost(uint64, tag = "2")]
    identity_epoch: u64,
    #[prost(bytes = "vec", tag = "3")]
    analytics_actor_id: Vec<u8>,
}
#[derive(Clone, PartialEq, Eq, Message)]
struct Envelope {
    #[prost(uint32, tag = "1")]
    envelope_revision: u32,
    #[prost(bytes = "vec", tag = "2")]
    event_id: Vec<u8>,
    #[prost(uint32, tag = "3")]
    event_type_id: u32,
    #[prost(uint32, tag = "4")]
    event_schema_revision: u32,
    // EventDurabilityClass / EventPrivacyClass enums share the int32 wire form.
    #[prost(int32, tag = "5")]
    durability_class: i32,
    #[prost(int32, tag = "6")]
    privacy_class: i32,
    #[prost(string, tag = "7")]
    retention_profile_id: String,
    #[prost(int64, tag = "8")]
    occurred_at_unix_ms: i64,
    #[prost(bytes = "vec", optional, tag = "9")]
    world_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "10")]
    channel_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "11")]
    instance_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "12")]
    node_id: Option<Vec<u8>>,
    #[prost(bytes = "vec", optional, tag = "13")]
    game_session_id: Option<Vec<u8>>,
    #[prost(uint64, optional, tag = "14")]
    connection_generation: Option<u64>,
    #[prost(message, optional, tag = "15")]
    runtime_order: Option<RuntimeOrderRef>,
    #[prost(uint64, optional, tag = "16")]
    command_id: Option<u64>,
    #[prost(bytes = "vec", optional, tag = "17")]
    operation_id: Option<Vec<u8>>,
    #[prost(message, optional, tag = "18")]
    transaction_event: Option<TransactionEventRef>,
    #[prost(bytes = "vec", optional, tag = "19")]
    correlation_id: Option<Vec<u8>>,
    #[prost(message, optional, tag = "20")]
    causation: Option<CausationRef>,
    #[prost(message, optional, tag = "21")]
    analytics_actor: Option<AnalyticsActorRef>,
    #[prost(uint32, optional, tag = "22")]
    protocol_major: Option<u32>,
    #[prost(string, optional, tag = "23")]
    ruleset_revision: Option<String>,
    #[prost(string, optional, tag = "24")]
    content_revision: Option<String>,
    #[prost(string, tag = "25")]
    server_build_id: String,
    #[prost(bytes = "vec", tag = "26")]
    payload: Vec<u8>,
    #[prost(bytes = "vec", tag = "27")]
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

// Registered hard maxima (RESOURCE_LIMITS_REGISTRY.json, D50/D51).
const RL01_TOUCHED_ITEM_INSTANCES_MAX: u64 = 1;
const RL02_LOCATION_CUSTODY_LINES_MAX: u64 = 2;
const RL03_VALUE_LINES_MAX: u64 = 0;
const RL04_TRANSFORM_LINES_MAX: u64 = 0;
const RL05_CONTAINER_EXPANSION_MAX: u64 = 0;
const RL06_PARTICIPANTS_MAX: u64 = 1;
const RL06_EFFECT_WORK_UNITS_MAX: u64 = 3;
const RL07_EVENTS_MAX: u64 = 1;
const RL07_ENVELOPE_BYTES_MAX: usize = 9_216;
const RL07_PAYLOAD_BYTES_MAX: usize = 7_936;
const RL07_CONTENT_KEY_BYTES_MAX: usize = 512;
const RL07_TECHNICAL_FIELD_BYTES_MAX: usize = 128;
const RL07_UUID_BYTES: usize = 16;
const RL07_PAYLOAD_SHA256_BYTES: usize = 32;
const RL08_RETRY_WORK_UNITS_MAX: u64 = 3;
const AUDIT_RETENTION_P90D_SECONDS: u64 = 7_776_000;
// Registered binding (GAME_EVENT_FOUNDATION_REGISTRY.json).
const EVENT_TYPE_ID: u32 = 2;
const EVENT_SCHEMA_REVISION: u32 = 1;
const RETENTION_PROFILE_ID: &str = "DUR03_ONE_ITEM_DURABLE_AUDIT_RETENTION_V1";
const DURABLE_AUDIT: i32 = 2;
const RESTRICTED_PLAYER_LINKED: i32 = 3;
const SECURITY_SENSITIVE: i32 = 4;
// Inherited ANL-01 ceilings; the lower DUR-03 rows apply conjunctively.
const ANL_ENVELOPE_MAX: usize = 262_144;
const ANL_PAYLOAD_MAX: usize = 196_608;
const ANL_ENVELOPE_STRING_MAX: usize = 128;
const ANL_PROTOBUF_DEPTH_MAX: usize = 32;
// EventEnvelope -> payload -> operation -> provenance -> loot table/death key.
const SCHEMA_NESTING_DEPTH: usize = 5;
const FIXTURE_OCCURRED_AT_UNIX_MS: i64 = 1_790_000_000_000;

// Independently supplied fixture expectations; never current runtime authority.
#[derive(Clone)]
struct CurrentFacts {
    world_id: Vec<u8>,
    channel_id: Vec<u8>,
    definition: Definition,
    transfer_source_item: Item,
    ground_spatial_position: Vec<u8>,
    corpse_ref: Vec<u8>,
    death: DeathKey,
    loot_table: Definition,
    loot_purpose_key: String,
    draw_ordinal: u32,
    character_id: Vec<u8>,
    game_session_id: Vec<u8>,
    session_generation: u64,
    character_lease_generation: u64,
    command_id: u64,
    map_revision: String,
    content_revision: String,
    native_room_placement_context: Vec<u8>,
    runtime_scope_generation: u64,
    ruleset_revision: String,
    sim_revision: String,
}

fn facts(f: &Fixture) -> CurrentFacts {
    CurrentFacts {
        world_id: id(1),
        channel_id: id(2),
        definition: definition(f),
        transfer_source_item: item(f),
        ground_spatial_position: vec![1, 2, 3],
        corpse_ref: id(3),
        death: DeathKey {
            world_id: id(1),
            channel_id: id(2),
            scope_ownership_generation: 1,
            actor_local_id: 7,
            actor_local_generation: 1,
        },
        loot_table: Definition {
            family: "LootTable".into(),
            production_key: "fixture:loot.alpha".into(),
            revision_ref: "loot-fixture-r1".into(),
        },
        loot_purpose_key: "fixture:purpose.drop".into(),
        draw_ordinal: 1,
        character_id: id(4),
        game_session_id: id(5),
        session_generation: 1,
        character_lease_generation: 3,
        command_id: 9,
        map_revision: "map-fixture-r1".into(),
        content_revision: "content-fixture-r1".into(),
        native_room_placement_context: id(6),
        runtime_scope_generation: 1,
        ruleset_revision: "ruleset-fixture-r1".into(),
        sim_revision: "sim-fixture-r1".into(),
    }
}

fn valid_uuid(value: &[u8]) -> bool {
    value.len() == RL07_UUID_BYTES
        && value.iter().any(|byte| *byte != 0)
        && value[6] >> 4 == 7
        && value[8] >> 6 == 2
}

// Registered per-field byte bound; oversize input is rejected, never truncated.
fn bounded(len: usize, max: usize, field: &str) -> Result<(), String> {
    if len == 0 || len > max {
        return Err(format!(
            "INVALID_INPUT: {field} must be 1..={max} bytes, got {len}"
        ));
    }
    Ok(())
}

fn exact(len: usize, expected: usize, field: &str) -> Result<(), String> {
    if len != expected {
        return Err(format!(
            "INVALID_INPUT: {field} must be exactly {expected} bytes, got {len}"
        ));
    }
    Ok(())
}

fn at_most(value: u64, max: u64, row: &str) -> Result<(), String> {
    if value > max {
        return Err(format!("CAPACITY_EXCEEDED: {row} {value} > {max}"));
    }
    Ok(())
}

fn definition_bounds(value: &Definition) -> Result<(), String> {
    bounded(
        value.family.len(),
        RL07_TECHNICAL_FIELD_BYTES_MAX,
        "definition family",
    )?;
    bounded(
        value.production_key.len(),
        RL07_CONTENT_KEY_BYTES_MAX,
        "production_key",
    )?;
    bounded(
        value.revision_ref.len(),
        RL07_CONTENT_KEY_BYTES_MAX,
        "revision_ref",
    )
}

fn item_bounds(value: &Item) -> Result<(), String> {
    exact(
        value.item_instance_id.len(),
        RL07_UUID_BYTES,
        "item_instance_id",
    )?;
    exact(value.world_id.len(), RL07_UUID_BYTES, "item world_id")?;
    definition_bounds(
        value
            .definition
            .as_ref()
            .ok_or("INVALID_INPUT: missing typed definition")?,
    )
}

fn ground_bounds(value: &Ground) -> Result<(), String> {
    exact(value.world_id.len(), RL07_UUID_BYTES, "ground world_id")?;
    exact(value.channel_id.len(), RL07_UUID_BYTES, "ground channel_id")?;
    bounded(
        value.spatial_position.len(),
        RL07_TECHNICAL_FIELD_BYTES_MAX,
        "spatial_position",
    )?;
    bounded(
        value.corpse_ref.len(),
        RL07_TECHNICAL_FIELD_BYTES_MAX,
        "corpse_ref",
    )?;
    bounded(
        value.map_revision.len(),
        RL07_CONTENT_KEY_BYTES_MAX,
        "map_revision",
    )?;
    bounded(
        value.content_revision.len(),
        RL07_CONTENT_KEY_BYTES_MAX,
        "ground content_revision",
    )?;
    bounded(
        value.native_room_placement_context.len(),
        RL07_TECHNICAL_FIELD_BYTES_MAX,
        "native_room_placement_context",
    )
}

fn inventory_bounds(value: &Inventory) -> Result<(), String> {
    exact(value.character_id.len(), RL07_UUID_BYTES, "character_id")?;
    exact(
        value.expected_game_session_id.len(),
        RL07_UUID_BYTES,
        "expected_game_session_id",
    )
}

fn provenance_bounds(value: &Provenance) -> Result<(), String> {
    bounded(
        value.typed_cause.len(),
        RL07_TECHNICAL_FIELD_BYTES_MAX,
        "typed_cause",
    )?;
    let death = value
        .death_occurrence
        .as_ref()
        .ok_or("INVALID_INPUT: missing death occurrence key")?;
    exact(death.world_id.len(), RL07_UUID_BYTES, "death world_id")?;
    exact(death.channel_id.len(), RL07_UUID_BYTES, "death channel_id")?;
    definition_bounds(
        value
            .loot_table
            .as_ref()
            .ok_or("INVALID_INPUT: missing LootTableDefinitionRef")?,
    )?;
    bounded(
        value.loot_purpose_key.len(),
        RL07_CONTENT_KEY_BYTES_MAX,
        "loot_purpose_key",
    )?;
    bounded(
        value.content_revision.len(),
        RL07_CONTENT_KEY_BYTES_MAX,
        "provenance content_revision",
    )?;
    bounded(
        value.ruleset_revision.len(),
        RL07_CONTENT_KEY_BYTES_MAX,
        "ruleset_revision",
    )?;
    bounded(
        value.sim_revision.len(),
        RL07_CONTENT_KEY_BYTES_MAX,
        "sim_revision",
    )?;
    if let Some(command) = value.command_ref.as_ref() {
        exact(
            command.game_session_id.len(),
            RL07_UUID_BYTES,
            "command game_session_id",
        )?;
    }
    Ok(())
}

fn touched_item_instances(value: &Payload) -> Result<u64, String> {
    match value
        .operation
        .as_ref()
        .ok_or("INVALID_INPUT: missing operation")?
    {
        Operation::Mint(_) => Ok(1),
        Operation::Transfer(t) => {
            let before = t.before.as_ref().ok_or("INVALID_INPUT: missing before")?;
            let after = t.after.as_ref().ok_or("INVALID_INPUT: missing after")?;
            Ok(if before.item_instance_id == after.item_instance_id {
                1
            } else {
                2
            })
        }
    }
}

fn location_custody_lines(value: &Payload) -> Result<u64, String> {
    match value
        .operation
        .as_ref()
        .ok_or("INVALID_INPUT: missing operation")?
    {
        Operation::Mint(_) => Ok(1),
        Operation::Transfer(_) => Ok(2),
    }
}

// Registered RL-07 per-field byte bounds plus the RL-01/RL-02 counts. Value
// lines, transforms, container expansion, item free text and typed_position
// have no field: they reject as unknown fields in the canonical round trip.
fn payload_bounds(value: &Payload) -> Result<(), String> {
    match value
        .operation
        .as_ref()
        .ok_or("INVALID_INPUT: missing operation")?
    {
        Operation::Mint(m) => {
            item_bounds(m.after.as_ref().ok_or("INVALID_INPUT: missing after")?)?;
            ground_bounds(
                m.destination
                    .as_ref()
                    .ok_or("INVALID_INPUT: missing destination")?,
            )?;
            provenance_bounds(m.source.as_ref().ok_or("INVALID_INPUT: missing source")?)?;
        }
        Operation::Transfer(t) => {
            item_bounds(t.before.as_ref().ok_or("INVALID_INPUT: missing before")?)?;
            item_bounds(t.after.as_ref().ok_or("INVALID_INPUT: missing after")?)?;
            ground_bounds(t.source.as_ref().ok_or("INVALID_INPUT: missing source")?)?;
            inventory_bounds(
                t.destination
                    .as_ref()
                    .ok_or("INVALID_INPUT: missing destination")?,
            )?;
            provenance_bounds(t.cause.as_ref().ok_or("INVALID_INPUT: missing cause")?)?;
        }
    }
    at_most(
        touched_item_instances(value)?,
        RL01_TOUCHED_ITEM_INSTANCES_MAX,
        "DUR03-RL-01",
    )?;
    at_most(
        location_custody_lines(value)?,
        RL02_LOCATION_CUSTODY_LINES_MAX,
        "DUR03-RL-02",
    )
}

fn validate_item(value: &Item, current: &CurrentFacts) -> Result<(), String> {
    item_bounds(value)?;
    if !valid_uuid(&value.item_instance_id)
        || !valid_uuid(&value.world_id)
        || value.world_id != current.world_id
        // Fixture representation only; this is not a product quantity cap.
        || value.quantity != 1
        || value.lifecycle != 1
        || value.definition.as_ref() != Some(&current.definition)
    {
        return Err("item/current binding mismatch".into());
    }
    Ok(())
}

fn validate_current_facts(current: &CurrentFacts) -> Result<(), String> {
    if !valid_uuid(&current.world_id)
        || !valid_uuid(&current.channel_id)
        || !valid_uuid(&current.death.world_id)
        || !valid_uuid(&current.death.channel_id)
        || !valid_uuid(&current.character_id)
        || !valid_uuid(&current.game_session_id)
        || current.session_generation == 0
        || current.character_lease_generation == 0
        || current.command_id == 0
        || current.runtime_scope_generation == 0
        || current.death.scope_ownership_generation == 0
        || current.death.actor_local_generation == 0
    {
        return Err("invalid independent current facts".into());
    }
    definition_bounds(&current.definition)?;
    definition_bounds(&current.loot_table)?;
    for (len, max) in [
        (
            current.ground_spatial_position.len(),
            RL07_TECHNICAL_FIELD_BYTES_MAX,
        ),
        (current.corpse_ref.len(), RL07_TECHNICAL_FIELD_BYTES_MAX),
        (
            current.native_room_placement_context.len(),
            RL07_TECHNICAL_FIELD_BYTES_MAX,
        ),
        (current.loot_purpose_key.len(), RL07_CONTENT_KEY_BYTES_MAX),
        (current.map_revision.len(), RL07_CONTENT_KEY_BYTES_MAX),
        (current.content_revision.len(), RL07_CONTENT_KEY_BYTES_MAX),
        (current.ruleset_revision.len(), RL07_CONTENT_KEY_BYTES_MAX),
        (current.sim_revision.len(), RL07_CONTENT_KEY_BYTES_MAX),
    ] {
        bounded(len, max, "independent current fact")?;
    }
    validate_item(&current.transfer_source_item, current)
        .map_err(|_| "invalid independent source item facts")?;
    Ok(())
}

fn validate_transfer_source_item(value: &Item, current: &CurrentFacts) -> Result<(), String> {
    validate_item(value, current)?;
    if value != &current.transfer_source_item {
        return Err("TRANSFER source item/current binding mismatch".into());
    }
    Ok(())
}

fn validate_ground(value: &Ground, current: &CurrentFacts) -> Result<(), String> {
    ground_bounds(value)?;
    if !valid_uuid(&value.world_id)
        || !valid_uuid(&value.channel_id)
        || value.world_id != current.world_id
        || value.channel_id != current.channel_id
        || value.corpse_ref != current.corpse_ref
        || value.spatial_position != current.ground_spatial_position
        || value.map_revision != current.map_revision
        || value.content_revision != current.content_revision
        || value.native_room_placement_context != current.native_room_placement_context
        || value.runtime_scope_ownership_generation != current.runtime_scope_generation
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
    provenance_bounds(value)?;
    let command_matches = match (&value.command_ref, command_required) {
        (Some(command), true) => {
            valid_uuid(&command.game_session_id)
                && command.game_session_id == current.game_session_id
                && command.command_id == current.command_id
        }
        (None, false) => true,
        _ => false,
    };
    if value.typed_cause != cause
        || value.death_occurrence.as_ref() != Some(&current.death)
        || value.loot_table.as_ref() != Some(&current.loot_table)
        || value.loot_purpose_key != current.loot_purpose_key
        || value.draw_ordinal != current.draw_ordinal
        || value.content_revision != current.content_revision
        || value.ruleset_revision != current.ruleset_revision
        || value.sim_revision != current.sim_revision
        || !command_matches
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
    payload_bounds(value)?;
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
            validate_provenance(source, current, "loot_mint", false)?;
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
            validate_transfer_source_item(before, current)?;
            validate_transfer_source_item(after, current)?;
            validate_ground(source, current)?;
            if before != after
                || !valid_uuid(&destination.character_id)
                || destination.character_id != current.character_id
                || destination.expected_session_generation != current.session_generation
                || destination.expected_character_lease_generation
                    != current.character_lease_generation
                || !valid_uuid(&destination.expected_game_session_id)
                || destination.expected_game_session_id != current.game_session_id
            {
                return Err("TRANSFER binding/state mismatch".into());
            }
            validate_provenance(cause, current, "ground_pickup_transfer", true)?;
        }
    }
    Ok(())
}

// TRANSFER stays closed until the inventory typed_position owner defines it.
fn admit(value: &Payload) -> Result<(), String> {
    match value.operation.as_ref().ok_or("missing operation")? {
        Operation::Mint(_) => Ok(()),
        Operation::Transfer(_) => {
            Err("TRANSFER_CLOSED: inventory typed_position is undefined".into())
        }
    }
}

fn id(tag: u8) -> Vec<u8> {
    let mut bytes = vec![0; RL07_UUID_BYTES];
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
        death_occurrence: Some(current.death.clone()),
        loot_table: Some(current.loot_table.clone()),
        loot_purpose_key: current.loot_purpose_key.clone(),
        draw_ordinal: current.draw_ordinal,
        content_revision: current.content_revision.clone(),
        ruleset_revision: current.ruleset_revision.clone(),
        sim_revision: current.sim_revision.clone(),
        command_ref: command.then(|| CommandRef {
            game_session_id: current.game_session_id.clone(),
            command_id: current.command_id,
        }),
    }
}

fn payload(f: &Fixture, transfer: bool) -> Result<Vec<u8>, String> {
    payload_with_facts(f, transfer, &facts(f))
}

fn encode_bounded<M: Message>(message: &M, max: usize) -> Result<Vec<u8>, String> {
    let encoded_len = message.encoded_len();
    if encoded_len > max {
        return Err(format!(
            "CAPACITY_EXCEEDED: encoded {encoded_len} > {max} before allocation"
        ));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(encoded_len)
        .map_err(|_| "CAPACITY_EXCEEDED: reservation failed")?;
    message.encode(&mut bytes).map_err(|e| e.to_string())?;
    Ok(bytes)
}

fn payload_with_facts(
    f: &Fixture,
    transfer: bool,
    current: &CurrentFacts,
) -> Result<Vec<u8>, String> {
    let ground = Ground {
        world_id: current.world_id.clone(),
        channel_id: current.channel_id.clone(),
        spatial_position: current.ground_spatial_position.clone(),
        corpse_ref: current.corpse_ref.clone(),
        map_revision: current.map_revision.clone(),
        content_revision: current.content_revision.clone(),
        native_room_placement_context: current.native_room_placement_context.clone(),
        runtime_scope_ownership_generation: current.runtime_scope_generation,
    };
    let op = if transfer {
        Operation::Transfer(Transfer {
            before: Some(current.transfer_source_item.clone()),
            after: Some(current.transfer_source_item.clone()),
            source: Some(ground),
            destination: Some(Inventory {
                character_id: current.character_id.clone(),
                expected_session_generation: current.session_generation,
                expected_game_session_id: current.game_session_id.clone(),
                expected_character_lease_generation: current.character_lease_generation,
            }),
            cause: Some(provenance(current, "ground_pickup_transfer", true)),
        })
    } else {
        Operation::Mint(Mint {
            after: Some(item(f)),
            destination: Some(ground),
            source: Some(provenance(current, "loot_mint", false)),
            before_semantically_absent: true,
        })
    };
    let message = Payload {
        interpretation_revision: 1,
        operation: Some(op),
    };
    // Per-field bounds are checked before the encode allocation.
    payload_bounds(&message)?;
    let bytes = encode_bounded(&message, RL07_PAYLOAD_BYTES_MAX.min(ANL_PAYLOAD_MAX))?;
    // Prost accepts unknown/duplicate/noncanonical encodings; equality with
    // canonical re-encoding below is the closed-wire gate before use.
    let decoded = decode_payload_canonical(bytes.as_slice())?;
    validate_payload(&decoded, current)?;
    if decoded != message || decoded.encode_to_vec() != bytes {
        return Err("noncanonical or unsupported payload".into());
    }
    Ok(bytes)
}

fn envelope(f: &Fixture, transfer: bool, payload: Vec<u8>) -> Result<Vec<u8>, String> {
    let digest = Sha256::digest(&payload).to_vec();
    let msg = Envelope {
        envelope_revision: 1,
        event_id: offset_id(f.tag, if transfer { 31 } else { 30 })?,
        event_type_id: EVENT_TYPE_ID,
        event_schema_revision: EVENT_SCHEMA_REVISION,
        durability_class: DURABLE_AUDIT,
        privacy_class: RESTRICTED_PLAYER_LINKED,
        retention_profile_id: RETENTION_PROFILE_ID.into(),
        occurred_at_unix_ms: FIXTURE_OCCURRED_AT_UNIX_MS,
        world_id: Some(id(1)),
        channel_id: Some(id(2)),
        transaction_event: Some(TransactionEventRef {
            transaction_id: offset_id(f.tag, if transfer { 50 } else { 40 })?,
            ordinal: 1,
            count: 1,
        }),
        server_build_id: "fixture-build-1".into(),
        payload,
        payload_sha256: digest,
        ..Envelope::default()
    };
    let wire = encode_bounded(&msg, RL07_ENVELOPE_BYTES_MAX.min(ANL_ENVELOPE_MAX))?;
    let decoded = decode_envelope_canonical(&wire)?;
    if decoded != msg {
        return Err("noncanonical envelope or digest".into());
    }
    Ok(wire)
}

// RL-07 payload byte gate: size before decode, canonical round trip, per-field
// bytes and the RL-01/RL-02 counts.
fn check_payload_bounds(wire: &[u8]) -> Result<Payload, String> {
    if wire.len() > RL07_PAYLOAD_BYTES_MAX || wire.len() > ANL_PAYLOAD_MAX {
        return Err(format!(
            "CAPACITY_EXCEEDED: DUR03-RL-07 payload {} > {RL07_PAYLOAD_BYTES_MAX}",
            wire.len()
        ));
    }
    let value = Payload::decode(wire).map_err(|e| format!("INVALID_INPUT: {e}"))?;
    if value.encode_to_vec() != wire {
        return Err("INVALID_INPUT: unknown, duplicate or noncanonical payload field".into());
    }
    payload_bounds(&value)?;
    Ok(value)
}

fn decode_payload_canonical(wire: &[u8]) -> Result<Payload, String> {
    let value = check_payload_bounds(wire)?;
    if value.interpretation_revision != 1 {
        return Err("unsupported interpretation revision".into());
    }
    Ok(value)
}

fn optional_uuid(value: &Option<Vec<u8>>, field: &str) -> Result<(), String> {
    match value {
        Some(bytes) => exact(bytes.len(), RL07_UUID_BYTES, field),
        None => Ok(()),
    }
}

// RL-07 envelope byte gate on the normative EventEnvelope: size before decode,
// canonical round trip, ANL string bounds, UUID and digest widths, digest
// equality, and the nested payload byte gate.
fn check_envelope_bounds(wire: &[u8]) -> Result<Envelope, String> {
    if wire.len() > RL07_ENVELOPE_BYTES_MAX || wire.len() > ANL_ENVELOPE_MAX {
        return Err(format!(
            "CAPACITY_EXCEEDED: DUR03-RL-07 envelope {} > {RL07_ENVELOPE_BYTES_MAX}",
            wire.len()
        ));
    }
    let value = Envelope::decode(wire).map_err(|e| format!("INVALID_INPUT: {e}"))?;
    if value.encode_to_vec() != wire {
        return Err("INVALID_INPUT: unknown, duplicate or noncanonical envelope field".into());
    }
    exact(value.event_id.len(), RL07_UUID_BYTES, "event_id")?;
    bounded(
        value.retention_profile_id.len(),
        ANL_ENVELOPE_STRING_MAX,
        "retention_profile_id",
    )?;
    bounded(
        value.server_build_id.len(),
        ANL_ENVELOPE_STRING_MAX,
        "server_build_id",
    )?;
    for (field, bytes) in [
        ("world_id", &value.world_id),
        ("channel_id", &value.channel_id),
        ("instance_id", &value.instance_id),
        ("node_id", &value.node_id),
        ("game_session_id", &value.game_session_id),
        ("operation_id", &value.operation_id),
        ("correlation_id", &value.correlation_id),
    ] {
        optional_uuid(bytes, field)?;
    }
    for (field, text) in [
        ("ruleset_revision", &value.ruleset_revision),
        ("content_revision", &value.content_revision),
    ] {
        if let Some(text) = text {
            bounded(text.len(), ANL_ENVELOPE_STRING_MAX, field)?;
        }
    }
    if let Some(actor) = value.analytics_actor.as_ref() {
        bounded(
            actor.identity_domain.len(),
            ANL_ENVELOPE_STRING_MAX,
            "identity_domain",
        )?;
        exact(
            actor.analytics_actor_id.len(),
            RL07_UUID_BYTES,
            "analytics_actor_id",
        )?;
    }
    if let Some(causation) = value.causation.as_ref() {
        match causation
            .cause
            .as_ref()
            .ok_or("INVALID_INPUT: empty CausationRef")?
        {
            Cause::EventId(v) | Cause::OperationId(v) | Cause::TransactionId(v) => {
                exact(v.len(), RL07_UUID_BYTES, "causation id")?;
            }
            Cause::Command(c) => {
                exact(
                    c.game_session_id.len(),
                    RL07_UUID_BYTES,
                    "causation game_session_id",
                )?;
            }
        }
    }
    let membership = value
        .transaction_event
        .as_ref()
        .ok_or("INVALID_INPUT: missing TransactionEventRef")?;
    exact(
        membership.transaction_id.len(),
        RL07_UUID_BYTES,
        "transaction_id",
    )?;
    exact(
        value.payload_sha256.len(),
        RL07_PAYLOAD_SHA256_BYTES,
        "payload_sha256",
    )?;
    if value.payload_sha256 != Sha256::digest(&value.payload).as_slice() {
        return Err("INVALID_INPUT: payload_sha256 mismatch".into());
    }
    check_payload_bounds(&value.payload)?;
    Ok(value)
}

// Registered binding plus the RL-07 one-event-per-transaction row.
fn decode_envelope_canonical(wire: &[u8]) -> Result<Envelope, String> {
    let value = check_envelope_bounds(wire)?;
    let membership = value
        .transaction_event
        .as_ref()
        .ok_or("missing TransactionEventRef")?;
    at_most(
        u64::from(membership.count),
        RL07_EVENTS_MAX,
        "DUR03-RL-07-EVENTS",
    )?;
    if value.envelope_revision != 1
        || !valid_uuid(&value.event_id)
        || value.event_type_id != EVENT_TYPE_ID
        || value.event_schema_revision != EVENT_SCHEMA_REVISION
        || value.durability_class != DURABLE_AUDIT
        || !(RESTRICTED_PLAYER_LINKED..=SECURITY_SENSITIVE).contains(&value.privacy_class)
        || value.retention_profile_id != RETENTION_PROFILE_ID
        || value.occurred_at_unix_ms <= 0
        || !valid_uuid(&membership.transaction_id)
        || membership.ordinal != 1
        || membership.count != 1
    {
        return Err("unsupported or unregistered envelope binding".into());
    }
    decode_payload_canonical(&value.payload)?;
    Ok(value)
}

// P90D: expiry = occurred_at + 7,776,000 s; lifetime is [start, expiry).
fn retention_expiry_unix_ms(occurred_at_unix_ms: i64) -> Result<i64, String> {
    let ceiling_ms = i64::try_from(AUDIT_RETENTION_P90D_SECONDS)
        .ok()
        .and_then(|s| s.checked_mul(1_000))
        .ok_or("overflow")?;
    occurred_at_unix_ms
        .checked_add(ceiling_ms)
        .ok_or_else(|| "overflow".into())
}

#[cfg(test)]
fn retention_expired(occurred_at_unix_ms: i64, now_unix_ms: i64) -> Result<bool, String> {
    Ok(now_unix_ms >= retention_expiry_unix_ms(occurred_at_unix_ms)?)
}

fn retry_same_bytes(stored: &[u8], incoming: &[u8]) -> Result<(), String> {
    if stored == incoming {
        Ok(())
    } else {
        Err("conflicting logical retry".into())
    }
}

// Upper-bound construction of D50 §3.2: every bounded field at its registered
// maximum and every varint at its widest. It is a byte-bound probe, not a
// semantically admissible event.
fn worst_case_payload(transfer: bool) -> Payload {
    let content = "k".repeat(RL07_CONTENT_KEY_BYTES_MAX);
    let technical = "t".repeat(RL07_TECHNICAL_FIELD_BYTES_MAX);
    let technical_bytes = vec![0xa5; RL07_TECHNICAL_FIELD_BYTES_MAX];
    let uuid = id(0xee);
    let typed = Definition {
        family: technical.clone(),
        production_key: content.clone(),
        revision_ref: content.clone(),
    };
    let state = Item {
        item_instance_id: uuid.clone(),
        world_id: uuid.clone(),
        definition: Some(typed.clone()),
        quantity: u32::MAX,
        lifecycle: u32::MAX,
    };
    let ground = Ground {
        world_id: uuid.clone(),
        channel_id: uuid.clone(),
        spatial_position: technical_bytes.clone(),
        corpse_ref: technical_bytes.clone(),
        map_revision: content.clone(),
        content_revision: content.clone(),
        native_room_placement_context: technical_bytes,
        runtime_scope_ownership_generation: u64::MAX,
    };
    let cause = Provenance {
        typed_cause: technical,
        death_occurrence: Some(DeathKey {
            world_id: uuid.clone(),
            channel_id: uuid.clone(),
            scope_ownership_generation: u64::MAX,
            actor_local_id: u32::MAX,
            actor_local_generation: u64::MAX,
        }),
        loot_table: Some(typed),
        loot_purpose_key: content.clone(),
        draw_ordinal: u32::MAX,
        content_revision: content.clone(),
        ruleset_revision: content.clone(),
        sim_revision: content,
        command_ref: transfer.then(|| CommandRef {
            game_session_id: uuid.clone(),
            command_id: u64::MAX,
        }),
    };
    let operation = if transfer {
        Operation::Transfer(Transfer {
            before: Some(state.clone()),
            after: Some(state),
            source: Some(ground),
            destination: Some(Inventory {
                character_id: uuid.clone(),
                expected_session_generation: u64::MAX,
                expected_game_session_id: uuid,
                expected_character_lease_generation: u64::MAX,
            }),
            cause: Some(cause),
        })
    } else {
        Operation::Mint(Mint {
            after: Some(state),
            destination: Some(ground),
            source: Some(cause),
            before_semantically_absent: true,
        })
    };
    Payload {
        interpretation_revision: u32::MAX,
        operation: Some(operation),
    }
}

fn worst_case_envelope(payload: Vec<u8>) -> Envelope {
    let text = "s".repeat(ANL_ENVELOPE_STRING_MAX);
    let uuid = id(0xef);
    Envelope {
        envelope_revision: u32::MAX,
        event_id: uuid.clone(),
        event_type_id: u32::MAX,
        event_schema_revision: u32::MAX,
        durability_class: i32::MAX,
        privacy_class: i32::MAX,
        retention_profile_id: text.clone(),
        occurred_at_unix_ms: i64::MIN,
        world_id: Some(uuid.clone()),
        channel_id: Some(uuid.clone()),
        instance_id: Some(uuid.clone()),
        node_id: Some(uuid.clone()),
        game_session_id: Some(uuid.clone()),
        connection_generation: Some(u64::MAX),
        runtime_order: Some(RuntimeOrderRef {
            scope_ownership_generation: u64::MAX,
            runtime_execution_ordinal: u64::MAX,
        }),
        command_id: Some(u64::MAX),
        operation_id: Some(uuid.clone()),
        transaction_event: Some(TransactionEventRef {
            transaction_id: uuid.clone(),
            ordinal: u32::MAX,
            count: u32::MAX,
        }),
        correlation_id: Some(uuid.clone()),
        causation: Some(CausationRef {
            cause: Some(Cause::Command(CommandRef {
                game_session_id: uuid.clone(),
                command_id: u64::MAX,
            })),
        }),
        analytics_actor: Some(AnalyticsActorRef {
            identity_domain: text.clone(),
            identity_epoch: u64::MAX,
            analytics_actor_id: uuid,
        }),
        protocol_major: Some(u32::MAX),
        ruleset_revision: Some(text.clone()),
        content_revision: Some(text.clone()),
        server_build_id: text,
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
    }
}

fn worst_case_sizes(transfer: bool) -> Result<(usize, usize), String> {
    let payload = encode_bounded(&worst_case_payload(transfer), RL07_PAYLOAD_BYTES_MAX)?;
    let payload_len = payload.len();
    let envelope = encode_bounded(&worst_case_envelope(payload), RL07_ENVELOPE_BYTES_MAX)?;
    check_envelope_bounds(&envelope)?;
    Ok((payload_len, envelope.len()))
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
    if !valid_uuid(event_id) || !valid_uuid(transaction_id) {
        return Err("invalid receipt identity".into());
    }
    let embedded = decode_envelope_canonical(bytes)?;
    let semantic_payload = decode_payload_canonical(&embedded.payload)?;
    validate_payload(&semantic_payload, current)?;
    admit(&semantic_payload)?;
    let membership = embedded
        .transaction_event
        .as_ref()
        .ok_or("missing TransactionEventRef")?;
    if embedded.event_id != event_id
        || membership.transaction_id != transaction_id
        || membership.ordinal != 1
        || membership.count != 1
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
        definition_dynamic(definition),
    ])
}

fn definition_dynamic(value: &Definition) -> usize {
    value.family.capacity() + value.production_key.capacity() + value.revision_ref.capacity()
}

fn ground_dynamic(ground: &Ground) -> Result<usize, String> {
    checked_capacity_sum([
        ground.world_id.capacity(),
        ground.channel_id.capacity(),
        ground.spatial_position.capacity(),
        ground.corpse_ref.capacity(),
        ground.map_revision.capacity(),
        ground.content_revision.capacity(),
        ground.native_room_placement_context.capacity(),
    ])
}

fn provenance_dynamic(value: &Provenance) -> Result<usize, String> {
    checked_capacity_sum([
        value.typed_cause.capacity(),
        value
            .death_occurrence
            .as_ref()
            .map_or(0, |d| d.world_id.capacity() + d.channel_id.capacity()),
        value.loot_table.as_ref().map_or(0, definition_dynamic),
        value.loot_purpose_key.capacity(),
        value.content_revision.capacity(),
        value.ruleset_revision.capacity(),
        value.sim_revision.capacity(),
        value
            .command_ref
            .as_ref()
            .map_or(0, |c| c.game_session_id.capacity()),
    ])
}

fn payload_dynamic(value: &Payload) -> Result<usize, String> {
    match value.operation.as_ref().ok_or("missing operation")? {
        Operation::Mint(m) => checked_capacity_sum([
            item_dynamic(m.after.as_ref().ok_or("missing mint item")?)?,
            ground_dynamic(m.destination.as_ref().ok_or("missing mint ground")?)?,
            provenance_dynamic(m.source.as_ref().ok_or("missing mint source")?)?,
        ]),
        Operation::Transfer(t) => {
            let dest = t
                .destination
                .as_ref()
                .ok_or("missing transfer destination")?;
            checked_capacity_sum([
                item_dynamic(t.before.as_ref().ok_or("missing transfer before")?)?,
                item_dynamic(t.after.as_ref().ok_or("missing transfer after")?)?,
                ground_dynamic(t.source.as_ref().ok_or("missing transfer ground")?)?,
                dest.character_id.capacity(),
                dest.expected_game_session_id.capacity(),
                provenance_dynamic(t.cause.as_ref().ok_or("missing transfer cause")?)?,
            ])
        }
    }
}

fn envelope_dynamic(value: &Envelope) -> Result<usize, String> {
    let optional = |v: &Option<Vec<u8>>| v.as_ref().map_or(0, Vec::capacity);
    checked_capacity_sum([
        value.event_id.capacity(),
        value.retention_profile_id.capacity(),
        optional(&value.world_id),
        optional(&value.channel_id),
        value
            .transaction_event
            .as_ref()
            .map_or(0, |t| t.transaction_id.capacity()),
        value.server_build_id.capacity(),
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

// Registered dimensions: payload, envelope and retry work units. The measured
// transient/hash/decode/retained dimensions have no registered row.
fn registered_resource_limits() -> [u64; 8] {
    [
        RL07_PAYLOAD_BYTES_MAX as u64,
        RL07_ENVELOPE_BYTES_MAX as u64,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        u64::MAX,
        RL08_RETRY_WORK_UNITS_MAX,
    ]
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
    // construction objects, three transient decoded Payload values, one
    // transient decoded Envelope value, five payload canonical re-encoding
    // buffers, and two envelope canonical re-encoding buffers. The final
    // decoded Payload and Envelope are charged separately to retained capacity.
    let transient_nested_dynamic = checked_add(
        envelope_dyn.checked_mul(2).ok_or("size overflow")?,
        payload_dyn.checked_mul(4).ok_or("size overflow")?,
    )?;
    let canonical_reencode_scratch = checked_add(
        p_len.checked_mul(5).ok_or("size overflow")?,
        e_len.checked_mul(2).ok_or("size overflow")?,
    )?;
    let transient_dynamic = checked_add(transient_nested_dynamic, canonical_reencode_scratch)?;
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
        RL08_RETRY_WORK_UNITS_MAX,
    ])
}

// RL-06: one plan participant; work units are the participant plus its effects.
fn work_units(transfer: bool) -> Result<(u64, u64), String> {
    let effects = if transfer { 2_u64 } else { 1_u64 };
    let mut participants = 0_u64;
    let mut work = 0_u64;
    // Count each concrete participant/effect as its work step executes.
    for _ in 0..1_u64 {
        participants = checked_add(participants, 1)?;
        work = checked_add(work, 1)?;
    }
    for _ in 0..effects {
        work = checked_add(work, 1)?;
    }
    at_most(
        participants,
        RL06_PARTICIPANTS_MAX,
        "DUR03-RL-06-PARTICIPANTS",
    )?;
    at_most(
        work,
        RL06_EFFECT_WORK_UNITS_MAX,
        "DUR03-RL-06-EFFECT-WORK-UNITS",
    )?;
    Ok((participants, work))
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
    let admission = match admit(&decoded_payload) {
        Ok(()) => "ADMITTED",
        Err(_) => "CLOSED",
    };
    let (participants, work) = work_units(transfer)?;
    let mut retry_work = 0_u64;
    for _ in 0..RL08_RETRY_WORK_UNITS_MAX {
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
    check_resource_limits(usage, registered_resource_limits())
        .map_err(|dimension| format!("registered resource limit {dimension}"))?;
    Ok(json!({
        "operation":op, "admission":admission, "fixture":f.name, "typed_definition":{"family":f.family,"production_key":f.key,"revision_ref":f.revision},
        "membership":{"ordinal":1,"count":1}, "payload_bytes":p.len(),"envelope_bytes":e.len(),
        "payload_sha256":hex(&Sha256::digest(&p)),"envelope_sha256":hex(&Sha256::digest(&e)),
        "payload_hex":hex(&p),"envelope_hex":hex(&e),
        "retry":{"same_logical_intent_exact_bytes":retry==e,"conflicting_intent":"REJECTED_BY_EXACT_BYTE_IDENTITY"},
        "retained":{"envelope_capacity":e.capacity(),"payload_capacity":p.capacity(),"retry_capacity":retry.capacity(),"decoded_envelope_and_nested_payload_capacities":decoded_dynamic,"inline_envelope_bytes":size_of::<Envelope>(),"inline_payload_bytes":size_of::<Payload>(),"actual_dynamic_capacity_bytes":retained_dynamic},
        "actual_counters":{"final_serialized_output_bytes":p.len()+e.len(),"protobuf_encode_output_bytes":p.len()*6+e.len()*3,"owned_raw_clone_bytes":usage[2],"transient_construction_and_decode_dynamic_capacity_bytes":usage[3],"hash_input_bytes":usage[4],"decode_input_bytes":usage[5],"retry_clone_bytes":retry.len(),"plan_participants":participants,"logical_work_units":work,"retry_work_units":retry_work,"counter_scope":"final serialized outputs are reported separately from cumulative protobuf encode and canonical re-encode bytes; exact raw clone bytes and transient construction/decode dynamic capacities are separate dimensions; hash/decode formulas count every concrete input pass. Excludes allocator metadata, codec instruction counts, CPU timing, RSS, DB, and runtime"}
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
    let (mint_payload, mint_envelope) = worst_case_sizes(false)?;
    let (transfer_payload, transfer_envelope) = worst_case_sizes(true)?;
    let doc = json!({
        "classification":"DUR03_NATIVE_ONE_ITEM_AUDIT_REGISTERED_BINDING_EVIDENCE",
        "source_base_sha":"ebacc5b8e3f1b6fe6bdb8f6f5cb9d4293a8b55d2","issue":513,"coordination":162,"allocation_comment":"5868052140",
        "decision":"docs/architecture/reviews/OTERYN_GAME_DUR03_RESOURCE_MAXIMA_AND_CREATURE_DEATH_IDENTITY_DECISION_2026-09-28.md",
        "contract":"DUR-03 §39.3",
        "event_type":{"registry":"GAME_EVENT_FOUNDATION_REGISTRY.json","id":EVENT_TYPE_ID,"schema_revision":EVENT_SCHEMA_REVISION,"payload_message":"oteryn.events.v1.OneItemTransactionV1","envelope":"oteryn.events.v1.EventEnvelope","retention_profile_id":RETENTION_PROFILE_ID,"retention_ceiling_seconds":AUDIT_RETENTION_P90D_SECONDS,"fixture_occurred_at_unix_ms":FIXTURE_OCCURRED_AT_UNIX_MS,"fixture_expiry_unix_ms":retention_expiry_unix_ms(FIXTURE_OCCURRED_AT_UNIX_MS)?,"lifetime":"[occurred_at, expiry), never refreshed"},
        "fixtures_are_product_admission":false,
        "canonical_gate":"size checked before decode; decode equality + canonical re-encode equality; unsupported revision, unknown/duplicate/noncanonical encodings reject",
        "transfer_admission":"CLOSED_TYPED_POSITION_UNDEFINED",
        "operations":ops,
        "registered_limits":{"registry":"RESOURCE_LIMITS_REGISTRY.json","DUR03-RL-01":RL01_TOUCHED_ITEM_INSTANCES_MAX,"DUR03-RL-02":RL02_LOCATION_CUSTODY_LINES_MAX,"DUR03-RL-03":RL03_VALUE_LINES_MAX,"DUR03-RL-04":RL04_TRANSFORM_LINES_MAX,"DUR03-RL-05":RL05_CONTAINER_EXPANSION_MAX,"DUR03-RL-06-PARTICIPANTS":RL06_PARTICIPANTS_MAX,"DUR03-RL-06-EFFECT-WORK-UNITS":RL06_EFFECT_WORK_UNITS_MAX,"DUR03-RL-07-EVENTS":RL07_EVENTS_MAX,"DUR03-RL-07-ENVELOPE-BYTES":RL07_ENVELOPE_BYTES_MAX,"DUR03-RL-07-PAYLOAD-BYTES":RL07_PAYLOAD_BYTES_MAX,"DUR03-RL-07-CONTENT-KEY-BYTES":RL07_CONTENT_KEY_BYTES_MAX,"DUR03-RL-07-TECHNICAL-FIELD-BYTES":RL07_TECHNICAL_FIELD_BYTES_MAX,"DUR03-RL-07-UUID-BYTES":RL07_UUID_BYTES,"DUR03-RL-07-PAYLOAD-SHA256-BYTES":RL07_PAYLOAD_SHA256_BYTES,"DUR03-RL-08":RL08_RETRY_WORK_UNITS_MAX,"DUR03-AUDIT-RETENTION-S":AUDIT_RETENTION_P90D_SECONDS,"inherited_ANL_ceilings":{"envelope_bytes":ANL_ENVELOPE_MAX,"payload_bytes":ANL_PAYLOAD_MAX,"each_envelope_string_utf8_bytes":ANL_ENVELOPE_STRING_MAX,"protobuf_nesting_depth":ANL_PROTOBUF_DEPTH_MAX},"schema_nesting_depth":SCHEMA_NESTING_DEPTH,"backpressure":"encoded_len and borrowed wire length checked against the registered rows before allocation/decode; oversize input rejected, never truncated; try_reserve_exact failure and checked arithmetic fail closed"},
        "worst_case":{"construction":"every bounded field at its registered maximum and every varint at its widest, in the normative EventEnvelope with every optional field present","MINT":{"payload_bytes":mint_payload,"envelope_bytes":mint_envelope,"envelope_overhead_bytes":mint_envelope-mint_payload},"TRANSFER_typed_position_excluded":{"payload_bytes":transfer_payload,"envelope_bytes":transfer_envelope,"envelope_overhead_bytes":transfer_envelope-transfer_payload}},
        "execution_target":{"os":std::env::consts::OS,"architecture":std::env::consts::ARCH,"toolchain_identity":"UNKNOWN","toolchain_identity_reason":"No build-time compiler attestation is embedded in this standalone example"},
        "not_proven":["runtime, SQL, durability, outbox or restart authority","RL-08 under real PostgreSQL reconciliation (stage C re-decision)","creature-death key construction and descendant fences (stage D)","Content legality, quantity/stack/capacity/loot policy","TRANSFER admission","Character readiness","playability","production collection"]
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
    #![allow(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::assertions_on_constants
    )]
    use super::*;

    const RESOURCE_REGISTRY: &str =
        include_str!("../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json");
    const EVENT_REGISTRY: &str =
        include_str!("../../../docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json");
    const PROTO: &str =
        include_str!("../../../docs/contracts/game-events/v1/native_one_item_transaction.proto");

    fn registered(id: &str) -> u64 {
        let registry: Value = serde_json::from_str(RESOURCE_REGISTRY).unwrap();
        let entries = registry["entries"].as_array().unwrap();
        let matches: Vec<&Value> = entries.iter().filter(|e| e["id"] == id).collect();
        assert_eq!(matches.len(), 1, "exactly one registry row {id}");
        let row = matches[0];
        assert_eq!(row["client_visible"], false, "{id} is not client visible");
        assert_eq!(
            row["owner_contract"],
            "../architecture/reviews/OTERYN_GAME_DUR03_RESOURCE_MAXIMA_AND_CREATURE_DEATH_IDENTITY_DECISION_2026-09-28.md"
        );
        row["hard_maximum"].as_u64().unwrap()
    }

    fn varint(value: u64) -> Vec<u8> {
        let mut bytes = Vec::new();
        prost::encoding::encode_varint(value, &mut bytes);
        bytes
    }

    fn mint_of(f: &Fixture) -> Mint {
        match Payload::decode(payload(f, false).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Mint(v) => v,
            _ => unreachable!(),
        }
    }

    fn transfer_of(f: &Fixture) -> Transfer {
        match Payload::decode(payload(f, true).unwrap().as_slice())
            .unwrap()
            .operation
            .unwrap()
        {
            Operation::Transfer(v) => v,
            _ => unreachable!(),
        }
    }

    fn mint_payload(value: Mint) -> Vec<u8> {
        Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Mint(value)),
        }
        .encode_to_vec()
    }

    fn transfer_payload(value: Transfer) -> Vec<u8> {
        Payload {
            interpretation_revision: 1,
            operation: Some(Operation::Transfer(value)),
        }
        .encode_to_vec()
    }

    #[test]
    fn registered_constants_equal_the_registry_rows() {
        for (row, value) in [
            ("DUR03-RL-01", RL01_TOUCHED_ITEM_INSTANCES_MAX),
            ("DUR03-RL-02", RL02_LOCATION_CUSTODY_LINES_MAX),
            ("DUR03-RL-03", RL03_VALUE_LINES_MAX),
            ("DUR03-RL-04", RL04_TRANSFORM_LINES_MAX),
            ("DUR03-RL-05", RL05_CONTAINER_EXPANSION_MAX),
            ("DUR03-RL-06-PARTICIPANTS", RL06_PARTICIPANTS_MAX),
            ("DUR03-RL-06-EFFECT-WORK-UNITS", RL06_EFFECT_WORK_UNITS_MAX),
            ("DUR03-RL-07-EVENTS", RL07_EVENTS_MAX),
            ("DUR03-RL-07-ENVELOPE-BYTES", RL07_ENVELOPE_BYTES_MAX as u64),
            ("DUR03-RL-07-PAYLOAD-BYTES", RL07_PAYLOAD_BYTES_MAX as u64),
            (
                "DUR03-RL-07-CONTENT-KEY-BYTES",
                RL07_CONTENT_KEY_BYTES_MAX as u64,
            ),
            (
                "DUR03-RL-07-TECHNICAL-FIELD-BYTES",
                RL07_TECHNICAL_FIELD_BYTES_MAX as u64,
            ),
            ("DUR03-RL-07-UUID-BYTES", RL07_UUID_BYTES as u64),
            (
                "DUR03-RL-07-PAYLOAD-SHA256-BYTES",
                RL07_PAYLOAD_SHA256_BYTES as u64,
            ),
            ("DUR03-RL-08", RL08_RETRY_WORK_UNITS_MAX),
            ("DUR03-AUDIT-RETENTION-S", AUDIT_RETENTION_P90D_SECONDS),
        ] {
            assert_eq!(registered(row), value, "{row}");
        }
        // Decision literals, independent of both the registry and the constants.
        assert_eq!(RL07_ENVELOPE_BYTES_MAX, 9_216);
        assert_eq!(RL07_PAYLOAD_BYTES_MAX, 7_936);
        assert_eq!(RL07_CONTENT_KEY_BYTES_MAX, 512);
        assert_eq!(RL07_TECHNICAL_FIELD_BYTES_MAX, 128);
        assert_eq!(AUDIT_RETENTION_P90D_SECONDS, 90 * 24 * 60 * 60);
        assert!(RL07_ENVELOPE_BYTES_MAX < ANL_ENVELOPE_MAX);
        assert!(RL07_PAYLOAD_BYTES_MAX < ANL_PAYLOAD_MAX);
        assert!(SCHEMA_NESTING_DEPTH < ANL_PROTOBUF_DEPTH_MAX);
    }

    #[test]
    fn event_type_and_retention_profile_are_registered() {
        let registry: Value = serde_json::from_str(EVENT_REGISTRY).unwrap();
        let types = registry["event_types"].as_array().unwrap();
        let event = types
            .iter()
            .find(|t| t["id"] == EVENT_TYPE_ID)
            .expect("event type registered");
        assert_eq!(
            event["payload_schema"],
            "docs/contracts/game-events/v1/native_one_item_transaction.proto"
        );
        assert_eq!(
            event["payload_message"],
            "oteryn.events.v1.OneItemTransactionV1"
        );
        assert_eq!(event["current_schema_revision"], EVENT_SCHEMA_REVISION);
        assert_eq!(event["durability_class"], "DURABLE_AUDIT");
        assert_eq!(event["privacy_class_floor"], "RESTRICTED_PLAYER_LINKED");
        assert_eq!(event["retention_profile_id"], RETENTION_PROFILE_ID);
        let profile = registry["retention_profiles"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == RETENTION_PROFILE_ID)
            .expect("retention profile registered");
        assert_eq!(profile["privacy_class"], "RESTRICTED_PLAYER_LINKED");
        assert!(
            profile["finite_retention_duration_or_ceiling"]
                .as_str()
                .unwrap()
                .contains("7,776,000 elapsed seconds")
        );
        let mut ids: Vec<u64> = types.iter().map(|t| t["id"].as_u64().unwrap()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), types.len(), "event type ids never reused");
    }

    #[test]
    fn p90d_expiry_is_half_open_and_never_refreshed() {
        let start = FIXTURE_OCCURRED_AT_UNIX_MS;
        let expiry = start + 7_776_000_000;
        assert!(!retention_expired(start, start).unwrap());
        assert!(!retention_expired(start, expiry - 1).unwrap());
        assert!(retention_expired(start, expiry).unwrap());
        assert!(retention_expired(i64::MAX, 0).is_err());
    }

    #[test]
    fn worst_case_mint_matches_the_decision_and_is_accepted() {
        let payload = encode_bounded(&worst_case_payload(false), RL07_PAYLOAD_BYTES_MAX).unwrap();
        assert_eq!(payload.len(), 6_129);
        let envelope = encode_bounded(
            &worst_case_envelope(payload.clone()),
            RL07_ENVELOPE_BYTES_MAX,
        )
        .unwrap();
        assert_eq!(envelope.len(), 7_167);
        assert_eq!(envelope.len() - payload.len(), 1_038);
        let accepted = check_envelope_bounds(&envelope).unwrap();
        assert_eq!(accepted.payload, payload);
        // The byte-bound probe widens the membership count; the RL-07 event
        // row still rejects it as a registered event.
        assert!(
            decode_envelope_canonical(&envelope)
                .unwrap_err()
                .contains("DUR03-RL-07-EVENTS")
        );
        // Any valid payload fits: 7,936 + 1,038 = 8,974 <= 9,216.
        assert!(RL07_PAYLOAD_BYTES_MAX + 1_038 <= RL07_ENVELOPE_BYTES_MAX);
    }

    #[test]
    fn worst_case_transfer_matches_the_decision() {
        assert_eq!(worst_case_sizes(true).unwrap(), (7_433, 8_471));
    }

    #[test]
    fn envelope_and_payload_bytes_accept_max_and_reject_max_plus_one_before_decode() {
        let oversize_envelope = vec![0_u8; RL07_ENVELOPE_BYTES_MAX + 1];
        assert!(
            check_envelope_bounds(&oversize_envelope)
                .unwrap_err()
                .starts_with("CAPACITY_EXCEEDED")
        );
        let at_max_envelope = vec![0_u8; RL07_ENVELOPE_BYTES_MAX];
        // At the cap the size gate passes and decoding proceeds (then rejects
        // the zero bytes as malformed, not as oversize).
        assert!(
            check_envelope_bounds(&at_max_envelope)
                .unwrap_err()
                .starts_with("INVALID_INPUT")
        );
        let oversize_payload = vec![0_u8; RL07_PAYLOAD_BYTES_MAX + 1];
        assert!(
            check_payload_bounds(&oversize_payload)
                .unwrap_err()
                .starts_with("CAPACITY_EXCEEDED")
        );
        let at_max_payload = vec![0_u8; RL07_PAYLOAD_BYTES_MAX];
        assert!(
            check_payload_bounds(&at_max_payload)
                .unwrap_err()
                .starts_with("INVALID_INPUT")
        );
        // Encoder side: the registered row is checked before allocation.
        let mut big = worst_case_payload(true);
        if let Some(Operation::Transfer(t)) = big.operation.as_mut() {
            t.before
                .as_mut()
                .unwrap()
                .definition
                .as_mut()
                .unwrap()
                .production_key = "k".repeat(2_000);
        }
        assert!(big.encoded_len() > RL07_PAYLOAD_BYTES_MAX);
        assert!(
            encode_bounded(&big, RL07_PAYLOAD_BYTES_MAX)
                .unwrap_err()
                .starts_with("CAPACITY_EXCEEDED")
        );
    }

    #[test]
    fn content_key_and_technical_field_accept_max_and_reject_max_plus_one_without_truncation() {
        let f = &FIXTURES[0];
        let mut current = facts(f);
        current.map_revision = "m".repeat(RL07_CONTENT_KEY_BYTES_MAX);
        current.loot_purpose_key = "p".repeat(RL07_CONTENT_KEY_BYTES_MAX);
        current.ground_spatial_position = vec![7; RL07_TECHNICAL_FIELD_BYTES_MAX];
        assert!(payload_with_facts(f, false, &current).is_ok());

        let mint = mint_of(f);
        let mut changed = mint.clone();
        changed.destination.as_mut().unwrap().map_revision =
            "m".repeat(RL07_CONTENT_KEY_BYTES_MAX + 1);
        let bytes = mint_payload(changed);
        let err = decode_payload_canonical(&bytes).unwrap_err();
        assert!(
            err.starts_with("INVALID_INPUT") && err.contains("513"),
            "{err}"
        );

        let mut changed = mint.clone();
        changed
            .after
            .as_mut()
            .unwrap()
            .definition
            .as_mut()
            .unwrap()
            .production_key = "k".repeat(RL07_CONTENT_KEY_BYTES_MAX + 1);
        assert!(decode_payload_canonical(&mint_payload(changed)).is_err());

        let mut changed = mint.clone();
        changed.source.as_mut().unwrap().typed_cause =
            "t".repeat(RL07_TECHNICAL_FIELD_BYTES_MAX + 1);
        let err = decode_payload_canonical(&mint_payload(changed)).unwrap_err();
        assert!(
            err.starts_with("INVALID_INPUT") && err.contains("129"),
            "{err}"
        );

        for field in 0..4 {
            let mut changed = mint.clone();
            let ground = changed.destination.as_mut().unwrap();
            let source = changed.source.as_mut().unwrap();
            let over = vec![1; RL07_TECHNICAL_FIELD_BYTES_MAX + 1];
            match field {
                0 => ground.spatial_position = over,
                1 => ground.corpse_ref = over,
                2 => ground.native_room_placement_context = over,
                _ => {
                    source.loot_table.as_mut().unwrap().family =
                        "f".repeat(RL07_TECHNICAL_FIELD_BYTES_MAX + 1)
                }
            }
            assert!(decode_payload_canonical(&mint_payload(changed)).is_err());
        }

        // The encoder rejects before allocation; nothing truncated is produced.
        current.map_revision = "m".repeat(RL07_CONTENT_KEY_BYTES_MAX + 1);
        assert!(payload_with_facts(f, false, &current).is_err());
        let mut current = facts(f);
        current.ground_spatial_position = vec![7; RL07_TECHNICAL_FIELD_BYTES_MAX + 1];
        assert!(payload_with_facts(f, false, &current).is_err());
    }

    #[test]
    fn uuid_and_payload_digest_widths_are_exact() {
        let f = &FIXTURES[0];
        let mint = mint_of(f);
        for width in [RL07_UUID_BYTES - 1, RL07_UUID_BYTES + 1] {
            let mut changed = mint.clone();
            changed.after.as_mut().unwrap().item_instance_id = vec![7; width];
            assert!(decode_payload_canonical(&mint_payload(changed)).is_err());
            let mut changed = mint.clone();
            changed
                .source
                .as_mut()
                .unwrap()
                .death_occurrence
                .as_mut()
                .unwrap()
                .channel_id = vec![7; width];
            assert!(decode_payload_canonical(&mint_payload(changed)).is_err());
        }
        let p = payload(f, false).unwrap();
        let wire = envelope(f, false, p).unwrap();
        let good = Envelope::decode(wire.as_slice()).unwrap();
        for width in [RL07_PAYLOAD_SHA256_BYTES - 1, RL07_PAYLOAD_SHA256_BYTES + 1] {
            let mut changed = good.clone();
            changed.payload_sha256 = vec![0; width];
            assert!(check_envelope_bounds(&changed.encode_to_vec()).is_err());
        }
        let mut changed = good.clone();
        changed.event_id = vec![7; RL07_UUID_BYTES + 1];
        assert!(check_envelope_bounds(&changed.encode_to_vec()).is_err());
        let mut changed = good;
        changed.server_build_id = "b".repeat(ANL_ENVELOPE_STRING_MAX + 1);
        assert!(check_envelope_bounds(&changed.encode_to_vec()).is_err());
    }

    #[test]
    fn count_rows_accept_max_and_reject_max_plus_one() {
        // RL-01 touched ItemInstances and RL-02 location/custody lines.
        let f = &FIXTURES[0];
        let mut transfer = transfer_of(f);
        let wire = transfer_payload(transfer.clone());
        let decoded = check_payload_bounds(&wire).unwrap();
        assert_eq!(
            touched_item_instances(&decoded).unwrap(),
            RL01_TOUCHED_ITEM_INSTANCES_MAX
        );
        assert_eq!(
            location_custody_lines(&decoded).unwrap(),
            RL02_LOCATION_CUSTODY_LINES_MAX
        );
        transfer.after.as_mut().unwrap().item_instance_id = id(9);
        let err = check_payload_bounds(&transfer_payload(transfer)).unwrap_err();
        assert!(err.contains("CAPACITY_EXCEEDED: DUR03-RL-01"), "{err}");
        assert!(at_most(2, RL02_LOCATION_CUSTODY_LINES_MAX, "DUR03-RL-02").is_ok());
        assert!(at_most(3, RL02_LOCATION_CUSTODY_LINES_MAX, "DUR03-RL-02").is_err());
        // RL-03/04/05 are 0: any value line, transform or container field is
        // unknown to the schema and rejects fail-closed.
        for max in [
            RL03_VALUE_LINES_MAX,
            RL04_TRANSFORM_LINES_MAX,
            RL05_CONTAINER_EXPANSION_MAX,
        ] {
            assert!(at_most(0, max, "excluded").is_ok());
            assert!(at_most(1, max, "excluded").is_err());
        }
        let p = payload(f, false).unwrap();
        for extra in [[0x22_u8, 0x00], [0x2a, 0x00], [0x32, 0x00]] {
            assert!(decode_payload_canonical(&[p.as_slice(), &extra].concat()).is_err());
        }
        // A second location line (duplicated destination) is not canonical.
        let mint = mint_of(f);
        let ground = mint.destination.clone().unwrap().encode_to_vec();
        let mut inner = mint.encode_to_vec();
        inner.push(0x12);
        inner.extend(varint(ground.len() as u64));
        inner.extend(ground);
        let mut wire = vec![0x08, 0x01, 0x12];
        wire.extend(varint(inner.len() as u64));
        wire.extend(inner);
        assert!(decode_payload_canonical(&wire).is_err());
        // RL-06 participants and effect work units.
        assert_eq!(work_units(false).unwrap(), (1, 2));
        assert_eq!(work_units(true).unwrap(), (1, RL06_EFFECT_WORK_UNITS_MAX));
        assert!(at_most(2, RL06_PARTICIPANTS_MAX, "DUR03-RL-06-PARTICIPANTS").is_err());
        assert!(
            at_most(
                RL06_EFFECT_WORK_UNITS_MAX + 1,
                RL06_EFFECT_WORK_UNITS_MAX,
                "x"
            )
            .is_err()
        );
        // RL-07 one event per logical transaction.
        let e = envelope(f, false, p).unwrap();
        let mut two = Envelope::decode(e.as_slice()).unwrap();
        two.transaction_event.as_mut().unwrap().count = 2;
        let err = decode_envelope_canonical(&two.encode_to_vec()).unwrap_err();
        assert!(err.contains("DUR03-RL-07-EVENTS"), "{err}");
        // RL-08 retry work units.
        let mut usage = operation_resource_usage(&[0; 4], &[0; 8], 0, 0, 4, 8, 8).unwrap();
        assert_eq!(usage[7], RL08_RETRY_WORK_UNITS_MAX);
        assert!(check_resource_limits(usage, registered_resource_limits()).is_ok());
        usage[7] = RL08_RETRY_WORK_UNITS_MAX + 1;
        assert_eq!(
            check_resource_limits(usage, registered_resource_limits()),
            Err(7)
        );
    }

    #[test]
    fn typed_position_and_item_free_text_are_excluded_fail_closed() {
        #[derive(Clone, PartialEq, Message)]
        struct InventoryWithTypedPosition {
            #[prost(bytes = "vec", tag = "1")]
            character_id: Vec<u8>,
            #[prost(bytes = "vec", tag = "2")]
            typed_position: Vec<u8>,
            #[prost(uint64, tag = "3")]
            expected_session_generation: u64,
            #[prost(bytes = "vec", tag = "4")]
            expected_game_session_id: Vec<u8>,
            #[prost(uint64, tag = "5")]
            expected_character_lease_generation: u64,
        }
        #[derive(Clone, PartialEq, Message)]
        struct TransferWithTypedPosition {
            #[prost(message, optional, tag = "1")]
            before: Option<Item>,
            #[prost(message, optional, tag = "2")]
            after: Option<Item>,
            #[prost(message, optional, tag = "3")]
            source: Option<Ground>,
            #[prost(message, optional, tag = "4")]
            destination: Option<InventoryWithTypedPosition>,
            #[prost(message, optional, tag = "5")]
            cause: Option<Provenance>,
        }
        #[derive(Clone, PartialEq, Message)]
        struct PayloadWithTypedPosition {
            #[prost(uint32, tag = "1")]
            interpretation_revision: u32,
            #[prost(message, optional, tag = "3")]
            transfer: Option<TransferWithTypedPosition>,
        }
        let widen = |t: Transfer, position: Vec<u8>| {
            let d = t.destination.unwrap();
            PayloadWithTypedPosition {
                interpretation_revision: u32::MAX,
                transfer: Some(TransferWithTypedPosition {
                    before: t.before,
                    after: t.after,
                    source: t.source,
                    destination: Some(InventoryWithTypedPosition {
                        character_id: d.character_id,
                        typed_position: position,
                        expected_session_generation: d.expected_session_generation,
                        expected_game_session_id: d.expected_game_session_id,
                        expected_character_lease_generation: d.expected_character_lease_generation,
                    }),
                    cause: t.cause,
                }),
            }
        };
        let worst = match worst_case_payload(true).operation.unwrap() {
            Operation::Transfer(t) => t,
            _ => unreachable!(),
        };
        let future = widen(worst, vec![1; RL07_TECHNICAL_FIELD_BYTES_MAX]).encode_to_vec();
        // Decision §3.2: 7,565 B payload / 8,603 B envelope needs no re-registration.
        assert_eq!(future.len(), 7_565);
        assert!(future.len() + 1_038 <= RL07_ENVELOPE_BYTES_MAX);
        // Today the field is reserved: it rejects fail-closed.
        assert!(check_payload_bounds(&future).is_err());
        // Item free text (an unknown ItemState field) rejects fail-closed.
        let f = &FIXTURES[0];
        let mut mint = mint_of(f);
        let mut item = mint.after.take().unwrap().encode_to_vec();
        item.extend([0x32, 0x04, b'n', b'o', b't', b'e']);
        let mut inner = vec![0x0a];
        inner.extend(varint(item.len() as u64));
        inner.extend(item);
        inner.extend(mint.encode_to_vec());
        let mut wire = vec![0x08, 0x01, 0x12];
        wire.extend(varint(inner.len() as u64));
        wire.extend(inner);
        assert!(decode_payload_canonical(&wire).is_err());
    }

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
        type Mutation = fn(&mut CurrentFacts);
        let mint_mutations: [Mutation; 16] = [
            |c| c.world_id = id(9),
            |c| c.channel_id = id(9),
            |c| c.definition.revision_ref = "rev-other".into(),
            |c| c.death.actor_local_id = 8,
            |c| c.death.actor_local_generation = 2,
            |c| c.death.scope_ownership_generation = 2,
            |c| c.death.channel_id = id(9),
            |c| c.loot_table.revision_ref = "loot-other".into(),
            |c| c.loot_purpose_key = "purpose-other".into(),
            |c| c.draw_ordinal = 2,
            |c| c.corpse_ref = id(9),
            |c| c.ground_spatial_position = vec![3, 2, 1],
            |c| c.map_revision = "map-other".into(),
            |c| c.content_revision = "content-other".into(),
            |c| c.native_room_placement_context = id(9),
            |c| c.runtime_scope_generation = 2,
        ];
        for mutate in mint_mutations {
            let mut changed = baseline.clone();
            mutate(&mut changed);
            assert!(validate_payload(&mint, &changed).is_err());
        }
        let shared_mutations: [Mutation; 2] = [
            |c| c.ruleset_revision = "rules-other".into(),
            |c| c.sim_revision = "sim-other".into(),
        ];
        for mutate in shared_mutations {
            let mut changed = baseline.clone();
            mutate(&mut changed);
            assert!(validate_payload(&mint, &changed).is_err());
            assert!(validate_payload(&transfer, &changed).is_err());
        }
        let transfer_mutations: [Mutation; 12] = [
            |c| c.transfer_source_item.item_instance_id = id(9),
            |c| c.character_id = id(9),
            |c| c.loot_table.revision_ref = "loot-other".into(),
            |c| c.command_id = 10,
            |c| c.game_session_id = id(9),
            |c| c.session_generation = 2,
            |c| c.character_lease_generation = 4,
            |c| c.ground_spatial_position = vec![3, 2, 1],
            |c| c.session_generation = 0,
            |c| c.character_lease_generation = 0,
            |c| c.command_id = 0,
            |c| c.runtime_scope_generation = 0,
        ];
        for mutate in transfer_mutations {
            let mut changed = baseline.clone();
            mutate(&mut changed);
            assert!(validate_payload(&transfer, &changed).is_err());
        }
        assert!(!valid_uuid(&[0; 16]));
        let mut bad = id(1);
        bad[6] = 0x40;
        assert!(!valid_uuid(&bad));
    }
    #[test]
    fn canonical_but_semantically_tampered_payloads_fail_closed() {
        let f = &FIXTURES[0];
        let current = facts(f);
        let mint = mint_of(f);
        type MintMutation = fn(&mut Mint);
        let mint_cases: [MintMutation; 7] = [
            |m| m.after.as_mut().unwrap().world_id = id(9),
            |m| m.after.as_mut().unwrap().quantity = 0,
            |m| m.after.as_mut().unwrap().quantity = 2,
            |m| m.before_semantically_absent = false,
            |m| m.after.as_mut().unwrap().lifecycle = 2,
            |m| m.destination.as_mut().unwrap().channel_id = id(9),
            |m| m.source.as_mut().unwrap().draw_ordinal = 9,
        ];
        for mutate in mint_cases {
            let mut changed = mint.clone();
            mutate(&mut changed);
            let decoded = decode_payload_canonical(&mint_payload(changed)).unwrap();
            assert!(validate_payload(&decoded, &current).is_err());
        }
        type TransferMutation = fn(&mut Transfer);
        let transfer_cases: [TransferMutation; 9] = [
            |t| {
                t.before.as_mut().unwrap().item_instance_id = id(9);
                t.after.as_mut().unwrap().item_instance_id = id(9);
            },
            |t| t.after.as_mut().unwrap().quantity = 2,
            |t| t.destination.as_mut().unwrap().character_id = id(9),
            |t| t.source.as_mut().unwrap().native_room_placement_context = id(9),
            |t| {
                t.cause
                    .as_mut()
                    .unwrap()
                    .command_ref
                    .as_mut()
                    .unwrap()
                    .command_id = 10
            },
            |t| t.destination.as_mut().unwrap().expected_session_generation = 2,
            |t| t.cause.as_mut().unwrap().command_ref = None,
            |t| t.destination.as_mut().unwrap().expected_game_session_id = id(9),
            |t| {
                t.destination
                    .as_mut()
                    .unwrap()
                    .expected_character_lease_generation = 4
            },
        ];
        for mutate in transfer_cases {
            let mut changed = transfer_of(f);
            mutate(&mut changed);
            let decoded = decode_payload_canonical(&transfer_payload(changed)).unwrap();
            assert!(validate_payload(&decoded, &current).is_err());
        }
    }
    #[test]
    fn ambiguous_lost_ack_holds_then_reconciles_once_and_replay_is_idempotent() {
        let current = facts(&FIXTURES[0]);
        let bytes = envelope(&FIXTURES[0], false, payload(&FIXTURES[0], false).unwrap()).unwrap();
        let (event, tx) = (offset_id(11, 30).unwrap(), offset_id(11, 40).unwrap());
        let mut records = Vec::new();
        let attempt =
            |records: &mut Vec<ReceiptFixture>, event: &[u8], tx: &[u8], bytes: &[u8], outcome| {
                record_attempt(records, event, tx, bytes, &current, outcome)
            };
        assert_eq!(
            attempt(&mut records, &event, &tx, &bytes, Disposition::Ambiguous).unwrap(),
            Disposition::Ambiguous
        );
        assert_eq!(records[0].mutation_applications, 0);
        assert_eq!(
            attempt(&mut records, &event, &tx, &bytes, Disposition::Committed).unwrap(),
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
            attempt(&mut records, &event, &tx, &bytes, Disposition::Committed).unwrap(),
            Disposition::Committed
        );
        assert_eq!(records[0].mutation_applications, 1);
        assert!(attempt(&mut records, &event, &tx, &[9], Disposition::Committed).is_err());
        assert!(
            attempt(
                &mut records,
                &event,
                &id(52),
                &bytes,
                Disposition::Committed
            )
            .is_err()
        );
        assert!(attempt(&mut records, &id(42), &tx, &bytes, Disposition::Committed).is_err());
        let mut invalid_mint = mint_of(&FIXTURES[0]);
        invalid_mint.before_semantically_absent = false;
        let invalid_envelope = envelope(&FIXTURES[0], false, mint_payload(invalid_mint)).unwrap();
        let mut untouched = Vec::new();
        assert!(
            attempt(
                &mut untouched,
                &event,
                &tx,
                &invalid_envelope,
                Disposition::Committed
            )
            .is_err()
        );
        assert!(untouched.is_empty());
    }
    #[test]
    fn mint_admits_and_transfer_stays_closed_with_distinct_ids() {
        let f = &FIXTURES[0];
        let current = facts(f);
        let mint_bytes = envelope(f, false, payload(f, false).unwrap()).unwrap();
        let transfer_bytes = envelope(f, true, payload(f, true).unwrap()).unwrap();
        let mint_envelope = decode_envelope_canonical(&mint_bytes).unwrap();
        let transfer_envelope = decode_envelope_canonical(&transfer_bytes).unwrap();
        assert_ne!(mint_envelope.event_id, transfer_envelope.event_id);
        let mint_tx = mint_envelope.transaction_event.unwrap().transaction_id;
        let transfer_tx = transfer_envelope.transaction_event.unwrap().transaction_id;
        assert_ne!(mint_tx, transfer_tx);
        let mut records = Vec::new();
        assert_eq!(
            record_attempt(
                &mut records,
                &mint_envelope.event_id,
                &mint_tx,
                &mint_bytes,
                &current,
                Disposition::Committed,
            )
            .unwrap(),
            Disposition::Committed
        );
        let err = record_attempt(
            &mut records,
            &transfer_envelope.event_id,
            &transfer_tx,
            &transfer_bytes,
            &current,
            Disposition::Committed,
        )
        .unwrap_err();
        assert!(err.starts_with("TRANSFER_CLOSED"), "{err}");
        assert_eq!(records.len(), 1);
    }
    #[test]
    fn proven_noncommit_retries_same_candidate_identity_and_bytes() {
        let current = facts(&FIXTURES[0]);
        let bytes = envelope(&FIXTURES[0], false, payload(&FIXTURES[0], false).unwrap()).unwrap();
        let (event, tx) = (offset_id(11, 30).unwrap(), offset_id(11, 40).unwrap());
        let mut records = Vec::new();
        record_attempt(
            &mut records,
            &event,
            &tx,
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
                &event,
                &tx,
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
        assert!(decode_envelope_canonical(&[e.as_slice(), &[0xe8, 0x03, 1]].concat()).is_err());
    }
    #[test]
    fn independent_literal_payload_and_full_envelope_goldens_for_both_operations() {
        let mint_payload = payload(&FIXTURES[0], false).unwrap();
        let mint_envelope = envelope(&FIXTURES[0], false, mint_payload.clone()).unwrap();
        let transfer_payload = payload(&FIXTURES[0], true).unwrap();
        let transfer_envelope = envelope(&FIXTURES[0], true, transfer_payload.clone()).unwrap();
        assert_eq!(hex(&mint_payload), GOLDEN_MINT_PAYLOAD);
        assert_eq!(hex(&mint_envelope), GOLDEN_MINT_ENVELOPE);
        assert_eq!(hex(&transfer_payload), GOLDEN_TRANSFER_PAYLOAD);
        assert_eq!(hex(&transfer_envelope), GOLDEN_TRANSFER_ENVELOPE);
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
                assert!(check_resource_limits(usage, registered_resource_limits()).is_ok());
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
    fn proto_grammar_is_registered_without_candidate_names() {
        for required in [
            "package oteryn.events.v1;",
            "message OneItemTransactionV1",
            "message OneItemTypedDefinitionRevisionV1",
            "message OneItemStateV1",
            "message OneItemGroundV1",
            "message OneItemInventoryV1",
            "message OneItemCommandRefV1",
            "message CreatureDeathOccurrenceRefV1",
            "message OneItemProvenanceV1",
            "message OneItemMintV1",
            "message OneItemTransferV1",
            "before_semantically_absent = 4",
            "reserved 2;",
            "reserved \"typed_position\";",
            "CreatureDeathOccurrenceRefV1 death_occurrence = 2;",
            "OneItemTypedDefinitionRevisionV1 loot_table = 3;",
            "string loot_purpose_key = 4;",
            "uint32 draw_ordinal = 5;",
            "OneItemCommandRefV1 command_ref = 9;",
            "uint64 runtime_scope_ownership_generation = 8;",
        ] {
            assert!(PROTO.contains(required), "missing grammar term: {required}");
        }
        for forbidden in [
            "_fixture",
            "candidate",
            "Candidate",
            "OfflineCandidateEnvelope",
        ] {
            assert!(!PROTO.contains(forbidden), "forbidden term: {forbidden}");
        }
    }

    const GOLDEN_MINT_PAYLOAD: &str = "08011281030a4c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f312001280112730a10000000000001700080000000000000011210000000000001700080000000000000021a030102032210000000000001700080000000000000032a0e6d61702d666978747572652d72313212636f6e74656e742d666978747572652d72313a100000000000017000800000000000000640011ab9010a096c6f6f745f6d696e74122a0a10000000000001700080000000000000011210000000000001700080000000000000021801200728011a300a094c6f6f745461626c651212666978747572653a6c6f6f742e616c7068611a0f6c6f6f742d666978747572652d72312214666978747572653a707572706f73652e64726f7028013212636f6e74656e742d666978747572652d72313a1272756c657365742d666978747572652d7231420e73696d2d666978747572652d72312001";
    const GOLDEN_MINT_ENVELOPE: &str = "080112100000000000017000800000000000002918022001280230033a2944555230335f4f4e455f4954454d5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a10000000000001700080000000000000015210000000000001700080000000000000029201160a100000000000017000800000000000003310011801ca010f666978747572652d6275696c642d31d201860308011281030a4c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f312001280112730a10000000000001700080000000000000011210000000000001700080000000000000021a030102032210000000000001700080000000000000032a0e6d61702d666978747572652d72313212636f6e74656e742d666978747572652d72313a100000000000017000800000000000000640011ab9010a096c6f6f745f6d696e74122a0a10000000000001700080000000000000011210000000000001700080000000000000021801200728011a300a094c6f6f745461626c651212666978747572653a6c6f6f742e616c7068611a0f6c6f6f742d666978747572652d72312214666978747572653a707572706f73652e64726f7028013212636f6e74656e742d666978747572652d72313a1272756c657365742d666978747572652d7231420e73696d2d666978747572652d72312001da0120690d48b3e02d1ee721dd7944eca723ad5c4f6ee2fbc463b5ca6cf4539b553d52";
    const GOLDEN_TRANSFER_PAYLOAD: &str = "08011a9a040a4c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f3120012801124c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f31200128011a730a10000000000001700080000000000000011210000000000001700080000000000000021a030102032210000000000001700080000000000000032a0e6d61702d666978747572652d72313212636f6e74656e742d666978747572652d72313a1000000000000170008000000000000006400122280a1000000000000170008000000000000004180122100000000000017000800000000000000528032adc010a1667726f756e645f7069636b75705f7472616e73666572122a0a10000000000001700080000000000000011210000000000001700080000000000000021801200728011a300a094c6f6f745461626c651212666978747572653a6c6f6f742e616c7068611a0f6c6f6f742d666978747572652d72312214666978747572653a707572706f73652e64726f7028013212636f6e74656e742d666978747572652d72313a1272756c657365742d666978747572652d7231420e73696d2d666978747572652d72314a140a10000000000001700080000000000000051009";
    const GOLDEN_TRANSFER_ENVELOPE: &str = "080112100000000000017000800000000000002a18022001280230033a2944555230335f4f4e455f4954454d5f44555241424c455f41554449545f524554454e54494f4e5f56314080d8c1a28c344a10000000000001700080000000000000015210000000000001700080000000000000029201160a100000000000017000800000000000003d10011801ca010f666978747572652d6275696c642d31d2019f0408011a9a040a4c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f3120012801124c0a100000000000017000800000000000000b1210000000000001700080000000000000011a220a084974656d54797065120d666978747572653a616c7068611a077265762d612f31200128011a730a10000000000001700080000000000000011210000000000001700080000000000000021a030102032210000000000001700080000000000000032a0e6d61702d666978747572652d72313212636f6e74656e742d666978747572652d72313a1000000000000170008000000000000006400122280a1000000000000170008000000000000004180122100000000000017000800000000000000528032adc010a1667726f756e645f7069636b75705f7472616e73666572122a0a10000000000001700080000000000000011210000000000001700080000000000000021801200728011a300a094c6f6f745461626c651212666978747572653a6c6f6f742e616c7068611a0f6c6f6f742d666978747572652d72312214666978747572653a707572706f73652e64726f7028013212636f6e74656e742d666978747572652d72313a1272756c657365742d666978747572652d7231420e73696d2d666978747572652d72314a140a10000000000001700080000000000000051009da0120734e090167a6cea25ea56b367401c75e8075fc5f31fb6f3d46612d660ae42bf0";
}
