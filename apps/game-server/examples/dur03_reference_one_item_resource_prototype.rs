//! Nonproduction DUR-03 resource evidence for deterministic Gold Coin x1.
//! Unregistered closed candidate protobuf measurement; no runtime or production admission.

#![cfg_attr(
    not(test),
    allow(
        dead_code,
        reason = "negative capability variants are exercised by the focused example tests"
    )
)]

use oteryn_game_server::domain::{
    CharacterId, ItemDefinitionRef, ItemInstance, ItemInstanceId, ItemLocationRef,
    ItemPlacementPolicy, WorldId,
};
use serde_json::{Value, json};
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::mem::size_of;

type Item = ItemInstance<u32, u32>;
type Location = ItemLocationRef<u16, (), u16, GroundFixture, [i32; 3]>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GroundFixture {
    channel: u32,
    corpse_occurrence: u64,
}

// A private fixed-width identity fixture, not a production TransactionId API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EvidenceTransaction([u8; 16]);

impl EvidenceTransaction {
    fn new(bytes: [u8; 16]) -> Result<Self, Failure> {
        if bytes[6] >> 4 != 7 || bytes[8] >> 6 != 2 {
            return Err(Failure::Identity);
        }
        Ok(Self(bytes))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Mint,
    Transfer,
}

impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::Mint => "MINT",
            Self::Transfer => "TRANSFER",
        }
    }

    fn tag(self) -> u8 {
        match self {
            Self::Mint => 1,
            Self::Transfer => 2,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Intent {
    kind: Kind,
    transaction: EvidenceTransaction,
    cause: u64,
    item: Item,
    destination: Location,
    authority_generation: u64,
    corpse_revision: u64,
}

// The supported request has one ItemInstance field, never a variable item list.
// Unsupported capability variants do not carry a second item or value plan.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Request {
    OneItem(Intent),
    MultipleItems,
    ValueLine,
    AccountLine,
    Transform,
}

#[derive(Debug, Clone, Copy)]
struct CurrentFacts {
    world: WorldId,
    definition_key: u32,
    definition_revision: u32,
    authority_generation: u64,
    corpse_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Participant {
    item: Item,
    quantity: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EffectKind {
    Remove,
    Establish,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Effect {
    kind: EffectKind,
    item: ItemInstanceId,
    location: Location,
}

// Size-only synthetic input for checking the budget boundary. These are not
// emitted audit events, ANL payloads, or measurements of a mandatory event set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AuditBudgetProbe {
    contributions: u64,
    encoded_bytes: u64,
}

const DIMENSIONS: [&str; 12] = [
    "participants",
    "effects",
    "participant_retained_bytes",
    "effect_retained_bytes",
    "record_encoded_bytes",
    "planning_work",
    "application_work",
    "synthetic_audit_contributions",
    "synthetic_audit_encoded_bytes",
    "reconciliation_records",
    "reconciliation_retained_bytes",
    "cumulative_retry_work",
];

#[derive(Debug, Clone, Copy)]
struct EvidenceLimits([u64; 12]);

impl EvidenceLimits {
    fn check(self, dimension: usize, amount: u64) -> Result<(), Failure> {
        if amount > self.0[dimension] {
            return Err(Failure::Budget(DIMENSIONS[dimension]));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Plan {
    intent: Intent,
    participants: [Participant; 1],
    effects: [Option<Effect>; 2],
    audit_probe: AuditBudgetProbe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Observation {
    NotApplied,
    Committed,
    Ambiguous,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReconciliationRecord {
    plan: Plan,
    event: candidate::FrozenEvent,
    // This is the separate in-memory committed-store fixture fact. It does not
    // assert a DB receipt, process restart, isolation, or current authority.
    committed: bool,
    observation: Observation,
    retry_work: u64,
}

#[derive(Debug, Clone, Copy)]
enum Attempt {
    Acknowledged,
    CommittedResponseLost,
    AmbiguousWithoutCommit,
}

#[derive(Debug, Clone, Copy)]
enum Reconcile {
    InspectCommittedStore,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResultState {
    Committed(ItemInstanceId),
    NotApplied(ItemInstanceId),
    Held(ItemInstanceId),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct Model {
    // One immediate authoritative location is replaced as one tuple. Effects
    // are evidence lines, not competing source/destination truth authorities.
    custody: Option<(Participant, Location)>,
    // Exactly the MINT and TRANSFER evidence slots, not a production capacity.
    records: [Option<ReconciliationRecord>; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    Unsupported,
    Identity,
    Facts,
    Conflict,
    Custody,
    Overflow,
    Budget(&'static str),
}

impl Display for Failure {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "evidence failure: {self:?}")
    }
}

impl Error for Failure {}

fn add(left: u64, right: u64) -> Result<u64, Failure> {
    left.checked_add(right).ok_or(Failure::Overflow)
}

fn bytes(count: u64, width: usize) -> Result<u64, Failure> {
    count
        .checked_mul(u64::try_from(width).map_err(|_| Failure::Overflow)?)
        .ok_or(Failure::Overflow)
}

fn uuid_fixture(tag: u8) -> [u8; 16] {
    let mut value = [0; 16];
    value[5] = 1;
    value[6] = 0x70;
    value[8] = 0x80;
    value[15] = tag;
    value
}

fn fixture() -> Result<(Intent, Intent, CurrentFacts), Box<dyn Error>> {
    let world = WorldId::from_bytes(uuid_fixture(1))?;
    let character = CharacterId::from_bytes(uuid_fixture(2))?;
    let item = Item::new(
        ItemInstanceId::from_bytes(uuid_fixture(3))?,
        world,
        ItemDefinitionRef::new(1, 1),
    );
    let mint = Intent {
        kind: Kind::Mint,
        transaction: EvidenceTransaction::new(uuid_fixture(4))?,
        cause: 1,
        item: item.clone(),
        destination: Location::Ground {
            world_id: world,
            runtime_scope: GroundFixture {
                channel: 1,
                corpse_occurrence: 1,
            },
            spatial_position: [100, 100, 7],
        },
        authority_generation: 1,
        corpse_revision: 1,
    };
    let transfer = Intent {
        kind: Kind::Transfer,
        transaction: EvidenceTransaction::new(uuid_fixture(5))?,
        cause: 2,
        item,
        destination: Location::CharacterInventory {
            character_id: character,
            position: 0,
        },
        authority_generation: 1,
        corpse_revision: 1,
    };
    let facts = CurrentFacts {
        world,
        definition_key: 1,
        definition_revision: 1,
        authority_generation: 1,
        corpse_revision: 1,
    };
    Ok((mint, transfer, facts))
}

fn classify(request: Request) -> Result<Intent, Failure> {
    let Request::OneItem(intent) = request else {
        return Err(Failure::Unsupported);
    };
    // Reject before inspecting custody or constructing any participant/effect.
    match (&intent.kind, &intent.destination) {
        (Kind::Mint, Location::Ground { .. })
        | (Kind::Transfer, Location::CharacterInventory { .. }) => Ok(intent),
        _ => Err(Failure::Unsupported),
    }
}

fn validate_facts(intent: &Intent, facts: CurrentFacts) -> Result<(), Failure> {
    let policy = ItemPlacementPolicy::new(true, false, false, true);
    intent
        .item
        .validate_location(policy, &intent.destination)
        .map_err(|_| Failure::Facts)?;
    intent
        .item
        .definition()
        .ensure_revision(&facts.definition_revision)
        .map_err(|_| Failure::Facts)?;
    if intent.item.world() != facts.world
        || *intent.item.definition().key() != facts.definition_key
        || intent.authority_generation == 0
        || intent.authority_generation != facts.authority_generation
        || intent.corpse_revision == 0
        || intent.corpse_revision != facts.corpse_revision
        || intent.cause == 0
    {
        return Err(Failure::Facts);
    }
    Ok(())
}

fn location_encoded_len(location: &Location) -> Result<u64, Failure> {
    match location {
        Location::Ground { .. } => Ok(1 + 16 + 4 + 8 + 12),
        Location::CharacterInventory { .. } => Ok(1 + 16 + 2),
        _ => Err(Failure::Unsupported),
    }
}

impl Plan {
    fn usage(&self, reconciliation_count: u64) -> Result<[u64; 12], Failure> {
        let mut effects = 0;
        let mut encoded = bytes(1, 44)?;
        for effect in self.effects.iter().flatten() {
            effects = add(effects, 1)?;
            encoded = add(encoded, add(17, location_encoded_len(&effect.location)?)?)?;
        }
        Ok([
            1,
            effects,
            bytes(1, size_of::<Participant>())?,
            bytes(effects, size_of::<Effect>())?,
            encoded,
            add(1, effects)?,
            add(effects, 1)?,
            self.audit_probe.contributions,
            self.audit_probe.encoded_bytes,
            reconciliation_count,
            bytes(reconciliation_count, size_of::<ReconciliationRecord>())?,
            0,
        ])
    }

    fn encode_records(&self) -> Result<Vec<u8>, Failure> {
        // Explicit evidence encoding, never a production/wire serialization.
        let expected = usize::try_from(self.usage(1)?[4]).map_err(|_| Failure::Overflow)?;
        let mut encoded = Vec::with_capacity(expected);
        for participant in &self.participants {
            encoded.extend_from_slice(participant.item.id().as_bytes());
            encoded.extend_from_slice(participant.item.world().as_bytes());
            encoded.extend_from_slice(&participant.item.definition().key().to_be_bytes());
            encoded.extend_from_slice(&participant.item.definition().revision().to_be_bytes());
            encoded.extend_from_slice(&participant.quantity.to_be_bytes());
        }
        for effect in self.effects.iter().flatten() {
            encoded.push(match effect.kind {
                EffectKind::Remove => 1,
                EffectKind::Establish => 2,
            });
            encoded.extend_from_slice(effect.item.as_bytes());
            match &effect.location {
                Location::Ground {
                    world_id,
                    runtime_scope,
                    spatial_position,
                } => {
                    encoded.push(1);
                    encoded.extend_from_slice(world_id.as_bytes());
                    encoded.extend_from_slice(&runtime_scope.channel.to_be_bytes());
                    encoded.extend_from_slice(&runtime_scope.corpse_occurrence.to_be_bytes());
                    for coordinate in spatial_position {
                        encoded.extend_from_slice(&coordinate.to_be_bytes());
                    }
                }
                Location::CharacterInventory {
                    character_id,
                    position,
                } => {
                    encoded.push(2);
                    encoded.extend_from_slice(character_id.as_bytes());
                    encoded.extend_from_slice(&position.to_be_bytes());
                }
                _ => return Err(Failure::Unsupported),
            }
        }
        if encoded.len() != expected {
            return Err(Failure::Conflict);
        }
        Ok(encoded)
    }
}

impl Model {
    fn record_index(&self, transaction: EvidenceTransaction) -> Option<usize> {
        self.records.iter().position(|record| {
            record
                .as_ref()
                .is_some_and(|record| record.plan.intent.transaction == transaction)
        })
    }

    fn plan(&self, intent: Intent, audit_probe: AuditBudgetProbe) -> Result<Plan, Failure> {
        let participant = Participant {
            item: intent.item.clone(),
            quantity: 1,
        };
        let establish = Effect {
            kind: EffectKind::Establish,
            item: intent.item.id(),
            location: intent.destination.clone(),
        };
        let effects = match intent.kind {
            Kind::Mint => {
                if self.custody.is_some() {
                    return Err(Failure::Custody);
                }
                [Some(establish), None]
            }
            Kind::Transfer => {
                if !self.records.iter().flatten().any(|record| {
                    record.plan.intent.kind == Kind::Mint
                        && record.plan.intent.item.id() == intent.item.id()
                        && record.observation == Observation::Committed
                }) {
                    return Err(Failure::Custody);
                }
                let Some((existing, source @ Location::Ground { .. })) = &self.custody else {
                    return Err(Failure::Custody);
                };
                if existing != &participant {
                    return Err(Failure::Custody);
                }
                [
                    Some(Effect {
                        kind: EffectKind::Remove,
                        item: existing.item.id(),
                        location: source.clone(),
                    }),
                    Some(establish),
                ]
            }
        };
        Ok(Plan {
            intent,
            participants: [participant],
            effects,
            audit_probe,
        })
    }

    fn check_plan(&self, plan: &Plan, limits: EvidenceLimits) -> Result<(), Failure> {
        let retained = self
            .records
            .iter()
            .flatten()
            .try_fold(0, |n, _| add(n, 1))?;
        let usage = plan.usage(add(retained, 1)?)?;
        for (dimension, amount) in usage.into_iter().enumerate() {
            limits.check(dimension, amount)?;
        }
        Ok(())
    }

    fn publish_custody(&mut self, plan: &Plan) -> Result<(), Failure> {
        // Validate every line before changing the single custody tuple.
        let expected_establish = Effect {
            kind: EffectKind::Establish,
            item: plan.intent.item.id(),
            location: plan.intent.destination.clone(),
        };
        if plan.participants[0].item != plan.intent.item || plan.participants[0].quantity != 1 {
            return Err(Failure::Custody);
        }
        match plan.intent.kind {
            Kind::Mint
                if self.custody.is_none()
                    && plan.effects[0].as_ref() == Some(&expected_establish)
                    && plan.effects[1].is_none() => {}
            Kind::Transfer => {
                let source = plan.effects[0].as_ref().ok_or(Failure::Custody)?;
                if source.kind != EffectKind::Remove
                    || source.item != plan.intent.item.id()
                    || plan.effects[1].as_ref() != Some(&expected_establish)
                    || self.custody.as_ref()
                        != Some(&(plan.participants[0].clone(), source.location.clone()))
                {
                    return Err(Failure::Custody);
                }
            }
            _ => return Err(Failure::Custody),
        }
        self.custody = Some((
            plan.participants[0].clone(),
            plan.intent.destination.clone(),
        ));
        Ok(())
    }

    fn attempt(
        &mut self,
        request: Request,
        facts: CurrentFacts,
        audit_probe: AuditBudgetProbe,
        limits: EvidenceLimits,
        outcome: Attempt,
    ) -> Result<ResultState, Failure> {
        let intent = classify(request)?;
        validate_facts(&intent, facts)?;
        if let Some(index) = self.record_index(intent.transaction) {
            let record = self.records[index].as_ref().ok_or(Failure::Conflict)?;
            if record.plan.intent != intent || record.plan.audit_probe != audit_probe {
                return Err(Failure::Conflict);
            }
            record.event.check()?;
            if record.observation == Observation::NotApplied {
                // Frozen candidate, same ID and output slot; no rematerialization.
                let plan = record.plan.clone();
                let work = add(record.retry_work, 3)?;
                limits.check(11, work)?;
                let count = u64::try_from(self.records.iter().flatten().count())
                    .map_err(|_| Failure::Overflow)?;
                for (dimension, amount) in plan.usage(count)?.into_iter().enumerate().take(11) {
                    limits.check(dimension, amount)?;
                }
                let committed = !matches!(outcome, Attempt::AmbiguousWithoutCommit);
                if committed {
                    self.publish_custody(&plan)?;
                }
                let record = self.records[index].as_mut().ok_or(Failure::Conflict)?;
                record.committed = committed;
                record.retry_work = work;
                record.observation = if matches!(outcome, Attempt::Acknowledged) {
                    Observation::Committed
                } else {
                    Observation::Ambiguous
                };
                return Ok(if record.observation == Observation::Committed {
                    ResultState::Committed(intent.item.id())
                } else {
                    ResultState::Held(intent.item.id())
                });
            }
            return self.reconcile(intent.transaction, limits, Reconcile::Unknown);
        }
        // This evidence child has exactly one logical MINT and one TRANSFER.
        // A timeout or changed cause cannot open a second transaction/output
        // slot for either operation while the first result remains ambiguous.
        if self
            .records
            .iter()
            .flatten()
            .any(|record| record.plan.intent.kind == intent.kind)
        {
            return Err(Failure::Conflict);
        }
        let slot = self
            .records
            .iter()
            .position(Option::is_none)
            .ok_or(Failure::Unsupported)?;
        let plan = self.plan(intent, audit_probe)?;
        self.check_plan(&plan, limits)?;
        // Encoding occurs only after all checked preflight limits have passed.
        let _encoded = plan.encode_records()?;
        // Freeze exact event identity, semantic envelope and bytes before ambiguity.
        let event = candidate::freeze(&plan)?;
        let committed = !matches!(outcome, Attempt::AmbiguousWithoutCommit);
        if committed {
            self.publish_custody(&plan)?;
        }
        let observation = if matches!(outcome, Attempt::Acknowledged) {
            Observation::Committed
        } else {
            Observation::Ambiguous
        };
        let id = plan.intent.item.id();
        self.records[slot] = Some(ReconciliationRecord {
            plan,
            event,
            committed,
            observation,
            retry_work: 0,
        });
        if observation == Observation::Committed {
            Ok(ResultState::Committed(id))
        } else {
            Ok(ResultState::Held(id))
        }
    }

    fn reconcile(
        &mut self,
        transaction: EvidenceTransaction,
        limits: EvidenceLimits,
        evidence: Reconcile,
    ) -> Result<ResultState, Failure> {
        let index = self.record_index(transaction).ok_or(Failure::Conflict)?;
        let record = self.records[index].as_mut().ok_or(Failure::Conflict)?;
        record.event.check()?;
        // Three deterministic units: receipt lookup, identity check, resolution.
        let work = add(record.retry_work, 3)?;
        limits.check(11, work)?;
        record.retry_work = work;
        if matches!(evidence, Reconcile::InspectCommittedStore) {
            record.observation = if record.committed {
                Observation::Committed
            } else {
                Observation::NotApplied
            };
        }
        let id = record.plan.intent.item.id();
        Ok(match record.observation {
            Observation::Committed => ResultState::Committed(id),
            Observation::NotApplied => ResultState::NotApplied(id),
            Observation::Ambiguous => ResultState::Held(id),
        })
    }
}

fn audit_probe() -> AuditBudgetProbe {
    // The 16 bytes merely match the accepted identity width, not an event size.
    AuditBudgetProbe {
        contributions: 1,
        encoded_bytes: 16,
    }
}

// Schema-specific, example-only candidate; nothing is exported by the server crate.
mod candidate {
    use super::{Failure, Kind, Location, Plan, add, uuid_fixture};
    use prost::Message;
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    use std::mem::size_of;

    const PAYLOAD_MAX: usize = 196_608;
    const ENVELOPE_MAX: usize = 262_144;
    const STRING_MAX: usize = 128;
    const DEPTH_MAX: usize = 32;

    #[derive(Clone, PartialEq, Eq, Message)]
    struct ItemState {
        #[prost(bytes = "vec", tag = "1")]
        item_instance_id: Vec<u8>,
        #[prost(bytes = "vec", tag = "2")]
        world_id: Vec<u8>,
        #[prost(uint32, tag = "3")]
        definition_key: u32,
        #[prost(uint32, tag = "4")]
        definition_revision: u32,
        #[prost(uint32, tag = "5")]
        quantity: u32,
        #[prost(uint32, tag = "6")]
        lifecycle: u32,
    }
    #[derive(Clone, PartialEq, Eq, Message)]
    struct Ground {
        #[prost(bytes = "vec", tag = "1")]
        world_id: Vec<u8>,
        #[prost(bytes = "vec", tag = "2")]
        channel_id: Vec<u8>,
        #[prost(bytes = "vec", tag = "3")]
        corpse_occurrence_id: Vec<u8>,
        #[prost(uint64, tag = "4")]
        corpse_revision: u64,
        #[prost(uint32, tag = "5")]
        x: u32,
        #[prost(uint32, tag = "6")]
        y: u32,
        #[prost(uint32, tag = "7")]
        z: u32,
    }
    #[derive(Clone, PartialEq, Eq, Message)]
    struct Inventory {
        #[prost(bytes = "vec", tag = "1")]
        character_id: Vec<u8>,
        #[prost(uint64, tag = "2")]
        session_generation: u64,
        #[prost(uint32, tag = "3")]
        direct_root_slot: u32,
    }
    #[derive(Clone, PartialEq, Eq, Message)]
    struct Provenance {
        #[prost(uint32, tag = "1")]
        source_kind: u32,
        #[prost(bytes = "vec", tag = "2")]
        occurrence_id: Vec<u8>,
        #[prost(bytes = "vec", tag = "3")]
        cause_id: Vec<u8>,
    }
    #[derive(Clone, PartialEq, Eq, Message)]
    struct Mint {
        #[prost(message, optional, tag = "1")]
        after: Option<ItemState>,
        #[prost(message, optional, tag = "2")]
        destination: Option<Ground>,
        #[prost(message, optional, tag = "3")]
        source: Option<Provenance>,
    }
    #[derive(Clone, PartialEq, Eq, Message)]
    struct Transfer {
        #[prost(message, optional, tag = "1")]
        before: Option<ItemState>,
        #[prost(message, optional, tag = "2")]
        after: Option<ItemState>,
        #[prost(message, optional, tag = "3")]
        source: Option<Ground>,
        #[prost(message, optional, tag = "4")]
        destination: Option<Inventory>,
        #[prost(message, optional, tag = "5")]
        cause: Option<Provenance>,
    }
    #[derive(Clone, PartialEq, Eq, prost::Oneof)]
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
    struct RuntimeOrder {
        #[prost(uint64, tag = "1")]
        scope_ownership_generation: u64,
        #[prost(uint64, tag = "2")]
        runtime_execution_ordinal: u64,
    }
    #[derive(Clone, PartialEq, Eq, Message)]
    struct Membership {
        #[prost(bytes = "vec", tag = "1")]
        transaction_id: Vec<u8>,
        #[prost(uint32, tag = "2")]
        ordinal: u32,
        #[prost(uint32, tag = "3")]
        count: u32,
    }
    #[derive(Clone, PartialEq, Eq, Message)]
    struct Command {
        #[prost(bytes = "vec", tag = "1")]
        game_session_id: Vec<u8>,
        #[prost(uint64, tag = "2")]
        command_id: u64,
    }
    #[derive(Clone, PartialEq, Eq, prost::Oneof)]
    enum Cause {
        #[prost(bytes, tag = "1")]
        Event(Vec<u8>),
        #[prost(message, tag = "2")]
        Command(Command),
        #[prost(bytes, tag = "3")]
        Operation(Vec<u8>),
        #[prost(bytes, tag = "4")]
        Transaction(Vec<u8>),
    }
    #[derive(Clone, PartialEq, Eq, Message)]
    struct Causation {
        #[prost(oneof = "Cause", tags = "1, 2, 3, 4")]
        cause: Option<Cause>,
    }
    #[derive(Clone, PartialEq, Eq, Message)]
    struct Actor {
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
        runtime_order: Option<RuntimeOrder>,
        #[prost(uint64, optional, tag = "16")]
        command_id: Option<u64>,
        #[prost(bytes = "vec", optional, tag = "17")]
        operation_id: Option<Vec<u8>>,
        #[prost(message, optional, tag = "18")]
        transaction_event: Option<Membership>,
        #[prost(bytes = "vec", optional, tag = "19")]
        correlation_id: Option<Vec<u8>>,
        #[prost(message, optional, tag = "20")]
        causation: Option<Causation>,
        #[prost(message, optional, tag = "21")]
        analytics_actor: Option<Actor>,
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(super) struct FrozenEvent {
        semantic: Envelope,
        wire: Vec<u8>,
    }

    // Independent current-fact fixture supplied to validation, not reconstructed
    // from the event/plan. No claim of production lease/session/current authority.
    #[derive(Clone)]
    struct Facts {
        world: [u8; 16],
        channel: [u8; 16],
        corpse: [u8; 16],
        character: [u8; 16],
        session: [u8; 16],
        generation: u64,
        corpse_revision: u64,
        definition_revision: u32,
    }
    fn current_fixture() -> Facts {
        Facts {
            world: uuid_fixture(1),
            channel: uuid_fixture(6),
            corpse: uuid_fixture(12),
            character: uuid_fixture(2),
            session: uuid_fixture(7),
            generation: 1,
            corpse_revision: 1,
            definition_revision: 1,
        }
    }
    fn bounded(amount: usize, maximum: usize) -> Result<(), Failure> {
        if amount > maximum {
            return Err(Failure::Budget("ANL shared limit"));
        }
        Ok(())
    }
    fn identity(value: &[u8]) -> Result<(), Failure> {
        if value.len() != 16
            || value.iter().all(|byte| *byte == 0)
            || value[6] >> 4 != 7
            || value[8] >> 6 != 2
        {
            return Err(Failure::Identity);
        }
        Ok(())
    }
    fn string(value: &str) -> Result<(), Failure> {
        bounded(value.len(), STRING_MAX)?;
        if value.is_empty()
            || !value
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        {
            return Err(Failure::Facts);
        }
        Ok(())
    }
    fn state(value: &ItemState, facts: &Facts) -> Result<(), Failure> {
        identity(&value.item_instance_id)?;
        identity(&value.world_id)?;
        if value.world_id != facts.world
            || value.definition_key != 1
            || value.definition_revision != facts.definition_revision
            || value.quantity == 0
            || value.lifecycle != 1
        {
            return Err(Failure::Facts);
        }
        Ok(())
    }
    fn ground(value: &Ground, facts: &Facts) -> Result<(), Failure> {
        for id in [
            &value.world_id,
            &value.channel_id,
            &value.corpse_occurrence_id,
        ] {
            identity(id)?;
        }
        if value.world_id != facts.world
            || value.channel_id != facts.channel
            || value.corpse_occurrence_id != facts.corpse
            || value.corpse_revision == 0
            || value.corpse_revision != facts.corpse_revision
            || [value.x, value.y, value.z] != [100, 100, 7]
        {
            return Err(Failure::Facts);
        }
        Ok(())
    }
    fn provenance(value: &Provenance, kind: Kind, facts: &Facts) -> Result<(), Failure> {
        identity(&value.occurrence_id)?;
        identity(&value.cause_id)?;
        if value.source_kind != 1
            || value.occurrence_id != facts.corpse
            || value.cause_id != uuid_fixture(if kind == Kind::Mint { 10 } else { 11 })
        {
            return Err(Failure::Facts);
        }
        Ok(())
    }
    fn validate_payload(value: &Payload, facts: &Facts) -> Result<Kind, Failure> {
        if value.interpretation_revision != 1 {
            return Err(Failure::Unsupported);
        }
        match value.operation.as_ref().ok_or(Failure::Unsupported)? {
            Operation::Mint(mint) => {
                state(mint.after.as_ref().ok_or(Failure::Facts)?, facts)?;
                ground(mint.destination.as_ref().ok_or(Failure::Facts)?, facts)?;
                provenance(
                    mint.source.as_ref().ok_or(Failure::Facts)?,
                    Kind::Mint,
                    facts,
                )?;
                Ok(Kind::Mint)
            }
            Operation::Transfer(transfer) => {
                let before = transfer.before.as_ref().ok_or(Failure::Facts)?;
                let after = transfer.after.as_ref().ok_or(Failure::Facts)?;
                state(before, facts)?;
                state(after, facts)?;
                if before != after {
                    return Err(Failure::Conflict);
                }
                ground(transfer.source.as_ref().ok_or(Failure::Facts)?, facts)?;
                let destination = transfer.destination.as_ref().ok_or(Failure::Facts)?;
                identity(&destination.character_id)?;
                if destination.character_id != facts.character
                    || destination.session_generation == 0
                    || destination.session_generation != facts.generation
                    || destination.direct_root_slot != 0
                {
                    return Err(Failure::Facts);
                }
                provenance(
                    transfer.cause.as_ref().ok_or(Failure::Facts)?,
                    Kind::Transfer,
                    facts,
                )?;
                Ok(Kind::Transfer)
            }
        }
    }

    // Borrowed preflight knows only this finite candidate/foundation graph.
    // Unknown additive scalars/opaque bytes are skipped; groups are rejected,
    // including unknown groups, before prost's fixed 100-level recursion path.
    #[derive(Clone, Copy)]
    enum Schema {
        Envelope,
        Payload,
        Mint,
        Transfer,
        State,
        Ground,
        Inventory,
        Provenance,
        Runtime,
        Membership,
        Causation,
        Command,
        Actor,
    }
    fn field(schema: Schema, tag: u32) -> Option<(u8, Option<Schema>, usize)> {
        use Schema::*;
        let result = match (schema, tag) {
            (Envelope, 1 | 3 | 4 | 5 | 6 | 8 | 14 | 16 | 22)
            | (Payload, 1)
            | (State, 3..=6)
            | (Ground, 4..=7)
            | (Inventory, 2 | 3)
            | (Provenance, 1)
            | (Runtime, 1 | 2)
            | (Membership, 2 | 3)
            | (Command, 2)
            | (Actor, 2) => (0, None, 0),
            (Envelope, 7 | 23 | 24 | 25) | (Actor, 1) => (2, None, STRING_MAX),
            (Envelope, 15) => (2, Some(Runtime), ENVELOPE_MAX),
            (Envelope, 18) => (2, Some(Membership), ENVELOPE_MAX),
            (Envelope, 20) => (2, Some(Causation), ENVELOPE_MAX),
            (Envelope, 21) => (2, Some(Actor), ENVELOPE_MAX),
            (Envelope, 26) => (2, Some(Payload), PAYLOAD_MAX),
            (Envelope, 27) => (2, None, 32),
            (Payload, 2) => (2, Some(Mint), PAYLOAD_MAX),
            (Payload, 3) => (2, Some(Transfer), PAYLOAD_MAX),
            (Mint, 1) | (Transfer, 1 | 2) => (2, Some(State), PAYLOAD_MAX),
            (Mint, 2) | (Transfer, 3) => (2, Some(Ground), PAYLOAD_MAX),
            (Mint, 3) | (Transfer, 5) => (2, Some(Provenance), PAYLOAD_MAX),
            (Transfer, 4) => (2, Some(Inventory), PAYLOAD_MAX),
            (Causation, 2) => (2, Some(Command), ENVELOPE_MAX),
            (Envelope, 2 | 9..=13 | 17 | 19)
            | (State, 1 | 2)
            | (Ground, 1..=3)
            | (Inventory, 1)
            | (Provenance, 2 | 3)
            | (Membership, 1)
            | (Command, 1)
            | (Causation, 1 | 3 | 4)
            | (Actor, 3) => (2, None, 16),
            _ => return None,
        };
        Some(result)
    }
    fn varint(input: &[u8], cursor: &mut usize) -> Result<u64, Failure> {
        let mut result = 0;
        for shift in (0..70).step_by(7) {
            let byte = *input.get(*cursor).ok_or(Failure::Facts)?;
            *cursor = cursor.checked_add(1).ok_or(Failure::Overflow)?;
            if shift == 63 && byte > 1 {
                return Err(Failure::Facts);
            }
            result |= u64::from(byte & 127) << shift;
            if byte & 128 == 0 {
                return Ok(result);
            }
        }
        Err(Failure::Facts)
    }
    fn preflight(input: &[u8], schema: Schema, depth: usize) -> Result<(), Failure> {
        bounded(depth, DEPTH_MAX)?;
        let mut cursor = 0;
        let mut seen = [false; 28];
        let mut oneof_seen = false;
        while cursor < input.len() {
            let key = varint(input, &mut cursor)?;
            let tag = u32::try_from(key >> 3).map_err(|_| Failure::Facts)?;
            if tag == 0 || tag > 536_870_911 {
                return Err(Failure::Facts);
            }
            let wire = u8::try_from(key & 7).map_err(|_| Failure::Facts)?;
            let known = field(schema, tag);
            if let Some((expected, _, _)) = known {
                if wire != expected {
                    return Err(Failure::Facts);
                }
                let index = usize::try_from(tag).map_err(|_| Failure::Overflow)?;
                if seen[index] {
                    return Err(Failure::Conflict);
                }
                seen[index] = true;
                if matches!(schema, Schema::Payload) && matches!(tag, 2 | 3)
                    || matches!(schema, Schema::Causation)
                {
                    if oneof_seen {
                        return Err(Failure::Conflict);
                    }
                    oneof_seen = true;
                }
            }
            match wire {
                0 => {
                    varint(input, &mut cursor)?;
                }
                1 | 5 => {
                    cursor = cursor
                        .checked_add(if wire == 1 { 8 } else { 4 })
                        .ok_or(Failure::Overflow)?;
                    if cursor > input.len() {
                        return Err(Failure::Facts);
                    }
                }
                2 => {
                    // Unknown opaque bytes cannot be admitted by this closed
                    // candidate: their identity/privacy meaning is unresolved.
                    // Additive primitive numeric fields remain compatible.
                    if known.is_none() {
                        return Err(Failure::Unsupported);
                    }
                    let length = usize::try_from(varint(input, &mut cursor)?)
                        .map_err(|_| Failure::Overflow)?;
                    let end = cursor.checked_add(length).ok_or(Failure::Overflow)?;
                    let borrowed = input.get(cursor..end).ok_or(Failure::Facts)?;
                    if let Some((_, child, maximum)) = known {
                        bounded(length, maximum)?;
                        if let Some(child) = child {
                            preflight(
                                borrowed,
                                child,
                                depth.checked_add(1).ok_or(Failure::Overflow)?,
                            )?;
                        }
                    }
                    cursor = end;
                }
                _ => return Err(Failure::Unsupported),
            }
        }
        Ok(())
    }
    fn decode_payload(input: &[u8]) -> Result<Payload, Failure> {
        bounded(input.len(), PAYLOAD_MAX)?;
        preflight(input, Schema::Payload, 2)?;
        Payload::decode(input).map_err(|_| Failure::Facts)
    }
    fn validate_envelope(value: &Envelope, facts: &Facts) -> Result<Kind, Failure> {
        identity(&value.event_id)?;
        for id in [&value.world_id, &value.channel_id] {
            identity(id.as_ref().ok_or(Failure::Facts)?)?;
        }
        for text in [&value.retention_profile_id, &value.server_build_id] {
            string(text)?;
        }
        for text in [&value.ruleset_revision, &value.content_revision]
            .into_iter()
            .flatten()
        {
            string(text)?;
        }
        if value.envelope_revision != 1
            || value.event_schema_revision != 1
            || value.event_type_id != 0
        {
            return Err(Failure::Unsupported);
        }
        if value.durability_class != 2
            || !matches!(value.privacy_class, 3 | 4)
            || value.retention_profile_id != "offline_item_unaccepted"
            || value.occurred_at_unix_ms <= 0
            || value.world_id.as_deref() != Some(&facts.world)
            || value.channel_id.as_deref() != Some(&facts.channel)
            || value.ruleset_revision.as_deref() != Some("fixture1")
            || value.content_revision.as_deref() != Some("fixture1")
            || value.server_build_id != "offline"
            || value.instance_id.is_some()
            || value.node_id.is_some()
            || value.operation_id.is_some()
            || value.correlation_id.is_some()
            || value.analytics_actor.is_some()
            || value.protocol_major.is_some()
        {
            return Err(Failure::Facts);
        }
        let order = value.runtime_order.as_ref().ok_or(Failure::Facts)?;
        if order.scope_ownership_generation == 0
            || order.scope_ownership_generation != facts.generation
            || order.runtime_execution_ordinal == 0
        {
            return Err(Failure::Facts);
        }
        let member = value.transaction_event.as_ref().ok_or(Failure::Facts)?;
        identity(&member.transaction_id)?;
        if member.ordinal != 1 || member.count != 1 {
            return Err(Failure::Conflict);
        }
        bounded(value.payload.len(), PAYLOAD_MAX)?;
        if value.payload_sha256.as_slice() != Sha256::digest(&value.payload).as_slice() {
            return Err(Failure::Conflict);
        }
        let kind = validate_payload(&decode_payload(&value.payload)?, facts)?;
        if member.transaction_id != uuid_fixture(if kind == Kind::Mint { 4 } else { 5 })
            || order.runtime_execution_ordinal != u64::from(kind.tag())
        {
            return Err(Failure::Facts);
        }
        match kind {
            Kind::Mint
                if value.game_session_id.is_none()
                    && value.connection_generation.is_none()
                    && value.command_id.is_none()
                    && value.causation.is_none() => {}
            Kind::Transfer => {
                identity(value.game_session_id.as_ref().ok_or(Failure::Facts)?)?;
                if value.game_session_id.as_deref() != Some(&facts.session)
                    || value.connection_generation != Some(facts.generation)
                    || value.command_id != Some(2)
                {
                    return Err(Failure::Facts);
                }
                let Some(Cause::Command(command)) = value
                    .causation
                    .as_ref()
                    .and_then(|cause| cause.cause.as_ref())
                else {
                    return Err(Failure::Facts);
                };
                if command.game_session_id != facts.session || command.command_id != 2 {
                    return Err(Failure::Facts);
                }
            }
            _ => return Err(Failure::Facts),
        }
        Ok(kind)
    }

    // Checked shape accounting precedes prost sizing/encoding and every
    // size-controlled vector allocation. No padding/filler to reach a maximum.
    fn encode<M: Message>(
        value: &M,
        checked_ceiling: usize,
        limit: usize,
    ) -> Result<Vec<u8>, Failure> {
        bounded(checked_ceiling, limit)?;
        let length = value.encoded_len();
        bounded(length, checked_ceiling)?;
        let mut output = Vec::new();
        output
            .try_reserve_exact(length)
            .map_err(|_| Failure::Budget("allocation"))?;
        value.encode(&mut output).map_err(|_| Failure::Facts)?;
        if output.len() != length {
            return Err(Failure::Conflict);
        }
        Ok(output)
    }
    fn raw_shape_charge(plan: &Plan) -> Result<usize, Failure> {
        if plan.participants[0].quantity == 0
            || plan.intent.cause != u64::from(plan.intent.kind.tag())
        {
            return Err(Failure::Facts);
        }
        // Finite nonrecursive shape: <=64 fields, <=3 bytes key/length and
        // <=10 bytes scalar; at most 13 fixed 16-byte IDs. Checked before copy.
        let upper = add(64_u64.checked_mul(13).ok_or(Failure::Overflow)?, 13 * 16)?;
        usize::try_from(upper).map_err(|_| Failure::Overflow)
    }
    fn payload(plan: &Plan) -> Result<Payload, Failure> {
        let checked = raw_shape_charge(plan)?;
        bounded(checked, PAYLOAD_MAX)?;
        let state = ItemState {
            item_instance_id: plan.intent.item.id().as_bytes().to_vec(),
            world_id: plan.intent.item.world().as_bytes().to_vec(),
            definition_key: *plan.intent.item.definition().key(),
            definition_revision: *plan.intent.item.definition().revision(),
            quantity: plan.participants[0].quantity,
            lifecycle: 1,
        };
        let location = plan
            .effects
            .iter()
            .flatten()
            .find_map(|effect| match &effect.location {
                Location::Ground {
                    world_id,
                    runtime_scope,
                    spatial_position,
                } => Some((world_id, runtime_scope, spatial_position)),
                _ => None,
            })
            .ok_or(Failure::Facts)?;
        if location.1.channel != 1 || location.1.corpse_occurrence != 1 {
            return Err(Failure::Facts);
        }
        let ground = Ground {
            world_id: location.0.as_bytes().to_vec(),
            channel_id: uuid_fixture(6).to_vec(),
            corpse_occurrence_id: uuid_fixture(12).to_vec(),
            corpse_revision: plan.intent.corpse_revision,
            x: u32::try_from(location.2[0]).map_err(|_| Failure::Facts)?,
            y: u32::try_from(location.2[1]).map_err(|_| Failure::Facts)?,
            z: u32::try_from(location.2[2]).map_err(|_| Failure::Facts)?,
        };
        let source = Provenance {
            source_kind: 1,
            occurrence_id: uuid_fixture(12).to_vec(),
            cause_id: uuid_fixture(if plan.intent.kind == Kind::Mint {
                10
            } else {
                11
            })
            .to_vec(),
        };
        let operation = match plan.intent.kind {
            Kind::Mint => Operation::Mint(Mint {
                after: Some(state),
                destination: Some(ground),
                source: Some(source),
            }),
            Kind::Transfer => {
                let Location::CharacterInventory {
                    character_id,
                    position,
                } = &plan.intent.destination
                else {
                    return Err(Failure::Unsupported);
                };
                Operation::Transfer(Transfer {
                    before: Some(state.clone()),
                    after: Some(state),
                    source: Some(ground),
                    destination: Some(Inventory {
                        character_id: character_id.as_bytes().to_vec(),
                        session_generation: plan.intent.authority_generation,
                        direct_root_slot: u32::from(*position),
                    }),
                    cause: Some(source),
                })
            }
        };
        let value = Payload {
            interpretation_revision: 1,
            operation: Some(operation),
        };
        validate_payload(&value, &current_fixture())?;
        Ok(value)
    }
    pub(super) fn freeze(plan: &Plan) -> Result<FrozenEvent, Failure> {
        let checked = raw_shape_charge(plan)?;
        // Reserve the finite retained shape before any private prost buffers:
        // payload <= checked, envelope <= checked+1024, bounded identity/string
        // side buffers <=174, plus the inline carrier. This is a derived offline
        // fixture bound, not a registry or production resource maximum.
        let retained_reservation = checked
            .checked_mul(2)
            .and_then(|n| n.checked_add(1024 + 174 + size_of::<FrozenEvent>()))
            .ok_or(Failure::Overflow)?;
        let bytes = encode(&payload(plan)?, checked, PAYLOAD_MAX)?;
        // All fixed envelope strings/IDs checked as literals; reserve for actual
        // payload plus finite envelope overhead before any envelope copies.
        let envelope_ceiling = usize::try_from(add(
            u64::try_from(bytes.len()).map_err(|_| Failure::Overflow)?,
            1024,
        )?)
        .map_err(|_| Failure::Overflow)?;
        bounded(envelope_ceiling, ENVELOPE_MAX)?;
        let transfer = plan.intent.kind == Kind::Transfer;
        let value = Envelope {
            envelope_revision: 1,
            event_id: uuid_fixture(if transfer { 9 } else { 8 }).to_vec(),
            event_type_id: 0,
            event_schema_revision: 1,
            durability_class: 2,
            privacy_class: 3,
            retention_profile_id: "offline_item_unaccepted".into(),
            occurred_at_unix_ms: 1,
            world_id: Some(plan.intent.item.world().as_bytes().to_vec()),
            channel_id: Some(uuid_fixture(6).to_vec()),
            runtime_order: Some(RuntimeOrder {
                scope_ownership_generation: plan.intent.authority_generation,
                runtime_execution_ordinal: u64::from(plan.intent.kind.tag()),
            }),
            transaction_event: Some(Membership {
                transaction_id: plan.intent.transaction.0.to_vec(),
                ordinal: 1,
                count: 1,
            }),
            game_session_id: transfer.then(|| uuid_fixture(7).to_vec()),
            connection_generation: transfer.then_some(1),
            command_id: transfer.then_some(2),
            causation: transfer.then(|| Causation {
                cause: Some(Cause::Command(Command {
                    game_session_id: uuid_fixture(7).to_vec(),
                    command_id: 2,
                })),
            }),
            ruleset_revision: Some("fixture1".into()),
            content_revision: Some("fixture1".into()),
            server_build_id: "offline".into(),
            payload_sha256: Sha256::digest(&bytes).to_vec(),
            payload: bytes,
            ..Envelope::default()
        };
        validate_envelope(&value, &current_fixture())?;
        let wire = encode(&value, envelope_ceiling, ENVELOPE_MAX)?;
        preflight(&wire, Schema::Envelope, 1)?;
        let frozen = FrozenEvent {
            semantic: value,
            wire,
        };
        bounded(
            size_of::<FrozenEvent>()
                .checked_add(frozen.dynamic_retained_bytes()?)
                .ok_or(Failure::Overflow)?,
            retained_reservation,
        )?;
        Ok(frozen)
    }
    impl FrozenEvent {
        fn decode(wire: &[u8], facts: &Facts) -> Result<Self, Failure> {
            bounded(wire.len(), ENVELOPE_MAX)?;
            preflight(wire, Schema::Envelope, 1)?;
            let semantic = Envelope::decode(wire).map_err(|_| Failure::Facts)?;
            validate_envelope(&semantic, facts)?;
            Ok(Self {
                semantic,
                wire: wire.to_vec(),
            })
        }
        pub(super) fn check(&self) -> Result<(), Failure> {
            let decoded = Self::decode(&self.wire, &current_fixture())?;
            if decoded.semantic != self.semantic {
                return Err(Failure::Conflict);
            }
            Ok(())
        }
        // Semantic equality + exact payload bytes/hash; reordered outer envelope
        // wire is permitted by ANL. Retrying this fixture nevertheless retains
        // its original outer wire as well, without reserialization.
        fn duplicate(&self, other: &Self) -> Result<(), Failure> {
            self.check()?;
            other.check()?;
            if self.semantic != other.semantic {
                return Err(Failure::Conflict);
            }
            Ok(())
        }
        fn replay(&self, model: &super::Model) -> Result<Value, Failure> {
            self.check()?;
            // Read-only model borrow: replay cannot call publication or attempt.
            Ok(
                json!({"derived_only":true,"authoritative_items":usize::from(model.custody.is_some())}),
            )
        }
        fn production_admission(&self) -> Result<(), Failure> {
            // No registered type or accepted item retention/access profile exists.
            Err(Failure::Unsupported)
        }
        fn raw_id_export(&self, authorized_item_profile: bool) -> Result<&[u8], Failure> {
            if !authorized_item_profile {
                return Err(Failure::Facts);
            }
            // Even caller authorization cannot substitute for registry/profile.
            self.production_admission()?;
            Ok(&self.semantic.payload)
        }
        pub(super) fn dynamic_retained_bytes(&self) -> Result<usize, Failure> {
            let value = &self.semantic;
            let mut dynamic = self.wire.capacity();
            let mut charge = |amount: usize| -> Result<(), Failure> {
                dynamic = dynamic.checked_add(amount).ok_or(Failure::Overflow)?;
                Ok(())
            };
            for bytes in [&value.event_id, &value.payload, &value.payload_sha256] {
                charge(bytes.capacity())?;
            }
            for bytes in [&value.world_id, &value.channel_id, &value.game_session_id]
                .into_iter()
                .flatten()
            {
                charge(bytes.capacity())?;
            }
            for text in [&value.retention_profile_id, &value.server_build_id] {
                charge(text.capacity())?;
            }
            for text in [&value.ruleset_revision, &value.content_revision]
                .into_iter()
                .flatten()
            {
                charge(text.capacity())?;
            }
            if let Some(member) = &value.transaction_event {
                charge(member.transaction_id.capacity())?;
            }
            if let Some(Cause::Command(command)) = value
                .causation
                .as_ref()
                .and_then(|cause| cause.cause.as_ref())
            {
                charge(command.game_session_id.capacity())?;
            }
            Ok(dynamic)
        }
        pub(super) fn measurement(&self) -> Result<Value, Failure> {
            self.check()?;
            let value = &self.semantic;
            let dynamic = self.dynamic_retained_bytes()?;
            Ok(json!({
                "classification":"UNREGISTERED_OFFLINE_CANDIDATE",
                "registered_event_type_id":null,"retention_profile_accepted":false,
                "event_id":hex(&value.event_id),"payload_sha256":hex(&value.payload_sha256),
                "payload_encoded_bytes":value.payload.len(),"full_envelope_encoded_bytes":self.wire.len(),
                "one_event_aggregate_count":1,"one_event_aggregate_encoded_bytes":self.wire.len(),
                "envelope_wire_vector_capacity_bytes":self.wire.capacity(),
                "payload_vector_capacity_bytes":value.payload.capacity(),
                "frozen_carrier_inline_bytes":size_of::<Self>(),
                "retained_dynamic_vector_and_string_capacity_bytes":dynamic,
                "retained_inline_plus_dynamic_bytes":size_of::<Self>().checked_add(dynamic).ok_or(Failure::Overflow)?,
                "transient_decode_or_allocator_overhead_measured":false,
                "membership":{"ordinal":1,"count":1},
                "exact_payload_hex":hex(&value.payload),"exact_envelope_hex":hex(&self.wire)
            }))
        }
    }
    fn hex(value: &[u8]) -> String {
        value.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    // Independent revision: never reinterpret historical unsigned revision 1.
    pub(super) mod v2 {
        use super::*;

        #[derive(Clone, PartialEq, Eq, Message)]
        struct SignedGround {
            #[prost(bytes = "vec", tag = "1")]
            world_id: Vec<u8>,
            #[prost(bytes = "vec", tag = "2")]
            channel_id: Vec<u8>,
            #[prost(bytes = "vec", tag = "3")]
            corpse_occurrence_id: Vec<u8>,
            #[prost(uint64, tag = "4")]
            corpse_revision: u64,
            #[prost(sint32, tag = "5")]
            x: i32,
            #[prost(sint32, tag = "6")]
            y: i32,
            #[prost(sint32, tag = "7")]
            floor: i32,
        }
        #[derive(Clone, Copy, Debug, PartialEq, Eq, prost::Enumeration)]
        #[repr(i32)]
        enum FixtureRootPosition {
            Unspecified = 0,
            SyntheticDirectRoot = 1,
        }
        #[derive(Clone, PartialEq, Eq, Message)]
        struct RootInventory {
            #[prost(bytes = "vec", tag = "1")]
            character_id: Vec<u8>,
            #[prost(uint64, tag = "2")]
            session_generation: u64,
            #[prost(enumeration = "FixtureRootPosition", tag = "3")]
            fixture_root_position: i32,
        }
        #[derive(Clone, PartialEq, Eq, Message)]
        struct SignedMint {
            #[prost(message, optional, tag = "1")]
            after: Option<ItemState>,
            #[prost(message, optional, tag = "2")]
            destination: Option<SignedGround>,
            #[prost(message, optional, tag = "3")]
            source: Option<Provenance>,
        }
        #[derive(Clone, PartialEq, Eq, Message)]
        struct SignedTransfer {
            #[prost(message, optional, tag = "1")]
            before: Option<ItemState>,
            #[prost(message, optional, tag = "2")]
            after: Option<ItemState>,
            #[prost(message, optional, tag = "3")]
            source: Option<SignedGround>,
            #[prost(message, optional, tag = "4")]
            destination: Option<RootInventory>,
            #[prost(message, optional, tag = "5")]
            cause: Option<Provenance>,
        }
        #[derive(Clone, PartialEq, Eq, prost::Oneof)]
        enum SignedOperation {
            #[prost(message, tag = "2")]
            Mint(SignedMint),
            #[prost(message, tag = "3")]
            Transfer(SignedTransfer),
        }
        #[derive(Clone, PartialEq, Eq, Message)]
        struct SignedPayload {
            #[prost(uint32, tag = "1")]
            interpretation_revision: u32,
            #[prost(oneof = "SignedOperation", tags = "2, 3")]
            operation: Option<SignedOperation>,
        }

        // Supplied independently by the caller, never reconstructed from event
        // geometry or immutable provenance. No product World activation implied.
        #[derive(Clone)]
        struct WorldFacts {
            bindings: Facts,
            revision: &'static str,
            x: [i64; 2],
            y: [i64; 2],
            floors: &'static [i16],
        }
        impl WorldFacts {
            fn check(&self) -> Result<(), Failure> {
                let low = i64::from(i32::MIN);
                let high = i64::from(i32::MAX) + 1;
                for [min, max] in [self.x, self.y] {
                    if min < low || max > high || min >= max {
                        return Err(Failure::Facts);
                    }
                }
                string(self.revision)?;
                if self.floors.is_empty() || !self.floors.windows(2).all(|pair| pair[0] < pair[1]) {
                    return Err(Failure::Facts);
                }
                for id in [
                    self.bindings.world,
                    self.bindings.channel,
                    self.bindings.corpse,
                    self.bindings.character,
                    self.bindings.session,
                ] {
                    identity(&id)?;
                }
                if self.bindings.generation == 0
                    || self.bindings.corpse_revision == 0
                    || self.bindings.definition_revision == 0
                {
                    return Err(Failure::Facts);
                }
                Ok(())
            }
            fn position(&self, position: [i32; 3]) -> Result<(), Failure> {
                self.check()?;
                let floor = i16::try_from(position[2]).map_err(|_| Failure::Facts)?;
                if i64::from(position[0]) < self.x[0]
                    || i64::from(position[0]) >= self.x[1]
                    || i64::from(position[1]) < self.y[0]
                    || i64::from(position[1]) >= self.y[1]
                    || self.floors.binary_search(&floor).is_err()
                {
                    return Err(Failure::Facts);
                }
                Ok(())
            }
        }
        fn world_fixture() -> WorldFacts {
            WorldFacts {
                bindings: current_fixture(),
                revision: "synthetic_world_r2",
                x: [-200, 200],
                y: [-200, 200],
                floors: &[-1, 0, 7],
            }
        }

        fn sum(values: &[usize]) -> Result<usize, Failure> {
            values.iter().try_fold(0_usize, |n, value| {
                n.checked_add(*value).ok_or(Failure::Overflow)
            })
        }
        fn varint_width(mut value: u64) -> usize {
            let mut width = 1;
            while value >= 128 {
                width += 1;
                value >>= 7;
            }
            width
        }
        fn delimited(tag: u32, length: usize) -> Result<usize, Failure> {
            sum(&[
                varint_width((u64::from(tag) << 3) | 2),
                varint_width(u64::try_from(length).map_err(|_| Failure::Overflow)?),
                length,
            ])
        }
        fn scalar(tag: u32, width: usize) -> Result<usize, Failure> {
            sum(&[varint_width(u64::from(tag) << 3), width])
        }
        #[derive(Clone, Copy)]
        struct Upper {
            state: usize,
            ground: usize,
            inventory: usize,
            provenance: usize,
            inner: usize,
            payload: usize,
            envelope_overhead: usize,
            envelope: usize,
            retained_dynamic: usize,
        }
        // Algebra only: never prost::encoded_len, observed fixtures, padding or
        // a gameplay ceiling. Includes mutually incompatible optional scopes.
        fn upper(kind: Kind) -> Result<Upper, Failure> {
            let state = sum(&[18, 18, 6, 6, 6, 2])?;
            let ground = sum(&[18, 18, 18, 11, 6, 6, 4])?;
            let inventory = sum(&[18, 11, 2])?;
            let provenance = sum(&[2, 18, 18])?;
            let inner = if kind == Kind::Mint {
                sum(&[
                    delimited(1, state)?,
                    delimited(2, ground)?,
                    delimited(3, provenance)?,
                ])?
            } else {
                sum(&[
                    delimited(1, state)?,
                    delimited(2, state)?,
                    delimited(3, ground)?,
                    delimited(4, inventory)?,
                    delimited(5, provenance)?,
                ])?
            };
            let payload = sum(&[
                scalar(1, 1)?,
                delimited(if kind == Kind::Mint { 2 } else { 3 }, inner)?,
            ])?;
            let runtime = sum(&[11, 11])?;
            let membership = sum(&[18, 2, 2])?;
            let command = sum(&[18, 11])?;
            let cause = delimited(2, command)?;
            let actor = sum(&[delimited(1, STRING_MAX)?, 11, 18])?;
            let envelope_overhead = sum(&[
                2,
                18,
                2,
                2,
                2,
                delimited(7, STRING_MAX)?,
                11,
                18,
                18,
                18,
                18,
                18,
                11,
                delimited(15, runtime)?,
                12,
                19,
                delimited(18, membership)?,
                19,
                delimited(20, cause)?,
                delimited(21, actor)?,
                7,
                delimited(23, STRING_MAX)?,
                delimited(24, STRING_MAX)?,
                delimited(25, STRING_MAX)?,
                delimited(27, 32)?,
            ])?;
            let envelope = sum(&[envelope_overhead, delimited(26, payload)?])?;
            // Owned semantic payload and full wire are separate retained copies.
            // All optional UUID side vectors + four strings and actor domain.
            let retained_dynamic = sum(&[
                envelope,
                payload,
                16,
                32,
                80,
                32,
                16,
                16,
                16,
                5 * STRING_MAX,
            ])?;
            Ok(Upper {
                state,
                ground,
                inventory,
                provenance,
                inner,
                payload,
                envelope_overhead,
                envelope,
                retained_dynamic,
            })
        }

        #[derive(Clone, Copy)]
        enum Schema2 {
            Envelope,
            Payload,
            Mint,
            Transfer,
            State,
            Ground,
            Inventory,
            Provenance,
            Runtime,
            Membership,
            Causation,
            Command,
            Actor,
        }
        #[derive(Clone, Copy)]
        enum Field2 {
            U32,
            U64,
            Constant(u64),
            Floor,
            Id,
            Hash,
            Text,
            Message(Schema2),
        }
        fn field2(schema: Schema2, tag: u32) -> Result<Field2, Failure> {
            use Field2::{Constant, Floor, Hash, Id, Message as Nested, Text, U32, U64};
            use Schema2::*;
            let result = match (schema, tag) {
                (Envelope, 1) => Constant(1),
                (Envelope, 4 | 5) => Constant(2),
                (Envelope, 6) => Constant(3),
                (Envelope, 2 | 9..=13 | 17 | 19) => Id,
                (Envelope, 7 | 23..=25) | (Actor, 1) => Text,
                (Envelope, 8 | 14 | 16) | (Runtime, 1 | 2) | (Command, 2) | (Actor, 2) => U64,
                (Envelope, 22) => U32,
                (Envelope, 15) => Nested(Runtime),
                (Envelope, 18) => Nested(Membership),
                (Envelope, 20) => Nested(Causation),
                (Envelope, 21) => Nested(Actor),
                (Envelope, 26) => Nested(Payload),
                (Envelope, 27) => Hash,
                (Payload, 1) => Constant(2),
                (Payload, 2) => Nested(Mint),
                (Payload, 3) => Nested(Transfer),
                (Mint, 1) | (Transfer, 1 | 2) => Nested(State),
                (Mint, 2) | (Transfer, 3) => Nested(Ground),
                (Mint, 3) | (Transfer, 5) => Nested(Provenance),
                (Transfer, 4) => Nested(Inventory),
                (State, 1 | 2)
                | (Ground, 1..=3)
                | (Inventory | Membership | Command, 1)
                | (Provenance, 2 | 3)
                | (Causation, 1 | 3 | 4)
                | (Actor, 3) => Id,
                (State, 3..=5) | (Ground, 5 | 6) => U32,
                (State, 6) | (Provenance, 1) | (Inventory, 3) | (Membership, 2 | 3) => Constant(1),
                (Ground, 4) | (Inventory, 2) => U64,
                (Ground, 7) => Floor,
                (Causation, 2) => Nested(Command),
                _ => return Err(Failure::Unsupported),
            };
            Ok(result)
        }
        fn read_varint(input: &[u8], cursor: &mut usize) -> Result<u64, Failure> {
            let mut value = 0_u64;
            for index in 0..10 {
                let byte = *input.get(*cursor).ok_or(Failure::Facts)?;
                *cursor = cursor.checked_add(1).ok_or(Failure::Overflow)?;
                if index == 9 && byte > 1 {
                    return Err(Failure::Overflow);
                }
                value |= u64::from(byte & 127) << (index * 7);
                if byte < 128 {
                    if varint_width(value) != index + 1 {
                        return Err(Failure::Facts);
                    }
                    return Ok(value);
                }
            }
            Err(Failure::Overflow)
        }
        fn required(schema: Schema2) -> u32 {
            let tags: &[u32] = match schema {
                Schema2::Envelope => &[1, 2, 4, 5, 6, 7, 8, 9, 10, 15, 18, 23, 24, 25, 26, 27],
                Schema2::Payload => &[1],
                Schema2::Mint
                | Schema2::Inventory
                | Schema2::Provenance
                | Schema2::Membership
                | Schema2::Actor => &[1, 2, 3],
                Schema2::Transfer => &[1, 2, 3, 4, 5],
                Schema2::State => &[1, 2, 3, 4, 5, 6],
                Schema2::Ground => &[1, 2, 3, 4],
                Schema2::Runtime | Schema2::Command => &[1, 2],
                Schema2::Causation => &[],
            };
            tags.iter().fold(0, |mask, tag| mask | (1_u32 << tag))
        }
        // Borrowed finite grammar walk before prost or any input-sized copy.
        // Strict canonical order is candidate-local, NOT general ANL admission.
        fn preflight2(input: &[u8], schema: Schema2, depth: usize) -> Result<(), Failure> {
            bounded(depth, 5)?;
            let maximum = match schema {
                Schema2::Envelope => upper(Kind::Transfer)?.envelope,
                Schema2::Payload => upper(Kind::Transfer)?.payload,
                Schema2::Mint => upper(Kind::Mint)?.inner,
                Schema2::Transfer => upper(Kind::Transfer)?.inner,
                Schema2::State => 56,
                Schema2::Ground => 81,
                Schema2::Inventory => 31,
                Schema2::Provenance => 38,
                Schema2::Runtime => 22,
                Schema2::Membership => 22,
                Schema2::Causation => 31,
                Schema2::Command => 29,
                Schema2::Actor => 160,
            };
            bounded(input.len(), maximum)?;
            let mut cursor = 0;
            let mut seen = 0_u32;
            let mut previous = 0;
            while cursor < input.len() {
                let key = read_varint(input, &mut cursor)?;
                let tag = u32::try_from(key >> 3).map_err(|_| Failure::Unsupported)?;
                if tag == 0 || tag > 27 || tag <= previous {
                    return Err(Failure::Unsupported);
                }
                previous = tag;
                seen |= 1_u32 << tag;
                let field = field2(schema, tag)?;
                match field {
                    Field2::U32 | Field2::U64 | Field2::Constant(_) | Field2::Floor => {
                        if key & 7 != 0 {
                            return Err(Failure::Facts);
                        }
                        let value = read_varint(input, &mut cursor)?;
                        match field {
                            Field2::U32 if value > u64::from(u32::MAX) => {
                                return Err(Failure::Facts);
                            }
                            Field2::Floor if value > u64::from(u16::MAX) => {
                                return Err(Failure::Facts);
                            }
                            Field2::Constant(expected) if value != expected => {
                                return Err(Failure::Unsupported);
                            }
                            _ => {}
                        }
                        // Encoded zero non-optional scalar is not canonical;
                        // optional presence can encode zero, then semantics reject.
                        if value == 0
                            && !(matches!(schema, Schema2::Envelope) && matches!(tag, 14 | 16 | 22))
                        {
                            return Err(Failure::Facts);
                        }
                    }
                    _ => {
                        if key & 7 != 2 {
                            return Err(Failure::Facts);
                        }
                        let length = usize::try_from(read_varint(input, &mut cursor)?)
                            .map_err(|_| Failure::Overflow)?;
                        let end = cursor.checked_add(length).ok_or(Failure::Overflow)?;
                        let value = input.get(cursor..end).ok_or(Failure::Facts)?;
                        match field {
                            Field2::Id => identity(value)?,
                            Field2::Hash => {
                                if value.len() != 32 {
                                    return Err(Failure::Facts);
                                }
                            }
                            Field2::Text => {
                                string(std::str::from_utf8(value).map_err(|_| Failure::Facts)?)?
                            }
                            Field2::Message(nested) => preflight2(
                                value,
                                nested,
                                depth.checked_add(1).ok_or(Failure::Overflow)?,
                            )?,
                            _ => return Err(Failure::Facts),
                        }
                        cursor = end;
                    }
                }
            }
            let mask = required(schema);
            if seen & mask != mask {
                return Err(Failure::Facts);
            }
            if matches!(schema, Schema2::Payload)
                && (seen & ((1 << 2) | (1 << 3))).count_ones() != 1
                || matches!(schema, Schema2::Causation) && seen.count_ones() != 1
            {
                return Err(Failure::Unsupported);
            }
            Ok(())
        }

        fn make_payload(
            kind: Kind,
            position: [i32; 3],
            facts: &WorldFacts,
        ) -> Result<SignedPayload, Failure> {
            facts.position(position)?;
            let base = &facts.bindings;
            let state = ItemState {
                item_instance_id: uuid_fixture(3).to_vec(),
                world_id: base.world.to_vec(),
                definition_key: 1,
                definition_revision: base.definition_revision,
                quantity: 1,
                lifecycle: 1,
            };
            let ground = SignedGround {
                world_id: base.world.to_vec(),
                channel_id: base.channel.to_vec(),
                corpse_occurrence_id: base.corpse.to_vec(),
                corpse_revision: base.corpse_revision,
                x: position[0],
                y: position[1],
                floor: position[2],
            };
            let provenance = Provenance {
                source_kind: 1,
                occurrence_id: base.corpse.to_vec(),
                cause_id: uuid_fixture(if kind == Kind::Mint { 10 } else { 11 }).to_vec(),
            };
            let operation = if kind == Kind::Mint {
                SignedOperation::Mint(SignedMint {
                    after: Some(state),
                    destination: Some(ground),
                    source: Some(provenance),
                })
            } else {
                SignedOperation::Transfer(SignedTransfer {
                    before: Some(state.clone()),
                    after: Some(state),
                    source: Some(ground),
                    destination: Some(RootInventory {
                        character_id: base.character.to_vec(),
                        session_generation: base.generation,
                        fixture_root_position: FixtureRootPosition::SyntheticDirectRoot as i32,
                    }),
                    cause: Some(provenance),
                })
            };
            Ok(SignedPayload {
                interpretation_revision: 2,
                operation: Some(operation),
            })
        }
        fn validate2(value: &Envelope, facts: &WorldFacts) -> Result<Kind, Failure> {
            facts.check()?;
            preflight2(&value.payload, Schema2::Payload, 1)?;
            let payload =
                SignedPayload::decode(value.payload.as_slice()).map_err(|_| Failure::Facts)?;
            let (kind, before, after, ground, inventory, source) =
                match payload.operation.as_ref().ok_or(Failure::Unsupported)? {
                    SignedOperation::Mint(mint) => (
                        Kind::Mint,
                        None,
                        mint.after.as_ref(),
                        mint.destination.as_ref(),
                        None,
                        mint.source.as_ref(),
                    ),
                    SignedOperation::Transfer(transfer) => (
                        Kind::Transfer,
                        transfer.before.as_ref(),
                        transfer.after.as_ref(),
                        transfer.source.as_ref(),
                        transfer.destination.as_ref(),
                        transfer.cause.as_ref(),
                    ),
                };
            let after = after.ok_or(Failure::Facts)?;
            state(after, &facts.bindings)?;
            // This positive fixture's quantity=1 is not a promoted u32/game cap.
            if after.item_instance_id != uuid_fixture(3) || after.quantity != 1 {
                return Err(Failure::Facts);
            }
            if kind == Kind::Transfer && before != Some(after) {
                return Err(Failure::Conflict);
            }
            let ground = ground.ok_or(Failure::Facts)?;
            if ground.world_id != facts.bindings.world
                || ground.channel_id != facts.bindings.channel
                || ground.corpse_occurrence_id != facts.bindings.corpse
                || ground.corpse_revision != facts.bindings.corpse_revision
            {
                return Err(Failure::Facts);
            }
            facts.position([ground.x, ground.y, ground.floor])?;
            provenance(source.ok_or(Failure::Facts)?, kind, &facts.bindings)?;
            if kind == Kind::Transfer {
                let root = inventory.ok_or(Failure::Facts)?;
                if root.character_id != facts.bindings.character
                    || root.session_generation != facts.bindings.generation
                    || root.fixture_root_position != FixtureRootPosition::SyntheticDirectRoot as i32
                {
                    return Err(Failure::Facts);
                }
            }
            let is_transfer = kind == Kind::Transfer;
            let runtime = value.runtime_order.as_ref().ok_or(Failure::Facts)?;
            let member = value.transaction_event.as_ref().ok_or(Failure::Facts)?;
            if value.envelope_revision != 1
                || value.event_type_id != 0
                || value.event_schema_revision != 2
                || value.durability_class != 2
                || value.privacy_class != 3
                || value.occurred_at_unix_ms <= 0
                || value.retention_profile_id != "offline_item_unaccepted"
                || value.server_build_id != "offline"
                || value.world_id.as_deref() != Some(facts.bindings.world.as_slice())
                || value.channel_id.as_deref() != Some(facts.bindings.channel.as_slice())
                || value.instance_id.is_some()
                || runtime.scope_ownership_generation != facts.bindings.generation
                || runtime.runtime_execution_ordinal != u64::from(kind.tag())
                || member.transaction_id != uuid_fixture(if is_transfer { 23 } else { 22 })
                || member.ordinal != 1
                || member.count != 1
                || value.event_id != uuid_fixture(if is_transfer { 21 } else { 20 })
                || value.ruleset_revision.as_deref() != Some("fixture2")
                || value.content_revision.as_deref() != Some(facts.revision)
                || value.payload_sha256.as_slice() != Sha256::digest(&value.payload).as_slice()
                || value.game_session_id.as_deref()
                    != is_transfer.then_some(facts.bindings.session.as_slice())
                || value.connection_generation != is_transfer.then_some(facts.bindings.generation)
                || value.command_id != is_transfer.then_some(2)
                || value.node_id.is_some()
                || value.operation_id.is_some()
                || value.correlation_id.is_some()
                || value.analytics_actor.is_some()
                || value.protocol_major.is_some()
            {
                return Err(Failure::Facts);
            }
            match value
                .causation
                .as_ref()
                .and_then(|cause| cause.cause.as_ref())
            {
                None if !is_transfer => {}
                Some(Cause::Command(command))
                    if is_transfer
                        && command.game_session_id == facts.bindings.session
                        && command.command_id == 2 => {}
                _ => return Err(Failure::Facts),
            }
            Ok(kind)
        }
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct Frozen2 {
            semantic: Envelope,
            wire: Vec<u8>,
        }
        fn dynamic2(value: &Frozen2) -> Result<usize, Failure> {
            let env = &value.semantic;
            let mut charges = vec![
                value.wire.capacity(),
                env.payload.capacity(),
                env.event_id.capacity(),
                env.payload_sha256.capacity(),
                env.retention_profile_id.capacity(),
                env.server_build_id.capacity(),
            ];
            for id in [
                &env.world_id,
                &env.channel_id,
                &env.instance_id,
                &env.node_id,
                &env.game_session_id,
                &env.operation_id,
                &env.correlation_id,
            ]
            .into_iter()
            .flatten()
            {
                charges.push(id.capacity());
            }
            for text in [&env.ruleset_revision, &env.content_revision]
                .into_iter()
                .flatten()
            {
                charges.push(text.capacity());
            }
            if let Some(member) = &env.transaction_event {
                charges.push(member.transaction_id.capacity());
            }
            if let Some(cause) = env
                .causation
                .as_ref()
                .and_then(|cause| cause.cause.as_ref())
            {
                charges.push(match cause {
                    Cause::Command(command) => command.game_session_id.capacity(),
                    Cause::Event(id) | Cause::Operation(id) | Cause::Transaction(id) => {
                        id.capacity()
                    }
                });
            }
            if let Some(actor) = &env.analytics_actor {
                charges.extend([
                    actor.identity_domain.capacity(),
                    actor.analytics_actor_id.capacity(),
                ]);
            }
            sum(&charges)
        }
        impl Frozen2 {
            fn decode(wire: &[u8], facts: &WorldFacts) -> Result<Self, Failure> {
                facts.check()?;
                preflight2(wire, Schema2::Envelope, 1)?;
                let semantic = Envelope::decode(wire).map_err(|_| Failure::Facts)?;
                let kind = validate2(&semantic, facts)?;
                bounded(wire.len(), upper(kind)?.envelope)?;
                let value = Self {
                    semantic,
                    wire: wire.to_vec(),
                };
                bounded(dynamic2(&value)?, upper(kind)?.retained_dynamic)?;
                Ok(value)
            }
            fn check(&self, facts: &WorldFacts) -> Result<Kind, Failure> {
                preflight2(&self.wire, Schema2::Envelope, 1)?;
                let decoded = Envelope::decode(self.wire.as_slice()).map_err(|_| Failure::Facts)?;
                if decoded != self.semantic {
                    return Err(Failure::Conflict);
                }
                let kind = validate2(&self.semantic, facts)?;
                bounded(dynamic2(self)?, upper(kind)?.retained_dynamic)?;
                Ok(kind)
            }
        }
        #[derive(Clone, Copy)]
        struct Budget2 {
            payload: usize,
            envelope: usize,
            retained: usize,
            retry_work: u64,
        }
        fn budget2() -> Result<Budget2, Failure> {
            Ok(Budget2 {
                payload: upper(Kind::Transfer)?.payload,
                envelope: upper(Kind::Transfer)?.envelope,
                retained: sum(&[
                    size_of::<Model2>(),
                    upper(Kind::Mint)?.retained_dynamic,
                    upper(Kind::Transfer)?.retained_dynamic,
                ])?,
                retry_work: 6,
            })
        }
        fn freeze2(
            kind: Kind,
            position: [i32; 3],
            facts: &WorldFacts,
            budget: Budget2,
        ) -> Result<Frozen2, Failure> {
            // All caller facts and finite reservations precede private carriers.
            facts.position(position)?;
            let maximum = upper(kind)?;
            bounded(maximum.payload, budget.payload)?;
            bounded(maximum.envelope, budget.envelope)?;
            bounded(budget2()?.retained, budget.retained)?;
            let payload = encode(
                &make_payload(kind, position, facts)?,
                maximum.payload,
                budget.payload,
            )?;
            let transfer = kind == Kind::Transfer;
            let value = Envelope {
                envelope_revision: 1,
                event_id: uuid_fixture(if transfer { 21 } else { 20 }).to_vec(),
                event_type_id: 0,
                event_schema_revision: 2,
                durability_class: 2,
                privacy_class: 3,
                retention_profile_id: "offline_item_unaccepted".into(),
                occurred_at_unix_ms: 1,
                world_id: Some(facts.bindings.world.to_vec()),
                channel_id: Some(facts.bindings.channel.to_vec()),
                runtime_order: Some(RuntimeOrder {
                    scope_ownership_generation: facts.bindings.generation,
                    runtime_execution_ordinal: u64::from(kind.tag()),
                }),
                transaction_event: Some(Membership {
                    transaction_id: uuid_fixture(if transfer { 23 } else { 22 }).to_vec(),
                    ordinal: 1,
                    count: 1,
                }),
                game_session_id: transfer.then(|| facts.bindings.session.to_vec()),
                connection_generation: transfer.then_some(facts.bindings.generation),
                command_id: transfer.then_some(2),
                causation: transfer.then(|| Causation {
                    cause: Some(Cause::Command(Command {
                        game_session_id: facts.bindings.session.to_vec(),
                        command_id: 2,
                    })),
                }),
                ruleset_revision: Some("fixture2".into()),
                content_revision: Some(facts.revision.into()),
                server_build_id: "offline".into(),
                payload_sha256: Sha256::digest(&payload).to_vec(),
                payload,
                ..Envelope::default()
            };
            validate2(&value, facts)?;
            let wire = encode(&value, maximum.envelope, budget.envelope)?;
            preflight2(&wire, Schema2::Envelope, 1)?;
            let frozen = Frozen2 {
                semantic: value,
                wire,
            };
            bounded(dynamic2(&frozen)?, maximum.retained_dynamic)?;
            Ok(frozen)
        }
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        // Identity/world/quantity/lifecycle are the closed fixed synthetic tuple
        // validated above; Ground additionally retains its actual signed position.
        enum Custody2 {
            Absent,
            Ground([i32; 3]),
            Inventory,
        }
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        enum Outcome2 {
            KnownNoncommit,
            CommittedResponseLost,
            Acknowledged,
        }
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct Record2 {
            event: Frozen2,
            committed: bool,
            acknowledged: bool,
            retry_work: u64,
        }
        #[derive(Clone, Debug, PartialEq, Eq)]
        struct Model2 {
            custody: Custody2,
            records: [Option<Record2>; 2],
        }
        impl Model2 {
            fn new() -> Self {
                Self {
                    custody: Custody2::Absent,
                    records: [None, None],
                }
            }
            fn attempt(
                &mut self,
                event: Frozen2,
                facts: &WorldFacts,
                budget: Budget2,
                outcome: Outcome2,
            ) -> Result<(), Failure> {
                let kind = event.check(facts)?;
                let slot = usize::from(kind == Kind::Transfer);
                if let Some(record) = &mut self.records[slot] {
                    if record.event != event {
                        return Err(Failure::Conflict);
                    }
                    let work = record.retry_work.checked_add(3).ok_or(Failure::Overflow)?;
                    if work > budget.retry_work {
                        return Err(Failure::Budget("retry work"));
                    }
                    // Ambiguous immutable record cannot fall back to remint.
                    if record.committed && !record.acknowledged {
                        return Err(Failure::Conflict);
                    }
                    if record.committed {
                        record.retry_work = work;
                        return Ok(());
                    }
                    // Known noncommit: same frozen bytes, never new identities.
                    if outcome == Outcome2::KnownNoncommit {
                        record.retry_work = work;
                        return Ok(());
                    }
                }
                let payload = SignedPayload::decode(event.semantic.payload.as_slice())
                    .map_err(|_| Failure::Facts)?;
                let ground = match payload.operation.as_ref().ok_or(Failure::Facts)? {
                    SignedOperation::Mint(mint) => mint.destination.as_ref(),
                    SignedOperation::Transfer(transfer) => transfer.source.as_ref(),
                }
                .ok_or(Failure::Facts)?;
                let position = [ground.x, ground.y, ground.floor];
                let expected = if kind == Kind::Mint {
                    Custody2::Absent
                } else {
                    Custody2::Ground(position)
                };
                if self.custody != expected
                    || kind == Kind::Transfer
                        && !self.records[0]
                            .as_ref()
                            .is_some_and(|record| record.committed && record.acknowledged)
                {
                    return Err(Failure::Custody);
                }
                if let Some(SignedOperation::Transfer(transfer)) = payload.operation.as_ref() {
                    let minted = self.records[0].as_ref().ok_or(Failure::Custody)?;
                    preflight2(&minted.event.semantic.payload, Schema2::Payload, 1)?;
                    let minted_payload =
                        SignedPayload::decode(minted.event.semantic.payload.as_slice())
                            .map_err(|_| Failure::Facts)?;
                    let Some(SignedOperation::Mint(mint)) = minted_payload.operation.as_ref()
                    else {
                        return Err(Failure::Custody);
                    };
                    // Current facts validate this attempted event, but cannot
                    // replace the immutable source tuple created by prior MINT.
                    if mint.after != transfer.before
                        || mint.destination != transfer.source
                        || minted.event.semantic.content_revision != event.semantic.content_revision
                    {
                        return Err(Failure::Custody);
                    }
                }
                let maximum = upper(kind)?;
                bounded(maximum.payload, budget.payload)?;
                bounded(maximum.envelope, budget.envelope)?;
                bounded(budget2()?.retained, budget.retained)?;
                let committed = outcome != Outcome2::KnownNoncommit;
                let work = self.records[slot].as_ref().map_or(Ok(0), |record| {
                    record.retry_work.checked_add(3).ok_or(Failure::Overflow)
                })?;
                let record = Record2 {
                    event,
                    committed,
                    acknowledged: outcome == Outcome2::Acknowledged,
                    retry_work: work,
                };
                // Single fixture publication after the complete budget decision.
                self.records[slot] = Some(record);
                if committed {
                    self.custody = if kind == Kind::Mint {
                        Custody2::Ground(position)
                    } else {
                        Custody2::Inventory
                    };
                }
                Ok(())
            }
            fn reconcile(
                &mut self,
                kind: Kind,
                observed_commit: Option<bool>,
                budget: Budget2,
            ) -> Result<(), Failure> {
                let record = self.records[usize::from(kind == Kind::Transfer)]
                    .as_mut()
                    .ok_or(Failure::Conflict)?;
                let work = record.retry_work.checked_add(3).ok_or(Failure::Overflow)?;
                if work > budget.retry_work {
                    return Err(Failure::Budget("retry work"));
                }
                if observed_commit.is_some_and(|committed| committed != record.committed) {
                    return Err(Failure::Conflict);
                }
                record.retry_work = work;
                if observed_commit == Some(true) {
                    record.acknowledged = true;
                }
                Ok(())
            }
        }

        // Canonically encoded width witnesses, deliberately NOT semantically
        // admitted transactions. Full optional superset may violate scope rules.
        fn algebraic_vector(kind: Kind) -> Result<Envelope, Failure> {
            let mut payload = make_payload(kind, [100, -100, 7], &world_fixture())?;
            let (states, ground, inventory) =
                match payload.operation.as_mut().ok_or(Failure::Facts)? {
                    SignedOperation::Mint(mint) => (
                        vec![mint.after.as_mut().ok_or(Failure::Facts)?],
                        mint.destination.as_mut().ok_or(Failure::Facts)?,
                        None,
                    ),
                    SignedOperation::Transfer(transfer) => (
                        vec![
                            transfer.before.as_mut().ok_or(Failure::Facts)?,
                            transfer.after.as_mut().ok_or(Failure::Facts)?,
                        ],
                        transfer.source.as_mut().ok_or(Failure::Facts)?,
                        transfer.destination.as_mut(),
                    ),
                };
            for state in states {
                state.definition_key = u32::MAX;
                state.definition_revision = u32::MAX;
                state.quantity = u32::MAX;
            }
            ground.corpse_revision = u64::MAX;
            ground.x = i32::MIN;
            ground.y = i32::MAX;
            ground.floor = i32::from(i16::MIN);
            if let Some(root) = inventory {
                root.session_generation = u64::MAX;
            }
            let payload = encode(&payload, upper(kind)?.payload, PAYLOAD_MAX)?;
            let mut value = freeze2(kind, [100, -100, 7], &world_fixture(), budget2()?)?.semantic;
            value.payload_sha256 = Sha256::digest(&payload).to_vec();
            value.payload = payload;
            value.retention_profile_id = "r".repeat(STRING_MAX);
            value.occurred_at_unix_ms = i64::MIN;
            value.instance_id = Some(uuid_fixture(30).to_vec());
            value.node_id = Some(uuid_fixture(31).to_vec());
            value.game_session_id = Some(uuid_fixture(7).to_vec());
            value.connection_generation = Some(u64::MAX);
            value.runtime_order = Some(RuntimeOrder {
                scope_ownership_generation: u64::MAX,
                runtime_execution_ordinal: u64::MAX,
            });
            value.command_id = Some(u64::MAX);
            value.operation_id = Some(uuid_fixture(32).to_vec());
            value.correlation_id = Some(uuid_fixture(33).to_vec());
            value.causation = Some(Causation {
                cause: Some(Cause::Command(Command {
                    game_session_id: uuid_fixture(7).to_vec(),
                    command_id: u64::MAX,
                })),
            });
            value.analytics_actor = Some(Actor {
                identity_domain: "a".repeat(STRING_MAX),
                identity_epoch: u64::MAX,
                analytics_actor_id: uuid_fixture(34).to_vec(),
            });
            value.protocol_major = Some(u32::MAX);
            value.ruleset_revision = Some("r".repeat(STRING_MAX));
            value.content_revision = Some("c".repeat(STRING_MAX));
            value.server_build_id = "b".repeat(STRING_MAX);
            Ok(value)
        }
        pub(crate) fn report(reverse: bool) -> Result<Value, Failure> {
            let facts = world_fixture();
            let budget = budget2()?;
            let mut model = Model2::new();
            for kind in [Kind::Mint, Kind::Transfer] {
                let event = freeze2(kind, [100, -100, 7], &facts, budget)?;
                // A distinct duplicate MINT exercises the acknowledged terminal
                // path without creating another item or changing its frozen bytes.
                if kind == Kind::Mint {
                    model.attempt(event.clone(), &facts, budget, Outcome2::KnownNoncommit)?;
                    model.attempt(event.clone(), &facts, budget, Outcome2::Acknowledged)?;
                    model.attempt(event, &facts, budget, Outcome2::Acknowledged)?;
                    continue;
                }
                model.attempt(event, &facts, budget, Outcome2::CommittedResponseLost)?;
                if kind == Kind::Transfer {
                    model.reconcile(kind, None, budget)?;
                }
                model.reconcile(kind, Some(true), budget)?;
            }
            let mut operations = Vec::new();
            for (slot, record) in model.records.iter().enumerate() {
                let record = record.as_ref().ok_or(Failure::Conflict)?;
                let kind = if slot == 0 {
                    Kind::Mint
                } else {
                    Kind::Transfer
                };
                let bound = upper(kind)?;
                let algebra = algebraic_vector(kind)?;
                let wire = encode(&algebra, bound.envelope, ENVELOPE_MAX)?;
                preflight2(&wire, Schema2::Envelope, 1)?;
                let actual = &record.event;
                if Frozen2::decode(&actual.wire, &facts)? != *actual {
                    return Err(Failure::Conflict);
                }
                operations.push(json!({
                    "operation":kind.label(),"payload_encoded_bytes":actual.semantic.payload.len(),"envelope_encoded_bytes":actual.wire.len(),"aggregate":{"count":1,"ordinal":1,"bytes":actual.wire.len()},
                    "event_id":hex(&actual.semantic.event_id),"transaction_id":hex(&actual.semantic.transaction_event.as_ref().ok_or(Failure::Facts)?.transaction_id),"payload_sha256":hex(&actual.semantic.payload_sha256),"exact_payload_hex":hex(&actual.semantic.payload),"exact_envelope_hex":hex(&actual.wire),
                    "representation_superset_upper":{"item_state":bound.state,"signed_ground":bound.ground,"typed_inventory":bound.inventory,"provenance":bound.provenance,"operation_body":bound.inner,"payload":bound.payload,"envelope_nonpayload_fields":bound.envelope_overhead,"envelope_and_complete_aggregate":bound.envelope,"retained_dynamic_capacity_reservation":bound.retained_dynamic},
                    "algebraic_width_witness":{"semantically_admitted":false,"payload_bytes":algebra.payload.len(),"envelope_bytes":wire.len(),"payload_sha256":hex(&algebra.payload_sha256),"envelope_sha256":hex(&Sha256::digest(&wire))},
                    "actual_retained":{"frozen_inline_bytes":size_of::<Frozen2>(),"wire_len":actual.wire.len(),"wire_capacity":actual.wire.capacity(),"semantic_payload_len":actual.semantic.payload.len(),"semantic_payload_capacity":actual.semantic.payload.capacity(),"all_dynamic_vector_string_capacities":dynamic2(actual)?,"inline_plus_dynamic":sum(&[size_of::<Frozen2>(),dynamic2(actual)?])?},"retry_work_units":record.retry_work
                }));
            }
            if reverse {
                operations.reverse();
            }
            operations.sort_by_key(|value| if value["operation"] == "MINT" { 0 } else { 1 });
            let dynamic = model
                .records
                .iter()
                .flatten()
                .try_fold(0_usize, |total, record| {
                    sum(&[total, dynamic2(&record.event)?])
                })?;
            Ok(json!({
                "classification":"UNREGISTERED_OFFLINE_REVISION_2_EVIDENCE_CANDIDATE","interpretation_revision":2,"source_base_sha":"56e5c8a39bf2d899cbbc24735d2d7440d4ecaec1","allocation":"5856736427","custody_refinement":"5856742846",
                "source_blobs_at_base":{"example":"3e5400fa139854d986c6d8dc9240397f01a90ad3","historical_json":"0c63b65809bbd7b0541dc289d2aed0cfd6cae95d","historical_markdown":"022463e01180f408908b6d321894fc0234a62a09"},
                "historical_v1":"UNCHANGED_SCHEMA_GOLDENS_AND_MEASUREMENTS","bound_scope":"finite canonical serialized representation superset, not necessarily tight reachable shape; not a gameplay/production maximum","world":{"id":hex(&facts.bindings.world),"revision":facts.revision,"x_half_open":facts.x,"y_half_open":facts.y,"floors":facts.floors,"independent_current_facts":true,"production_activation":false},
                "fixture_root":"SyntheticDirectRoot enum 1; NOT revision1 slot0 or a product slot index","quantity_and_content":"private fixture key1 revision1 quantity1; production Content/stack semantics UNKNOWN",
                "closed_wire_preflight":"borrowed finite graph before prost allocations; exact UUID/hash widths, bounded ASCII text, canonical minimal key/length/scalar varints, u32 and checked-i16 floor widths; unknown/duplicate/out-of-order fields, unknown enums and crossrevision reject",
                "operations":operations,"retained_model":{"fixed_inline_including_two_slots":size_of::<Model2>(),"actual_dynamic_capacity_bytes":dynamic,"actual_inline_plus_dynamic":sum(&[size_of::<Model2>(),dynamic])?,"injected_reservation":budget.retained,"records":2,"custody":"CharacterInventory/synthetic-direct-root","authoritative_items":1,"immediate_locations":1},
                "work":{"logical_shape_units_derived_not_execution_measurement":{"plan":{"MINT":2,"TRANSFER":3},"apply":{"MINT":2,"TRANSFER":3}},"retry_per_call":3,"injected_retry_work_per_record":budget.retry_work,"scope":"one logical unit per participant/custody effect, publication receipt, or retry inspect/compare/disposition; excludes parser/hash/allocation/string scans/World profile checks, CPU instructions, latency, RSS, SQL and production retry ceiling"},
                "rows":{"DUR03-RL-01":"one item, qty1 positive fixture; u32 width is only conservative representation bound","DUR03-RL-02":"MINT one establish; TRANSFER remove+establish; single tuple publication","DUR03-RL-03":"not applicable/0; no value/account surface","DUR03-RL-04":"not applicable/0; no transform surface","DUR03-RL-05":"closed synthetic direct-root only; no container expansion","DUR03-RL-06":"physically represented fixed two-record model plus owned vector/string capacities","DUR03-RL-07":"PROVEN candidate-only finite payload/envelope/complete one-member aggregate upper; production EVIDENCE_GAP unchanged","DUR03-RL-08":"in-memory immutable frozen ambiguity/retry/reconcile only; restart/DB EVIDENCE_GAP"},
                "not_proven":["production event/profile/resource registration","Content quantity/stack/root legality","runtime or PostgreSQL atomicity/fencing/restart","allocator metadata, transient decode/copies or peak RSS","Tibia floor adapter or Reference parity"]
            }))
        }

        #[cfg(test)]
        mod tests2 {
            use super::*;
            #[test]
            fn independent_literal_signed_fixture_goldens() -> Result<(), Failure> {
                // Independently hand-encoded by the read-only oracle: field keys,
                // fixed IDs, zigzag 200/199/14 and nested length prefixes.
                let literals = [
                    (
                        Kind::Mint,
                        "08021298010a2c0a1000000000000170008000000000000003121000000000000170008000000000000001180120012801300112400a10000000000001700080000000000000011210000000000001700080000000000000061a100000000000017000800000000000000c200128c80130c701380e1a26080112100000000000017000800000000000000c1a100000000000017000800000000000000a",
                        157,
                    ),
                    (
                        Kind::Transfer,
                        "08021ade010a2c0a10000000000001700080000000000000031210000000000001700080000000000000011801200128013001122c0a100000000000017000800000000000000312100000000000017000800000000000000118012001280130011a400a10000000000001700080000000000000011210000000000001700080000000000000061a100000000000017000800000000000000c200128c80130c701380e22160a1000000000000170008000000000000002100118012a26080112100000000000017000800000000000000c1a100000000000017000800000000000000b",
                        227,
                    ),
                ];
                for (kind, literal, size) in literals {
                    let event = freeze2(kind, [100, -100, 7], &world_fixture(), budget2()?)?;
                    assert_eq!(hex(&event.semantic.payload), literal);
                    assert_eq!(event.semantic.payload.len(), size);
                    preflight2(&event.semantic.payload, Schema2::Payload, 1)?;
                    let decoded = SignedPayload::decode(event.semantic.payload.as_slice())
                        .map_err(|_| Failure::Facts)?;
                    assert_eq!(decoded.encode_to_vec(), event.semantic.payload);
                }
                Ok(())
            }
            #[test]
            fn conservative_algebra_matches_canonical_width_witness_not_admission()
            -> Result<(), Failure> {
                for (kind, payload, envelope) in
                    [(Kind::Mint, 186, 1194), (Kind::Transfer, 277, 1285)]
                {
                    let bound = upper(kind)?;
                    assert_eq!(
                        (
                            bound.state,
                            bound.ground,
                            bound.inventory,
                            bound.provenance,
                            bound.payload,
                            bound.envelope_overhead,
                            bound.envelope
                        ),
                        (56, 81, 31, 38, payload, 1004, envelope)
                    );
                    let algebra = algebraic_vector(kind)?;
                    let wire = encode(&algebra, bound.envelope, ENVELOPE_MAX)?;
                    assert_eq!((algebra.payload.len(), wire.len()), (payload, envelope));
                    preflight2(&wire, Schema2::Envelope, 1)?;
                    assert!(validate2(&algebra, &world_fixture()).is_err());
                }
                Ok(())
            }
            #[test]
            fn signed_world_extremes_and_independent_profile_rejections() -> Result<(), Failure> {
                let mut facts = world_fixture();
                facts.x = [i64::from(i32::MIN), i64::from(i32::MAX) + 1];
                facts.y = facts.x;
                facts.floors = &[i16::MIN, 0, i16::MAX];
                for position in [
                    [i32::MIN, i32::MAX, i32::from(i16::MIN)],
                    [i32::MAX, i32::MIN, i32::from(i16::MAX)],
                    [0, 0, 0],
                ] {
                    let event = freeze2(Kind::Mint, position, &facts, budget2()?)?;
                    Frozen2::decode(&event.wire, &facts)?;
                }
                for position in [
                    [0, 0, i32::from(i16::MAX) + 1],
                    [0, 0, i32::from(i16::MIN) - 1],
                    [0, 0, 1],
                ] {
                    assert!(freeze2(Kind::Mint, position, &facts, budget2()?).is_err());
                }
                let event = freeze2(Kind::Mint, [100, -100, 7], &world_fixture(), budget2()?)?;
                for x in [
                    [-200, 100],
                    [101, 200],
                    [2, 2],
                    [2, 1],
                    [i64::from(i32::MIN) - 1, 200],
                    [-200, i64::from(i32::MAX) + 2],
                ] {
                    let mut wrong = world_fixture();
                    wrong.x = x;
                    assert!(Frozen2::decode(&event.wire, &wrong).is_err());
                }
                for floors in [&[][..], &[7, 7][..], &[7, 0][..], &[0, 1][..]] {
                    let mut wrong = world_fixture();
                    wrong.floors = floors;
                    assert!(Frozen2::decode(&event.wire, &wrong).is_err());
                }
                let mut wrong = world_fixture();
                wrong.revision = "synthetic_world_r3";
                assert!(Frozen2::decode(&event.wire, &wrong).is_err());
                let mut wrong = world_fixture();
                wrong.bindings.world = uuid_fixture(40);
                assert!(Frozen2::decode(&event.wire, &wrong).is_err());
                let mut wrong = world_fixture();
                wrong.bindings.definition_revision = 0;
                assert!(make_payload(Kind::Mint, [100, -100, 7], &wrong).is_err());
                Ok(())
            }
            #[test]
            fn canonical_parser_rejects_unknown_duplicate_varints_and_overwidth_before_decode()
            -> Result<(), Failure> {
                // Standalone scalar carriers exercise the borrowed grammar only.
                for bytes in [
                    &[0x08, 0x82, 0][..],
                    &[0x88, 0, 2][..],
                    &[0x08, 2, 0x08, 2][..],
                    &[0x08, 2, 0x20, 1][..],
                    &[0x08, 2, 0x12, 0x80, 0][..],
                ] {
                    assert!(preflight2(bytes, Schema2::Payload, 1).is_err());
                }
                let event = freeze2(Kind::Mint, [100, -100, 7], &world_fixture(), budget2()?)?;
                let mut unknown = event.wire.clone();
                unknown.extend([0xe0, 1, 1]);
                assert!(Frozen2::decode(&unknown, &world_fixture()).is_err());
                let mut overlarge = event.wire.clone();
                overlarge.resize(upper(Kind::Transfer)?.envelope + 1, 0);
                assert!(Frozen2::decode(&overlarge, &world_fixture()).is_err());
                let mut cursor = 0;
                assert_eq!(
                    read_varint(
                        &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 1],
                        &mut cursor
                    )?,
                    u64::MAX
                );
                for bytes in [
                    &[0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 2][..],
                    &[0x80][..],
                    &[0x80, 0][..],
                ] {
                    let mut cursor = 0;
                    assert!(read_varint(bytes, &mut cursor).is_err());
                }
                let mut algebra = algebraic_vector(Kind::Transfer)?;
                for length in [1, 127, 128] {
                    algebra.server_build_id = "b".repeat(length);
                    let wire = encode(&algebra, upper(Kind::Transfer)?.envelope, ENVELOPE_MAX)?;
                    preflight2(&wire, Schema2::Envelope, 1)?;
                }
                algebra.server_build_id = "b".repeat(129);
                let wire = algebra.encode_to_vec();
                assert!(preflight2(&wire, Schema2::Envelope, 1).is_err());
                assert_eq!(
                    (
                        varint_width(127),
                        varint_width(128),
                        varint_width(16383),
                        varint_width(16384)
                    ),
                    (1, 2, 2, 3)
                );
                assert_eq!(delimited(1, 127)?, 129);
                assert_eq!(delimited(1, 128)?, 131);
                assert_eq!(delimited(26, usize::MAX), Err(Failure::Overflow));
                assert_eq!(sum(&[usize::MAX, 1]), Err(Failure::Overflow));
                Ok(())
            }
            #[test]
            fn crossrevision_and_unknown_root_never_reinterpret() -> Result<(), Failure> {
                let event = freeze2(Kind::Transfer, [100, -100, 7], &world_fixture(), budget2()?)?;
                assert!(FrozenEvent::decode(&event.wire, &current_fixture()).is_err());
                let (mint, _, _) = super::super::super::fixture().map_err(|_| Failure::Facts)?;
                let plan = super::super::super::Model::default()
                    .plan(mint, super::super::super::audit_probe())?;
                let old = freeze(&plan)?;
                assert!(Frozen2::decode(&old.wire, &world_fixture()).is_err());
                let mut payload = make_payload(Kind::Transfer, [100, -100, 7], &world_fixture())?;
                for root in [0, 2, i32::MAX] {
                    if let Some(SignedOperation::Transfer(transfer)) = &mut payload.operation {
                        transfer
                            .destination
                            .as_mut()
                            .ok_or(Failure::Facts)?
                            .fixture_root_position = root;
                    }
                    assert!(preflight2(&payload.encode_to_vec(), Schema2::Payload, 1).is_err());
                }
                Ok(())
            }
            #[test]
            fn one_invariant_per_semantic_negative_and_actual_source_tuple() -> Result<(), Failure>
            {
                let facts = world_fixture();
                let budget = budget2()?;
                let event = freeze2(Kind::Transfer, [100, -100, 7], &facts, budget)?;
                let mutations: &[fn(&mut Envelope)] = &[
                    |e| {
                        e.transaction_event
                            .as_mut()
                            .expect("fixture membership")
                            .count = 2
                    },
                    |e| {
                        e.transaction_event
                            .as_mut()
                            .expect("fixture membership")
                            .ordinal = 2
                    },
                    |e| e.world_id = Some(uuid_fixture(40).to_vec()),
                    |e| e.channel_id = Some(uuid_fixture(40).to_vec()),
                    |e| e.game_session_id = Some(uuid_fixture(40).to_vec()),
                    |e| e.connection_generation = Some(2),
                    |e| {
                        e.runtime_order
                            .as_mut()
                            .expect("fixture order")
                            .scope_ownership_generation = 2
                    },
                    |e| e.content_revision = Some("synthetic_world_r3".into()),
                    |e| e.event_schema_revision = 1,
                    |e| e.payload_sha256[0] ^= 1,
                    |e| e.command_id = Some(3),
                ];
                for mutate in mutations {
                    let mut wrong = event.semantic.clone();
                    mutate(&mut wrong);
                    assert!(Frozen2::decode(&wrong.encode_to_vec(), &facts).is_err());
                }
                // Valid syntactic carrier, wrong quantity/source/floor or omitted
                // mandatory state. Rehashing cannot turn these into admission.
                for changed in 0..4 {
                    let mut payload = make_payload(Kind::Transfer, [100, -100, 7], &facts)?;
                    if let Some(SignedOperation::Transfer(transfer)) = &mut payload.operation {
                        match changed {
                            0 => transfer.after.as_mut().ok_or(Failure::Facts)?.quantity = 2,
                            1 => {
                                transfer.cause.as_mut().ok_or(Failure::Facts)?.cause_id =
                                    uuid_fixture(40).to_vec()
                            }
                            2 => transfer.source.as_mut().ok_or(Failure::Facts)?.floor = 32768,
                            _ => transfer.before = None,
                        }
                    }
                    let mut wrong = event.semantic.clone();
                    wrong.payload = payload.encode_to_vec();
                    wrong.payload_sha256 = Sha256::digest(&wrong.payload).to_vec();
                    assert!(Frozen2::decode(&wrong.encode_to_vec(), &facts).is_err());
                }
                let mut model = Model2::new();
                model.attempt(
                    freeze2(Kind::Mint, [100, -100, 7], &facts, budget)?,
                    &facts,
                    budget,
                    Outcome2::Acknowledged,
                )?;
                let unchanged = model.clone();
                // World-admitted coordinate but not the retained source tuple.
                assert!(
                    model
                        .attempt(
                            freeze2(Kind::Transfer, [101, -100, 7], &facts, budget)?,
                            &facts,
                            budget,
                            Outcome2::Acknowledged
                        )
                        .is_err()
                );
                assert_eq!(model, unchanged);
                // A new valid payload under the same EventId/TransactionId is a
                // conflict, not a second mint or a source resurrection.
                assert!(
                    model
                        .attempt(
                            freeze2(Kind::Mint, [101, -100, 7], &facts, budget)?,
                            &facts,
                            budget,
                            Outcome2::Acknowledged
                        )
                        .is_err()
                );
                assert_eq!(model, unchanged);
                model.records[0].as_mut().ok_or(Failure::Facts)?.retry_work = u64::MAX;
                let unchanged = model.clone();
                assert_eq!(
                    model.reconcile(
                        Kind::Mint,
                        None,
                        Budget2 {
                            retry_work: u64::MAX,
                            ..budget
                        }
                    ),
                    Err(Failure::Overflow)
                );
                assert_eq!(model, unchanged);
                Ok(())
            }
            #[test]
            fn otherwise_complete_fields_cannot_hide_overwidth_or_default_type()
            -> Result<(), Failure> {
                let payload = make_payload(Kind::Mint, [100, -100, 7], &world_fixture())?;
                let Some(SignedOperation::Mint(mint)) = payload.operation else {
                    return Err(Failure::Facts);
                };
                let mut state = mint.after.ok_or(Failure::Facts)?.encode_to_vec();
                state.truncate(state.len() - 4);
                // quantity = 2^32 (five minimal bytes), then lifecycle=1.
                state.extend([0x28, 0x80, 0x80, 0x80, 0x80, 0x10, 0x30, 1]);
                assert!(preflight2(&state, Schema2::State, 1).is_err());
                let event = freeze2(Kind::Mint, [100, -100, 7], &world_fixture(), budget2()?)?;
                let mut default_type = event.wire.clone();
                // After revision and EventId, tag3 default0 is explicitly present.
                default_type.splice(20..20, [0x18, 0]);
                assert!(preflight2(&default_type, Schema2::Envelope, 1).is_err());
                Ok(())
            }
            #[test]
            fn cross_transaction_source_fact_substitution_rejects_before_publication()
            -> Result<(), Failure> {
                let original = world_fixture();
                let budget = budget2()?;
                let mutations: &[fn(&mut WorldFacts)] = &[
                    |f| f.bindings.world = uuid_fixture(24),
                    |f| f.bindings.definition_revision = 2,
                    |f| f.bindings.channel = uuid_fixture(24),
                    |f| f.bindings.corpse = uuid_fixture(24),
                    |f| f.bindings.corpse_revision = 2,
                    |f| f.revision = "synthetic_world_r3",
                ];
                for (case, mutate) in mutations.iter().enumerate() {
                    let mut model = Model2::new();
                    model.attempt(
                        freeze2(Kind::Mint, [100, -100, 7], &original, budget)?,
                        &original,
                        budget,
                        Outcome2::Acknowledged,
                    )?;
                    let mut changed = original.clone();
                    mutate(&mut changed);
                    let transfer = freeze2(Kind::Transfer, [100, -100, 7], &changed, budget)?;
                    // Counterfactual is otherwise internally valid/current under
                    // independently supplied facts, changing only one binding.
                    assert_eq!(transfer.check(&changed)?, Kind::Transfer);
                    let unchanged = model.clone();
                    assert!(
                        model
                            .attempt(transfer, &changed, budget, Outcome2::Acknowledged)
                            .is_err(),
                        "source substitution case {case}"
                    );
                    assert_eq!(model, unchanged);
                }
                Ok(())
            }
            #[test]
            fn proposed_retained_capacity_is_charged_before_publication() -> Result<(), Failure> {
                let facts = world_fixture();
                let budget = budget2()?;
                for capacity_case in 0..4 {
                    let mut mint = freeze2(Kind::Mint, [100, -100, 7], &facts, budget)?;
                    match capacity_case {
                        0 => mint.semantic.server_build_id.reserve(budget.retained),
                        1 => mint.wire.reserve(budget.retained),
                        2 => mint.semantic.payload.reserve(budget.retained),
                        _ => mint
                            .semantic
                            .transaction_event
                            .as_mut()
                            .ok_or(Failure::Facts)?
                            .transaction_id
                            .reserve(budget.retained),
                    }
                    assert_eq!(mint.semantic.server_build_id, "offline");
                    let mut model = Model2::new();
                    let unchanged = model.clone();
                    assert!(
                        model
                            .attempt(mint, &facts, budget, Outcome2::Acknowledged)
                            .is_err(),
                        "capacity case {capacity_case}"
                    );
                    assert_eq!(model, unchanged);
                }
                Ok(())
            }
            #[test]
            fn budget_failure_before_publication_and_frozen_ambiguity() -> Result<(), Failure> {
                let facts = world_fixture();
                let budget = budget2()?;
                let mint = freeze2(Kind::Mint, [100, -100, 7], &facts, budget)?;
                for lowered in [
                    Budget2 {
                        payload: 185,
                        ..budget
                    },
                    Budget2 {
                        envelope: 1193,
                        ..budget
                    },
                    Budget2 {
                        retained: budget.retained - 1,
                        ..budget
                    },
                ] {
                    assert!(freeze2(Kind::Mint, [100, -100, 7], &facts, lowered).is_err());
                }
                let mut model = Model2::new();
                let unchanged = model.clone();
                assert!(
                    model
                        .attempt(
                            mint.clone(),
                            &facts,
                            Budget2 {
                                retained: budget.retained - 1,
                                ..budget
                            },
                            Outcome2::Acknowledged
                        )
                        .is_err()
                );
                assert_eq!(model, unchanged);
                model.attempt(mint.clone(), &facts, budget, Outcome2::KnownNoncommit)?;
                assert_eq!(model.custody, Custody2::Absent);
                model.attempt(
                    mint.clone(),
                    &facts,
                    budget,
                    Outcome2::CommittedResponseLost,
                )?;
                assert_eq!(model.custody, Custody2::Ground([100, -100, 7]));
                let unchanged = model.clone();
                assert!(
                    model
                        .attempt(mint.clone(), &facts, budget, Outcome2::Acknowledged)
                        .is_err()
                );
                assert_eq!(model, unchanged);
                model.reconcile(Kind::Mint, Some(true), budget)?;
                let transfer = freeze2(Kind::Transfer, [100, -100, 7], &facts, budget)?;
                model.attempt(
                    transfer.clone(),
                    &facts,
                    budget,
                    Outcome2::CommittedResponseLost,
                )?;
                let unchanged = model.clone();
                assert!(
                    model
                        .reconcile(Kind::Transfer, Some(false), budget)
                        .is_err()
                );
                assert_eq!(model, unchanged);
                model.reconcile(Kind::Transfer, None, budget)?;
                assert_eq!(model.custody, Custody2::Inventory);
                assert_eq!(
                    model.records[1].as_ref().ok_or(Failure::Facts)?.event,
                    transfer
                );
                model.reconcile(Kind::Transfer, Some(true), budget)?;
                let unchanged = model.clone();
                assert!(model.reconcile(Kind::Transfer, None, budget).is_err());
                assert_eq!(model, unchanged);
                let mut conflicting = mint;
                conflicting.semantic.event_id = uuid_fixture(41).to_vec();
                assert!(
                    model
                        .attempt(conflicting, &facts, budget, Outcome2::Acknowledged)
                        .is_err()
                );
                assert_eq!(model.custody, Custody2::Inventory);
                Ok(())
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::{
            Attempt, Model, Observation, Reconcile, Request, ResultState, audit_probe,
            evidence_limits, fixture,
        };
        use super::*;
        use std::error::Error;

        // Literal octets hand-derived from the field/tag/wire/varint/length
        // table in the retained Markdown. Neither prost nor encode() made them.
        const MINT_PAYLOAD: &str = concat!(
            "08011296010a2c0a10000000000001700080000000000000031210000000000001700080000000000000011801200128",
            "013001123e0a10000000000001700080000000000000011210000000000001700080000000000000061a100000000000",
            "017000800000000000000c20012864306438071a26080112100000000000017000800000000000000c1a100000000000",
            "017000800000000000000a"
        );
        const TRANSFER_PAYLOAD: &str = concat!(
            "08011ada010a2c0a10000000000001700080000000000000031210000000000001700080000000000000011801200128",
            "013001122c0a100000000000017000800000000000000312100000000000017000800000000000000118012001280130",
            "011a3e0a10000000000001700080000000000000011210000000000001700080000000000000061a1000000000000170",
            "00800000000000000c200128643064380722140a100000000000017000800000000000000210012a2608011210000000",
            "0000017000800000000000000c1a100000000000017000800000000000000b"
        );
        const MINT_ENVELOPE: &str = concat!(
            "08011210000000000001700080000000000000082001280230033a176f66666c696e655f6974656d5f756e6163636570",
            "74656440014a10000000000001700080000000000000015210000000000001700080000000000000067a040801100192",
            "01160a100000000000017000800000000000000410011801ba01086669787475726531c201086669787475726531ca01",
            "076f66666c696e65d2019b0108011296010a2c0a10000000000001700080000000000000031210000000000001700080",
            "000000000000011801200128013001123e0a100000000000017000800000000000000112100000000000017000800000",
            "00000000061a100000000000017000800000000000000c20012864306438071a26080112100000000000017000800000",
            "000000000c1a100000000000017000800000000000000ada0120d92805b6ed5196cb0f41844f9270b96f1730cda8b5b0",
            "659c818a0419fa1b2947"
        );
        const TRANSFER_ENVELOPE: &str = concat!(
            "08011210000000000001700080000000000000092001280230033a176f66666c696e655f6974656d5f756e6163636570",
            "74656440014a10000000000001700080000000000000015210000000000001700080000000000000066a100000000000",
            "017000800000000000000770017a04080110028001029201160a100000000000017000800000000000000510011801a2",
            "011612140a10000000000001700080000000000000071002ba01086669787475726531c201086669787475726531ca01",
            "076f66666c696e65d201df0108011ada010a2c0a10000000000001700080000000000000031210000000000001700080",
            "000000000000011801200128013001122c0a100000000000017000800000000000000312100000000000017000800000",
            "000000000118012001280130011a3e0a1000000000000170008000000000000001121000000000000170008000000000",
            "0000061a100000000000017000800000000000000c200128643064380722140a10000000000001700080000000000000",
            "0210012a26080112100000000000017000800000000000000c1a100000000000017000800000000000000bda01201398",
            "65089c9450ade078899260f69f620ca17e3969844906f288eee2729665ef"
        );

        fn unhex(value: &str) -> Result<Vec<u8>, Box<dyn Error>> {
            value
                .as_bytes()
                .chunks_exact(2)
                .map(|pair| Ok(u8::from_str_radix(std::str::from_utf8(pair)?, 16)?))
                .collect()
        }
        fn event(kind: Kind) -> Result<FrozenEvent, Box<dyn Error>> {
            let (mint, transfer, facts) = fixture()?;
            let mut model = Model::default();
            if kind == Kind::Transfer {
                model.attempt(
                    Request::OneItem(mint.clone()),
                    facts,
                    audit_probe(),
                    evidence_limits()?,
                    Attempt::Acknowledged,
                )?;
            }
            Ok(freeze(&model.plan(
                if kind == Kind::Mint { mint } else { transfer },
                audit_probe(),
            )?)?)
        }
        fn rewrite(value: &Envelope) -> Result<Vec<u8>, Failure> {
            // Negative fixtures, no production authoring path.
            encode(value, ENVELOPE_MAX, ENVELOPE_MAX)
        }
        fn reject_envelope(mutator: impl FnOnce(&mut Envelope)) -> Result<(), Box<dyn Error>> {
            let original = event(Kind::Transfer)?;
            let mut changed = original.semantic.clone();
            mutator(&mut changed);
            assert!(FrozenEvent::decode(&rewrite(&changed)?, &current_fixture()).is_err());
            assert_eq!(original, event(Kind::Transfer)?);
            Ok(())
        }
        fn reject_payload(mutator: impl FnOnce(&mut Transfer)) -> Result<(), Box<dyn Error>> {
            let mut envelope = event(Kind::Transfer)?.semantic;
            let mut value = decode_payload(&envelope.payload)?;
            let Some(Operation::Transfer(transfer)) = &mut value.operation else {
                return Err(Failure::Unsupported.into());
            };
            mutator(transfer);
            envelope.payload = encode(&value, PAYLOAD_MAX, PAYLOAD_MAX)?;
            envelope.payload_sha256 = Sha256::digest(&envelope.payload).to_vec();
            assert!(FrozenEvent::decode(&rewrite(&envelope)?, &current_fixture()).is_err());
            Ok(())
        }

        #[test]
        fn literal_payload_envelope_goldens_and_roundtrip() -> Result<(), Box<dyn Error>> {
            for (kind, payload_hex, envelope_hex) in [
                (Kind::Mint, MINT_PAYLOAD, MINT_ENVELOPE),
                (Kind::Transfer, TRANSFER_PAYLOAD, TRANSFER_ENVELOPE),
            ] {
                let frozen = event(kind)?;
                assert_eq!(hex(&frozen.semantic.payload), payload_hex);
                assert_eq!(hex(&frozen.wire), envelope_hex);
                let decoded = FrozenEvent::decode(&unhex(envelope_hex)?, &current_fixture())?;
                assert_eq!(decoded, frozen);
                let payload = decode_payload(&unhex(payload_hex)?)?;
                assert_eq!(
                    encode(&payload, PAYLOAD_MAX, PAYLOAD_MAX)?,
                    frozen.semantic.payload
                );
                assert!(frozen.production_admission().is_err());
            }
            Ok(())
        }

        fn idl_fields(
            source: &str,
            message: &str,
        ) -> Result<std::collections::BTreeMap<String, u32>, Box<dyn Error>> {
            let start = source
                .find(&format!("message {message} {{"))
                .ok_or(Failure::Facts)?;
            let body = &source[start..];
            let body = body.split_once('}').ok_or(Failure::Facts)?.0;
            let mut fields = std::collections::BTreeMap::new();
            for line in body.lines().skip(1) {
                let line = line.split("//").next().ok_or(Failure::Facts)?.trim();
                if let Some((left, right)) = line.split_once('=') {
                    let name = left.split_whitespace().last().ok_or(Failure::Facts)?;
                    fields.insert(
                        name.to_string(),
                        right.trim().trim_end_matches(';').parse()?,
                    );
                }
            }
            Ok(fields)
        }
        fn rust_fields(
            source: &str,
            message: &str,
        ) -> Result<std::collections::BTreeMap<String, u32>, Box<dyn Error>> {
            let start = source
                .find(&format!("struct {message} {{"))
                .ok_or(Failure::Facts)?;
            let body = source[start..]
                .split_once("\n    }")
                .ok_or(Failure::Facts)?
                .0;
            let mut fields = std::collections::BTreeMap::new();
            let mut pending = None;
            for line in body.lines().skip(1) {
                if let Some((_, right)) = line.split_once("tag = \"") {
                    pending = Some(
                        right
                            .split_once('"')
                            .ok_or(Failure::Facts)?
                            .0
                            .parse::<u32>()?,
                    );
                } else if line.trim().starts_with("#[prost(oneof") {
                    pending = None;
                } else if let Some((name, _)) = line.trim().split_once(':') {
                    if let Some(tag) = pending.take() {
                        fields.insert(name.to_string(), tag);
                    }
                }
            }
            Ok(fields)
        }

        #[test]
        fn every_private_prost_field_matches_candidate_and_foundation_idl_tags()
        -> Result<(), Box<dyn Error>> {
            let source = include_str!("dur03_reference_one_item_resource_prototype.rs");
            let candidate =
                include_str!("../../../docs/contracts/game-events/v1/item_transaction.proto");
            let foundation =
                include_str!("../../../docs/contracts/game-events/v1/foundation.proto");
            for (rust, idl, text) in [
                ("ItemState", "CandidateItemState", candidate),
                ("Ground", "CandidateGround", candidate),
                ("Inventory", "CandidateInventory", candidate),
                ("Provenance", "CandidateProvenance", candidate),
                ("Mint", "OneItemMint", candidate),
                ("Transfer", "OneItemTransfer", candidate),
                ("Envelope", "EventEnvelope", foundation),
                ("Membership", "TransactionEventRef", foundation),
                ("RuntimeOrder", "RuntimeOrderRef", foundation),
                ("Command", "CommandRef", foundation),
                ("Actor", "AnalyticsActorRef", foundation),
            ] {
                assert_eq!(
                    rust_fields(source, rust)?,
                    idl_fields(text, idl)?,
                    "{rust}/{idl}"
                );
            }
            let fields = idl_fields(candidate, "OneItemTransactionCandidate")?;
            assert_eq!(fields["interpretation_revision"], 1);
            assert_eq!(fields["mint"], 2);
            assert_eq!(fields["transfer"], 3);
            assert!(source.contains("#[prost(message, tag = \"2\")]\n        Mint(Mint)"));
            assert!(source.contains("#[prost(message, tag = \"3\")]\n        Transfer(Transfer)"));
            assert_eq!(
                idl_fields(foundation, "CausationRef")?
                    .into_values()
                    .collect::<Vec<_>>(),
                vec![2, 1, 3, 4]
            );
            Ok(())
        }

        #[test]
        fn reordered_fields_and_additive_unknowns_preserve_exact_payload_identity()
        -> Result<(), Box<dyn Error>> {
            for kind in [Kind::Mint, Kind::Transfer] {
                let original = event(kind)?;
                // Move envelope_revision from start to end and add unknown tag64
                // primitive scalar (key 512 = 80 04), which is safely additive.
                let mut reordered = original.wire[2..].to_vec();
                reordered.extend_from_slice(&[8, 1, 0x80, 0x04, 1]);
                let decoded = FrozenEvent::decode(&reordered, &current_fixture())?;
                original.duplicate(&decoded)?;
                assert_eq!(original.semantic.payload, decoded.semantic.payload);
                assert_ne!(original.wire, decoded.wire);
                let mut changed = original.semantic.clone();
                changed.payload.extend_from_slice(&[0x80, 0x04, 1]);
                changed.payload_sha256 = Sha256::digest(&changed.payload).to_vec();
                let accepted = FrozenEvent::decode(&rewrite(&changed)?, &current_fixture())?;
                // Same semantics, different exact payload bytes is a conflict.
                assert_eq!(original.duplicate(&accepted), Err(Failure::Conflict));
            }
            Ok(())
        }

        #[test]
        fn malformed_truncated_varint_wire_groups_duplicates_and_closed_variant_reject()
        -> Result<(), Box<dyn Error>> {
            for input in [
                vec![0],
                vec![0x80],
                vec![8, 0x80],
                vec![
                    0x08, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 2,
                ],
                vec![0x0a, 0],
                vec![0x12, 9, 1],
                vec![0x83, 4, 0x84, 4],
                vec![0x0d, 1],
                vec![0x0e],
                vec![0x0f],
                vec![8, 1, 8, 1],
                vec![8, 1, 0x22, 0],
                vec![8, 1, 0x12, 0, 0x1a, 0],
            ] {
                assert!(
                    decode_payload(&input)
                        .and_then(|p| validate_payload(&p, &current_fixture()))
                        .is_err(),
                    "{input:?}"
                );
            }
            for kind in [Kind::Mint, Kind::Transfer] {
                let frozen = event(kind)?;
                for length in 0..frozen.wire.len() {
                    assert!(
                        FrozenEvent::decode(&frozen.wire[..length], &current_fixture()).is_err(),
                        "prefix {length}"
                    );
                }
            }
            let mut envelope = event(Kind::Mint)?.semantic;
            envelope.payload[1] = 2; // unsupported interpretation revision
            envelope.payload_sha256 = Sha256::digest(&envelope.payload).to_vec();
            assert!(FrozenEvent::decode(&rewrite(&envelope)?, &current_fixture()).is_err());
            reject_envelope(|e| e.event_schema_revision = 2)?;
            reject_envelope(|e| e.envelope_revision = 2)?;
            reject_envelope(|e| e.event_type_id = 1)?;
            Ok(())
        }

        #[test]
        fn shared_limits_exact_max_plus_one_checked_arithmetic_before_allocation()
        -> Result<(), Box<dyn Error>> {
            for maximum in [PAYLOAD_MAX, ENVELOPE_MAX, STRING_MAX, DEPTH_MAX] {
                bounded(maximum, maximum)?;
                assert!(
                    bounded(maximum.checked_add(1).ok_or(Failure::Overflow)?, maximum).is_err()
                );
            }
            let registry: Value = serde_json::from_str(include_str!(
                "../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
            ))?;
            let text = serde_json::to_string(&registry)?;
            for (id, maximum) in [
                ("ANL01-EVENT-PAYLOAD-BYTES", PAYLOAD_MAX),
                ("ANL01-EVENT-ENVELOPE-BYTES", ENVELOPE_MAX),
                ("ANL01-ENVELOPE-STRING-BYTES", STRING_MAX),
                ("ANL01-PROTOBUF-NESTING-DEPTH", DEPTH_MAX),
            ] {
                let begin = text
                    .find(&format!("\"id\":\"{id}\""))
                    .ok_or(Failure::Facts)?;
                let entry = text[..begin].rfind('{').ok_or(Failure::Facts)?;
                let end = text[begin..]
                    .find("},\"")
                    .map(|n| begin + n)
                    .unwrap_or(text.len());
                assert!(text[entry..end].contains(&format!("\"hard_maximum\":{maximum}")));
            }
            // Actual finite shape is shallow. Exercise the borrowed boundary
            // at depth32 and a known child at depth33; groups reject outright.
            preflight(&[8, 1], Schema::Runtime, 32)?;
            preflight(&[0x7a, 2, 8, 1], Schema::Envelope, 31)?;
            assert!(preflight(&[0x7a, 2, 8, 1], Schema::Envelope, 32).is_err());
            assert!(preflight(&[0x83, 4, 0x84, 4], Schema::Envelope, 1).is_err());
            preflight(&[0x3a, 0x80, 1], Schema::Envelope, 1)
                .err()
                .ok_or(Failure::Facts)?;
            let mut long = vec![0x3a, 0x80, 1];
            long.extend(std::iter::repeat_n(b'a', 128));
            preflight(&long, Schema::Envelope, 1)?;
            long[1] = 0x81;
            long.push(b'a');
            assert!(preflight(&long, Schema::Envelope, 1).is_err());
            assert!(usize::MAX.checked_add(1).is_none());
            assert_eq!(add(u64::MAX, 1), Err(Failure::Overflow));
            assert!(encode(&RuntimeOrder::default(), ENVELOPE_MAX + 1, ENVELOPE_MAX).is_err());
            Ok(())
        }

        #[test]
        fn missing_nil_wrong_scope_identity_session_revision_and_provenance_reject()
        -> Result<(), Box<dyn Error>> {
            reject_envelope(|e| e.event_id = vec![0; 16])?;
            reject_envelope(|e| e.world_id = None)?;
            reject_envelope(|e| e.world_id = Some(uuid_fixture(13).to_vec()))?;
            reject_envelope(|e| e.channel_id = None)?;
            reject_envelope(|e| e.channel_id = Some(uuid_fixture(13).to_vec()))?;
            reject_envelope(|e| e.runtime_order = None)?;
            reject_envelope(|e| {
                if let Some(r) = &mut e.runtime_order {
                    r.scope_ownership_generation = 2;
                }
            })?;
            reject_envelope(|e| e.game_session_id = None)?;
            reject_envelope(|e| e.game_session_id = Some(uuid_fixture(13).to_vec()))?;
            reject_envelope(|e| e.connection_generation = None)?;
            reject_envelope(|e| e.connection_generation = Some(2))?;
            reject_envelope(|e| e.command_id = Some(0))?;
            reject_envelope(|e| e.causation = None)?;
            reject_envelope(|e| e.content_revision = Some("other".into()))?;
            reject_payload(|t| t.after = None)?;
            reject_payload(|t| {
                if let Some(s) = &mut t.after {
                    s.item_instance_id = vec![0; 16];
                }
            })?;
            reject_payload(|t| {
                if let Some(g) = &mut t.source {
                    g.world_id = uuid_fixture(13).to_vec();
                }
            })?;
            reject_payload(|t| {
                if let Some(g) = &mut t.source {
                    g.channel_id = uuid_fixture(13).to_vec();
                }
            })?;
            reject_payload(|t| {
                if let Some(g) = &mut t.source {
                    g.corpse_occurrence_id = uuid_fixture(13).to_vec();
                }
            })?;
            reject_payload(|t| {
                if let Some(g) = &mut t.source {
                    g.corpse_revision = 2;
                }
            })?;
            reject_payload(|t| t.destination = None)?;
            reject_payload(|t| {
                if let Some(d) = &mut t.destination {
                    d.character_id = uuid_fixture(13).to_vec();
                }
            })?;
            reject_payload(|t| {
                if let Some(d) = &mut t.destination {
                    d.session_generation = 2;
                }
            })?;
            reject_payload(|t| {
                if let Some(d) = &mut t.destination {
                    d.direct_root_slot = 1;
                }
            })?;
            reject_payload(|t| t.cause = None)?;
            reject_payload(|t| {
                if let Some(c) = &mut t.cause {
                    c.cause_id = uuid_fixture(13).to_vec();
                }
            })?;
            // Current facts are independently substituted one invariant at time.
            let event = event(Kind::Transfer)?;
            for changed in 0..8 {
                let mut facts = current_fixture();
                match changed {
                    0 => facts.world = uuid_fixture(13),
                    1 => facts.channel = uuid_fixture(13),
                    2 => facts.corpse = uuid_fixture(13),
                    3 => facts.character = uuid_fixture(13),
                    4 => facts.session = uuid_fixture(13),
                    5 => facts.generation = 2,
                    6 => facts.corpse_revision = 2,
                    _ => facts.definition_revision = 2,
                }
                assert!(FrozenEvent::decode(&event.wire, &facts).is_err());
            }
            Ok(())
        }

        #[test]
        fn lifecycle_type_quantity_conservation_and_partial_transfer_reject_without_publication()
        -> Result<(), Box<dyn Error>> {
            reject_payload(|t| {
                if let Some(s) = &mut t.after {
                    s.lifecycle = 2;
                }
            })?;
            reject_payload(|t| {
                if let Some(s) = &mut t.after {
                    s.definition_key = 2;
                }
            })?;
            reject_payload(|t| {
                if let Some(s) = &mut t.after {
                    s.definition_revision = 2;
                }
            })?;
            reject_payload(|t| {
                if let Some(s) = &mut t.after {
                    s.quantity = 0;
                }
            })?;
            reject_payload(|t| {
                if let Some(s) = &mut t.after {
                    s.quantity = 2;
                }
            })?;
            reject_payload(|t| {
                if let Some(s) = &mut t.after {
                    s.item_instance_id = uuid_fixture(13).to_vec();
                }
            })?;
            let (mint, transfer, facts) = fixture()?;
            let mut model = Model::default();
            model.attempt(
                Request::OneItem(mint),
                facts,
                audit_probe(),
                evidence_limits()?,
                Attempt::Acknowledged,
            )?;
            let before = model.clone();
            let mut plan = model.plan(transfer, audit_probe())?;
            plan.effects[1] = None;
            assert_eq!(model.publish_custody(&plan), Err(Failure::Custody));
            assert_eq!(model, before);
            Ok(())
        }

        #[test]
        fn complete_membership_duplicate_gap_out_of_order_and_identity_conflict()
        -> Result<(), Box<dyn Error>> {
            reject_envelope(|e| e.transaction_event = None)?;
            for (ordinal, count) in [(0, 1), (2, 1), (1, 0), (1, 2), (2, 2)] {
                reject_envelope(|e| {
                    if let Some(m) = &mut e.transaction_event {
                        m.ordinal = ordinal;
                        m.count = count;
                    }
                })?;
            }
            reject_envelope(|e| {
                if let Some(m) = &mut e.transaction_event {
                    m.transaction_id = uuid_fixture(13).to_vec();
                }
            })?;
            let original = event(Kind::Transfer)?;
            original.duplicate(&original)?;
            let mut changed = original.semantic.clone();
            changed.occurred_at_unix_ms = 2;
            let changed = FrozenEvent::decode(&rewrite(&changed)?, &current_fixture())?;
            assert_eq!(original.duplicate(&changed), Err(Failure::Conflict));
            assert_ne!(
                event(Kind::Mint)?.semantic.event_id,
                original.semantic.event_id
            );
            Ok(())
        }

        #[test]
        fn frozen_bytes_hash_envelope_and_membership_survive_ambiguity_abort_retry_and_replay()
        -> Result<(), Box<dyn Error>> {
            for kind in [Kind::Mint, Kind::Transfer] {
                for outcome in [
                    Attempt::CommittedResponseLost,
                    Attempt::AmbiguousWithoutCommit,
                ] {
                    let (mint, transfer, facts) = fixture()?;
                    let mut model = Model::default();
                    if kind == Kind::Transfer {
                        model.attempt(
                            Request::OneItem(mint.clone()),
                            facts,
                            audit_probe(),
                            evidence_limits()?,
                            Attempt::Acknowledged,
                        )?;
                    }
                    let intent = if kind == Kind::Mint { mint } else { transfer };
                    model.attempt(
                        Request::OneItem(intent.clone()),
                        facts,
                        audit_probe(),
                        evidence_limits()?,
                        outcome,
                    )?;
                    let index = model
                        .record_index(intent.transaction)
                        .ok_or(Failure::Conflict)?;
                    let frozen = model.records[index]
                        .as_ref()
                        .ok_or(Failure::Conflict)?
                        .event
                        .clone();
                    model.reconcile(
                        intent.transaction,
                        evidence_limits()?,
                        Reconcile::InspectCommittedStore,
                    )?;
                    let not_applied = model.records[index]
                        .as_ref()
                        .ok_or(Failure::Conflict)?
                        .observation
                        == Observation::NotApplied;
                    if not_applied {
                        model.attempt(
                            Request::OneItem(intent.clone()),
                            facts,
                            audit_probe(),
                            evidence_limits()?,
                            Attempt::Acknowledged,
                        )?;
                    } else {
                        assert_eq!(
                            model.attempt(
                                Request::OneItem(intent.clone()),
                                facts,
                                audit_probe(),
                                evidence_limits()?,
                                Attempt::Acknowledged
                            )?,
                            ResultState::Committed(intent.item.id())
                        );
                    }
                    let retained = &model.records[index]
                        .as_ref()
                        .ok_or(Failure::Conflict)?
                        .event;
                    assert_eq!(retained, &frozen);
                    let snapshot = model.clone();
                    frozen.replay(&model)?;
                    assert_eq!(model, snapshot);
                    // Known abort cannot mutate admitted bytes, hash, timestamp,
                    // membership, profile, identity or permit stale current facts.
                    let mut wrong = facts;
                    wrong.authority_generation = 2;
                    assert!(
                        model
                            .attempt(
                                Request::OneItem(intent),
                                wrong,
                                audit_probe(),
                                evidence_limits()?,
                                Attempt::Acknowledged
                            )
                            .is_err()
                    );
                    assert_eq!(model, snapshot);
                }
            }
            Ok(())
        }

        #[test]
        fn privacy_missing_profile_secret_and_unauthorized_raw_export_fail_closed()
        -> Result<(), Box<dyn Error>> {
            for privacy in [0, 1, 2, 5] {
                reject_envelope(|e| e.privacy_class = privacy)?;
            }
            reject_envelope(|e| e.retention_profile_id.clear())?;
            reject_envelope(|e| {
                e.retention_profile_id = "CHARACTER_AUTHORITY_DURABLE_AUDIT_RETENTION_V1".into()
            })?;
            reject_envelope(|e| e.server_build_id = "Bearer secret".into())?;
            reject_envelope(|e| e.durability_class = 1)?;
            let event = event(Kind::Transfer)?;
            assert!(event.raw_id_export(false).is_err());
            assert!(event.raw_id_export(true).is_err());
            assert!(event.production_admission().is_err());
            Ok(())
        }

        #[test]
        fn mint_missing_nil_state_source_and_provenance_fail_closed() -> Result<(), Box<dyn Error>>
        {
            for case in 0..10 {
                let mut envelope = event(Kind::Mint)?.semantic;
                let mut value = decode_payload(&envelope.payload)?;
                let Some(Operation::Mint(mint)) = &mut value.operation else {
                    return Err(Failure::Unsupported.into());
                };
                match case {
                    0 => mint.after = None,
                    1 => mint.destination = None,
                    2 => mint.source = None,
                    3 => {
                        if let Some(s) = &mut mint.after {
                            s.item_instance_id = vec![0; 16];
                        }
                    }
                    4 => {
                        if let Some(s) = &mut mint.after {
                            s.quantity = 0;
                        }
                    }
                    5 => {
                        if let Some(s) = &mut mint.after {
                            s.lifecycle = 0;
                        }
                    }
                    6 => {
                        if let Some(g) = &mut mint.destination {
                            g.corpse_occurrence_id = vec![0; 16];
                        }
                    }
                    7 => {
                        if let Some(g) = &mut mint.destination {
                            g.corpse_revision = 0;
                        }
                    }
                    8 => {
                        if let Some(p) = &mut mint.source {
                            p.source_kind = 2;
                        }
                    }
                    _ => {
                        if let Some(p) = &mut mint.source {
                            p.occurrence_id = uuid_fixture(13).to_vec();
                        }
                    }
                }
                envelope.payload = encode(&value, PAYLOAD_MAX, PAYLOAD_MAX)?;
                envelope.payload_sha256 = Sha256::digest(&envelope.payload).to_vec();
                assert!(FrozenEvent::decode(&rewrite(&envelope)?, &current_fixture()).is_err());
            }
            Ok(())
        }

        #[test]
        fn numeric_fixture_source_cause_cannot_be_silently_reinterpreted()
        -> Result<(), Box<dyn Error>> {
            let (mut mint, _, facts) = fixture()?;
            mint.cause = 9;
            let mut model = Model::default();
            assert!(
                model
                    .attempt(
                        Request::OneItem(mint),
                        facts,
                        audit_probe(),
                        evidence_limits()?,
                        Attempt::Acknowledged
                    )
                    .is_err()
            );
            assert_eq!(model, Model::default());
            Ok(())
        }

        #[test]
        fn oversized_borrowed_inputs_and_unknown_private_bytes_reject_before_decode()
        -> Result<(), Box<dyn Error>> {
            assert!(decode_payload(&vec![0; PAYLOAD_MAX + 1]).is_err());
            assert!(FrozenEvent::decode(&vec![0; ENVELOPE_MAX + 1], &current_fixture()).is_err());
            // Uninterpreted bytes might conceal private IDs/secrets. Only safe
            // additive numeric unknowns are admitted by this closed candidate.
            assert!(
                preflight(
                    &[0x82, 4, 6, b's', b'e', b'c', b'r', b'e', b't'],
                    Schema::Payload,
                    2
                )
                .is_err()
            );
            let mut changed = event(Kind::Transfer)?.semantic;
            if let Some(Causation {
                cause: Some(Cause::Command(command)),
            }) = &mut changed.causation
            {
                command.game_session_id = uuid_fixture(13).to_vec();
            }
            assert!(FrozenEvent::decode(&rewrite(&changed)?, &current_fixture()).is_err());
            Ok(())
        }

        #[test]
        fn corrupt_frozen_carrier_cannot_reconcile_or_retry_a_known_abort()
        -> Result<(), Box<dyn Error>> {
            for corrupt in 0..4 {
                let (mint, _, facts) = fixture()?;
                let mut model = Model::default();
                model.attempt(
                    Request::OneItem(mint.clone()),
                    facts,
                    audit_probe(),
                    evidence_limits()?,
                    Attempt::AmbiguousWithoutCommit,
                )?;
                let record = model.records[0].as_mut().ok_or(Failure::Conflict)?;
                match corrupt {
                    0 => record.event.semantic.payload_sha256[0] ^= 1,
                    1 => record.event.semantic.occurred_at_unix_ms += 1,
                    2 => record.event.wire[0] ^= 1,
                    _ => record.event.semantic.event_id = uuid_fixture(13).to_vec(),
                }
                let snapshot = model.clone();
                assert!(
                    model
                        .reconcile(
                            mint.transaction,
                            evidence_limits()?,
                            Reconcile::InspectCommittedStore
                        )
                        .is_err()
                );
                assert_eq!(model, snapshot);
                assert!(
                    model
                        .attempt(
                            Request::OneItem(mint),
                            facts,
                            audit_probe(),
                            evidence_limits()?,
                            Attempt::Acknowledged
                        )
                        .is_err()
                );
                assert_eq!(model, snapshot);
            }
            Ok(())
        }
    }
}

fn evidence_limits() -> Result<EvidenceLimits, Failure> {
    Ok(EvidenceLimits([
        1,
        2,
        bytes(1, size_of::<Participant>())?,
        bytes(2, size_of::<Effect>())?,
        138,
        3,
        3,
        1,
        16,
        2,
        bytes(2, size_of::<ReconciliationRecord>())?,
        6,
    ]))
}

fn identity_hex(bytes: &[u8; 16]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn measurements(record: &ReconciliationRecord) -> Result<Value, Failure> {
    let plan = &record.plan;
    let usage = plan.usage(1)?;
    let effects = plan.effects.iter().flatten().map(|effect| {
        Ok(json!({
            "kind": match effect.kind { EffectKind::Remove => "REMOVE", EffectKind::Establish => "ESTABLISH" },
            "location": match &effect.location { Location::Ground { .. } => "Ground/corpse fixture", Location::CharacterInventory { .. } => "CharacterInventory/direct-root", _ => return Err(Failure::Unsupported) },
            "encoded_bytes": add(17, location_encoded_len(&effect.location)?)?,
            "retained_bytes": size_of::<Effect>()
        }))
    }).collect::<Result<Vec<_>, Failure>>()?;
    Ok(json!({
        "operation": plan.intent.kind.label(),
        "item_instance_id": identity_hex(plan.intent.item.id().as_bytes()),
        "logical_transaction_identity": identity_hex(&plan.intent.transaction.0),
        "transaction_identity_retained_bytes": size_of::<EvidenceTransaction>(),
        "touched_items": 1,
        "quantity": 1,
        "participant_records": usage[0],
        "participant_retained_bytes": usage[2],
        "participant_encoded_bytes": 44,
        "custody_effect_records": usage[1],
        "custody_effect_retained_bytes": usage[3],
        "custody_effect_encoded_bytes": usage[4] - 44,
        "participant_and_effect_encoded_bytes": plan.encode_records()?.len(),
        "fixed_plan_retained_bytes_including_spare_slots_and_intent": size_of::<Plan>(),
        "planning_work_units": usage[5],
        "application_work_units": usage[6],
        "work_definition": "one unit per participant/effect during planning; one per effect plus receipt during application; excludes serialization, lookup implementation cost and wall time",
        "container_expansion_work": 0,
        "value_account_lines": 0,
        "transform_inputs_outputs": 0,
        "reconciliation_state_records": 1,
        "reconciliation_state_retained_bytes": size_of::<ReconciliationRecord>(),
        "reconciliation_size_scope": "inline carrier including FrozenEvent; its dynamic vectors/strings are charged separately",
        "retry_reconcile_work_units_per_call": 3,
        "effects": effects,
        "offline_candidate_audit": record.event.measurement()?,
        "mandatory_audit": {
            "classification": "EVIDENCE_GAP",
            "event_contribution_count": null,
            "aggregate_encoded_bytes": null,
            "reason": "No executable registered DUR-03 event family/payload exists in the consumed public substrate; ANL envelope semantics do not determine it"
        },
        "synthetic_audit_budget_probe_only": { "contributions": plan.audit_probe.contributions, "encoded_bytes": plan.audit_probe.encoded_bytes, "event_emitted": false }
    }))
}

fn measurement_target() -> Value {
    json!({
        "os": std::env::consts::OS,
        "architecture": std::env::consts::ARCH,
        "build_toolchain_identity": "UNKNOWN",
        "build_toolchain_identity_reason": "This standalone example has no build-time compiler attestation",
        "retained_size_basis": "size_of for the executable target, including padding",
        "encoding": "prototype big-endian records and unregistered protobuf candidate measured separately"
    })
}

fn normalized_report(reverse: bool) -> Result<String, Box<dyn Error>> {
    let (mint, transfer, facts) = fixture()?;
    let limits = evidence_limits()?;
    let mut model = Model::default();
    model.attempt(
        Request::OneItem(mint.clone()),
        facts,
        audit_probe(),
        limits,
        Attempt::CommittedResponseLost,
    )?;
    model.reconcile(mint.transaction, limits, Reconcile::InspectCommittedStore)?;
    model.attempt(
        Request::OneItem(mint.clone()),
        facts,
        audit_probe(),
        limits,
        Attempt::Acknowledged,
    )?;
    model.attempt(
        Request::OneItem(transfer.clone()),
        facts,
        audit_probe(),
        limits,
        Attempt::CommittedResponseLost,
    )?;
    model.reconcile(transfer.transaction, limits, Reconcile::Unknown)?;
    model.reconcile(
        transfer.transaction,
        limits,
        Reconcile::InspectCommittedStore,
    )?;
    let mut plans = model.records.iter().flatten().collect::<Vec<_>>();
    if reverse {
        plans.reverse();
    }
    // Sort presentation, never execution, by explicit mutation class order.
    plans.sort_by_key(|record| record.plan.intent.kind.tag());
    let operations = plans
        .into_iter()
        .map(measurements)
        .collect::<Result<Vec<_>, _>>()?;
    let retained_dynamic = model
        .records
        .iter()
        .flatten()
        .try_fold(0_usize, |total, record| {
            total
                .checked_add(record.event.dynamic_retained_bytes()?)
                .ok_or(Failure::Overflow)
        })?;
    let mut report = json!({
        "schema_version": 1,
        "classification": "NONPRODUCTION_OFFLINE_CANDIDATE_SCHEMA_MEASUREMENT",
        "task_id": "OTV2-20260927-dur03-one-item-audit-offline-measurement-513",
        "source_base_sha": "a822326c9cf4607100e58bbc3673748f3fa299bb",
        "source_revisions": {
            "DUR03_section_39_1": "a822326c9cf4607100e58bbc3673748f3fa299bb",
            "ANL01_foundation_resource_registry": "a822326c9cf4607100e58bbc3673748f3fa299bb",
            "META_policy": "1bfb5ff98c8aa156e73669a14e083a1d464c29fb",
            "prost": "0.14.4"
        },
        "fixture": "Gold Coin x1",
        "item_definition_ref": { "key": 1, "revision": 1, "scope": "private fixture mapping only; no Content catalog resolution" },
        "natural_drop_probability": "UNKNOWN/NOT_ASSERTED",
        "production_maxima_selected": false,
        "production_durability_or_parity_proven": false,
        "measurement_target": measurement_target(),
        "evidence_limits": DIMENSIONS.into_iter().zip(limits.0).collect::<std::collections::BTreeMap<_, _>>(),
        "operations": operations,
        "final_state": {
            "authoritative_items": usize::from(model.custody.is_some()),
            "immediate_locations": usize::from(model.custody.is_some()),
            "location": "CharacterInventory/direct-root",
            "quantity": 1,
            "item_instance_id": identity_hex(transfer.item.id().as_bytes()),
            "retained_reconciliation_records": model.records.iter().flatten().count(),
            "retained_reconciliation_record_bytes": bytes(2, size_of::<ReconciliationRecord>())?,
            "fixed_model_retained_bytes_including_spare_slots": size_of::<Model>(),
            "retained_event_dynamic_vector_and_string_bytes": retained_dynamic,
            "model_inline_plus_retained_dynamic_bytes": size_of::<Model>().checked_add(retained_dynamic).ok_or(Failure::Overflow)?,
            "MINT_retry_work_units": model.records[0].as_ref().ok_or(Failure::Conflict)?.retry_work,
            "TRANSFER_retry_work_units": model.records[1].as_ref().ok_or(Failure::Conflict)?.retry_work
        },
        "rows": {
            "DUR03-RL-01": "ONE_ITEM_SEMANTIC_FIXTURE; multiple-item capability rejects before planning",
            "DUR03-RL-02": "MEASURED_PROTOTYPE_ONLY; one establish MINT; remove+establish TRANSFER; one custody tuple publication",
            "DUR03-RL-03": "NOT_EXERCISED/0; value/account capability rejects before planning",
            "DUR03-RL-04": "NOT_EXERCISED/0; transform capability rejects before planning",
            "DUR03-RL-05": "DIRECT_ROOT_ONLY/0; Container rejects before planning; no general container bound",
            "DUR03-RL-06": "MEASURED_PROTOTYPE_ONLY; counts derived from physically represented records",
            "DUR03-RL-07": "MEASURED_UNREGISTERED_CANDIDATE separately; production mandatory count/bytes EVIDENCE_GAP; synthetic probe retained separately",
            "DUR03-RL-08": "MEASURED_IN_MEMORY_STATE_MACHINE_ONLY; stable transaction/output identities; no DB/restart proof"
        },
        "evidence_gaps": [
            "registered DUR-03 mandatory audit event contribution count and exact payload/envelope byte total",
            "production transaction identity allocator, PostgreSQL receipts/atomic commit/isolation/fencing and crash recovery",
            "production resource maxima, replay/retention horizon and connected Combat/loot/pickup integration",
            "real Content definition mapping, natural Rat loot probability and Reference parity"
        ]
    });
    report["signed_offline_candidate_v2"] = candidate::v2::report(reverse)?;
    Ok(format!("{}\n", serde_json::to_string_pretty(&report)?))
}

fn main() -> Result<(), Box<dyn Error>> {
    let reverse = std::env::args()
        .skip(1)
        .any(|arg| arg == "--reverse-fixtures");
    print!("{}", normalized_report(reverse)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared(kind: Kind) -> Result<(Model, Intent, CurrentFacts), Box<dyn Error>> {
        let (mint, transfer, facts) = fixture()?;
        let mut model = Model::default();
        if kind == Kind::Transfer {
            model.attempt(
                Request::OneItem(mint.clone()),
                facts,
                audit_probe(),
                evidence_limits()?,
                Attempt::Acknowledged,
            )?;
        }
        Ok((
            model,
            if kind == Kind::Mint { mint } else { transfer },
            facts,
        ))
    }

    #[test]
    fn both_operations_measure_physical_records_and_exact_encoding() -> Result<(), Box<dyn Error>> {
        for (kind, effects, encoded) in [(Kind::Mint, 1, 102), (Kind::Transfer, 2, 138)] {
            let (model, intent, _) = prepared(kind)?;
            let plan = model.plan(intent, audit_probe())?;
            assert_eq!(plan.usage(1)?[0], 1);
            assert_eq!(plan.usage(1)?[1], effects);
            assert_eq!(plan.encode_records()?.len(), encoded);
            assert_eq!(plan.usage(1)?[4], u64::try_from(encoded)?);
        }
        Ok(())
    }

    #[test]
    fn all_plan_dimensions_accept_max_and_reject_max_plus_one_without_publication()
    -> Result<(), Box<dyn Error>> {
        for kind in [Kind::Mint, Kind::Transfer] {
            let (baseline, intent, facts) = prepared(kind)?;
            let plan = baseline.plan(intent.clone(), audit_probe())?;
            let count = add(u64::try_from(baseline.records.iter().flatten().count())?, 1)?;
            let usage = plan.usage(count)?;
            for dimension in 0..11 {
                let mut exact = evidence_limits()?;
                exact.0[dimension] = usage[dimension];
                let mut accepted = baseline.clone();
                accepted.attempt(
                    Request::OneItem(intent.clone()),
                    facts,
                    audit_probe(),
                    exact,
                    Attempt::Acknowledged,
                )?;
                let mut over = exact;
                over.0[dimension] = usage[dimension].checked_sub(1).ok_or(Failure::Overflow)?;
                let mut rejected = baseline.clone();
                assert_eq!(
                    rejected.attempt(
                        Request::OneItem(intent.clone()),
                        facts,
                        audit_probe(),
                        over,
                        Attempt::Acknowledged
                    ),
                    Err(Failure::Budget(DIMENSIONS[dimension]))
                );
                assert_eq!(rejected, baseline);
            }
        }
        Ok(())
    }

    #[test]
    fn unsupported_capabilities_and_container_reject_before_planning() -> Result<(), Box<dyn Error>>
    {
        let (_, transfer, facts) = fixture()?;
        let mut container = transfer;
        container.destination = Location::Container {
            parent_item_instance_id: container.item.id(),
            entry: 0,
        };
        for request in [
            Request::MultipleItems,
            Request::ValueLine,
            Request::AccountLine,
            Request::Transform,
            Request::OneItem(container),
        ] {
            let mut model = Model::default();
            assert_eq!(
                model.attempt(
                    request,
                    facts,
                    audit_probe(),
                    EvidenceLimits([0; 12]),
                    Attempt::Acknowledged
                ),
                Err(Failure::Unsupported)
            );
            assert_eq!(model, Model::default());
        }
        Ok(())
    }

    #[test]
    fn one_changed_current_fact_rejects_each_operation_before_publication()
    -> Result<(), Box<dyn Error>> {
        for kind in [Kind::Mint, Kind::Transfer] {
            let (baseline, intent, facts) = prepared(kind)?;
            let bad_world = WorldId::from_bytes(uuid_fixture(9))?;
            for wrong in [
                CurrentFacts {
                    world: bad_world,
                    ..facts
                },
                CurrentFacts {
                    definition_key: 2,
                    ..facts
                },
                CurrentFacts {
                    definition_revision: 2,
                    ..facts
                },
                CurrentFacts {
                    authority_generation: 2,
                    ..facts
                },
                CurrentFacts {
                    corpse_revision: 2,
                    ..facts
                },
            ] {
                let mut model = baseline.clone();
                assert_eq!(
                    model.attempt(
                        Request::OneItem(intent.clone()),
                        wrong,
                        audit_probe(),
                        evidence_limits()?,
                        Attempt::Acknowledged
                    ),
                    Err(Failure::Facts)
                );
                assert_eq!(model, baseline);
            }
        }
        Ok(())
    }

    #[test]
    fn lost_response_duplicates_and_ghost_projection_never_reapply() -> Result<(), Box<dyn Error>> {
        let (mint, transfer, facts) = fixture()?;
        let mut model = Model::default();
        let limits = evidence_limits()?;
        for intent in [&mint, &transfer] {
            let id = intent.item.id();
            assert_eq!(
                model.attempt(
                    Request::OneItem(intent.clone()),
                    facts,
                    audit_probe(),
                    limits,
                    Attempt::CommittedResponseLost
                )?,
                ResultState::Held(id)
            );
            let custody = model.custody.clone();
            assert_eq!(
                model.attempt(
                    Request::OneItem(intent.clone()),
                    facts,
                    audit_probe(),
                    limits,
                    Attempt::Acknowledged
                )?,
                ResultState::Held(id)
            );
            assert_eq!(model.custody, custody);
            assert_eq!(
                model.reconcile(intent.transaction, limits, Reconcile::InspectCommittedStore)?,
                ResultState::Committed(id)
            );
            assert_eq!(model.custody, custody);
        }
        let snapshot = model.clone();
        let mut remint = mint;
        remint.transaction = EvidenceTransaction::new(uuid_fixture(8))?;
        assert_eq!(
            model.attempt(
                Request::OneItem(remint),
                facts,
                audit_probe(),
                limits,
                Attempt::Acknowledged
            ),
            Err(Failure::Conflict)
        );
        assert_eq!(model, snapshot);
        let (item, location) = model.custody.as_ref().ok_or(Failure::Custody)?;
        assert_eq!(item.item.id(), transfer.item.id());
        assert_eq!(item.quantity, 1);
        assert_eq!(location, &transfer.destination);
        Ok(())
    }

    #[test]
    fn unknown_ambiguous_outcome_holds_exact_candidate_then_known_abort_retries()
    -> Result<(), Box<dyn Error>> {
        for kind in [Kind::Mint, Kind::Transfer] {
            let (baseline, intent, facts) = prepared(kind)?;
            let mut model = baseline.clone();
            let mut limits = evidence_limits()?;
            limits.0[11] = 9;
            let id = intent.item.id();
            model.attempt(
                Request::OneItem(intent.clone()),
                facts,
                audit_probe(),
                limits,
                Attempt::AmbiguousWithoutCommit,
            )?;
            let index = model
                .record_index(intent.transaction)
                .ok_or(Failure::Conflict)?;
            let frozen = model.records[index]
                .as_ref()
                .ok_or(Failure::Conflict)?
                .plan
                .clone();
            assert_eq!(model.custody, baseline.custody);
            assert_eq!(
                model.reconcile(intent.transaction, limits, Reconcile::Unknown)?,
                ResultState::Held(id)
            );
            assert_eq!(model.custody, baseline.custody);
            assert_eq!(
                model.reconcile(intent.transaction, limits, Reconcile::InspectCommittedStore)?,
                ResultState::NotApplied(id)
            );
            assert_eq!(
                model.attempt(
                    Request::OneItem(intent.clone()),
                    facts,
                    audit_probe(),
                    limits,
                    Attempt::Acknowledged
                )?,
                ResultState::Committed(id)
            );
            assert_eq!(
                model.records[index].as_ref().ok_or(Failure::Conflict)?.plan,
                frozen
            );
            assert_eq!(
                model.custody.as_ref().ok_or(Failure::Custody)?.1,
                intent.destination
            );
        }
        Ok(())
    }

    #[test]
    fn retry_work_exact_limit_and_exhaustion_preserve_original_transaction()
    -> Result<(), Box<dyn Error>> {
        for kind in [Kind::Mint, Kind::Transfer] {
            let (mut model, intent, facts) = prepared(kind)?;
            let mut limits = evidence_limits()?;
            limits.0[11] = 3;
            model.attempt(
                Request::OneItem(intent.clone()),
                facts,
                audit_probe(),
                limits,
                Attempt::CommittedResponseLost,
            )?;
            model.reconcile(intent.transaction, limits, Reconcile::Unknown)?;
            let snapshot = model.clone();
            assert_eq!(
                model.reconcile(intent.transaction, limits, Reconcile::InspectCommittedStore),
                Err(Failure::Budget("cumulative_retry_work"))
            );
            assert_eq!(model, snapshot);
            limits.0[11] = 6;
            assert_eq!(
                model.reconcile(intent.transaction, limits, Reconcile::InspectCommittedStore)?,
                ResultState::Committed(intent.item.id())
            );
        }
        Ok(())
    }

    #[test]
    fn same_transaction_conflicting_intent_or_probe_rejects() -> Result<(), Box<dyn Error>> {
        let (mut model, mint, facts) = prepared(Kind::Mint)?;
        model.attempt(
            Request::OneItem(mint.clone()),
            facts,
            audit_probe(),
            evidence_limits()?,
            Attempt::CommittedResponseLost,
        )?;
        let snapshot = model.clone();
        let mut conflicting = mint.clone();
        conflicting.cause = 9;
        assert_eq!(
            model.attempt(
                Request::OneItem(conflicting),
                facts,
                audit_probe(),
                evidence_limits()?,
                Attempt::Acknowledged
            ),
            Err(Failure::Conflict)
        );
        assert_eq!(
            model.attempt(
                Request::OneItem(mint),
                facts,
                AuditBudgetProbe {
                    contributions: 2,
                    encoded_bytes: 16
                },
                evidence_limits()?,
                Attempt::Acknowledged
            ),
            Err(Failure::Conflict)
        );
        assert_eq!(model, snapshot);
        Ok(())
    }

    #[test]
    fn known_abort_retry_rechecks_audit_and_work_before_publication() -> Result<(), Box<dyn Error>>
    {
        for kind in [Kind::Mint, Kind::Transfer] {
            let (mut model, intent, facts) = prepared(kind)?;
            let limits = evidence_limits()?;
            model.attempt(
                Request::OneItem(intent.clone()),
                facts,
                audit_probe(),
                limits,
                Attempt::AmbiguousWithoutCommit,
            )?;
            model.reconcile(intent.transaction, limits, Reconcile::InspectCommittedStore)?;
            let snapshot = model.clone();
            for dimension in [7, 8, 11] {
                let mut exhausted = limits;
                exhausted.0[dimension] = 0;
                assert_eq!(
                    model.attempt(
                        Request::OneItem(intent.clone()),
                        facts,
                        audit_probe(),
                        exhausted,
                        Attempt::Acknowledged
                    ),
                    Err(Failure::Budget(DIMENSIONS[dimension]))
                );
                assert_eq!(model, snapshot);
            }
            assert_eq!(
                model.attempt(
                    Request::OneItem(intent.clone()),
                    facts,
                    audit_probe(),
                    limits,
                    Attempt::CommittedResponseLost
                )?,
                ResultState::Held(intent.item.id())
            );
            let record = model.records[model
                .record_index(intent.transaction)
                .ok_or(Failure::Conflict)?]
            .as_ref()
            .ok_or(Failure::Conflict)?;
            assert_eq!(record.observation, Observation::Ambiguous);
            assert_eq!(record.plan.intent.transaction, intent.transaction);
        }
        Ok(())
    }

    #[test]
    fn mismatched_ground_world_and_transfer_item_reject_without_custody_change()
    -> Result<(), Box<dyn Error>> {
        let (mut model, mut mint, facts) = prepared(Kind::Mint)?;
        if let Location::Ground { world_id, .. } = &mut mint.destination {
            *world_id = WorldId::from_bytes(uuid_fixture(9))?;
        }
        assert_eq!(
            model.attempt(
                Request::OneItem(mint),
                facts,
                audit_probe(),
                evidence_limits()?,
                Attempt::Acknowledged
            ),
            Err(Failure::Facts)
        );
        assert_eq!(model, Model::default());
        let (mut model, mut transfer, facts) = prepared(Kind::Transfer)?;
        let snapshot = model.clone();
        transfer.item = Item::new(
            ItemInstanceId::from_bytes(uuid_fixture(8))?,
            facts.world,
            ItemDefinitionRef::new(1, 1),
        );
        assert_eq!(
            model.attempt(
                Request::OneItem(transfer),
                facts,
                audit_probe(),
                evidence_limits()?,
                Attempt::Acknowledged
            ),
            Err(Failure::Custody)
        );
        assert_eq!(model, snapshot);
        Ok(())
    }

    #[test]
    fn checked_count_byte_and_retry_overflow_fail_without_publication() -> Result<(), Box<dyn Error>>
    {
        assert_eq!(add(u64::MAX, 1), Err(Failure::Overflow));
        assert_eq!(bytes(u64::MAX, size_of::<Effect>()), Err(Failure::Overflow));
        let (mut model, intent, facts) = prepared(Kind::Mint)?;
        let plan = model.plan(intent.clone(), audit_probe())?;
        assert_eq!(plan.usage(u64::MAX), Err(Failure::Overflow));
        assert_eq!(model, Model::default());
        model.attempt(
            Request::OneItem(intent.clone()),
            facts,
            audit_probe(),
            evidence_limits()?,
            Attempt::CommittedResponseLost,
        )?;
        let index = model
            .record_index(intent.transaction)
            .ok_or(Failure::Conflict)?;
        model.records[index]
            .as_mut()
            .ok_or(Failure::Conflict)?
            .retry_work = u64::MAX;
        let snapshot = model.clone();
        assert_eq!(
            model.reconcile(
                intent.transaction,
                evidence_limits()?,
                Reconcile::InspectCommittedStore
            ),
            Err(Failure::Overflow)
        );
        assert_eq!(model, snapshot);
        Ok(())
    }

    #[test]
    fn private_transaction_identity_rejects_wrong_version_and_variant() {
        assert_eq!(size_of::<EvidenceTransaction>(), 16);
        assert_eq!(EvidenceTransaction::new([0; 16]), Err(Failure::Identity));
        let mut value = uuid_fixture(4);
        value[6] = 0x40;
        assert_eq!(EvidenceTransaction::new(value), Err(Failure::Identity));
        value = uuid_fixture(4);
        value[8] = 0;
        assert_eq!(EvidenceTransaction::new(value), Err(Failure::Identity));
    }

    #[test]
    fn second_logical_transaction_cannot_replace_an_ambiguous_operation()
    -> Result<(), Box<dyn Error>> {
        for kind in [Kind::Mint, Kind::Transfer] {
            let (mut model, intent, facts) = prepared(kind)?;
            model.attempt(
                Request::OneItem(intent.clone()),
                facts,
                audit_probe(),
                evidence_limits()?,
                Attempt::AmbiguousWithoutCommit,
            )?;
            let snapshot = model.clone();
            let mut replacement = intent;
            replacement.transaction = EvidenceTransaction::new(uuid_fixture(9))?;
            replacement.cause = 9;
            assert_eq!(
                model.attempt(
                    Request::OneItem(replacement),
                    facts,
                    audit_probe(),
                    evidence_limits()?,
                    Attempt::Acknowledged
                ),
                Err(Failure::Conflict)
            );
            assert_eq!(model, snapshot);
        }
        Ok(())
    }

    #[test]
    fn transfer_requires_acknowledged_mint_not_only_a_ghost_corpse_projection()
    -> Result<(), Box<dyn Error>> {
        let (mint, transfer, facts) = fixture()?;
        let mut model = Model::default();
        let limits = evidence_limits()?;
        model.attempt(
            Request::OneItem(mint.clone()),
            facts,
            audit_probe(),
            limits,
            Attempt::CommittedResponseLost,
        )?;
        let snapshot = model.clone();
        assert_eq!(
            model.attempt(
                Request::OneItem(transfer.clone()),
                facts,
                audit_probe(),
                limits,
                Attempt::Acknowledged
            ),
            Err(Failure::Custody)
        );
        assert_eq!(model, snapshot);
        model.reconcile(mint.transaction, limits, Reconcile::InspectCommittedStore)?;
        assert_eq!(
            model.attempt(
                Request::OneItem(transfer.clone()),
                facts,
                audit_probe(),
                limits,
                Attempt::Acknowledged
            )?,
            ResultState::Committed(transfer.item.id())
        );
        Ok(())
    }

    #[test]
    fn measurement_target_identifies_executable_target_without_build_attestation() {
        let target = measurement_target();
        assert_eq!(target["os"], std::env::consts::OS);
        assert_eq!(target["architecture"], std::env::consts::ARCH);
        assert_eq!(target["build_toolchain_identity"], "UNKNOWN");
        assert_eq!(
            target["build_toolchain_identity_reason"],
            "This standalone example has no build-time compiler attestation"
        );
        assert!(target.get("rustc_identity_at_execution").is_none());
        assert!(target.get("toolchain_identity_scope").is_none());
    }

    #[test]
    fn normalization_is_byte_stable_under_reordered_fixture_presentation()
    -> Result<(), Box<dyn Error>> {
        assert_eq!(normalized_report(false)?, normalized_report(false)?);
        assert_eq!(normalized_report(false)?, normalized_report(true)?);
        let report: Value = serde_json::from_str(&normalized_report(false)?)?;
        assert_eq!(report["natural_drop_probability"], "UNKNOWN/NOT_ASSERTED");
        assert_eq!(
            report["operations"][0]["mandatory_audit"]["event_contribution_count"],
            Value::Null
        );
        Ok(())
    }
}
