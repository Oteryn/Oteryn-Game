//! Local presentation percentages; world clock, weather and floors remain server owned.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GraphicsPreferences {
    pub ambient: u8,
    pub separator: u8,
    pub indoor: u8,
    pub cloud: u8,
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
