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
    /// Coalesces consecutive repeats and reports saturation. A full queue yields to logout.
    pub(super) fn queue(&mut self, input: Input) {
        if (self.death.dead || super::feature_0450_server_modal::view::active())
            && !matches!(input, Input::Logout)
        {
            return;
        }
        if self.inputs.back() == Some(&input) {
            return;
        }
        if matches!(input, Input::Logout) {
            self.inputs.clear();
        } else if self.inputs.len() >= MAX_QUEUED_INPUTS {
            self.action_line = Some(tr!(
                "Resolve the pending operation before starting another."
            ));
            return;
        }
        self.inputs.push_back(input);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distinct_inputs_survive_intervening_actions() -> Result<(), oteryn_renderer::BatchError> {
        let mut view = super::super::tests::view()?;
        view.in_flight = true;
        view.arrow(StepDir::North);
        view.cast(NonZeroU32::MIN);
        view.arrow(StepDir::East);
        view.cast(NonZeroU32::MIN.saturating_add(1));
        view.arrow(StepDir::North);
        assert_eq!(
            view.inputs.iter().copied().collect::<Vec<_>>(),
            [
                Input::Arrow(StepDir::North),
                Input::Cast(NonZeroU32::MIN),
                Input::Arrow(StepDir::East),
                Input::Cast(NonZeroU32::MIN.saturating_add(1)),
                Input::Arrow(StepDir::North),
            ]
        );
        Ok(())
    }

    #[test]
    fn saturation_reports_rejection_and_preserves_logout() -> Result<(), oteryn_renderer::BatchError>
    {
        let mut view = super::super::tests::view()?;
        for slot in 0..MAX_QUEUED_INPUTS as u32 {
            view.queue(Input::Cast(NonZeroU32::MIN.saturating_add(slot)));
        }
        assert_eq!(view.inputs.len(), MAX_QUEUED_INPUTS);
        view.queue(Input::Arrow(StepDir::South));
        assert!(view.action_line.is_some());
        view.queue(Input::Logout);
        assert_eq!(view.inputs.front(), Some(&Input::Logout));
        Ok(())
    }
}
