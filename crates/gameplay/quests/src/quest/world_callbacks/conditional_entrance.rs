//! Read-only predicates for an entrance's existing StepIn or USE callback.
use super::{ActorKind, Callback, Position, Trigger};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Entrance {
    pub guards: Vec<Guard>,
    /// Keep the exact stair or portal tile for StepIn before native routing.
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub intercept_teleport: bool,
    /// The closed door's exact USE target is the scripted landing.
    #[serde(default, skip_serializing_if = "super::is_false")]
    pub teleport_to_source: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_admit_speech: Option<Speech>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Guard {
    pub when: Predicate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refusal: Option<Refusal>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Predicate {
    All {
        predicates: Vec<Self>,
    },
    Any {
        predicates: Vec<Self>,
    },
    Storage {
        range: StorageRange,
    },
    // Crystal soul_war/moveevent-soul_war_entrances.lua:68-81 counts each boss once.
    DistinctFlags {
        minimum: usize,
        flags: Vec<Flag>,
    },
    Carried {
        item: String,
        count: u16,
    },
    Level {
        minimum: u32,
    },
    Premium,
    Unoccupied {
        minimum: Position,
        maximum: Position,
    },
    Calendar {
        start: MonthDay,
        end: MonthDay,
    },
    NoPvpLock,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StorageRange {
    pub source: StorageSource,
    pub minimum: i64,
    pub maximum: i64,
}

impl StorageRange {
    pub fn contains(&self, value: i64) -> bool {
        (self.minimum..=self.maximum).contains(&value)
    }

    fn valid(&self) -> bool {
        self.minimum <= self.maximum
            && match &self.source {
                StorageSource::Track { track } => {
                    track.starts_with("oteryn:quest-progress/")
                        && track.len() <= 128
                        && super::super::valid_quest_key(track)
                }
                StorageSource::SoulWarBoss { boss } => boss.bit() != 0,
                StorageSource::SoulWarCompleted => true,
            }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum StorageSource {
    Track {
        track: String,
    },
    SoulWarBoss {
        boss: super::super::soul_war::SoulWarBoss,
    },
    SoulWarCompleted,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Flag {
    pub range: StorageRange,
    pub label: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(deny_unknown_fields)]
pub struct MonthDay(pub u8, pub u8);

impl MonthDay {
    fn valid(self) -> bool {
        let Self(month, day) = self;
        let limit = match month {
            2 => 29,
            4 | 6 | 9 | 11 => 30,
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            _ => 0,
        };
        (1..=limit).contains(&day)
    }

    pub fn in_window(self, start: Self, end: Self) -> bool {
        if start <= end {
            (start..=end).contains(&self)
        } else {
            self >= start || self <= end
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Refusal {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "MessageKind::is_default")]
    pub message_kind: MessageKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speech_position: Option<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub push_back: Option<PushBack>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<u8>,
    #[serde(default, skip_serializing_if = "EffectAt::is_default")]
    pub effect_at: EffectAt,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    #[default]
    Advance,
    Status,
    MonsterSay,
    MonsterYell,
}

impl MessageKind {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }

    pub fn monster_talk_type(self) -> Option<u8> {
        match self {
            Self::MonsterSay => Some(36),
            Self::MonsterYell => Some(37),
            Self::Advance | Self::Status => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Speech {
    pub message: String,
    pub message_kind: MessageKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub position: Option<Position>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EffectAt {
    #[default]
    Actor,
    Source,
    Both,
}

impl EffectAt {
    fn is_default(&self) -> bool {
        *self == Self::default()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum PushBack {
    From { dx: i8, dy: i8 },
    Fixed { position: Position },
}

impl Predicate {
    pub fn visit(&self, visitor: &mut impl FnMut(&Self)) {
        visitor(self);
        if let Self::All { predicates } | Self::Any { predicates } = self {
            for predicate in predicates {
                predicate.visit(visitor);
            }
        }
    }

    fn valid(&self, remaining: &mut usize, depth: u8) -> bool {
        if *remaining == 0 || depth > 4 {
            return false;
        }
        *remaining -= 1;
        match self {
            Self::All { predicates } | Self::Any { predicates } => {
                !predicates.is_empty() && predicates.iter().all(|p| p.valid(remaining, depth + 1))
            }
            Self::Storage { range } => range.valid(),
            Self::DistinctFlags { minimum, flags } => {
                (1..=16).contains(&flags.len())
                    && (1..=flags.len()).contains(minimum)
                    && flags
                        .iter()
                        .all(|f| f.range.valid() && valid_text(&f.label, 48))
                    && flags
                        .iter()
                        .map(|f| &f.range.source)
                        .collect::<BTreeSet<_>>()
                        .len()
                        == flags.len()
            }
            Self::Carried { item, count } => {
                (1..=100).contains(count)
                    && item.starts_with("oteryn:item.")
                    && super::super::valid_quest_key(item)
            }
            Self::Level { minimum } => *minimum > 0,
            Self::Unoccupied { minimum, maximum } => {
                valid_position(*minimum)
                    && minimum.floor == maximum.floor
                    && minimum.x <= maximum.x
                    && minimum.y <= maximum.y
                    && maximum.x - minimum.x < 64
                    && maximum.y - minimum.y < 64
            }
            Self::Calendar { start, end } => start.valid() && end.valid(),
            Self::Premium | Self::NoPvpLock => true,
        }
    }
}

fn valid_text(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && !value.chars().any(char::is_control)
}

fn valid_position(pos: Position) -> bool {
    (-15..=0).contains(&pos.floor)
}

pub(super) fn valid(callback: &Callback) -> bool {
    let Some(gate) = &callback.effects.conditional_entrance else {
        return true;
    };
    let is_step = callback.trigger == Trigger::StepIn;
    let mut remaining = 32;
    callback.actor_kind == ActorKind::Player
        && (is_step || callback.trigger == Trigger::Use)
        && (!gate.intercept_teleport
            || is_step
                && (callback.selector.position.is_some() || callback.selector.native.is_some()))
        && gate.on_admit_speech.as_ref().is_none_or(|speech| {
            is_step
                && speech.message_kind.monster_talk_type().is_some()
                && valid_text(&speech.message, 255)
                && speech.position.is_none_or(valid_position)
        })
        && (!gate.teleport_to_source
            || callback.trigger == Trigger::Use
                && callback.teleport.is_none()
                && callback.selector.appearance.is_some()
                && (callback.selector.unique.is_some() || callback.selector.native.is_some())
                && callback.effects.transform.is_some()
                && !callback.requires_durable_commit()
                && callback.passages.is_empty()
                && callback.effects.scene.is_none())
        && callback.native_rule.is_none()
        && callback.lever.is_none()
        && !callback.return_on_failure
        && (!gate.guards.is_empty() || gate.intercept_teleport && callback.teleport.is_some())
        && gate.guards.iter().all(|guard| {
            guard.when.valid(&mut remaining, 0)
                && guard.refusal.as_ref().is_none_or(|failure| {
                    failure
                        .message
                        .as_deref()
                        .is_none_or(|message| valid_text(message, 255))
                        && failure.effect != Some(0)
                        && (failure.effect_at.is_default() || failure.effect.is_some())
                        && (failure.message.is_some() || failure.message_kind.is_default())
                        && failure.speech_position.is_none_or(|position| {
                            failure.message_kind.monster_talk_type().is_some()
                                && valid_position(position)
                        })
                        && match failure.push_back {
                            Some(PushBack::From { dx, dy }) => {
                                is_step && dx.abs_diff(0) <= 1 && dy.abs_diff(0) <= 1
                            }
                            Some(PushBack::Fixed { position }) => {
                                is_step && valid_position(position)
                            }
                            None => true,
                        }
                })
        })
}
