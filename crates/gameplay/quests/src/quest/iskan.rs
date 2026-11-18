//! Atomic Iskan mission continuation. Crystal 201189f8 data-global/npc/iskan.lua:72-118.
use super::{QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition};

pub const NPC: &str = "oteryn:npc.iskan";
pub const BARBARIAN: &str = super::barbarian_mead::TRACKS[0];
pub const QUESTLINE: &str = super::ice_islands_seals::QUESTLINE;
pub const MISSION: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/mission01";
pub const HUSKY_KILL: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/husky_kill";
pub const ACCEPT: &str = "oteryn:quest-transition/ice-islands/iskan-accept";
pub const REPORT: &str = "oteryn:quest-transition/ice-islands/iskan-report";

pub(super) fn extend(mut catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let quest = super::ice_islands_seals::QUEST;
    let previous = catalogue
        .definition_hash(quest)
        .ok_or("missing Ice Islands definition")?;
    let mut transitions = Vec::new();
    let set = |track: &str, from, value| QuestEffect {
        track: track.into(),
        from,
        effect: QuestEffectKind::Set(value),
    };
    for (key, before, after) in [(ACCEPT, 1, 1), (REPORT, 2, 3)] {
        let mut effects = vec![
            QuestEffect {
                track: BARBARIAN.into(),
                from: QuestComparison::Ge(8),
                effect: QuestEffectKind::RaiseTo(8),
            },
            set(QUESTLINE, QuestComparison::Eq(before), after),
            set(MISSION, QuestComparison::Any, after),
        ];
        if key == ACCEPT {
            effects.push(QuestEffect {
                track: HUSKY_KILL.into(),
                from: QuestComparison::Lt(1),
                effect: QuestEffectKind::Add(0),
            });
        }
        transitions.push(QuestTransition {
            key: key.into(),
            quest: quest.into(),
            effects,
            completes: false,
            experience: None,
            world_change: None,
        });
    }
    let checked = QuestStateCatalogue::new(
        catalogue.content_revision.clone(),
        catalogue.tracks.values().cloned().collect(),
        catalogue
            .transitions
            .values()
            .cloned()
            .chain(transitions.iter().cloned())
            .collect(),
    )
    .map_err(|error| format!("{error:?}"))?;
    let hash = checked
        .definition_hash(quest)
        .ok_or("missing Iskan definition")?;
    catalogue.hashes.insert(quest.into(), hash);
    catalogue.transitions.extend(
        transitions
            .into_iter()
            .map(|transition| (transition.key.clone(), transition)),
    );
    catalogue.predecessors = catalogue.predecessors;
    catalogue
        .predecessors
        .entry(quest.into())
        .or_default()
        .push(super::NpcDefinitionPredecessor {
            hash: previous,
            defaults: catalogue
                .tracks
                .values()
                .filter(|track| track.quest == quest)
                .map(|track| (track.key.clone(), track.initial))
                .collect(),
        });
    Ok(catalogue)
}
