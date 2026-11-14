use super::state::{Layout, State};
use crate::play::feature_0023_presentation::view::pixel;
use oteryn_renderer::{AtlasImage, BatchError, SpriteBatch, TileView};
use oteryn_session::PresentationColor;

pub(in crate::play) fn draw(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    state: &State,
) -> Result<(), BatchError> {
    if !state.visible {
        return Ok(());
    }
    let layout = Layout::new(view);
    let scale = layout.scale as f32;
    let center = layout.center.map(|v| v as f32);
    let radius = layout.radius as f32;
    // Eight markers make the steering sectors visible; the centre is the dead zone.
    for [dx, dy] in [
        [0.0, -1.0],
        [0.7, -0.7],
        [1.0, 0.0],
        [0.7, 0.7],
        [0.0, 1.0],
        [-0.7, 0.7],
        [-1.0, 0.0],
        [-0.7, -0.7],
    ] {
        pixel(
            batch,
            view,
            atlas,
            [
                center[0] + dx * (radius - 8.0 * scale) - 4.0 * scale,
                center[1] + dy * (radius - 8.0 * scale) - 4.0 * scale,
            ],
            [8.0 * scale; 2],
            PresentationColor::LightGrey,
        )?;
    }
    pixel(
        batch,
        view,
        atlas,
        [center[0] - 16.0 * scale, center[1] - 16.0 * scale],
        [32.0 * scale; 2],
        if state.direction.is_some() {
            PresentationColor::LightGreen
        } else {
            PresentationColor::Blue
        },
    )?;
    text(batch, view, atlas, center, tr!("Move"), 38.0 * scale, scale)?;
    for (index, label) in [tr!("Attack"), tr!("Look"), tr!("Follow"), tr!("Use")]
        .into_iter()
        .enumerate()
    {
        let center = layout.button_center(index).map(|v| v as f32);
        pixel(
            batch,
            view,
            atlas,
            [center[0] - 33.0 * scale, center[1] - 24.0 * scale],
            [66.0 * scale, 48.0 * scale],
            if state.mode == Some(index) {
                PresentationColor::LightGreen
            } else {
                PresentationColor::Blue
            },
        )?;
        text(batch, view, atlas, center, label, 60.0 * scale, scale)?;
    }
    Ok(())
}

fn text(
    batch: &mut SpriteBatch,
    view: &TileView,
    atlas: &AtlasImage,
    center: [f32; 2],
    label: &str,
    width: f32,
    scale: f32,
) -> Result<(), BatchError> {
    let scale = (width / (label.chars().count().max(1) * 6) as f32).min(2.0 * scale);
    let left = center[0] - label.chars().count() as f32 * 3.0 * scale;
    for (index, character) in label.chars().enumerate() {
        let glyph = oteryn_renderer::name_text::glyph_rgba(character);
        for y in 0..7usize {
            for x in 0..5usize {
                if glyph[(((y * 32 + 16) / 8) * 32 + (x * 32 + 16) / 6) * 4 + 3] == 255 {
                    pixel(
                        batch,
                        view,
                        atlas,
                        [
                            left + (index * 6 + x) as f32 * scale,
                            center[1] + (y as f32 - 3.5) * scale,
                        ],
                        [scale; 2],
                        PresentationColor::White,
                    )?;
                }
            }
        }
    }
    Ok(())
}

/// Shared frontend reads the retained overlay; it does not keep a second mode store.
#[cfg(feature = "bevy-shell")]
impl crate::play::PlayView {
    pub fn touch_controls_available(&self, link: &crate::play::PlayLink) -> bool {
        let mailbox = crate::play::mailbox::lock(&link.mailbox);
        !link.commands.is_closed()
            && !mailbox.world_input_blocked()
            && !mailbox.server_modal.is_active()
            && !self.death.dead
    }

    pub fn touch_controls_tap_contact(&self, id: u64) -> bool {
        self.touch.tap.is_some_and(|(contact, _)| contact == id)
    }

    pub fn touch_controls_mode(&self) -> Option<usize> {
        self.touch.mode
    }

    pub fn touch_controls_visible(&self) -> bool {
        self.touch.visible
    }

    /// Compare the captured local projection under the session owner lock. Closing
    /// is independent of gameplay admission and never clears pending commands.
    pub fn touch_controls_visibility_bound(
        &mut self,
        link: &crate::play::PlayLink,
        expected: bool,
        visible: bool,
    ) -> bool {
        let mailbox = crate::play::mailbox::lock(&link.mailbox);
        if link.commands.is_closed() || self.touch.visible != expected || expected == visible {
            return false;
        }
        if visible && (mailbox.world_input_blocked() || mailbox.server_modal.is_active()) {
            return false;
        }
        self.touch.visible = visible;
        if !visible {
            self.cancel_touch_controls();
        }
        true
    }
}

#[cfg(all(test, feature = "bevy-shell"))]
mod bound_tests {
    use crate::play::{PlayView, mailbox::lock, play_channel};
    #[test]
    fn touch_controls_close_preserves_pending_while_blocked()
    -> Result<(), oteryn_renderer::BatchError> {
        let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        view.use_shared_frontend();
        let (link, _commands, _) = play_channel();
        link.use_shared_frontend();
        view.enable_touch_controls();
        view.in_flight = true;
        view.touch.mode = Some(3);
        lock(&link.mailbox).death_reset_commands = true;
        assert!(view.touch_controls_visibility_bound(&link, true, false));
        assert!(!view.touch_controls_visible());
        assert!(view.in_flight);
        assert_eq!(view.touch.mode, Some(3));
        assert!(!view.touch_controls_visibility_bound(&link, false, true));
        lock(&link.mailbox).death_reset_commands = false;
        assert!(view.touch_controls_visibility_bound(&link, false, true));
        assert!(view.in_flight);
        assert_eq!(view.touch.mode, Some(3));
        Ok(())
    }
}
