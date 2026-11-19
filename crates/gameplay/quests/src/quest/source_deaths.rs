//! Closed native quest death bindings with qualified source witnesses.
//! HeartBossDeath: creaturescripts_heart_boss_death.lua:26-138 (room players);
//! Court Warlock: court_warlock_fight.lua:323-329 (room players);
//! MagmaBubbleDeath: creaturescripts_magma_bubble_death.lua:12-23 (damage map);
//! GoblinLeaderDeath: creaturescripts_goblin_leader_kill.lua:3-8 (damaging players).
//! Herald: Crystal 9f5a72c creaturescripts_herald_of_fire.lua:43-45, local top-damage recipient.
use super::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack, QuestTransition,
};
use serde::Deserialize;
use std::{collections::BTreeMap, sync::OnceLock};

#[derive(Debug, Clone, Deserialize)]
pub struct Effect {
    pub track: String,
    pub default: i64,
    pub from: Comparison,
    #[serde(default)]
    pub set: Option<i64>,
    #[serde(default)]
    pub add: Option<i64>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Comparison {
    kind: String,
    #[serde(default)]
    value: i64,
}
impl Comparison {
    fn comparison(&self) -> Result<QuestComparison, String> {
        match self.kind.as_str() {
            "any" => Ok(QuestComparison::Any),
            "eq" => Ok(QuestComparison::Eq(self.value)),
            "ne" => Ok(QuestComparison::Ne(self.value)),
            "lt" => Ok(QuestComparison::Lt(self.value)),
            "ge" => Ok(QuestComparison::Ge(self.value)),
            _ => Err("unknown source death comparison".into()),
        }
    }
    fn accepts(&self, value: i64) -> bool {
        match self.kind.as_str() {
            "any" => true,
            "eq" => value == self.value,
            "ne" => value != self.value,
            "lt" => value < self.value,
            "ge" => value >= self.value,
            _ => false,
        }
    }
}
#[derive(Debug, Clone, Deserialize)]
pub struct Rule {
    pub creature: String,
    pub transition: String,
    pub effects: Vec<Effect>,
    pub room: Option<[i32; 5]>,
    /// A qualified local source rule may require the present most-damage player.
    #[serde(default)]
    pub top_damage_only: bool,
    define_transition: bool,
    /// Only mission-start guards are revision predecessors for cumulative counters.
    /// Counter increments from earlier queued deaths must not invalidate later ones.
    pub epoch_tracks: Option<Vec<String>>,
    #[serde(skip)]
    pub quest: String,
}
impl Rule {
    pub fn active(&self, tracks: &BTreeMap<String, i64>) -> bool {
        self.effects.iter().all(|e| {
            e.from
                .accepts(tracks.get(&e.track).copied().unwrap_or(e.default))
        })
    }
    /// Source room bounds concern recipients, not damage ownership or social party.
    pub fn recipient(&self, position: [i32; 3], dealt_damage: bool) -> bool {
        match self.room {
            Some([x1, y1, x2, y2, z]) => {
                position[2] == z
                    && (x1..=x2).contains(&position[0])
                    && (y1..=y2).contains(&position[1])
            }
            None => dealt_damage,
        }
    }
    pub(super) fn transition(&self) -> Result<QuestTransition, String> {
        Ok(QuestTransition {
            key: self.transition.clone(),
            quest: self.quest.clone(),
            effects: self
                .effects
                .iter()
                .map(|e| {
                    Ok(QuestEffect {
                        track: e.track.clone(),
                        from: e.from.comparison()?,
                        effect: match (e.set, e.add) {
                            (Some(value), None) => QuestEffectKind::Set(value),
                            (None, Some(value)) => QuestEffectKind::Add(value),
                            _ => return Err("ambiguous source death effect".into()),
                        },
                    })
                })
                .collect::<Result<_, String>>()?,
            completes: false,
            experience: None,
            world_change: None,
        })
    }
}
#[derive(Deserialize)]
struct Document {
    schema: u8,
    runtime_enabled: bool,
    quest: String,
    rules: Vec<Rule>,
    tracks: Vec<Track>,
}
#[derive(Deserialize)]
struct Track {
    key: String,
    initial: i64,
    min: i64,
    max: i64,
}
const DOCUMENTS: &[&[u8]] = &[
    include_bytes!(
        "../../../../../content/quests/native-death-bindings/qmech-m24-a_piece_of_cake.json"
    ),
    include_bytes!(
        "../../../../../content/quests/native-death-bindings/qmech-m24-ancient_tombs.json"
    ),
    include_bytes!(
        "../../../../../content/quests/native-death-bindings/qmech-m24-targuna-herald.json"
    ),
    include_bytes!(
        "../../../../../content/quests/native-death-bindings/quest-batch3-c-heart_of_destruction.json"
    ),
    include_bytes!(
        "../../../../../content/quests/native-death-bindings/quest-batch3-c-the_order_of_the_stag.json"
    ),
    include_bytes!(
        "../../../../../content/quests/native-death-bindings/quest-batch3-c-tower_defence_quest.json"
    ),
    include_bytes!(
        "../../../../../content/quests/native-death-bindings/quest-batch3-c-the_primal_ordeal.json"
    ),
];
fn documents() -> Result<Vec<Document>, String> {
    DOCUMENTS
        .iter()
        .map(|bytes| {
            let doc: Document = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
            if doc.schema != 1 || !doc.runtime_enabled {
                return Err("source quest death binding disabled".into());
            }
            Ok(doc)
        })
        .collect()
}
pub const ARBAZILOTH_ENRAGED: &str = "oteryn:creature.enraged_arbaziloth";
const ARBAZILOTH_QUEST: &str = "oteryn:quest.no_rest_for_the_wicked_quest";
const HARD_KILLS: &str =
    "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/hard_kills";
const DOOR: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/questline";
const HARD_UNLOCK: &str = "oteryn:quest-transition/native/no_rest_for_the_wicked/hard_unlock";
const HARD_INCREMENT: &str = "oteryn:quest-transition/native/no_rest_for_the_wicked/hard_increment";
const HARD_FIRST_CLEAR: &str =
    "oteryn:quest-transition/native/no_rest_for_the_wicked/hard_first_clear";
const HARD_FIRST_CLEAR_WITH_DOOR_PROGRESS: &str =
    "oteryn:quest-transition/native/no_rest_for_the_wicked/hard_first_clear_with_door_progress";

const ARBAZILOTH_FIRST_CLEAR: &str =
    "oteryn:quest-transition/native/no_rest_for_the_wicked/first_clear";
const ARBAZILOTH_FIRST_CLEAR_WITH_DOOR_PROGRESS: &str =
    "oteryn:quest-transition/native/no_rest_for_the_wicked/first_clear_with_door_progress";

pub fn is_arbaziloth_first_clear(transition: &str) -> bool {
    matches!(
        transition,
        ARBAZILOTH_FIRST_CLEAR
            | ARBAZILOTH_FIRST_CLEAR_WITH_DOOR_PROGRESS
            | HARD_FIRST_CLEAR
            | HARD_FIRST_CLEAR_WITH_DOOR_PROGRESS
    )
}

// Crystal 201189f8 arbaziloth_death.lua:27-39. The first kill raises the
// door stage only when it is still below one, including for imported saves.
fn arbaziloth_first_clear_rules() -> [Rule; 3] {
    const TRACK: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked";
    let effect = |track, kind: &str, value, set| Effect {
        track: format!("{TRACK}/{track}"),
        default: 0,
        from: Comparison {
            kind: kind.into(),
            value,
        },
        set: Some(set),
        add: None,
    };
    let rule = |name, effects| Rule {
        creature: "oteryn:creature.arbaziloth".into(),
        transition: String::from(name),
        effects,
        room: None,
        top_damage_only: false,
        define_transition: true,
        epoch_tracks: None,
        quest: ARBAZILOTH_QUEST.into(),
    };
    [
        rule(
            ARBAZILOTH_FIRST_CLEAR,
            vec![
                effect("kill_arbaziloth/questline", "lt", 2, 2),
                effect("forgemaster_door/questline", "lt", 1, 1),
            ],
        ),
        rule(
            ARBAZILOTH_FIRST_CLEAR_WITH_DOOR_PROGRESS,
            vec![
                effect("kill_arbaziloth/questline", "lt", 2, 2),
                Effect {
                    set: None,
                    add: Some(0),
                    ..effect("forgemaster_door/questline", "ge", 1, 0)
                },
            ],
        ),
        rule(
            "oteryn:quest-transition/native/no_rest_for_the_wicked/start_questlog",
            vec![effect("questlog", "lt", 1, 1)],
        ),
    ]
}

fn arbaziloth_hard_rules() -> Vec<Rule> {
    let effect = |track: &str, kind: &str, value, set, add| Effect {
        track: track.into(),
        default: 0,
        from: Comparison {
            kind: kind.into(),
            value,
        },
        set,
        add,
    };
    let hard_rule = |suffix: &str, effects| Rule {
        creature: ARBAZILOTH_ENRAGED.into(),
        transition: format!("oteryn:quest-transition/native/no_rest_for_the_wicked/{suffix}"),
        effects,
        room: None,
        top_damage_only: false,
        define_transition: true,
        epoch_tracks: Some(vec![DOOR.into()]),
        quest: ARBAZILOTH_QUEST.into(),
    };
    // Crystal 201189f8 arbaziloth_death.lua:45-55. Try the tenth-win
    // transition before the increment so a ninth win cannot advance both.
    let mut rules = vec![
        hard_rule(
            "hard_unlock",
            vec![
                effect(HARD_KILLS, "eq", 9, Some(10), None),
                effect(DOOR, "lt", 2, Some(2), None),
            ],
        ),
        hard_rule(
            "hard_increment",
            vec![
                effect(HARD_KILLS, "lt", 9, None, Some(1)),
                effect(DOOR, "lt", 2, None, Some(0)),
            ],
        ),
    ];
    let first = arbaziloth_first_clear_rules();
    rules.extend(
        first
            .into_iter()
            .zip([
                HARD_FIRST_CLEAR,
                HARD_FIRST_CLEAR_WITH_DOOR_PROGRESS,
                "oteryn:quest-transition/native/no_rest_for_the_wicked/hard_start_questlog",
            ])
            .map(|(mut rule, key)| {
                rule.creature = ARBAZILOTH_ENRAGED.into();
                rule.transition = key.into();
                rule
            }),
    );
    // Apply first-clear changes after the hard count, using the durable stage
    // from the same death. Door progress never regresses.
    for rule in &mut rules[2..4] {
        rule.epoch_tracks = Some(vec![rule.effects[0].track.clone()]);
    }
    rules
}

pub fn arbaziloth_hard_transitions() -> impl Iterator<Item = &'static str> {
    rules()
        .into_iter()
        .flatten()
        .filter(|rule| rule.creature == ARBAZILOTH_ENRAGED)
        .map(|rule| rule.transition.as_str())
}

/// Freeze one counter operation at admission so replay cannot turn an
/// already counted ninth kill into the tenth kill.
pub fn arbaziloth_hard_counter(tracks: &BTreeMap<String, i64>) -> Option<&'static str> {
    [HARD_UNLOCK, HARD_INCREMENT]
        .into_iter()
        .find(|key| rule(key).is_some_and(|rule| rule.active(tracks)))
}

pub fn is_arbaziloth_hard_counter(key: &str) -> bool {
    matches!(key, HARD_UNLOCK | HARD_INCREMENT)
}

pub fn rules() -> Result<&'static [Rule], &'static str> {
    static RULES: OnceLock<Result<Vec<Rule>, String>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            let mut rules = Vec::from(arbaziloth_first_clear_rules());
            rules.extend(arbaziloth_hard_rules());
            for doc in documents()? {
                for mut rule in doc.rules {
                    // Already owned atomically by character_hazard/primal.rs; never issue it twice.
                    if rule.creature == "oteryn:creature.the_primal_menace" {
                        continue;
                    }
                    rule.quest.clone_from(&doc.quest);
                    if rule.epoch_tracks.as_ref().is_some_and(|guards| {
                        guards.is_empty()
                            || guards
                                .iter()
                                .any(|key| !rule.effects.iter().any(|e| &e.track == key))
                    }) {
                        return Err("invalid source death mission guards".into());
                    }
                    if rule.effects.is_empty()
                        || rule.effects.iter().any(|e| e.from.comparison().is_err())
                        || rules.iter().any(|r: &Rule| r.transition == rule.transition)
                    {
                        return Err("invalid source death rule".into());
                    }
                    rules.push(rule);
                }
            }
            Ok(rules)
        })
        .as_deref()
        .map_err(String::as_str)
}
pub fn rule(key: &str) -> Option<&'static Rule> {
    rules().ok()?.iter().find(|rule| rule.transition == key)
}
pub fn tracks_creature(creature: &str) -> bool {
    rules().is_ok_and(|rules| rules.iter().any(|rule| rule.creature == creature))
}
pub fn active(tracks: &BTreeMap<String, i64>) -> Vec<&'static str> {
    rules().map_or_else(
        |_| Vec::new(),
        |rules| {
            rules
                .iter()
                .filter(|r| r.creature != ARBAZILOTH_ENRAGED && r.active(tracks))
                .map(|r| r.transition.as_str())
                .collect()
        },
    )
}
pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks = catalogue.tracks;
    let mut transitions = catalogue.transitions;
    tracks.insert(
        HARD_KILLS.into(),
        QuestTrack {
            key: HARD_KILLS.into(),
            quest: ARBAZILOTH_QUEST.into(),
            initial: 0,
            min: 0,
            max: 10,
        },
    );
    for doc in documents()? {
        for track in doc.tracks {
            let candidate = QuestTrack {
                key: track.key.clone(),
                quest: doc.quest.clone(),
                initial: track.initial,
                min: track.min,
                max: track.max,
            };
            if let Some(current) = tracks.get(&track.key) {
                if current != &candidate {
                    return Err(format!("source death track conflict: {}", track.key));
                }
            } else {
                tracks.insert(track.key, candidate);
            }
        }
    }
    for rule in rules().map_err(str::to_owned)? {
        if rule.define_transition {
            let candidate = rule.transition()?;
            if let Some(existing) = transitions.insert(rule.transition.clone(), candidate.clone())
                && existing != candidate
            {
                return Err(format!(
                    "source death transition conflict: {}",
                    rule.transition
                ));
            }
        } else if transitions.get(&rule.transition) != Some(&rule.transition()?) {
            return Err(format!(
                "source death transition missing: {}",
                rule.transition
            ));
        }
    }
    QuestStateCatalogue::new(
        catalogue.content_revision,
        tracks.into_values().collect(),
        transitions.into_values().collect(),
    )
    .map_err(|error| format!("{error:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arbaziloth_hard_win_selects_one_counter_operation() {
        let mut tracks = BTreeMap::from([(HARD_KILLS.into(), 8), (DOOR.into(), 1)]);
        assert_eq!(arbaziloth_hard_counter(&tracks), Some(HARD_INCREMENT));
        tracks.insert(HARD_KILLS.into(), 9);
        assert_eq!(arbaziloth_hard_counter(&tracks), Some(HARD_UNLOCK));
        assert!(!active(&tracks).contains(&HARD_UNLOCK));
        tracks.insert(DOOR.into(), 2);
        assert_eq!(arbaziloth_hard_counter(&tracks), None);
        tracks.insert(HARD_KILLS.into(), 10);
        assert_eq!(arbaziloth_hard_counter(&tracks), None);
    }
}
