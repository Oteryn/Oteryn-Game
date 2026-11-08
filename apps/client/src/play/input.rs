//! Chronological native play input queue.
use super::{PlayView, StepDir};
use oteryn_session::EntityRef;
use std::num::NonZeroU32;

/// One input waiting for its turn on the session task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Input {
    Arrow(StepDir),
    Cast(NonZeroU32),
    CastAt(NonZeroU32, oteryn_session::SpellTargetPosition),
    Attack(EntityRef),
    TargetWalk(oteryn_session::ActorPosition),
    Logout,
}

const MAX_QUEUED_INPUTS: usize = 16;

impl PlayView {
    /// Appends `input` unless the same one already waits (a repeated press of a slot or click of
    /// a target is one command) or the queue is full.
    pub(super) fn queue(&mut self, input: Input) {
        if (self.death.dead || super::feature_0449_server_modal::view::active())
            && !matches!(input, Input::Logout)
        {
            return;
        }
        if self.inputs.len() < MAX_QUEUED_INPUTS && !self.inputs.contains(&input) {
            self.inputs.push_back(input);
        }
    }
}
