//! Crystal 201189f8 data-global/scripts/quests/demon_oak/actions_demon_oak.lua:14-54,62-126
//! and movements_area_damage.lua:3-16.
use super::{Callback, Position, Trigger};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EarthHit {
    pub chance: Chance,
    pub minimum: u16,
    pub maximum: u16,
    pub effect: u16,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Chance {
    pub wins: u16,
    pub rolls: u16,
}

impl Chance {
    fn valid(self) -> bool {
        self.wins > 0 && self.wins <= self.rolls
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Retaliation {
    pub progress: String,
    pub weak_points: Vec<String>,
    pub completed_at: i64,
    pub positions: Vec<Position>,
    pub ordinary_chance: Chance,
    pub ordinary: Vec<String>,
    pub special: Vec<SpecialWave>,
    pub last: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SpecialWave {
    pub before: i64,
    pub creatures: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FinishArea {
    pub minimum: Position,
    pub maximum: Position,
}

impl FinishArea {
    pub fn contains(&self, position: Position) -> bool {
        position.floor == self.minimum.floor
            && (self.minimum.x..=self.maximum.x).contains(&position.x)
            && (self.minimum.y..=self.maximum.y).contains(&position.y)
    }

    fn valid(&self, callback: &Callback) -> bool {
        callback.trigger == Trigger::Use
            && callback.requires_world_admission
            && callback.item_on_target.is_some()
            && callback.teleport.is_some()
            && callback.has_state_writes()
            && callback.effects.retaliation.is_none()
            && callback.effects.earth_hit.is_none()
            && (-15..=0).contains(&self.minimum.floor)
            && self.minimum.floor == self.maximum.floor
            && self.minimum.x <= self.maximum.x
            && self.minimum.y <= self.maximum.y
            && self.maximum.x - self.minimum.x < 64
            && self.maximum.y - self.minimum.y < 64
    }
}

pub(super) fn valid(callback: &Callback) -> bool {
    let effects = &callback.effects;
    if effects
        .finish_area
        .as_ref()
        .is_some_and(|area| !area.valid(callback))
    {
        return false;
    }
    let Some(hit) = effects.earth_hit else {
        return effects.retaliation.is_none();
    };
    if !hit.chance.valid()
        || hit.minimum == 0
        || hit.minimum > hit.maximum
        || !(1..=255).contains(&hit.effect)
        || effects.encounter.is_some()
    {
        return false;
    }
    let Some(plan) = &effects.retaliation else {
        return callback.trigger == Trigger::StepIn
            && !callback.has_state()
            && callback.teleport.is_none();
    };
    let creatures = |values: &[String]| {
        (1..=8).contains(&values.len())
            && values.iter().all(|key| crate::quest::valid_quest_key(key))
    };
    callback.trigger == Trigger::Use
        && callback.requires_world_admission
        && callback.item_on_target.is_some()
        && callback.teleport.is_none()
        && (1..=8).contains(&plan.weak_points.len())
        && plan.weak_points.iter().enumerate().all(|(i, key)| {
            !plan.weak_points[..i].contains(key)
                && callback.storage.iter().any(|state| {
                    state.track == *key
                        && !state.read_only
                        && if *key == plan.progress {
                            !state.preserve
                                && state.add == Some(1)
                                && state.set == plan.completed_at
                        } else {
                            state.preserve
                        }
                })
        })
        && plan.weak_points.contains(&plan.progress)
        && (1..=16).contains(&plan.positions.len())
        && plan.positions.iter().all(|p| (-15..=0).contains(&p.floor))
        && plan.ordinary_chance.valid()
        && creatures(&plan.ordinary)
        && (1..=8).contains(&plan.special.len())
        && plan
            .special
            .windows(2)
            .all(|pair| pair[0].before < pair[1].before)
        && plan
            .special
            .iter()
            .all(|stage| stage.before < plan.completed_at && creatures(&stage.creatures))
        && crate::quest::valid_quest_key(&plan.last)
}
