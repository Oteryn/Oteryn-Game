use crate::input::{StepDir, StepResult};
use crate::play::PlayView;
use oteryn_renderer::BatchError;
use oteryn_session::{StepDirection, StepDisposition, StepOutcome};

pub(in crate::play) const fn step_direction(direction: StepDir) -> StepDirection {
    match direction {
        StepDir::North => StepDirection::North,
        StepDir::East => StepDirection::East,
        StepDir::South => StepDirection::South,
        StepDir::West => StepDirection::West,
    }
}

impl PlayView {
    /// Applies one step outcome: the domain-1 delta moves the own actor, the domain-2 delta
    /// moves a marker. A refused step ends the click walk.
    pub fn apply(&mut self, outcome: &StepOutcome) -> Result<(), BatchError> {
        self.in_flight = false;
        self.walk
            .on_result(if outcome.disposition == StepDisposition::Moved {
                StepResult::Moved
            } else {
                StepResult::Refused
            });
        if let Some(delta) = &outcome.world_spatial_delta {
            self.place(delta.value.actor_position);
        }
        if let Some(delta) = &outcome.world_object_overlay_delta {
            self.set_marker(&delta.value);
        }
        self.rebuild()
    }
}
