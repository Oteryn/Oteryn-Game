//! Durable Ground guard composition.
use super::{ActorKind, Callback, Trigger};

impl Callback {
    pub fn has_ground_stack_requirements(&self) -> bool {
        self.world_requirements
            .iter()
            .any(|r| r.ground_stack.is_some())
    }

    /// Personal progress-only actions may include a bounded dense STEP teleport.
    /// Item consumption, rewards and reversible map actions retain their contracts.
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
            && (self.teleport.is_none() || super::dense_step_teleport::admitted(self))
            && self.effects.is_empty()
            && self.offerings.is_empty()
            && self.item_on_target.is_none()
            && self.experience.is_none()
            && self.monk_reward.is_none()
            && self.native_rule.is_none()
            && self.world_change.is_none()
    }
}
