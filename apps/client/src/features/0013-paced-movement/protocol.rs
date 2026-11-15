use crate::input::StepDir;
use crate::play::PlayView;
use oteryn_client_simulation::movement::{DEFAULT_GROUND_SPEED, StepSpeedTable};
use oteryn_client_simulation::walk::Glide;
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

impl PlayView {
    /// When the own step now being smoothed began; `None` at rest or after a snap.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn walk_started(&self) -> Option<Instant> {
        self.walk.map(|(_, started)| started)
    }

    #[cfg(test)]
    #[must_use]
    pub(crate) fn walk_duration(&self) -> Option<Duration> {
        self.walk.map(|(glide, _)| glide.duration())
    }

    /// Screen pixels the world scrolls by at `now` while the camera follows the own step.
    #[must_use]
    pub fn camera_offset(&self, now: Instant) -> Option<[f32; 2]> {
        let (glide, started) = self.walk?;
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
        let previous = self.walk;
        let prediction = self.prewalk.take();
        if !moved && let Some((origin, _)) = prediction {
            self.place(origin);
        }
        if !moved {
            self.walk = None;
        }
        if let Some(delta) = &outcome.world_spatial_delta {
            let (from, floor) = (self.own, self.floor);
            self.place(delta.value.actor_position);
            if moved && floor == self.floor {
                // Arrival gaps contain transport jitter, not the server's movement clock.
                let duration = if outcome.step_wait_ms == 0 {
                    default_step_duration()
                } else {
                    Duration::from_millis(u64::from(outcome.step_wait_ms))
                };
                if let Some((origin, predicted)) = prediction {
                    self.walk = (predicted == self.own)
                        .then(|| {
                            Glide::new(
                                oteryn_renderer::TileCoord::new(origin.x, origin.y),
                                predicted,
                                duration,
                            )
                            .zip(previous.map(|(_, started)| started))
                        })
                        .flatten();
                } else {
                    self.walk = Glide::new(from, self.own, duration).map(|glide| {
                        let glide = previous.map_or(glide, |(old, start)| {
                            glide.continuing(old, now.saturating_duration_since(start))
                        });
                        (glide, now)
                    });
                }
            }
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
    /// Releases and focus resets must arrive even when another handler consumes input.
    pub fn observe_arrow_releases(
        &mut self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) {
        use oteryn_input_actions::{ButtonState, NormalizedInputEvent as Event};
        for event in events {
            if !matches!(
                event,
                Event::Key {
                    state: ButtonState::Pressed,
                    ..
                }
            ) {
                self.chord.step(std::slice::from_ref(event));
            }
        }
    }

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
        let chord = self.chord.step(events);
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

#[cfg(all(test, feature = "bevy-shell"))]
pub(crate) fn bevy_diagonal_fixture(outbox: &crate::play::Outbox) {
    outbox.diagonal_movement_initialize(true);
}

impl PlayView {
    /// Pick world artwork in scene pixels, reversing the walking camera translation.
    pub fn click_tile(&self, x: f64, y: f64) -> Option<oteryn_renderer::TileCoord> {
        let view = self.scene.view();
        let (w, h) = view.viewport_px();
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.
            || y < 0.
            || x >= f64::from(w)
            || y >= f64::from(h)
        {
            return None;
        }
        let [dx, dy] = if self.shared_frontend {
            self.camera_offset(Instant::now()).unwrap_or([0.; 2])
        } else {
            [0.; 2]
        };
        let side = f64::from(view.tile_px());
        Some(oteryn_renderer::TileCoord::new(
            view.origin()
                .x
                .checked_add(((x - f64::from(dx)) / side).floor() as i32)?,
            view.origin()
                .y
                .checked_add(((y - f64::from(dy)) / side).floor() as i32)?,
        ))
    }
}
