//! Shared quest fixture: derive one XP-bearing transition from real admitted content.
use super::TestResult;
use super::fixture::debug;
use oteryn_gameplay_quests::quest::{QuestComparison, QuestStateCatalogue};
use std::sync::Arc;

pub(super) const TRANSITION: &str =
    "oteryn:quest-transition/quest/u8_6/afathers_burden/quest_log/npc_1";
pub(super) fn catalogue() -> TestResult<Arc<QuestStateCatalogue>> {
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let key = TRANSITION;
    let mut transition = lowered
        .catalogue()
        .transition(key)
        .ok_or("real transition")?
        .clone();
    transition.experience = Some(50);
    transition.effects[0].from = QuestComparison::Eq(0);
    let tracks = transition
        .effects
        .iter()
        .map(|effect| {
            lowered
                .catalogue()
                .track(&effect.track)
                .cloned()
                .ok_or("real track")
        })
        .collect::<Result<Vec<_>, _>>()?;
    let catalogue =
        Arc::new(QuestStateCatalogue::new("content-1", tracks, vec![transition]).map_err(debug)?);
    Ok(catalogue)
}
