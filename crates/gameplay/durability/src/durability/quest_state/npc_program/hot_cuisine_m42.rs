//! Adapt the existing UTC cooking planner to the fenced source-NPC owner.
use super::*;
use oteryn_gameplay_quests::quest::{
    hot_cuisine::{HotCuisineProgram, YearMonth},
    npc_rules::NpcRuleError,
};

pub(super) fn owns(request: &NpcProgramRequest) -> bool {
    request.npc == oteryn_gameplay_quests::quest::hot_cuisine::NPC
        && request.trigger == NpcRuleTrigger::Say
        && HotCuisineProgram::embedded().is_ok_and(|p| p.is_recipe_topic(request.topic))
}

pub(super) fn optimistic() -> std::result::Result<NpcInventorySnapshot, NpcRuleError> {
    HotCuisineProgram::embedded()?.optimistic_inventory()
}

pub(super) fn keys() -> std::result::Result<Vec<String>, NpcRuleError> {
    Ok(optimistic()?
        .items
        .into_keys()
        .map(|key| key.item)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect())
}

pub(super) fn plan(
    source: &NpcRuleCatalog,
    input: &NpcRuleInput<'_>,
) -> std::result::Result<Option<NpcRulePlan>, NpcRuleError> {
    let native = HotCuisineProgram::embedded()?;
    let generic = source
        .program(oteryn_gameplay_quests::quest::hot_cuisine::QUEST)
        .ok_or_else(|| {
            NpcRuleError::MissingNativeResolution("Hot Cuisine source programme".into())
        })?;
    let calendar = native.calendar_snapshot(input, YearMonth::from_unix_seconds)?;
    if let Some(plan) = native.plan(generic, input, &calendar)? {
        return Ok(Some(plan));
    }
    // Menus/cancellation still belong to the ordinary source dialogue. A
    // denied native callback cannot fall through into an unfenced item effect.
    match source.plan_storage(input) {
        Err(NpcRuleError::MissingNativeResolution(_)) => Ok(None),
        result => result,
    }
}

/// A permitted cook increments its own persisted count; each occurrence has one MINT.
/// Other source NPC rewards keep their original once-only claim identity.
pub(in crate::durability) fn claim_key(
    plan: &NpcRulePlan,
) -> std::result::Result<String, DurabilityError> {
    let rule = plan
        .selected_rule_keys
        .first()
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut input = rule.clone();
    if plan.quest == oteryn_gameplay_quests::quest::hot_cuisine::QUEST
        && let Some(index) =
            rule.strip_prefix("oteryn:quest-npc/hot_cuisine_quest/jean_pierre/native-cook-")
    {
        let index: u8 = index
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        if !(1..=15).contains(&index) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let key = format!(
            "oteryn:quest-progress/quest/u8_5/hot_cuisine_quest/kv/cookcount-{}",
            index + 1
        );
        let changes: Vec<_> = plan
            .track_changes
            .iter()
            .filter(|change| change.key == key)
            .collect();
        let [change] = changes.as_slice() else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if change.before < 0 || change.before.checked_add(1) != Some(change.after) {
            return Err(DurabilityError::InvalidStoredState);
        }
        input.push_str(&format!("/{}", change.after));
    }
    let hash = Sha256::digest(input.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(format!("oteryn:reward-claim.npc-program.{hash}"))
}
