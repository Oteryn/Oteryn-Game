//! Authored Drome facts with synthetic speed and appearance for the composed wave regression.
use super::{TestResult, debug};
use crate::foundation::{ChannelRuntimeV1, CompiledCreaturePolicies, ExactActorRef};

use oteryn_gameplay_content::content::{
    ProjectV2AuthoringProfile, ProjectV2AuthoringProfileData, ProjectV2CandidateValue,
    validate_native_gameplay_profile,
};
use oteryn_gameplay_entities::ConditionType;

const CANDIDATES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../../docs/reference/content-prep/arena-rules-content-1/admission-evidence/tibiadrome-creature-candidates.json"
));

const EXPECTED: [(&str, i64); 5] = [
    ("oteryn:creature.domestikion", 1100),
    ("oteryn:creature.hoodinion", 1173),
    ("oteryn:creature.mearidion", 1246),
    ("oteryn:creature.murmillion", 1613),
    ("oteryn:creature.scissorion", 1393),
];

pub(super) fn fixture_policies(
    original: &CompiledCreaturePolicies,
) -> TestResult<CompiledCreaturePolicies> {
    let mut rows = original
        .policies()
        .iter()
        .map(|row| row.as_ref().clone())
        .collect::<Vec<_>>();
    // Test-only infrastructure for fields Global does not publish. There are no
    // attacks or placements authored from this table, and it is never activated.
    let base = rows
        .iter()
        .find(|row| row.definition_key == "oteryn:creature.rat")
        .ok_or("fixture rat policy")?
        .clone();
    let candidates: Vec<ProjectV2AuthoringProfile> = serde_json::from_str(CANDIDATES)?;
    assert_eq!(candidates.len(), EXPECTED.len());
    for candidate in candidates {
        validate_native_gameplay_profile(&candidate).map_err(debug)?;
        let ProjectV2AuthoringProfileData::Creature(profile) = candidate.data else {
            return Err("Drome creature candidate".into());
        };
        let bool_fact = |path: &str| -> TestResult<bool> {
            profile
                .fields
                .iter()
                .find_map(|field| match &field.value {
                    ProjectV2CandidateValue::Boolean(value) if field.field_path == path => {
                        Some(*value)
                    }
                    _ => None,
                })
                .ok_or_else(|| format!("missing Drome boolean fact {path}").into())
        };
        let mut row = base.clone();
        row.definition_key = candidate.target.key;
        row.definition_revision = candidate.target.revision;
        row.display_name = row.definition_key.clone();
        row.maximum_health = i64::try_from(profile.health.ok_or("Drome base HP")?)?;
        row.flags.illusionable = bool_fact("oteryn:creature.flags.illusionable")?;
        row.summonable = bool_fact("oteryn:creature.summoning.summonable")?;
        row.convinceable = bool_fact("oteryn:creature.summoning.convinceable")?;
        row.mana_cost = None;
        row.condition_immunities.clear();
        if bool_fact("oteryn:creature.condition_immunities.paralysis")? {
            row.condition_immunities.push(ConditionType::Paralysis);
        }
        rows.push(row);
    }
    CompiledCreaturePolicies::from_active_artifact(original.source_digest(), rows).map_err(debug)
}

pub(super) fn assert_wave(
    runtime: &ChannelRuntimeV1,
    targets: &[ExactActorRef],
    wave: u32,
) -> TestResult {
    for &target in targets {
        let snapshot = runtime.companion_snapshot(target).map_err(debug)?;
        let key = &snapshot.state.policy.definition_key;
        let (_, expected) = EXPECTED
            .iter()
            .find(|(native, _)| *native == key.as_str())
            .ok_or_else(|| {
                format!("Drome spawned an opponent outside the five native keys: {key}")
            })?;
        let policy = &snapshot.state.policy;
        assert!(
            policy
                .condition_immunities
                .contains(&ConditionType::Paralysis),
            "{key}"
        );
        assert!(
            !policy.flags.illusionable && !policy.summonable && !policy.convinceable,
            "{key}"
        );
        assert_eq!(
            (snapshot.health, snapshot.maximum_health),
            (*expected, *expected),
            "{key} in wave {wave}"
        );
    }
    Ok(())
}
