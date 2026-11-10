//! Test catalogue containing both production owners of receipt-gated rewards.
use super::super::{QuestStateCatalogue, loader, wrath_sleeping_dragon as dream};

pub(super) fn catalogue() -> Result<QuestStateCatalogue, String> {
    let base = super::super::grizzly_adams::extend(
        QuestStateCatalogue::empty("receipt-predicate-test").ok_or("empty catalogue")?,
    )?;
    let loaded = loader::load_embedded_quest_state("receipt-predicate-test")
        .map_err(|error| format!("{error:?}"))?;
    let source = loaded.catalogue();
    let tracks = [dream::QUESTLINE, dream::MISSION, dream::POTION]
        .into_iter()
        .map(|key| source.track(key).cloned().ok_or_else(|| key.to_owned()))
        .collect::<Result<Vec<_>, _>>()?;
    let finale = source
        .transition(dream::COMPLETE)
        .cloned()
        .ok_or("dream finale")?;
    base.with_quest(tracks, vec![finale])
        .map_err(|error| format!("{error:?}"))
}
