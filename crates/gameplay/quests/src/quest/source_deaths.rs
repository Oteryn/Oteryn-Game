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
    #[serde(default)]
    creature_aliases: Vec<String>,
    pub transition: String,
    pub effects: Vec<Effect>,
    pub room: Option<[i32; 5]>,
    /// A qualified local source rule may require the present most-damage player.
    #[serde(default)]
    pub top_damage_only: bool,
    /// Crystal onDeathForParty uses the present top-damage player's shared-XP roster.
    #[serde(default)]
    pub top_damage_party: bool,
    define_transition: bool,
    /// Only mission-start guards are revision predecessors for cumulative counters.
    /// Counter increments from earlier queued deaths must not invalidate later ones.
    pub epoch_tracks: Option<Vec<String>>,
    #[serde(skip)]
    pub quest: String,
}
impl Rule {
    pub fn matches_creature(&self, creature: &str) -> bool {
        self.creature == creature || self.creature_aliases.iter().any(|alias| alias == creature)
    }

    pub fn active(&self, tracks: &BTreeMap<String, i64>) -> bool {
        self.effects.iter().all(|e| {
            e.from
                .accepts(tracks.get(&e.track).copied().unwrap_or(e.default))
        })
    }
    /// Room or damage evidence without a qualified party observation.
    pub fn recipient(&self, position: [i32; 3], dealt_damage: bool) -> bool {
        self.recipient_with_party(position, dealt_damage, false)
    }

    pub fn recipient_with_party(
        &self,
        position: [i32; 3],
        dealt_damage: bool,
        in_top_damage_party: bool,
    ) -> bool {
        if self.top_damage_party {
            return in_top_damage_party;
        }
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
    include_bytes!("../../../../../content/quests/native-death-bindings/uneasy-alliance.json"),
    include_bytes!("../../../../../content/quests/native-death-bindings/secret-service.json"),
    include_bytes!("../../../../../content/quests/native-death-bindings/thieves-guild.json"),
];
fn documents() -> Result<Vec<Document>, String> {
    let mut documents: Vec<Document> = DOCUMENTS
        .iter()
        .map(|bytes| {
            let doc: Document = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
            if doc.schema != 1 || !doc.runtime_enabled {
                return Err("source quest death binding disabled".into());
            }
            Ok(doc)
        })
        .collect::<Result<_, String>>()?;
    // Crystal 9f5a72c and 201189f8, elemental_spheres/creaturescripts_overlord_kill.lua:2-5,24-26.
    // Every damaging player shares BossStorage (storages.lua:1084, numeric key 41652).
    let track = "oteryn:quest-progress/quest/u8_2/elemental_spheres/boss_storage";
    documents.push(Document {
        schema: 1,
        runtime_enabled: true,
        quest: "oteryn:quest.elemental_spheres_quest".into(),
        tracks: vec![Track {
            key: track.into(),
            initial: -1,
            min: -1,
            max: 1,
        }],
        rules: ["earth", "energy", "fire", "ice"]
            .into_iter()
            .map(|element| Rule {
                creature: format!("oteryn:creature.{element}_overlord"),
                creature_aliases: Vec::new(),
                transition: format!(
                    "oteryn:quest-transition/elemental-spheres/{element}-overlord-death"
                ),
                effects: vec![Effect {
                    track: track.into(),
                    default: -1,
                    from: Comparison {
                        kind: "lt".into(),
                        value: 1,
                    },
                    set: Some(1),
                    add: None,
                }],
                room: None,
                top_damage_only: false,
                top_damage_party: false,
                define_transition: true,
                epoch_tracks: None,
                quest: String::new(),
            })
            .collect(),
    });
    Ok(documents)
}
pub const ARBAZILOTH_ENRAGED: &str = "oteryn:creature.enraged_arbaziloth";
const ARBAZILOTH_QUEST: &str = "oteryn:quest.no_rest_for_the_wicked_quest";
const HARD_KILLS: &str =
    "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/hard_kills";
const DOOR: &str = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked/forgemaster_door/questline";
pub const HARD_UNLOCK: &str = "oteryn:quest-transition/native/no_rest_for_the_wicked/hard_unlock";
pub const HARD_INCREMENT: &str =
    "oteryn:quest-transition/native/no_rest_for_the_wicked/hard_increment";
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
        creature_aliases: Vec::new(),
        transition: String::from(name),
        effects,
        room: None,
        top_damage_only: false,
        top_damage_party: false,
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
        creature_aliases: Vec::new(),
        transition: format!("oteryn:quest-transition/native/no_rest_for_the_wicked/{suffix}"),
        effects,
        room: None,
        top_damage_only: false,
        top_damage_party: false,
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

/// Choose the operation from the saved tracks while holding the Character lock.
pub fn arbaziloth_hard_counter(tracks: &BTreeMap<String, i64>) -> Option<&'static str> {
    [HARD_UNLOCK, HARD_INCREMENT]
        .into_iter()
        .find(|key| rule(key).is_some_and(|rule| rule.active(tracks)))
}

pub const ARBAZILOTH_HARD_COUNTER_TRACKS: [&str; 2] = [HARD_KILLS, DOOR];

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
                        || (rule.top_damage_party && (rule.top_damage_only || rule.room.is_some()))
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
    rules().is_ok_and(|rules| rules.iter().any(|rule| rule.matches_creature(creature)))
}

#[cfg(test)]
#[test]
fn black_knight_loads_the_avin_damage_objective() -> Result<(), String> {
    let catalogue = super::loader::load_embedded_quest_state("cq16-test")
        .map_err(|error| format!("{error:?}"))?;
    let track = "oteryn:quest-progress/quest/u8_1/secret_service/avinmission04";
    let transition =
        "oteryn:quest-transition/quest/u8_1/secret_service/avinmission04/creature_event_1";
    assert!(catalogue.catalogue().transition(transition).is_some());
    let binding = rule(transition).ok_or("Black Knight native death binding")?;
    assert_eq!(binding.creature, "oteryn:creature.black_knight");
    assert!(!binding.top_damage_only);
    assert!(binding.recipient([0, 0, -7], true));
    assert!(!binding.recipient([0, 0, -7], false));
    for value in [0, 1, 2, 3] {
        assert_eq!(
            binding.active(&BTreeMap::from([(track.into(), value)])),
            value == 1
        );
    }
    Ok(())
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

/// Closed donor callback owned atomically by the native death settlement.
pub fn overlord(creature: &str) -> Option<&'static Rule> {
    if !matches!(
        creature,
        "oteryn:creature.earth_overlord"
            | "oteryn:creature.energy_overlord"
            | "oteryn:creature.fire_overlord"
            | "oteryn:creature.ice_overlord"
    ) {
        return None;
    }
    rules().ok()?.iter().find(|rule| rule.creature == creature)
}

/// Exact predecessor of the additive Elemental Spheres death callback. Existing
/// tracks and transitions are unchanged; only the four callbacks and flag are new.
pub fn overlord_predecessor(
    catalogue: &QuestStateCatalogue,
) -> Result<QuestStateCatalogue, String> {
    let quest = "oteryn:quest.elemental_spheres_quest";
    let flag = "oteryn:quest-progress/quest/u8_2/elemental_spheres/boss_storage";
    QuestStateCatalogue::new(
        catalogue.content_revision.clone(),
        catalogue
            .tracks
            .values()
            .filter(|track| track.quest == quest && track.key != flag)
            .cloned()
            .collect(),
        catalogue
            .transitions
            .values()
            .filter(|transition| {
                transition.quest == quest
                    && !matches!(
                        transition.key.as_str(),
                        "oteryn:quest-transition/elemental-spheres/earth-overlord-death"
                            | "oteryn:quest-transition/elemental-spheres/energy-overlord-death"
                            | "oteryn:quest-transition/elemental-spheres/fire-overlord-death"
                            | "oteryn:quest-transition/elemental-spheres/ice-overlord-death"
                    )
            })
            .cloned()
            .collect(),
    )
    .map_err(|error| format!("{error:?}"))
}

pub(super) fn retain_overlord_predecessor(
    catalogue: &mut QuestStateCatalogue,
) -> Result<(), String> {
    let quest = "oteryn:quest.elemental_spheres_quest";
    let previous = overlord_predecessor(catalogue)?;
    let hash = previous
        .definition_hash(quest)
        .ok_or("Elemental Spheres predecessor missing")?;
    let defaults = previous
        .declared_tracks()
        .map(|track| (track.key.clone(), track.initial))
        .collect();
    let predecessors = catalogue.predecessors.entry(quest.into()).or_default();
    if !predecessors
        .iter()
        .any(|predecessor| predecessor.hash == hash)
    {
        predecessors.push(super::NpcDefinitionPredecessor { hash, defaults });
    }
    Ok(())
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
