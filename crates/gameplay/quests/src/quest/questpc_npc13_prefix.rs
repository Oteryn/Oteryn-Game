//! NPC13 precedes the QENGINE reward callbacks and QNPCFX flag-bound tightening.
//! Reconstruct that validated source cohort; never accept a caller-supplied hash.
use super::{NpcDefinitionPredecessor, QuestStateCatalogue};
const QUEST: &str = "oteryn:quest.tibia_tales";
const LATER: [&str; 2] = [
    "oteryn:quest-reward/hallowed-holy-water",
    "oteryn:quest-reward/outfox-helmet",
];
pub(super) fn retain(
    mut current: QuestStateCatalogue,
    layer: u8,
) -> Result<QuestStateCatalogue, String> {
    let mut tracks = current.tracks.values().cloned().collect::<Vec<_>>();
    for suffix in ["01", "06", "08"] {
        let key =
            format!("oteryn:quest-progress/quest/u11_50/hidden_threats/corym_rescued{suffix}");
        let track = tracks
            .iter_mut()
            .find(|t| t.key == key)
            .ok_or("NPC13 source flag")?;
        track.min = -1;
    }
    let transitions = current
        .transitions
        .values()
        .filter(|t| !LATER.contains(&t.key.as_str()))
        .cloned()
        .collect();
    let mut prior = QuestStateCatalogue::new(current.content_revision.clone(), tracks, transitions)
        .map_err(|e| format!("{e:?}"))?;
    prior.npc_legacy.clone_from(&current.npc_legacy);
    prior.predecessors.clone_from(&current.predecessors);
    if layer == 0 {
        return Ok(prior);
    }
    let hash = prior
        .definition_hash(QUEST)
        .ok_or("NPC13 historical definition")?;
    let defaults = prior
        .tracks
        .values()
        .filter(|t| t.quest == QUEST)
        .map(|t| (t.key.clone(), t.initial))
        .collect();
    let predecessors = current.predecessors.entry(QUEST.into()).or_default();
    if !predecessors.iter().any(|p| p.hash == hash) {
        predecessors.insert(0, NpcDefinitionPredecessor { hash, defaults });
    }
    Ok(current)
}
