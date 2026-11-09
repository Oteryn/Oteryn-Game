//! Reuse the quest writer's already locked authoritative progression sample.
use oteryn_gameplay_quests::quest::QuestStateCatalogue;

pub(super) fn permits_level(catalogue: &QuestStateCatalogue, quest: &str, level: i64) -> bool {
    catalogue
        .authored_bindings()
        .and_then(|bindings| {
            bindings
                .quests
                .iter()
                .find(|binding| binding.quest == quest)
        })
        .and_then(|binding| binding.admission.as_ref())
        .is_none_or(|admission| level >= i64::from(admission.minimum_level))
}
