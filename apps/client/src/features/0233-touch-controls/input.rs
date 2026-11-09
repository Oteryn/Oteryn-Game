use super::state::{Layout, Phase};
use crate::play::{PlayLink, PlayView, mailbox::lock};
use oteryn_client_runtime::world_interaction::Action;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_renderer::{BatchError, TileCoord};

impl PlayView {
    pub fn enable_touch_controls(&mut self) {
        self.touch.visible = true;
    }

    /// Cancelling a contact never changes the selected mode or hides the controls.
    pub fn cancel_touch_controls(&mut self) {
        self.touch.joystick = None;
        self.touch.direction = None;
        self.touch.tap = None;
        self.touch.alternate = false;
    }

    /// Real native touch input, before the mouse adapter. Mouse events never enter here.
    pub fn touch_input(
        &mut self,
        link: &PlayLink,
        id: u64,
        phase: Phase,
        at: [f64; 2],
    ) -> Result<(), BatchError> {
        self.enable_touch_controls();
        if lock(&link.mailbox).world_input_blocked()
            || self.death.dead
            || crate::play::feature_0450_server_modal::view::active()
        {
            self.cancel_touch_controls();
            return Ok(());
        }
        let layout = Layout::new(self.scene.view());
        match phase {
            Phase::Started => {
                if layout.distance(at) <= layout.radius {
                    if self.touch.joystick.is_none() {
                        self.touch.joystick = Some(id);
                        self.touch.direction = layout.direction(at);
                        self.touch.alternate = false;
                    }
                } else if let Some(mode) = layout.button(at) {
                    self.touch.mode = (self.touch.mode != Some(mode)).then_some(mode);
                } else if self.touch.tap.is_none() && at.iter().all(|v| v.is_finite()) {
                    self.touch.tap = Some((id, at));
                }
            }
            Phase::Moved if self.touch.joystick == Some(id) => {
                let direction = layout.direction(at);
                if direction != self.touch.direction {
                    self.touch.direction = direction;
                    self.touch.alternate = false;
                }
            }
            Phase::Ended | Phase::Cancelled if self.touch.joystick == Some(id) => {
                self.touch.joystick = None;
                self.touch.direction = None;
                self.touch.alternate = false;
            }
            Phase::Ended | Phase::Cancelled => {
                if let Some((contact, start)) = self.touch.tap
                    && contact == id
                {
                    self.touch.tap = None;
                    if phase == Phase::Ended
                        && (at[0] - start[0]).hypot(at[1] - start[1]) <= 12.0 * layout.scale
                        && layout.button(at).is_none()
                        && layout.distance(at) > layout.radius
                        && let Some(tile) =
                            crate::input::click_tile(self.scene.view(), at[0], at[1])
                    {
                        self.touch_tile(link, tile)?;
                    }
                }
            }
            Phase::Moved => {
                if let Some((contact, start)) = self.touch.tap
                    && contact == id
                    && (at[0] - start[0]).hypot(at[1] - start[1]) > 12.0 * layout.scale
                {
                    self.touch.tap = None;
                }
            }
        }
        Ok(())
    }

    pub(in crate::play) fn pump_touch_step(&mut self, link: &PlayLink) {
        if lock(&link.mailbox).world_input_blocked()
            || self.death.dead
            || crate::play::feature_0450_server_modal::view::active()
        {
            self.cancel_touch_controls();
            return;
        }
        // Never buffer a held step: release before the next receipt stops immediately.
        if !self.in_flight
            && self.inputs.is_empty()
            && let Some(directions) = self.touch.direction
        {
            let direction = directions[usize::from(self.touch.alternate)];
            self.touch.alternate = !self.touch.alternate;
            self.arrow(direction);
        }
    }

    fn touch_tile(&mut self, link: &PlayLink, tile: TileCoord) -> Result<(), BatchError> {
        match self.touch.mode {
            Some(0) => {
                if self
                    .scene
                    .target_at(tile)
                    .and_then(|target| target.entity)
                    .is_some()
                {
                    self.click(tile)?;
                }
            }
            Some(1 | 3) => {
                if self.touch.mode == Some(3) && self.use_with_click(tile, link) {
                    return Ok(());
                }
                if let Some(menu) = self.world_context(tile, link) {
                    self.world_context_choose(
                        link,
                        &menu,
                        if self.touch.mode == Some(1) {
                            Action::Look
                        } else {
                            Action::Use
                        },
                    );
                }
            }
            Some(2) => {
                self.scene.select_tile(tile)?;
                // Reuse Ctrl+F including visibility, generation and negotiated-capability checks.
                if let Ok(code) = KeyCode::new(9) {
                    self.target_move_input(
                        link,
                        &[NormalizedInputEvent::Key {
                            code,
                            state: ButtonState::Pressed,
                            modifiers: Modifiers::one(Modifier::Control),
                            repeat: false,
                        }],
                    );
                }
            }
            _ => self.primary_click(tile, link)?,
        }
        Ok(())
    }
}
