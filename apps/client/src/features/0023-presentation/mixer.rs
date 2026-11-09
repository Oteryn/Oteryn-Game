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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
#[repr(usize)]
pub enum BattleClass {
    Attack,
    Healing,
    Support,
}
impl AudioSettings {
    pub fn classified_gain(
        self,
        source: PresentationSource,
        channel: SoundChannel,
        class: Option<BattleClass>,
    ) -> f32 {
        if channel == SoundChannel::Battle {
            let index = match source {
                PresentationSource::Own => Some(0),
                PresentationSource::Others => Some(1),
                PresentationSource::Creatures => Some(2),
                PresentationSource::Global => None,
            };
            if let (Some(index), Some(class)) = (index, class)
                && !self.battle_filters[index][class as usize]
            {
                return 0.0;
            }
        }
        self.channel_gain(source, channel)
    }
}
