//! Replay the ordered, pre-draw Forge census against the reported selection.
use super::TestResult;
use crate::node::{operator_files::hex_bytes, uuid_bytes};
use oteryn_gameplay_domain::domain::forge_creature::{
    FIENDISH_RESPAWN_MICROS, FIENDISH_SIMULTANEOUS_MAX, ForgeCreature, ForgeSpawnCandidate,
    INFLUENCED_SIMULTANEOUS_MAX, forge_population_roll, influenced_available_slots,
    plan_fiendish_tick, plan_influenced_tick,
};
use serde::Deserialize;
use serde_json::Value;

const FIRST_CYCLE: u64 = 10_000_000;

#[derive(Deserialize)]
struct DrawEvidence {
    now_micros: u64,
    due_micros: u64,
    seed: u64,
    artifact_digest: String,
    fiendish_slots: Vec<FiendishSlotEvidence>,
    candidates: Vec<CandidateEvidence>,
}

#[derive(Deserialize)]
struct FiendishSlotEvidence {
    replacement_due_micros: Option<u64>,
}

#[derive(Deserialize)]
struct CandidateEvidence {
    id: String,
    generation: u64,
    spawn_identity: u64,
    stacks: Option<u8>,
    owned: bool,
    forge_difficulty: Option<u8>,
    creature_key: String,
    position: Value,
}

impl CandidateEvidence {
    fn selection(&self, kind: ForgeCreature) -> Value {
        serde_json::json!({
            "id": self.id,
            "generation": self.generation,
            "stacks": kind.reward_stacks(),
            "creature_key": self.creature_key,
            "position": self.position,
        })
    }
}

// Replay uses the ordered pre-draw snapshots. The reported selection, vacancy
// and allowances are assertions, never inputs to the plan.
pub(crate) fn assert_replayed_selection(census: &Value) -> TestResult {
    let evidence = DrawEvidence::deserialize(census)?;
    assert!(evidence.now_micros >= evidence.due_micros);
    assert_eq!(census["fiendish_cap"], FIENDISH_SIMULTANEOUS_MAX);
    assert_eq!(census["influenced_cap"], INFLUENCED_SIMULTANEOUS_MAX);
    assert_eq!(
        census["influenced_per_tick_cap"],
        influenced_available_slots(0)
    );
    assert_eq!(census["fiendish_respawn_micros"], FIENDISH_RESPAWN_MICROS);
    assert_eq!(census["population_tick_micros"], FIRST_CYCLE);

    let artifact =
        hex_bytes::<32>(&evidence.artifact_digest).map_err(|error| format!("{error:?}"))?;
    let held = evidence
        .fiendish_slots
        .iter()
        .filter(|slot| {
            slot.replacement_due_micros
                .is_none_or(|due| evidence.now_micros < due)
        })
        .count();
    let available = usize::from(FIENDISH_SIMULTANEOUS_MAX).saturating_sub(held);
    assert_eq!(census["fiendish_available_slots"], available);

    let fiendish = ForgeCreature::Fiendish.reward_stacks();
    let mut existing = Vec::new();
    let mut candidates = Vec::new();
    let mut eligible = Vec::new();
    let mut influenced_count = 0;
    for candidate in &evidence.candidates {
        let is_fiendish = candidate.stacks == Some(fiendish);
        let is_influenced = candidate.stacks.is_some() && !is_fiendish;
        let unowned_forgeable = !candidate.owned && candidate.forge_difficulty.is_some();
        if is_fiendish {
            existing.push(candidate.spawn_identity);
        }
        influenced_count += usize::from(is_influenced);
        let is_eligible = unowned_forgeable && !is_influenced;
        if is_eligible {
            eligible.push(serde_json::json!({
                "id": candidate.id,
                "generation": candidate.generation,
                "spawn_identity": candidate.spawn_identity,
            }));
        }
        candidates.push(ForgeSpawnCandidate {
            spawn_identity: candidate.spawn_identity,
            eligible: is_eligible,
            alive: true,
        });
    }

    assert_eq!(census["eligible"], Value::Array(eligible));
    let plan = plan_fiendish_tick(evidence.now_micros, evidence.seed, &existing, &candidates);
    assert_eq!(census["fiendish_vacancy"], plan.vacancy);
    let additions: Vec<_> = plan
        .selected
        .into_iter()
        .filter(|identity| !existing.contains(identity))
        .take(available)
        .collect();
    let mut selected = Vec::new();
    for identity in &additions {
        let candidate = evidence
            .candidates
            .iter()
            .find(|candidate| candidate.spawn_identity == *identity)
            .ok_or("missing planned fiendish spawn")?;
        selected.push(candidate.selection(ForgeCreature::Fiendish));
    }

    let influenced_available = influenced_available_slots(influenced_count);
    assert_eq!(census["influenced_count"], influenced_count);
    assert_eq!(census["influenced_available_slots"], influenced_available);
    let mut rolls = Vec::new();
    for candidate in &evidence.candidates {
        if candidate.owned
            || candidate.forge_difficulty.is_none()
            || candidate.stacks.is_some()
            || additions.contains(&candidate.spawn_identity)
        {
            continue;
        }
        let placement = uuid_bytes(&candidate.id).ok_or("invalid candidate placement identity")?;
        let roll = forge_population_roll(
            artifact,
            placement,
            candidate.generation,
            Some(evidence.seed),
        );
        rolls.push((roll, candidate));
    }
    if let Some((candidate, kind)) = plan_influenced_tick(influenced_available, rolls.into_iter()) {
        selected.push(candidate.selection(kind));
    }
    assert_eq!(census["selected"], Value::Array(selected));
    Ok(())
}
