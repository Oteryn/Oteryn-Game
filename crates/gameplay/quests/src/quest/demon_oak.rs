//! Character-owned Demon Oak cuts from Crystal's four independent axe-blow storages.
use super::{
    NpcDefinitionPredecessor, QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue,
    QuestTrack, QuestTransition,
};

pub const QUEST: &str = "oteryn:quest.demon_oak";
pub const DONE: &str = "oteryn:quest-progress/quest/u8_2/the_demon_oak/done";
pub const PROGRESS: &str = "oteryn:quest-progress/quest/u8_2/the_demon_oak/progress";
pub const AXE: &str = "oteryn:item.tibia.i919";
pub const CUT_TRACKS: [&str; 4] = [
    "oteryn:quest-progress/quest/u8_2/the_demon_oak/axe_blows_bird",
    "oteryn:quest-progress/quest/u8_2/the_demon_oak/axe_blows_left",
    "oteryn:quest-progress/quest/u8_2/the_demon_oak/axe_blows_right",
    "oteryn:quest-progress/quest/u8_2/the_demon_oak/axe_blows_face",
];
pub const CUT_TRANSITIONS: [&str; 4] = [
    "oteryn:quest-transition/demon-oak/axe-blows-bird",
    "oteryn:quest-transition/demon-oak/axe-blows-left",
    "oteryn:quest-transition/demon-oak/axe-blows-right",
    "oteryn:quest-transition/demon-oak/axe-blows-face",
];
pub const FINISH: &str = "oteryn:quest-transition/demon-oak/finish";
pub const FINISHED_CUTS: i64 = 11;

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let predecessor = NpcDefinitionPredecessor {
        hash: catalogue
            .definition_hash(QUEST)
            .ok_or("Demon Oak definition")?,
        defaults: catalogue
            .tracks
            .values()
            .filter(|track| track.quest == QUEST)
            .map(|track| (track.key.clone(), track.initial))
            .collect(),
    };
    let tracks = CUT_TRACKS.map(|key| QuestTrack {
        key: key.into(),
        quest: QUEST.into(),
        initial: 0,
        min: 0,
        max: FINISHED_CUTS,
    });
    let effect = |track: &str, from, effect| QuestEffect {
        track: track.into(),
        from,
        effect,
    };
    let transition = |key: &str, effects| QuestTransition {
        key: key.into(),
        quest: QUEST.into(),
        effects,
        completes: false,
        experience: None,
        world_change: None,
    };
    let cuts = CUT_TRANSITIONS
        .into_iter()
        .zip(CUT_TRACKS)
        .map(|(key, track)| {
            transition(
                key,
                vec![
                    effect(DONE, QuestComparison::Lt(1), QuestEffectKind::Add(0)),
                    effect(
                        track,
                        QuestComparison::Lt(FINISHED_CUTS),
                        QuestEffectKind::AddAtLeast {
                            delta: 1,
                            minimum: 2,
                        },
                    ),
                ],
            )
        });
    let finish = transition(
        FINISH,
        CUT_TRACKS
            .into_iter()
            .map(|track| {
                effect(
                    track,
                    QuestComparison::Eq(FINISHED_CUTS),
                    QuestEffectKind::Add(0),
                )
            })
            .chain([
                effect(DONE, QuestComparison::Lt(1), QuestEffectKind::Set(1)),
                effect(PROGRESS, QuestComparison::Any, QuestEffectKind::Set(3)),
            ])
            .collect(),
    );
    let mut updated = QuestStateCatalogue::new(
        catalogue.content_revision,
        catalogue.tracks.values().cloned().chain(tracks).collect(),
        catalogue
            .transitions
            .values()
            .cloned()
            .chain(cuts)
            .chain([finish])
            .collect(),
    )
    .map_err(|error| format!("{error:?}"))?;
    updated.hashes.extend(
        catalogue
            .hashes
            .into_iter()
            .filter(|(quest, _)| quest != QUEST),
    );
    updated.completion_sets = catalogue.completion_sets;
    updated.authored_bindings = catalogue.authored_bindings;
    updated.native_recipe_evidence = catalogue.native_recipe_evidence;
    updated.npc_legacy = catalogue.npc_legacy;
    updated.predecessors = catalogue.predecessors;
    updated
        .predecessors
        .entry(QUEST.into())
        .or_default()
        .push(predecessor);
    Ok(updated)
}
