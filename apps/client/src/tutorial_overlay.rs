//! Opt-in contextual hints and an expiring, bounded mouse trail.
use crate::play::PlayView;
use oteryn_input_actions::{ButtonState, Modifier, Modifiers, MouseButton, NormalizedInputEvent};
use std::{
    collections::VecDeque,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Default)]
pub(crate) struct Overlay {
    pub enabled: bool,
    pub interaction: bool,
    pub marker: Option<[i32; 2]>,
    pub trail: VecDeque<(Instant, [i32; 2])>,
}
impl Overlay {
    pub fn points(&self, now: Instant) -> impl Iterator<Item = [i32; 2]> + '_ {
        self.trail
            .iter()
            .filter(move |(at, _)| now.saturating_duration_since(*at) < Duration::from_secs(1))
            .map(|(_, point)| *point)
    }
    pub fn hint(&self) -> &'static str {
        if self.interaction {
            tr!("right click: object actions")
        } else {
            tr!("left click: walk; arrows: move")
        }
    }
}
impl PlayView {
    /// Ctrl+Alt+Shift+N toggles the overlay; input continues through normal gameplay routing.
    pub fn tutorial_input(
        &mut self,
        events: &[NormalizedInputEvent],
    ) -> Result<bool, oteryn_renderer::BatchError> {
        let now = Instant::now();
        let chord = Modifiers::one(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift);
        let mut toggled = false;
        for event in events {
            match event {
                NormalizedInputEvent::Key {
                    code,
                    state: ButtonState::Pressed,
                    modifiers,
                    repeat: false,
                } if code.get() == 17 && *modifiers == chord => {
                    self.tutorial_overlay.enabled = !self.tutorial_overlay.enabled;
                    self.tutorial_overlay.trail.clear();
                    self.tutorial_overlay.marker = None;
                    toggled = true;
                }
                NormalizedInputEvent::PointerMoved { position, .. }
                    if self.tutorial_overlay.enabled =>
                {
                    let point = [position.x().get(), position.y().get()];
                    let (width, height) = self.scene.view().viewport_px();
                    if point[0] >= 0
                        && point[1] >= 0
                        && point[0] < width as i32
                        && point[1] < height as i32
                    {
                        self.tutorial_overlay.marker = Some(point);
                        if self.tutorial_overlay.trail.len() == 48 {
                            self.tutorial_overlay.trail.pop_front();
                        }
                        self.tutorial_overlay.trail.push_back((now, point));
                    } else {
                        self.tutorial_overlay.marker = None;
                    }
                }
                NormalizedInputEvent::MouseButton {
                    button,
                    state: ButtonState::Pressed,
                    ..
                } => {
                    self.tutorial_overlay.interaction = *button == MouseButton::SECONDARY;
                }
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::DeviceLost => {
                    self.tutorial_overlay.trail.clear();
                    self.tutorial_overlay.marker = None;
                }
                _ => {}
            }
        }
        if toggled || self.tutorial_overlay.enabled {
            self.project_presentation(now)?;
        }
        Ok(toggled)
    }
}
