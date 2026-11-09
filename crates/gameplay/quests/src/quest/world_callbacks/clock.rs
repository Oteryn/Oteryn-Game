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
    /// Each new native flower revision may grant again after its shared regrowth.
    pub fn has_fenced_regrowth_reward(&self) -> bool {
        self.trigger == super::Trigger::Use
            && self.actor_kind == super::ActorKind::Player
            && self.requires_world_admission
            && self
                .selector
                .native
                .as_ref()
                .is_some_and(|s| s.appearance == 21388)
            && self.effects.transform == Some(21935)
            && self.effects.reset_after_ms == Some(60_000)
            && self
                .effects
                .item_reward
                .as_ref()
                .is_some_and(|r| r.item == "oteryn:item.tibia.i21389" && r.count == 1)
            && self.effects.item_reward_cycle.is_none()
            && self
                .key
                .strip_prefix("oteryn:quest-callback/m44-lions-flower-")
                .is_some_and(|tail| tail.len() == 1 && (b'0'..=b'7').contains(&tail.as_bytes()[0]))
    }
    /// Item custody is fenced even when the source writes no personal progress.
    pub fn requires_durable_commit(&self) -> bool {
        self.has_personal_state()
            || super::super::dreamer_fire_bug::shared_input(self)
            || super::super::dreamer_carrot::shared_input(self)
    }

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
