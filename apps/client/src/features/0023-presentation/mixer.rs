//! Independent mixer gains; cue routing is part of the pinned audio manifest.
use super::AudioSettings;
use oteryn_session::PresentationSource;
use serde::{Deserialize, Serialize};
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundChannel {
    #[default]
    Battle,
    Music,
    Items,
    Events,
    Ui,
}
impl AudioSettings {
    pub fn channel_gain(self, source: PresentationSource, channel: SoundChannel) -> f32 {
        let index = match channel {
            SoundChannel::Battle => return self.source_gain(source),
            SoundChannel::Music => 0,
            SoundChannel::Items => 1,
            SoundChannel::Events => 2,
            SoundChannel::Ui => 3,
        };
        self.gain() * f32::from(self.channel_volumes[index].min(100)) / 100.0
    }
}
