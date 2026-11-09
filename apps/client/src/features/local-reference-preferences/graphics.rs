//! Supported local graphics requests. Engine and AA reflect the actual renderer build.
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct GraphicsPreferences {
    pub ambient: u8,
    pub separator: u8,
    pub indoor: u8,
    pub cloud: u8,
    pub fullscreen: bool,
    pub vsync: bool,
    pub integer_scaling: bool,
}
impl Default for GraphicsPreferences {
    fn default() -> Self {
        Self {
            ambient: 0,
            separator: 0,
            indoor: 0,
            cloud: 0,
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
impl GraphicsPreferences {
    pub(super) fn valid(self) -> bool {
        [self.ambient, self.separator, self.indoor, self.cloud]
            .into_iter()
            .all(|value| value <= 100)
    }
    pub(crate) fn ambient_rgb(
        self,
        server: [u8; 3],
        indoors: bool,
        weather: Option<oteryn_session::world_weather::WorldWeather>,
    ) -> [u8; 3] {
        let dimming = if indoors {
            u16::from(self.indoor)
        } else if let Some(oteryn_session::world_weather::WorldWeather::Active {
            kind: 2..=9,
            intensity,
            ..
        }) = weather
        {
            u16::from(self.cloud) * u16::from(intensity.min(3)) / 3
        } else {
            0
        };
        let minimum = u16::from(self.ambient) * 255 / 100;
        server.map(|channel| (u16::from(channel).max(minimum) * (100 - dimming) / 100) as u8)
    }
}
