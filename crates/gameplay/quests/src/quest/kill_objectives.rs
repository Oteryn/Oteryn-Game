//! Character-owned native kill objectives, composed with the existing Quest catalogue.
//! Elvenbane's accepted recipe: authored68/recipes.json:2164-2333. Exact spawn
//! declarations: spawns-02000-03999.json:1 records26,28,1440; and
//! spawns-00000-01999.json:1 record1175 (hashes in questpc2/CONTRACT-NOTES.md).
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};

pub const QUEST: &str = "oteryn:quest.authored.elvenbane_quest";
pub const PHASE: &str = "oteryn:quest-progress/elvenbane/phase";

/// Player-visible progress for the earned source route and its terminal receipt.
pub(super) fn journal() -> super::log::QuestLogQuest {
    use super::log::{JournalText, QuestJournal, QuestLogKind, QuestLogMission, QuestLogQuest};
    let mut missions = vec![QuestLogMission {
        name: "Elvenbane".into(),
        track: PHASE.into(),
        start_value: 0,
        end_value: 4,
        journal: QuestJournal::PerStage(std::collections::BTreeMap::from([
            (0, JournalText::Text("Find the entrance beneath Elvenbane.".into())),
            (1, JournalText::Text("You entered the basement. Cross the poison chamber and reach the dragon.".into())),
            (2, JournalText::Text("You crossed the poison chamber. Defeat the dragon and climb the fortress.".into())),
            (3, JournalText::Text("You defeated the dragon. Defeat the upper guards, recover all four treasures, and return from the fortress.".into())),
            (4, JournalText::Text("You defeated the guards, recovered all four treasures, and returned from the fortress.".into())),
        ])),
    }];
    for (objective, name) in OBJECTIVES.iter().skip(1).zip([
        "Orc Shaman",
        "Minotaur Mage",
        "Elf Arcanist",
        "Demon Skeletons",
    ]) {
        if let Some((track, maximum)) = objective.counter {
            missions.push(QuestLogMission {
                name: format!("Upper guards: {name}"),
                track: track.into(),
                start_value: 1,
                end_value: maximum,
                journal: QuestJournal::Fixed(JournalText::Text(format!(
                    "Defeat the {name} guarding the upper fortress."
                ))),
            });
        }
    }
    QuestLogQuest {
        key: QUEST.into(),
        kind: QuestLogKind::Storyline,
        shown_in_quest_log: true,
        start: Some((PHASE.into(), 1)),
        hide_when_completed: false,
        missions,
    }
}

#[derive(Clone, Copy, Debug)]
pub struct KillObjective {
    pub transition: &'static str,
    pub quest: &'static str,
    pub phase_track: &'static str,
    pub phase_before: i64,
    pub phase_after: i64,
    pub creature: &'static str,
    pub counter: Option<(&'static str, i64)>,
    pub minimum: (i32, i32, i16),
    pub maximum: (i32, i32, i16),
}

impl KillObjective {
    pub fn contains(self, x: i32, y: i32, floor: i16) -> bool {
        (self.minimum.0..=self.maximum.0).contains(&x)
            && (self.minimum.1..=self.maximum.1).contains(&y)
            && (self.minimum.2..=self.maximum.2).contains(&floor)
    }

    fn effects(self) -> Vec<QuestEffect> {
        let mut effects = vec![QuestEffect {
            track: self.phase_track.into(),
            from: QuestComparison::Eq(self.phase_before),
            effect: QuestEffectKind::Set(self.phase_after),
        }];
        if let Some((track, maximum)) = self.counter {
            effects.push(QuestEffect {
                track: track.into(),
                from: QuestComparison::Between(0, maximum - 1),
                effect: QuestEffectKind::Add(1),
            });
        }
        effects
    }
}

const fn objective(
    transition: &'static str,
    creature: &'static str,
    counter: Option<(&'static str, i64)>,
    floor: i16,
) -> KillObjective {
    KillObjective {
        transition,
        quest: QUEST,
        phase_track: PHASE,
        phase_before: if counter.is_none() { 2 } else { 3 },
        phase_after: 3,
        creature,
        counter,
        minimum: (32580, 31635, floor),
        maximum: (32600, 31655, floor),
    }
}

pub const OBJECTIVES: [KillObjective; 5] = [
    objective(
        "oteryn:quest-transition/elvenbane/dragon",
        "oteryn:creature.dragon",
        None,
        5,
    ),
    objective(
        "oteryn:quest-transition/elvenbane/orc-shaman",
        "oteryn:creature.orc_shaman",
        Some(("oteryn:quest-progress/elvenbane/orc-shaman", 1)),
        4,
    ),
    objective(
        "oteryn:quest-transition/elvenbane/minotaur-mage",
        "oteryn:creature.minotaur_mage",
        Some(("oteryn:quest-progress/elvenbane/minotaur-mage", 1)),
        4,
    ),
    objective(
        "oteryn:quest-transition/elvenbane/elf-arcanist",
        "oteryn:creature.elf_arcanist",
        Some(("oteryn:quest-progress/elvenbane/elf-arcanist", 1)),
        3,
    ),
    objective(
        "oteryn:quest-transition/elvenbane/demon-skeleton",
        "oteryn:creature.demon_skeleton",
        Some(("oteryn:quest-progress/elvenbane/demon-skeleton", 2)),
        3,
    ),
];

pub fn binding(transition: &str) -> Option<KillObjective> {
    OBJECTIVES
        .iter()
        .copied()
        .find(|rule| rule.transition == transition)
}

pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks = catalogue.tracks.into_values().collect::<Vec<_>>();
    let mut transitions = catalogue.transitions.into_values().collect::<Vec<_>>();
    tracks.push(QuestTrack {
        key: PHASE.into(),
        quest: QUEST.into(),
        initial: 0,
        min: 0,
        max: 4,
    });
    for rule in OBJECTIVES {
        if let Some((key, max)) = rule.counter {
            tracks.push(QuestTrack {
                key: key.into(),
                quest: rule.quest.into(),
                initial: 0,
                min: 0,
                max,
            });
        }
        transitions.push(QuestTransition {
            world_change: None,
            key: rule.transition.into(),
            quest: rule.quest.into(),
            effects: rule.effects(),
            completes: false,
            experience: None,
        });
    }
    QuestStateCatalogue::new(catalogue.content_revision, tracks, transitions)
        .map_err(|error| format!("native kill objective catalogue: {error:?}"))
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "fixtures require the valid embedded catalogue"
)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn exact_native_tower_floors_and_defender_multiplicity() {
        let catalogue =
            super::super::loader::load_embedded_quest_state("content-1").expect("catalogue");
        let catalogue = catalogue.catalogue();
        let mut state = BTreeMap::from([(PHASE.to_owned(), 2)]);
        for rule in OBJECTIVES {
            if let Some((track, _)) = rule.counter {
                state.insert(track.into(), 0);
            }
        }
        let dragon = OBJECTIVES[0];
        assert!(dragon.contains(32590, 31645, 5));
        assert!(!dragon.contains(32590, 31645, -5));
        assert!(!dragon.contains(32799, 31559, 5));
        for rule in OBJECTIVES.into_iter().skip(1) {
            let transition = catalogue
                .transition(rule.transition)
                .expect("bound transition");
            assert!(
                catalogue.evaluate(transition, &state, 0).is_err(),
                "upper defenders require the actual Dragon stage"
            );
        }
        let advance = |state: &mut BTreeMap<String, i64>, rule: KillObjective| {
            for change in catalogue
                .evaluate(
                    catalogue.transition(rule.transition).expect("transition"),
                    state,
                    0,
                )
                .expect("earned stage")
            {
                state.insert(change.track, change.after);
            }
        };
        advance(&mut state, dragon);
        assert_eq!(state[PHASE], 3);
        assert!(
            catalogue
                .evaluate(
                    catalogue.transition(dragon.transition).expect("dragon"),
                    &state,
                    0
                )
                .is_err()
        );
        for rule in OBJECTIVES.into_iter().skip(1) {
            let (counter, maximum) = rule.counter.expect("counter");
            for _ in 0..maximum {
                advance(&mut state, rule);
            }
            assert_eq!(state[counter], maximum);
            assert!(
                catalogue
                    .evaluate(
                        catalogue.transition(rule.transition).expect("transition"),
                        &state,
                        0
                    )
                    .is_err(),
                "excess kills grant nothing"
            );
        }
    }
}
