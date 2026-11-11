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

impl crate::play::Outbox {
    pub(in crate::play) fn diagonal_movement_initialize(&self, selected: bool) {
        crate::play::mailbox::lock(&self.0).diagonal_movement_selected = selected;
    }
}
impl PlayView {
    pub fn diagonal_movement_input(
        &mut self,
        link: &crate::play::PlayLink,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> bool {
        let selected = {
            let mailbox = crate::play::mailbox::lock(&link.mailbox);
            mailbox.diagonal_movement_selected && !mailbox.world_input_blocked()
        };
        if selected
            && let Some(direction) =
                oteryn_client_ui::diagonal_movement::keypad_step(events, selected)
        {
            self.arrow(direction);
            true
        } else {
            false
        }
    }
}
