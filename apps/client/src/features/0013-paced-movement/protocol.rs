use crate::input::StepDir;
use crate::play::PlayView;
use oteryn_renderer::BatchError;
use oteryn_session::{StepDirection, StepOutcome};

pub(in crate::play) const fn step_direction(direction: StepDir) -> StepDirection {
    match direction {
        StepDir::North => StepDirection::North,
        StepDir::East => StepDirection::East,
        StepDir::South => StepDirection::South,
        StepDir::West => StepDirection::West,
        StepDir::NorthEast => StepDirection::NorthEast,
        StepDir::SouthEast => StepDirection::SouthEast,
        StepDir::SouthWest => StepDirection::SouthWest,
        StepDir::NorthWest => StepDirection::NorthWest,
    }
}

impl PlayView {
    /// Applies one step outcome: the domain-1 delta moves the own actor, the domain-2 delta
    /// moves a marker. Target walking is owned by the server.
    pub fn apply(&mut self, outcome: &StepOutcome) -> Result<(), BatchError> {
        self.in_flight = false;
        if let Some(delta) = &outcome.world_spatial_delta {
            self.place(delta.value.actor_position);
        }
        if let Some(delta) = &outcome.world_object_overlay_delta {
            self.set_marker(&delta.value);
        }
        self.rebuild()
    }
}
