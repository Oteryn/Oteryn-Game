//! Crystal 201189f8 data-global/scripts/quests: children_of_the_revolution/movements_click.lua:16-125;
//! the_new_frontier/action_arena.lua:1-89 and movement_minotaur_boss.lua:1-41.
use super::{Callback, Encounter, EncounterMonster, Position, TargetTransform, Trigger};
use crate::quest::{QuestComparison, QuestEffect, QuestEffectKind, QuestTransition};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WaveRun {
    pub timeout_ms: u32,
    pub timeout_exit: Position,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_credit: Option<WaveCredit>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub victory: Option<WaveVictory>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub eject_room: bool,
    /// Stations can open a route that participants subsequently take themselves.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stay_at_stations: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hatch: Option<TargetTransform>,
    /// A non-participant may operate the switch for the staged group.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub allow_unstaged_activator: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub member_guard: Option<crate::quest::reward_callbacks::TrackRequirement>,
    pub waves: Vec<Wave>,
}

/// An exact spawned boss's native death opens the reward exit.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WaveVictory {
    pub creature: String,
    pub exit: Position,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit: Option<WaveCredit>,
}

/// An egress can reference an existing transition or declare its exact quest writes.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged, deny_unknown_fields)]
pub enum WaveCredit {
    Reference(String),
    Bound {
        key: String,
        effects: Vec<WaveCreditEffect>,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WaveCreditEffect {
    pub track: String,
    pub minimum: i64,
    pub maximum: i64,
    pub set: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Wave {
    pub at_ms: u32,
    pub spawn: WaveSpawn,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum WaveSpawn {
    Fixed {
        monsters: Vec<EncounterMonster>,
    },
    Scatter {
        creature: String,
        count: u8,
        attempts_per_creature: u8,
        min: Position,
        max: Position,
    },
}

impl WaveSpawn {
    pub fn creatures(&self) -> impl Iterator<Item = &str> {
        let (monsters, scattered) = match self {
            Self::Fixed { monsters } => (monsters.as_slice(), None),
            Self::Scatter { creature, .. } => (&[][..], Some(creature.as_str())),
        };
        monsters
            .iter()
            .map(|monster| monster.creature.as_str())
            .chain(scattered)
    }
}

pub fn contains(room: &Encounter, at: Position) -> bool {
    at.floor == room.room_min.floor
        && (room.room_min.x..=room.room_max.x).contains(&at.x)
        && (room.room_min.y..=room.room_max.y).contains(&at.y)
}

pub const CREDIT_PREFIX: &str = "oteryn:quest-transition/wave-egress/";

impl WaveCredit {
    pub fn key(&self) -> &str {
        match self {
            Self::Reference(key) | Self::Bound { key, .. } => key,
        }
    }

    /// Admission and the survivor writer must agree before players enter the room.
    pub fn matches_transition(&self, quest: &str, transition: &QuestTransition) -> bool {
        if let Some(authored) = self.authored_transition(quest) {
            return &authored == transition;
        }
        self.key() == transition.key
            && transition.quest == quest
            && !transition.completes
            && transition.experience.is_none()
            && transition.world_change.is_none()
            && crate::quest::admission_binding::validate(transition, false).is_ok()
    }

    fn valid(&self) -> bool {
        let key = self.key();
        key.starts_with(CREDIT_PREFIX)
            && crate::quest::valid_quest_key(key)
            && match self {
                Self::Reference(_) => true,
                Self::Bound { effects, .. } => {
                    (1..=super::MAX_EFFECTS).contains(&effects.len())
                        && effects.iter().all(|effect| {
                            crate::quest::valid_quest_key(&effect.track)
                                && effect.minimum <= effect.maximum
                        })
                }
            }
    }

    pub fn authored_transition(&self, quest: &str) -> Option<QuestTransition> {
        let Self::Bound { key, effects } = self else {
            return None;
        };
        Some(QuestTransition {
            key: key.clone(),
            quest: quest.into(),
            completes: false,
            experience: None,
            world_change: None,
            effects: effects
                .iter()
                .map(|effect| QuestEffect {
                    track: effect.track.clone(),
                    from: QuestComparison::Between(effect.minimum, effect.maximum),
                    effect: QuestEffectKind::Set(effect.set),
                })
                .collect(),
        })
    }
}

impl WaveRun {
    pub fn credit(&self, victory: bool) -> Option<&WaveCredit> {
        if victory {
            self.victory.as_ref()?.credit.as_ref()
        } else {
            self.timeout_credit.as_ref()
        }
    }

    pub fn credit_transition(&self, victory: bool) -> Option<&str> {
        self.credit(victory).map(WaveCredit::key)
    }
}

pub(crate) fn authored_transitions(callback: &Callback) -> impl Iterator<Item = QuestTransition> {
    let run = callback
        .effects
        .encounter
        .as_ref()
        .and_then(|room| room.wave_run.as_ref());
    [false, true]
        .into_iter()
        .filter_map(move |victory| run?.credit(victory)?.authored_transition(&callback.quest))
}

pub(super) fn valid(callback: &Callback, room: &Encounter, run: &WaveRun) -> bool {
    let outside = |p: Position| (-15..=0).contains(&p.floor) && !contains(room, p);
    let area = (u32::from(room.room_max.x.saturating_sub(room.room_min.x)) + 1)
        .saturating_mul(u32::from(room.room_max.y.saturating_sub(room.room_min.y)) + 1);
    matches!(callback.trigger, Trigger::Use | Trigger::StepIn)
        && callback.teleport.is_none()
        && (!run.allow_unstaged_activator || callback.trigger == Trigger::Use)
        && run.member_guard.as_ref().is_none_or(|guard| {
            callback.trigger == Trigger::Use && !callback.has_state()
                && crate::quest::valid_quest_key(&guard.track) && guard.minimum <= guard.maximum
        })
        && run.credit(false).is_none_or(WaveCredit::valid)
        && run.credit(true).is_none_or(WaveCredit::valid)
        && (1..=3_600_000).contains(&run.timeout_ms)
        && area <= 4096
        && room.phase.is_none()
        && room.timing.is_none()
        && room.monsters.is_empty()
        && outside(run.timeout_exit)
        && run.victory.as_ref().is_none_or(|victory| {
            outside(victory.exit)
                && run.waves.last().is_some_and(|last| {
                    matches!(&last.spawn, WaveSpawn::Fixed { monsters }
                        if monsters.iter().filter(|monster| monster.creature == victory.creature).count() == 1)
                })
        })
        && run.hatch.as_ref().is_none_or(|hatch| {
            run.stay_at_stations
                && outside(hatch.position)
                && hatch.from != 0
                && hatch.to != 0
                && hatch.from != hatch.to
                && (1..=run.timeout_ms).contains(&hatch.reset_after_ms)
        })
        && run.waves.len() <= 32
        && run
            .waves
            .windows(2)
            .all(|pair| pair[0].at_ms < pair[1].at_ms)
        && run.waves.iter().all(|wave| {
            wave.at_ms < run.timeout_ms
                && wave.spawn.creatures().all(crate::quest::valid_quest_key)
                && match &wave.spawn {
                    WaveSpawn::Fixed { monsters } => {
                        (1..=32).contains(&monsters.len())
                            && monsters.iter().enumerate().all(|(index, monster)| {
                                contains(room, monster.position)
                                    && monsters[..index]
                                        .iter()
                                        .all(|prior| prior.position != monster.position)
                            })
                    }
                    WaveSpawn::Scatter {
                        count,
                        attempts_per_creature,
                        min,
                        max,
                        ..
                    } => {
                        (1..=32).contains(count)
                            && (1..=10).contains(attempts_per_creature)
                            && contains(room, *min)
                            && contains(room, *max)
                            && min.x <= max.x
                            && min.y <= max.y
                    }
                }
        })
}
