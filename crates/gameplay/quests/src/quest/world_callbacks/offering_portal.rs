//! Crystal 201189f dreamers_challenge_quest/actions_stone_teleport.lua:1-68.
use super::{ActorKind, Callback, OFFERING_PORTAL_PREFIX, Position, Trigger};
use serde::{Deserialize, Serialize};

pub const FLUID_POOL: &str = "oteryn:item.tibia.i2886";
pub const SACRIFICE_FIRE: &str = "oteryn:item.tibia.i2118";
pub const STONE_KEY: &str = "oteryn:quest-callback/offering-portal/dreamers-challenge-stone";

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OfferingPortal {
    pub position: Position,
    pub sealed: u16,
    pub opened: u16,
}

impl OfferingPortal {
    pub(super) fn valid(self, callback: &Callback) -> bool {
        callback.key.starts_with(OFFERING_PORTAL_PREFIX)
            && callback.trigger == Trigger::Use
            && callback.actor_kind == ActorKind::Player
            && callback.selector.native.as_ref().is_some_and(|source| {
                source.position != self.position && source.position.floor == self.position.floor
            })
            && self.position.x > 0 && self.position.y > 0
            && (-15..=0).contains(&self.position.floor)
            && self.sealed > 0
            && self.opened > 0
            && self.sealed != self.opened
            && callback.lever.is_some()
            && callback.reversible
            && callback.requires_world_admission
            && callback.opening_only
            && !callback.has_personal_state()
            && callback.teleport.is_none()
            && callback.item_on_target.is_none()
            && callback.offerings.len() > 1
            && (callback.key == STONE_KEY
                || callback
                    .offerings
                    .iter()
                    .all(|offering| offering.item != FLUID_POOL && offering.item != SACRIFICE_FIRE))
            && callback.offerings.iter().all(|offering| {
                offering.position.floor == self.position.floor && offering.count == 1
            })
            && callback.passages.iter().all(|passage| {
                passage.if_present
                    && passage.position != self.position
                    && passage.position.floor == self.position.floor
                    && passage.replacement.is_some()
                    && passage.create_on_close.is_none()
            })
            && *callback.effects
                == super::CallbackEffects {
                    offering_portal: Some(self),
                    ..Default::default()
                }
    }
}
