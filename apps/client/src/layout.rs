//! Shared physical-pixel transform for rendering and pointer picking.
pub const HUD_SIDEBAR_WIDTH: f32 = 156.0;
pub const HUD_TOP_HEIGHT: f32 = 30.0;
pub const HUD_MARGIN: f32 = 4.0;
pub const HUD_CHAT_HEIGHT: f32 = 108.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameViewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl GameViewport {
    /// Fit using the applied settings, in physical pixels, for both drawing
    /// and pointer picking. Integral scaling counts the atlas's 32-pixel cells,
    /// not the scene's internal 48-pixel coordinates.
    #[must_use]
    pub fn for_settings(
        width: f32,
        height: f32,
        pixels_per_point: f32,
        settings: &crate::settings::ClientSettings,
    ) -> Option<Self> {
        let mut viewport = Self::fit_with_action_rows(
            width,
            height,
            pixels_per_point,
            settings.show_chat,
            settings.action_bar.visible_rows(),
        )?;
        if settings.stored_bool("graphics.graphics.integer_scale") == Some(true) {
            let native_width = (crate::scene::SCENE_COLUMNS * oteryn_client_assets::CELL_PX) as f32;
            let native_height = (crate::scene::SCENE_ROWS * oteryn_client_assets::CELL_PX) as f32;
            let multiple = (viewport.width / native_width)
                .min(viewport.height / native_height)
                .floor();
            if multiple < 1.0 {
                return None;
            }
            let width = native_width * multiple;
            let height = native_height * multiple;
            viewport.x = (viewport.x + (viewport.width - width) / 2.0).round();
            viewport.y = (viewport.y + (viewport.height - height) / 2.0).round();
            viewport.width = width;
            viewport.height = height;
        }
        Some(viewport)
    }

    #[must_use]
    pub fn fit(width: f32, height: f32, pixels_per_point: f32, chat: bool) -> Option<Self> {
        Self::fit_with_actions(width, height, pixels_per_point, chat, false)
    }

    #[must_use]
    pub fn fit_with_actions(
        width: f32,
        height: f32,
        pixels_per_point: f32,
        chat: bool,
        actions: bool,
    ) -> Option<Self> {
        Self::fit_with_action_rows(
            width,
            height,
            pixels_per_point,
            chat,
            [u8::from(actions), 0, 0],
        )
    }

    /// Row counts are the visible bottom rows, left columns and right columns.
    /// The same reservations apply to scene rendering and pointer picking.
    #[must_use]
    pub fn fit_with_action_rows(
        width: f32,
        height: f32,
        pixels_per_point: f32,
        chat: bool,
        rows: [u8; 3],
    ) -> Option<Self> {
        if !width.is_finite()
            || !height.is_finite()
            || !pixels_per_point.is_finite()
            || pixels_per_point <= 0.0
            || rows.iter().any(|rows| *rows > 3)
        {
            return None;
        }
        let margin = HUD_MARGIN * pixels_per_point;
        let top = HUD_TOP_HEIGHT * pixels_per_point;
        let bottom =
            if chat {
                HUD_CHAT_HEIGHT * pixels_per_point
            } else {
                HUD_MARGIN * pixels_per_point
            } + f32::from(rows[0]) * crate::action_bar::ACTION_ROW_HEIGHT * pixels_per_point;
        let left = f32::from(rows[1]) * crate::action_bar::ACTION_COLUMN_WIDTH * pixels_per_point;
        let right = f32::from(rows[2]) * crate::action_bar::ACTION_COLUMN_WIDTH * pixels_per_point;
        let available_w =
            width - HUD_SIDEBAR_WIDTH * pixels_per_point - margin * 2.0 - left - right;
        let available_h = height - top - bottom - margin * 2.0;
        let scale = (available_w / 720.0).min(available_h / 528.0);
        if scale <= 0.0 {
            return None;
        }
        let view_w = 720.0 * scale;
        let view_h = 528.0 * scale;
        Some(Self {
            x: margin + left + (available_w - view_w) / 2.0,
            y: top + margin + (available_h - view_h) / 2.0,
            width: view_w,
            height: view_h,
        })
    }

    #[must_use]
    pub fn scene_point(self, x: f64, y: f64) -> Option<(f64, f64)> {
        let local_x = x - f64::from(self.x);
        let local_y = y - f64::from(self.y);
        if !local_x.is_finite()
            || !local_y.is_finite()
            || local_x < 0.0
            || local_y < 0.0
            || local_x >= f64::from(self.width)
            || local_y >= f64::from(self.height)
        {
            return None;
        }
        Some((
            local_x * 720.0 / f64::from(self.width),
            local_y * 528.0 / f64::from(self.height),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integral_scaling_preserves_physical_sprite_pixels_and_picking() -> Result<(), &'static str> {
        let mut settings = crate::settings::ClientSettings::default();
        settings.future_preferences.insert(
            "graphics.graphics.integer_scale".into(),
            crate::settings_catalog::FutureValue::Bool(true),
        );
        for (width, height, dpi) in [
            (900.0, 620.0, 1.0),
            (1125.0, 775.0, 1.25),
            (1920.0, 1080.0, 1.5),
        ] {
            let view = GameViewport::for_settings(width, height, dpi, &settings)
                .ok_or("usable integral scene")?;
            let sprite_scale =
                view.width / (crate::scene::SCENE_COLUMNS * oteryn_client_assets::CELL_PX) as f32;
            assert_eq!(sprite_scale.fract(), 0.0);
            assert!(sprite_scale >= 1.0);
            assert_eq!(view.x.fract(), 0.0);
            assert_eq!(view.y.fract(), 0.0);
            let center = view
                .scene_point(
                    f64::from(view.x + view.width / 2.0),
                    f64::from(view.y + view.height / 2.0),
                )
                .ok_or("center remains pickable")?;
            assert_eq!(center, (360.0, 264.0));
            assert!(
                view.scene_point(f64::from(view.x - 1.0), f64::from(view.y))
                    .is_none()
            );
        }
        assert!(GameViewport::for_settings(400.0, 300.0, 1.0, &settings).is_none());
        settings.future_preferences.clear();
        assert_eq!(
            GameViewport::for_settings(900.0, 620.0, 1.0, &settings),
            GameViewport::fit_with_action_rows(
                900.0,
                620.0,
                1.0,
                settings.show_chat,
                settings.action_bar.visible_rows()
            ),
        );
        Ok(())
    }

    #[test]
    fn picking_matches_rendered_scene_across_resize_and_ui_scale()
    -> Result<(), Box<dyn std::error::Error>> {
        for (w, h, scale) in [
            (900.0, 620.0, 1.0),
            (1920.0, 1080.0, 1.5),
            (800.0, 600.0, 1.8),
        ] {
            for chat in [false, true] {
                let viewport = GameViewport::fit(w, h, scale, chat).ok_or("usable viewport")?;
                let (x, y) = viewport
                    .scene_point(
                        f64::from(viewport.x + viewport.width / 2.0),
                        f64::from(viewport.y + viewport.height / 2.0),
                    )
                    .ok_or("center")?;
                assert!((x - 360.0).abs() < 0.001);
                assert!((y - 264.0).abs() < 0.001);
                assert!(viewport.scene_point(-1.0, 0.0).is_none());
                assert!(viewport.scene_point(f64::NAN, 0.0).is_none());
                assert!(viewport.x + viewport.width <= w - HUD_SIDEBAR_WIDTH * scale);
                assert!(viewport.y + viewport.height <= h);
            }
        }
        assert!(GameViewport::fit(1.0, 1.0, 1.0, true).is_none());
        Ok(())
    }

    #[test]
    fn action_rows_reserve_space_without_changing_scene_coordinates()
    -> Result<(), Box<dyn std::error::Error>> {
        for (width, height, scale) in [(900.0, 620.0, 1.0), (1920.0, 1080.0, 1.5)] {
            for bottom in 0..=3 {
                for left in 0..=3 {
                    for right in 0..=3 {
                        let view = GameViewport::fit_with_action_rows(
                            width,
                            height,
                            scale,
                            true,
                            [bottom, left, right],
                        )
                        .ok_or("rows leave a usable scene")?;
                        let (x, y) = view
                            .scene_point(
                                f64::from(view.x + view.width / 2.0),
                                f64::from(view.y + view.height / 2.0),
                            )
                            .ok_or("scene center")?;
                        assert!((x - 360.0).abs() < 0.001 && (y - 264.0).abs() < 0.001);
                        assert!(
                            view.x
                                >= (HUD_MARGIN
                                    + f32::from(left) * crate::action_bar::ACTION_COLUMN_WIDTH)
                                    * scale
                        );
                        assert!(
                            view.x + view.width
                                <= width
                                    - (HUD_SIDEBAR_WIDTH
                                        + f32::from(right)
                                            * crate::action_bar::ACTION_COLUMN_WIDTH
                                        + HUD_MARGIN)
                                        * scale
                                    + 0.001
                        );
                        assert!(
                            view.y + view.height
                                <= height
                                    - (HUD_CHAT_HEIGHT
                                        + f32::from(bottom) * crate::action_bar::ACTION_ROW_HEIGHT
                                        + HUD_MARGIN)
                                        * scale
                                    + 0.001
                        );
                    }
                }
            }
        }
        assert_eq!(
            GameViewport::fit_with_actions(900.0, 620.0, 1.0, true, true),
            GameViewport::fit_with_action_rows(900.0, 620.0, 1.0, true, [1, 0, 0])
        );
        assert!(GameViewport::fit_with_action_rows(300.0, 200.0, 1.0, true, [3, 3, 3]).is_none());
        assert!(GameViewport::fit_with_action_rows(900.0, 620.0, 1.0, true, [4, 0, 0]).is_none());
        Ok(())
    }
}
