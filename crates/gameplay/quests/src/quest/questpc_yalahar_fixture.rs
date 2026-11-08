//! Component-test stage arrangement preserving all production compatibility history.
use super::*;

pub fn with_yalahar_stages(catalogue: &QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for stage in [20, 21, 72] {
        transitions.push(QuestTransition {
            world_change: None,
            key: format!("oteryn:quest-transition/test/questpc-yalahar-stage-{stage}"),
            quest: "oteryn:quest.in_service_of_yalahar_quest".into(),
            effects: vec![QuestEffect {
                track: "oteryn:quest-progress/quest/u8_4/in_service_of_yalahar/questline".into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(stage),
            }],
            completes: false,
            experience: None,
        });
    }
    let next = QuestStateCatalogue::new(
        catalogue.content_revision.clone(),
        catalogue.tracks.values().cloned().collect(),
        transitions,
    )
    .map_err(|error| format!("{error:?}"))?;
    catalogue.with_additive_extension(next)
}
