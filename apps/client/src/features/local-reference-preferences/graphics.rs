//! Supported local graphics requests. Engine and AA reflect the actual renderer build.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct GraphicsPreferences {
    pub fullscreen: bool,
    pub vsync: bool,
    pub integer_scaling: bool,
}
impl Default for GraphicsPreferences {
    fn default() -> Self {
        Self {
            fullscreen: false,
            vsync: true,
            integer_scaling: false,
        }
    }
}
impl GraphicsPreferences {
    pub fn presentation(self) -> oteryn_renderer::PresentationMode {
        if self.vsync {
            oteryn_renderer::PresentationMode::Synchronized
        } else {
            oteryn_renderer::PresentationMode::Unlocked
        }
    }
    pub fn tile_pixels(self, zoom: u16, viewport: [u32; 2]) -> u32 {
        let fit = (viewport[0] / crate::scene::SCENE_COLUMNS)
            .min(viewport[1] / crate::scene::SCENE_ROWS)
            .max(1);
        let base = crate::scene::SCENE_TILE_PX;
        let requested = base * u32::from(zoom) / 100;
        if self.integer_scaling {
            let cell = oteryn_renderer::PAGE_CELL_PX;
            ((requested + cell / 2) / cell)
                .max(1)
                .min((fit / cell).max(1))
                * cell
        } else {
            requested.min(fit)
        }
    }
}
