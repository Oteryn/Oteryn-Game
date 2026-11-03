//! The existing QuestState writer owns bounded changes and the atomic XP obligation.
//! Item grants and transient target/dedupe mutations do not enter this adapter.
use super::dialogue::{self, Book, Facts, Plan, main_track, report_track};
use super::{QUEST, Task};
use crate::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};

pub struct Selection {
    pub plan: Plan,
    pub transition: Option<String>,
}

fn start_key(tier: &str, tasks: &[String]) -> String {
    format!(
        "oteryn:quest-transition/spike-tasks/start/{tier}/{}",
        tasks.join("-")
    )
}
fn report_key(tier: &str, task: &str, negative_fame: bool) -> String {
    format!(
        "oteryn:quest-transition/spike-tasks/report/{tier}/{task}{}",
        if negative_fame { "/negative-fame" } else { "" }
    )
}

#[must_use]
pub fn select(
    book: &Book,
    npc: &str,
    topic: Option<&str>,
    message: &str,
    facts: &Facts<'_>,
) -> Option<Selection> {
    let plan = book.respond(npc, topic, message, facts)?;
    if plan.needs_inventory {
        return None;
    }
    let officer = book.officers.iter().find(|officer| officer.npc == npc)?;
    let transition = if let Some(task) = &plan.report {
        Some(report_key(
            &officer.tier,
            task,
            facts.tracks.get(dialogue::FAME).copied().unwrap_or(-1) < 0,
        ))
    } else if plan.starts.is_empty() {
        None
    } else {
        Some(start_key(&officer.tier, &plan.starts))
    };
    Some(Selection { plan, transition })
}

fn main(task: &Task, from: QuestComparison, effect: QuestEffectKind) -> QuestEffect {
    QuestEffect {
        track: main_track(task),
        from,
        effect,
    }
}

pub fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let book = dialogue::load()?;
    let mut tracks: Vec<_> = catalogue.tracks.values().cloned().collect();
    let mut proposed = Vec::new();
    for task in &book.tasks {
        proposed.push(QuestTrack {
            key: main_track(task),
            quest: QUEST.into(),
            initial: -1,
            min: -1,
            max: task.threshold,
        });
        proposed.push(QuestTrack {
            key: report_track(task),
            quest: QUEST.into(),
            initial: -1,
            min: -1,
            max: i64::MAX,
        });
    }
    // donor bug fix: Player.addFamePoint incorrectly indexes nonexistent Constants.
    // Use the same table's explicit44313, also used by its neighboring getters.
    // Crystal201189f data/libs/functions/player.lua:160-164 (and neighboring getters167-175);
    // data-global/lib/core/storages.lua:2090-2132 (44313 at2107).
    // QUESTION: https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589583342
    // ANSWER: https://github.com/Oteryn/Oteryn-Game/pull/2845#issuecomment-6589904926 (1).
    // Both source branches of max(0,points)+1 are guarded in the same report
    // transaction. Existing negative values are accepted; no saved zero is migrated.
    proposed.push(QuestTrack {
        key: dialogue::FAME.into(),
        quest: QUEST.into(),
        initial: -1,
        min: i64::MIN,
        max: i64::MAX,
    });
    for expected in proposed {
        if let Some(existing) = catalogue.track(&expected.key) {
            // The existing source lowering gives ADD counters the full i32
            // storage range. Keep that qualified range and every stored value;
            // the native report still requires the exact source threshold.
            // No alias, default change or migration is needed for a superset.
            if existing.quest != expected.quest
                || existing.initial != expected.initial
                || existing.min > expected.min
                || existing.max < expected.max
            {
                return Err(format!(
                    "Spike track ownership/bounds conflict {}",
                    expected.key
                ));
            }
        } else {
            tracks.push(expected);
        }
    }
    let mut transitions: Vec<_> = catalogue.transitions.values().cloned().collect();
    for task in &book.tasks {
        for negative in [false, true] {
            transitions.push(QuestTransition {
                key: report_key(&task.tier, &task.task, negative),
                quest: QUEST.into(),
                effects: vec![
                    main(
                        task,
                        QuestComparison::Eq(task.threshold),
                        QuestEffectKind::Set(-1),
                    ),
                    QuestEffect {
                        track: report_track(task),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::SetNow,
                    },
                    QuestEffect {
                        track: dialogue::FAME.into(),
                        from: if negative {
                            QuestComparison::Between(i64::MIN, -1)
                        } else {
                            QuestComparison::Between(0, i64::MAX)
                        },
                        effect: if negative {
                            QuestEffectKind::Set(1)
                        } else {
                            QuestEffectKind::Add(1)
                        },
                    },
                ],
                completes: false,
                experience: Some(task.experience),
            });
        }
    }
    for officer in &book.officers {
        let pure: Vec<_> = officer
            .tasks
            .iter()
            .filter_map(|text| book.task(&officer.tier, &text.task))
            .filter(|task| task.grant_item.is_none())
            .collect();
        // At most two pure tasks occur in one source officer's four blocks.
        // Each nonempty source-ordered subset is one atomic transition, not
        // multiple commits of a compound utterance.
        for mask in 1..(1_usize << pure.len()) {
            let selected: Vec<_> = pure
                .iter()
                .enumerate()
                .filter(|(index, _)| mask & (1_usize << *index) != 0)
                .map(|(_, task)| *task)
                .collect();
            let names: Vec<_> = selected.iter().map(|task| task.task.clone()).collect();
            transitions.push(QuestTransition {
                key: start_key(&officer.tier, &names),
                quest: QUEST.into(),
                // Confirmation checks the held topic only; level/cooldown/main
                // are checked when offering, not again in the donor yes branch.
                effects: selected
                    .into_iter()
                    .map(|task| main(task, QuestComparison::Any, QuestEffectKind::Set(0)))
                    .collect(),
                completes: false,
                experience: None,
            });
        }
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|e| format!("{e:?}"))
}

#[cfg(test)]
mod tests;
