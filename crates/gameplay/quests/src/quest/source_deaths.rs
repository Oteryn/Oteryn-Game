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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Effect {
    pub track: String,
    pub default: i64,
    pub from: Comparison,
    #[serde(default)]
    pub set: Option<i64>,
    #[serde(default)]
    pub add: Option<i64>,
}
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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
    #[serde(default)]
    pub delivery: Option<DeathDelivery>,
    #[serde(skip)]
    pub quest: String,
}

/// Crystal 201189f feaster_of_souls/creaturescripts_pale_worm_death.lua:9-19.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeathDelivery {
    PoltergeistAndBeyonder,
}

impl DeathDelivery {
    fn valid(self, rule: &Rule) -> bool {
        match self {
            Self::PoltergeistAndBeyonder => {
                rule.quest == "oteryn:quest.feaster_of_souls_quest"
                    && rule.creature == "oteryn:creature.the_pale_worm"
                    && rule.room == Some([33794, 31493, 33816, 31515, -14])
                    && !rule.top_damage_only
                    && rule.define_transition
                    && rule.effects.len() == 1
                    && rule.effects.iter().all(|effect| {
                        effect.default == -1 && effect.from.kind == "any"
                            && effect.set == Some(1) && effect.add.is_none()
                    })
            }
        }
    }
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
        .copied()
        .chain(super::authored_bindings::embedded::NATIVE_DEATH_REWARDS.iter()
            .map(|(_, document)| document.as_bytes()))
        .map(|bytes| {
            let doc: Document = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
            if doc.schema != 1 || !doc.runtime_enabled {
                return Err("source quest death binding disabled".into());
            }
            Ok(doc)
        })
        .collect()
}
pub fn rules() -> Result<&'static [Rule], &'static str> {
    static RULES: OnceLock<Result<Vec<Rule>, String>> = OnceLock::new();
    RULES
        .get_or_init(|| {
            let mut rules = Vec::new();
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
                        || rule.delivery.is_some_and(|delivery| !delivery.valid(&rule))
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

pub fn rule_for<'a>(catalogue: &'a QuestStateCatalogue, key: &str) -> Option<&'a Rule> {
    let _ = catalogue;
    #[cfg(any(test, feature = "test-support"))]
    if let Some(rule) = catalogue.fixture_source_deaths.iter().find(|r| r.transition == key) {
        return Some(rule);
    }
    rule(key)
}

pub fn active_for(
    catalogue: Option<&QuestStateCatalogue>, tracks: &BTreeMap<String, i64>,
) -> Vec<Rule> {
    let mut active: Vec<_> = rules().ok().into_iter().flatten()
        .filter(|rule| rule.active(tracks)).cloned().collect();
    let _ = (&catalogue, &mut active);
    #[cfg(any(test, feature = "test-support"))]
    if let Some(catalogue) = catalogue {
        active.extend(catalogue.fixture_source_deaths.iter()
            .filter(|rule| rule.active(tracks)).cloned());
    }
    active
}

#[cfg(any(test, feature = "test-support"))]
pub fn with_fixture_rule(
    catalogue: &QuestStateCatalogue, quest: &str, document: &str,
) -> Result<QuestStateCatalogue, String> {
    let mut rule: Rule = serde_json::from_str(document).map_err(|error| error.to_string())?;
    rule.quest = quest.into();
    if rule.delivery.is_none_or(|delivery| !delivery.valid(&rule))
        || self::rule(&rule.transition).is_some()
    {
        return Err("invalid fixture source death delivery".into());
    }
    let mut with_rule = catalogue.with_fixture_transition(rule.transition()?)
        .map_err(|error| format!("{error:?}"))?;
    with_rule.fixture_source_deaths.clone_from(&catalogue.fixture_source_deaths);
    with_rule.fixture_source_deaths.push(rule);
    Ok(with_rule)
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
                .filter(|r| r.active(tracks))
                .map(|r| r.transition.as_str())
                .collect()
        },
    )
}
pub(super) fn extend(catalogue: QuestStateCatalogue) -> Result<QuestStateCatalogue, String> {
    let mut tracks = catalogue.tracks;
    let mut transitions = catalogue.transitions;
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
