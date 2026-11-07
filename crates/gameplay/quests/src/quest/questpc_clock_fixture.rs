//! Citizen clock arrangement without removing production tracks or compatibility history.
use super::*;

pub fn with_citizen_clock(
    catalogue: &QuestStateCatalogue,
    at: i64,
) -> Result<QuestStateCatalogue, String> {
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    transitions.push(QuestTransition {
        key: "oteryn:quest-transition/test/questpc-citizen-clock".into(),
        quest: "oteryn:quest.outfit_and_addon_quests".into(),
        effects: vec![QuestEffect {
            track: "oteryn:quest-progress/quest/u7_8/citizen_outfits/addon_backpack_timer".into(),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Set(at),
        }],
        completes: false,
        experience: None,
    });
    let next = QuestStateCatalogue::new(
        catalogue.content_revision.clone(),
        catalogue.tracks.values().cloned().collect(),
        transitions,
    )
    .map_err(|error| format!("{error:?}"))?;
    catalogue.with_additive_extension(next)
}
