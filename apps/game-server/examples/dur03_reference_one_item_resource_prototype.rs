//! Nonproduction DUR-03 resource evidence for deterministic Gold Coin x1.
//! No database, runtime activation, production maxima, event family or wire schema.

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
        match plan.intent.kind {
            Kind::Mint if self.custody.is_none() => {}
            Kind::Transfer => {
                let source = plan.effects[0].as_ref().ok_or(Failure::Custody)?;
                if self.custody.as_ref()
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

fn measurements(plan: &Plan) -> Result<Value, Failure> {
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
        "retry_reconcile_work_units_per_call": 3,
        "effects": effects,
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
        "encoding": "explicit example-only big-endian record encoding"
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
    let mut plans = model
        .records
        .iter()
        .flatten()
        .map(|record| &record.plan)
        .collect::<Vec<_>>();
    if reverse {
        plans.reverse();
    }
    // Sort presentation, never execution, by explicit mutation class order.
    plans.sort_by_key(|plan| plan.intent.kind.tag());
    let operations = plans
        .into_iter()
        .map(measurements)
        .collect::<Result<Vec<_>, _>>()?;
    let report = json!({
        "schema_version": 1,
        "classification": "NON_PRODUCTION_RESOURCE_EVIDENCE_ONLY",
        "task_id": "OTV2-20260917-dur03-reference-one-item-resource-evidence-513",
        "source_base_sha": "fd504f5659fe900861b6556751275488c2a22dec",
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
            "DUR03-RL-07": "EVIDENCE_GAP for actual mandatory count/payload bytes; synthetic budget preflight only",
            "DUR03-RL-08": "MEASURED_IN_MEMORY_STATE_MACHINE_ONLY; stable transaction/output identities; no DB/restart proof"
        },
        "evidence_gaps": [
            "registered DUR-03 mandatory audit event contribution count and exact payload/envelope byte total",
            "production transaction identity allocator, PostgreSQL receipts/atomic commit/isolation/fencing and crash recovery",
            "production resource maxima, replay/retention horizon and connected Combat/loot/pickup integration",
            "real Content definition mapping, natural Rat loot probability and Reference parity"
        ]
    });
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
