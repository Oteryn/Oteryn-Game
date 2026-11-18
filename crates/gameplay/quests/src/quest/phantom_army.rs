//! Survivor settlement for Crystal 201189f8 movements_click.lua:29-44.
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};

pub const QUESTLINE: &str =
    "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/questline";
pub const SURVIVED: &str =
    "oteryn:quest-transition/children-of-the-revolution/phantom-army-survived";

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let quest = catalogue
        .track(QUESTLINE)
        .ok_or("missing Children questline")?
        .quest
        .clone();
    let transition = QuestTransition {
        key: SURVIVED.into(),
        quest,
        effects: vec![QuestEffect {
            track: QUESTLINE.into(),
            from: QuestComparison::Eq(19),
            effect: QuestEffectKind::Set(20),
        }],
        completes: false,
        experience: None,
        world_change: None,
    };
    let mut active = QuestStateCatalogue::new(
        catalogue.content_revision,
        catalogue.tracks.into_values().collect(),
        catalogue
            .transitions
            .into_values()
            .chain([transition])
            .collect(),
    )
    .map_err(|error| format!("{error:?}"))?;
    active.npc_legacy = catalogue.npc_legacy;
    active.predecessors = catalogue.predecessors;
    Ok(active)
}
