//! Crystal 201189f the_dream_courts/creaturescripts_izcandar.lua:1-72.
use super::{ActorKind, Callback, Position, Trigger};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PositionForms {
    pub delay_seconds: [u8; 2],
    pub regions: Vec<PositionForm>,
    pub exit: Position,
    pub time_limit_ms: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PositionForm {
    pub min: Position,
    pub max: Position,
    pub creature: String,
}

impl PositionForms {
    pub(super) fn valid(&self, callback: &Callback) -> bool {
        let Some(encounter) = &callback.effects.encounter else {
            return false;
        };
        let [boss] = encounter.monsters.as_slice() else {
            return false;
        };
        let [first, last] = self.delay_seconds;
        let within = |at: Position| {
            at.floor == encounter.room_min.floor
                && (encounter.room_min.x..=encounter.room_max.x).contains(&at.x)
                && (encounter.room_min.y..=encounter.room_max.y).contains(&at.y)
        };
        let mut creatures = BTreeSet::new();
        callback.trigger == Trigger::Use
            && callback.actor_kind == ActorKind::Player
            && callback.selector.native.is_some()
            && callback.lever.is_none()
            && !callback.reversible
            && !callback.has_personal_state()
            && callback.effects.threshold_rearm.is_none()
            && encounter.solo_expiry.is_none()
            && encounter.timing.is_none()
            && encounter.phase.is_none()
            && encounter.linked_arrival.is_none()
            && (1..=60).contains(&first)
            && (first..=60).contains(&last)
            && (2..=8).contains(&self.regions.len())
            && (u32::from(last) * 1000 + 1..=3_600_000).contains(&self.time_limit_ms)
            && (-15..=0).contains(&self.exit.floor)
            && !within(self.exit)
            && self.regions.iter().enumerate().all(|(index, region)| {
                let PositionForm { min, max, creature } = region;
                min.floor == encounter.room_min.floor
                    && max.floor == min.floor
                    && min.x <= max.x
                    && min.y <= max.y
                    && min.x <= encounter.room_max.x
                    && max.x >= encounter.room_min.x
                    && min.y <= encounter.room_max.y
                    && max.y >= encounter.room_min.y
                    && creature != &boss.creature
                    && creature.starts_with("oteryn:creature.")
                    && super::super::valid_quest_key(creature)
                    && creatures.insert(creature)
                    && self.regions[..index].iter().all(|prior| {
                        max.x < prior.min.x
                            || min.x > prior.max.x
                            || max.y < prior.min.y
                            || min.y > prior.max.y
                    })
            })
    }
}
