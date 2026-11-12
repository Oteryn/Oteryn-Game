//! Apply saved graphics choices on the window and surface's owning thread.
use super::*;
use winit::window::Fullscreen;
impl Application {
    pub(super) fn apply_graphics(&mut self) -> Result<(), ShellError> {
        let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) else {
            return Ok(());
        };
        let (requested, zoom) = self.settings.graphics_settings();
        if window.inner_size().width == 0 || window.inner_size().height == 0 {
            return Ok(());
        }
        if self.applied_graphics != Some(requested) {
            if renderer
                .set_antialiasing(self.generation, requested.antialiasing)
                .is_err()
            {
                let (_, antialiasing) = renderer.graphics();
                self.settings
                    .reject_graphics(requested.engine, antialiasing);
                return Ok(());
            }
            if window.fullscreen().is_some() != requested.fullscreen {
                window.set_fullscreen(requested.fullscreen.then_some(Fullscreen::Borderless(None)));
            }
            renderer
                .set_presentation(self.generation, requested.presentation())
                .map_err(|_| ShellError::RendererConfiguration)?;
            self.applied_graphics = Some(requested);
        }
        if self.play.is_none()
            && let Some(scene) = &mut self.scene
        {
            let size = window.inner_size();
            scene
                .set_tile_px(requested.tile_pixels(zoom, [size.width, size.height]))
                .map_err(|_| ShellError::RendererConfiguration)?;
        }
        Ok(())
    }
}
