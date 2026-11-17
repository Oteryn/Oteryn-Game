use crate::input::StepDir;
use crate::play::PlayView;
use oteryn_client_simulation::movement::{DEFAULT_GROUND_SPEED, StepSpeedTable};
use oteryn_client_simulation::walk::{Glide, glide_duration};
use oteryn_renderer::BatchError;
use oteryn_session::{StepDirection, StepDisposition, StepOutcome};
use std::time::{Duration, Instant};

/// A character's step before any step has set the pace: base speed 110 on ground speed 150.
fn default_step_duration() -> Duration {
    StepSpeedTable::embedded()
        .and_then(|table| table.step_duration(110, DEFAULT_GROUND_SPEED))
        .unwrap_or(Duration::from_millis(550))
}

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

/// The own walk as the camera shows it, between server step outcomes.
#[derive(Debug, Clone, Copy, Default)]
pub(in crate::play) struct WalkFeel {
    /// The own step being smoothed, and when it began.
    pub(in crate::play) glide: Option<(Glide, Instant)>,
    /// When the last step landed, to pace the next glide by.
    landed: Option<Instant>,
    /// Arrow keys held now, for diagonal chords.
    chord: oteryn_client_ui::diagonal_movement::ArrowChord,
}

impl PlayView {
    /// When the own step now being smoothed began; `None` at rest or after a snap.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn walk_started(&self) -> Option<Instant> {
        self.walk.glide.map(|(_, started)| started)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn walk_duration(&self) -> Option<Duration> {
        self.walk.glide.map(|(glide, _)| glide.duration())
    }

    /// Screen pixels the world scrolls by at `now` while the camera follows the own step.
    #[must_use]
    pub fn camera_offset(&self, now: Instant) -> Option<[f32; 2]> {
        let (glide, started) = self.walk.glide?;
        let elapsed = now.saturating_duration_since(started);
        (!glide.finished(elapsed))
            .then(|| glide.camera_offset(elapsed, self.scene.view().tile_px() as f32))
    }

    /// Applies one step outcome: the domain-1 delta moves the own actor, the domain-2 delta
    /// moves a marker. Target walking is owned by the server.
    pub fn apply(&mut self, outcome: &StepOutcome) -> Result<(), BatchError> {
        self.in_flight = false;
        let now = Instant::now();
        let moved = outcome.disposition == StepDisposition::Moved;
        if let Some(delta) = &outcome.world_spatial_delta {
            let (from, floor) = (self.own, self.floor);
            self.place(delta.value.actor_position);
            if moved && floor == self.floor {
                let gap = self.walk.landed.map(|landed| now.duration_since(landed));
                let duration = glide_duration(gap, default_step_duration());
                self.walk.glide = Glide::new(from, self.own, duration).map(|glide| (glide, now));
            }
        }
        self.walk.landed = moved.then_some(now);
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
        // Held state follows every event, selected or not, so a release is never missed.
        let chord = self.walk.chord.step(events);
        if !selected {
            return false;
        }
        if let Some(direction) =
            oteryn_client_ui::diagonal_movement::keypad_step(events, selected).or(chord)
        {
            self.arrow(direction);
            true
        } else {
            false
        }
    }
}
