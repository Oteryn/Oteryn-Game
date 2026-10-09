//! Shared physical-pixel transform for rendering and pointer picking.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GameViewport {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl GameViewport {
    #[must_use]
    pub fn fit(width: f32, height: f32, pixels_per_point: f32, chat: bool) -> Option<Self> {
        if !width.is_finite()
            || !height.is_finite()
            || !pixels_per_point.is_finite()
            || pixels_per_point <= 0.0
        {
            return None;
        }
        let margin = 12.0 * pixels_per_point;
        let top = 52.0 * pixels_per_point;
        let bottom = if chat {
            160.0 * pixels_per_point
        } else {
            12.0 * pixels_per_point
        };
        let available_w = width - 220.0 * pixels_per_point - margin * 2.0;
        let available_h = height - top - bottom - margin * 2.0;
        let scale = (available_w / 720.0).min(available_h / 528.0);
        if scale <= 0.0 {
            return None;
        }
        let view_w = 720.0 * scale;
        let view_h = 528.0 * scale;
        Some(Self {
            x: margin + (available_w - view_w) / 2.0,
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
                assert!(viewport.x + viewport.width <= w - 220.0 * scale);
                assert!(viewport.y + viewport.height <= h);
            }
        }
        assert!(GameViewport::fit(1.0, 1.0, 1.0, true).is_none());
        Ok(())
    }
}
