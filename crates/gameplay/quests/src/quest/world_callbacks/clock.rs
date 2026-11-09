//! Clock-only callbacks still require the personal fenced writer.
use super::Callback;

impl Callback {
    /// A command cannot manufacture a reward epoch without waiting on the SQL clock.
    pub fn clock_reward_epoch(&self, track: &str) -> bool {
        self.state_ops.iter().any(|operation| {
            operation.track == track
                && matches!(
                    operation.from,
                    crate::quest::reward_callbacks::StateCondition::ElapsedAtLeast { seconds: 1.. }
                )
                && operation.effect == crate::quest::reward_callbacks::StateMutation::SetNow
        })
    }

    pub fn has_personal_state(&self) -> bool {
        !self.storage.is_empty() || !self.state_ops.is_empty()
    }
}
