//! Clock-only callbacks still require the personal fenced writer.
use super::Callback;

impl Callback {
    /// The native amphora ring is a fenced physical STEP reward, never a command grant.
    pub fn has_fenced_step_reward(&self) -> bool {
        self.trigger == super::Trigger::StepIn
            && self.actor_kind == super::ActorKind::Player
            && self.requires_world_admission
            && self.selector.native.is_some()
            && self.effects.target_transform.is_some()
            && self.effects.item_reward.is_some()
            && self
                .key
                .strip_prefix("oteryn:quest-callback/m44-lions-amphora-")
                .is_some_and(|tail| {
                    tail.len() == 1
                        && tail.as_bytes()[0].is_ascii_digit()
                        && tail.as_bytes()[0] <= b'7'
                })
    }
    pub fn has_personal_state(&self) -> bool {
        !self.storage.is_empty() || !self.state_ops.is_empty()
    }
}
