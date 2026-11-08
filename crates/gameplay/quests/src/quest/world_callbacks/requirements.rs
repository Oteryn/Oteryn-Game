//! Keep immutable map appearances separate from live Ground custody predicates.
use super::{ActorKind, Callback, Position, Trigger};
use crate::quest::ground_stack::GroundStackRequirement;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged, deny_unknown_fields)]
pub enum WorldRequirement {
    /// Legacy wire shape and serialization remain exactly position + appearance.
    Immovable { position: Position, appearance: u16 },
    Ground {
        ground_stack: GroundStackRequirement,
    },
}

impl WorldRequirement {
    pub fn position(&self) -> Position {
        match self {
            Self::Immovable { position, .. } => *position,
            Self::Ground { ground_stack } => ground_stack.position,
        }
    }

    pub fn ground_stack(&self) -> Option<&GroundStackRequirement> {
        match self {
            Self::Ground { ground_stack } => Some(ground_stack),
            Self::Immovable { .. } => None,
        }
    }

    pub(super) fn valid(&self) -> bool {
        (-15..=0).contains(&self.position().floor)
            && match self {
                Self::Immovable { appearance, .. } => *appearance > 0,
                Self::Ground { ground_stack } => ground_stack.validate().is_ok(),
            }
    }
}

impl Callback {
    pub fn has_ground_stack_requirements(&self) -> bool {
        self.world_requirements
            .iter()
            .any(|r| r.ground_stack().is_some())
    }

    /// Initial composition is a personal progress-only action. Item consumption,
    /// rewards and reversible map actions retain their existing lock contracts.
    pub fn supports_ground_stack_guard(&self) -> bool {
        self.has_ground_stack_requirements()
            && self.actor_kind == ActorKind::Player
            && matches!(self.trigger, Trigger::Use | Trigger::StepIn)
            && self.storage.iter().any(|state| !state.preserve)
            && self.state_ops.is_empty()
            && !self.reversible
            && !self.opening_only
            && self.lever.is_none()
            && self.passages.is_empty()
            && self.teleport.is_none()
            && self.effects.is_empty()
            && self.offerings.is_empty()
            && self.item_on_target.is_none()
            && self.experience.is_none()
            && self.monk_reward.is_none()
            && self.native_rule.is_none()
            && self.world_change.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn immutable_requirement_retains_legacy_wire_shape() -> Result<(), serde_json::Error> {
        let old = serde_json::json!({"position":{"x":1,"y":1,"floor":-1},"appearance":2773});
        let parsed: WorldRequirement = serde_json::from_value(old.clone())?;
        assert!(parsed.ground_stack().is_none());
        assert_eq!(serde_json::to_value(parsed)?, old);
        Ok(())
    }
}
