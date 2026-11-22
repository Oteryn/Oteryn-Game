//! Test catalogue containing the production owners of receipt-gated rewards.
use super::super::{QuestStateCatalogue, loader, wrath_sleeping_dragon as dream};

pub(super) fn catalogue() -> Result<QuestStateCatalogue, String> {
    let base = super::super::grizzly_adams::extend(
        QuestStateCatalogue::empty("receipt-predicate-test").ok_or("empty catalogue")?,
    )?;
    let loaded = loader::load_embedded_quest_state("receipt-predicate-test")
        .map_err(|error| format!("{error:?}"))?;
    let source = loaded.catalogue();
    let tracks = [
        dream::QUESTLINE,
        dream::MISSION,
        dream::POTION,
        "oteryn:quest-progress/authored/nomads_land_quest/s4_window_obscured",
        "oteryn:quest-progress/quest/u8_1/tibia_tales/nomads_land_cabinet",
        "oteryn:quest-progress/quest/u8_1/tibia_tales/nomads_land_exit_reached",
    ]
    .into_iter()
    .map(|key| source.track(key).cloned().ok_or_else(|| key.to_owned()))
    .collect::<Result<Vec<_>, _>>()?;
    let transitions = [
        dream::COMPLETE,
        "oteryn:quest-callback/engenc2/nomads-land-window",
    ]
    .into_iter()
    .map(|key| {
        source
            .transition(key)
            .cloned()
            .ok_or_else(|| key.to_owned())
    })
    .collect::<Result<Vec<_>, _>>()?;
    base.with_quest(tracks, transitions)
        .map_err(|error| format!("{error:?}"))
}
