use std::collections::BTreeSet;

use super::*;
use crate::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};

/// Replace only Grizzly-owned track bounds and add contextual action declarations. The
/// old single-write source fragments cannot bypass the new compound rule evaluation.
pub fn extend(mut catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let data = super::catalogue().map_err(|error| format!("{error:?}"))?;
    let owned: BTreeSet<_> = data.storages.iter().map(|row| row.key.as_str()).collect();
    for storage in &data.storages {
        if catalogue
            .tracks
            .get(&storage.key)
            .is_some_and(|track| track.quest != QUEST)
        {
            return Err(format!(
                "Grizzly storage has another owner: {}",
                storage.key
            ));
        }
        catalogue.tracks.insert(
            storage.key.clone(),
            QuestTrack {
                key: storage.key.clone(),
                quest: QUEST.into(),
                initial: -1,
                min: -1,
                max: i64::from(i32::MAX),
            },
        );
    }
    for transition in catalogue.transitions.values_mut() {
        if transition.quest == QUEST
            && transition
                .effects
                .iter()
                .any(|effect| owned.contains(effect.track.as_str()))
        {
            for effect in &mut transition.effects {
                effect.effect = QuestEffectKind::Computed;
            }
        }
    }
    let mut actions = vec![
        GrizzlyAction::Join,
        GrizzlyAction::Promote,
        GrizzlyAction::StartSpecialTask,
    ];
    for task in &data.tasks {
        actions.extend([
            GrizzlyAction::StartTask(task.id),
            GrizzlyAction::CancelTask(task.id),
            GrizzlyAction::CompleteTask(task.id),
            GrizzlyAction::KillCreature {
                task_id: task.id,
                creature: String::new(),
                observed_revision: 1,
            },
        ]);
    }
    for boss in &data.bosses {
        actions.extend([
            GrizzlyAction::EnterBoss(boss.id),
            GrizzlyAction::BossKill {
                boss_id: boss.id,
                observed_revision: 1,
            },
        ]);
        if boss.task_id.is_some() {
            actions.extend([
                GrizzlyAction::SelectBoss(boss.id),
                GrizzlyAction::ReportBoss(boss.id),
            ]);
        }
    }
    for action in actions {
        let experience = match action {
            GrizzlyAction::CompleteTask(id) => task(id).and_then(|task| task.experience),
            _ => None,
        };
        let key = action.transition_key();
        if catalogue.transitions.contains_key(&key) {
            return Err(format!("duplicate Grizzly transition: {key}"));
        }
        catalogue
            .transitions
            .insert(key.clone(), marker(key, experience));
    }
    // The ordinary definition hash includes the source facts/rule version through this
    // refused marker key, and therefore survives every later catalogue extension.
    let digest: String = definition_hash()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let key = format!("{TRANSITION_PREFIX}rules-{digest}");
    catalogue.transitions.insert(key.clone(), marker(key, None));
    QuestStateCatalogue::new(
        catalogue.content_revision,
        catalogue.tracks.into_values().collect(),
        catalogue.transitions.into_values().collect(),
    )
    .map_err(|error| format!("{error:?}"))
}

fn marker(key: String, experience: Option<i64>) -> QuestTransition {
    QuestTransition {
        key,
        quest: QUEST.into(),
        completes: false,
        experience,
        effects: vec![QuestEffect {
            track: storage_track(JOIN_STORAGE),
            from: QuestComparison::Any,
            effect: QuestEffectKind::Computed,
        }],
    }
}

/// Native progress text for the existing Grizzly missions. Preserve the source mission
/// order/names and all other NPC missions; only the previously unbound Grizzly text changes.
#[must_use]
pub fn journal(
    name: &str,
    source_track: &str,
    start_value: i64,
    end_value: i64,
) -> Option<crate::quest::log::QuestLogMission> {
    use crate::quest::log::{
        JournalPiece as P, JournalText as T, QuestJournal as J, QuestLogMission,
    };
    let track = format!("oteryn:{}", source_track.split_once(':')?.1);
    let content = if track == storage_track(QUEST_LOG_STORAGE) {
        J::Fixed(T::Template(vec![
            P::Text("Paw and Fur points: ".into()),
            P::Track(storage_track(POINTS_STORAGE)),
            P::Text(". Boss points: ".into()),
            P::Track(storage_track(BOSS_POINTS_STORAGE)),
            P::Text(". Ask Grizzly Adams about tasks, promotions and bosses.".into()),
        ]))
    } else if track == storage_track(PROMOTION_STORAGE) {
        J::Fixed(T::Text(
            "Grizzly Adams has confirmed this Paw and Fur rank.".into(),
        ))
    } else if let Some(task) = super::catalogue()
        .ok()?
        .tasks
        .iter()
        .find(|task| track == storage_track(KILLS_STORAGE_BASE + u32::from(task.id)))
    {
        let mut pieces = vec![
            P::Text(format!("Hunted {}: ", task.name)),
            P::Track(storage_track(KILL_COUNTER_BASE + u32::from(task.id))),
            P::Text(format!(
                "/{}. Return to Grizzly Adams when the hunt is complete.",
                task.required
            )),
        ];
        for creature in &task.creatures {
            if let Some(storage) = creature.alt_storage {
                pieces.extend([
                    P::Text(format!(" {}: ", creature.name)),
                    P::Track(storage_track(storage)),
                ]);
            }
        }
        J::PerStage(BTreeMap::from([
            (0, T::Template(pieces)),
            (
                1,
                T::Text("You reported this hunt and received its rewards.".into()),
            ),
            (
                2,
                T::Text("You completed the available repetitions of this hunt.".into()),
            ),
        ]))
    } else {
        let boss = super::catalogue()
            .ok()?
            .bosses
            .iter()
            .find(|boss| track == storage_track(boss.storage))?;
        J::PerStage(BTreeMap::from([
            (
                1,
                T::Text(format!("You may enter the lair of {}.", boss.name)),
            ),
            (
                2,
                T::Template(vec![
                    P::Text(format!(
                        "You entered the lair of {}. Boss kills: ",
                        boss.name
                    )),
                    P::Track(storage_track(boss.kill_storage)),
                    P::Text(". Report to Grizzly Adams after the encounter.".into()),
                ]),
            ),
            (
                3,
                T::Text(format!(
                    "You defeated {} and reported the result.",
                    boss.name
                )),
            ),
        ]))
    };
    Some(QuestLogMission {
        name: name.into(),
        track,
        start_value,
        end_value,
        journal: content,
    })
}
