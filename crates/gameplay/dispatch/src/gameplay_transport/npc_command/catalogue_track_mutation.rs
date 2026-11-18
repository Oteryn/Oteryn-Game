//! Keep complete atomic track writes whenever any source value actually changes.
use oteryn_gameplay_quests::quest::npc_rules::NpcRulePlan;

pub(super) fn has_mutation(plan: &NpcRulePlan) -> bool {
    plan.requires_draw
        || plan
            .track_changes
            .iter()
            .any(|change| change.before != change.after)
}
