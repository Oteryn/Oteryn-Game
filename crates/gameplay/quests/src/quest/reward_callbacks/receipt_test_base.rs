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
        "oteryn:quest-progress/quest/u7_24/the_white_raven_monastery/diary",
        "oteryn:quest-progress/quest/u7_24/the_white_raven_monastery/diary_retrieved",
        "oteryn:quest-progress/quest/u7_24/the_white_raven_monastery/door",
        "oteryn:quest-progress/quest/u7_24/the_white_raven_monastery/quest_log",
    ]
    .into_iter()
    .map(|key| source.track(key).cloned().ok_or_else(|| key.to_owned()))
    .collect::<Result<Vec<_>, _>>()?;
    let transitions = [
        dream::COMPLETE,
        "oteryn:quest-callback/white-raven-monks-diary",
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
