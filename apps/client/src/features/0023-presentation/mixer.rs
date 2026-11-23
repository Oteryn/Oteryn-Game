//! Independent mixer gains; cue routing is part of the pinned audio manifest.
use super::AudioSettings;
use oteryn_session::PresentationSource;
use serde::{Deserialize, Serialize};
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SoundChannel {
    #[default]
    Battle,
    Eating,
    MoveItem,
    Raid,
    SystemMessage,
    PartyNotification,
    PrivateWithoutTab,
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
            SoundChannel::Eating if !self.eating => return 0.0,
            SoundChannel::MoveItem if !self.move_item => return 0.0,
            SoundChannel::Raid if !self.raid || !self.ui_sounds[3] => return 0.0,
            SoundChannel::Eating | SoundChannel::MoveItem | SoundChannel::Items => 1,
            SoundChannel::SystemMessage if !self.system || !self.ui_sounds[3] => return 0.0,
            SoundChannel::PartyNotification if !self.ui_sounds[1] => return 0.0,
            SoundChannel::PrivateWithoutTab if !self.private_without_tab || !self.ui_sounds[3] => {
                return 0.0;
            }
            SoundChannel::Raid
            | SoundChannel::SystemMessage
            | SoundChannel::PartyNotification
            | SoundChannel::PrivateWithoutTab => 3,
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
    Death,
    Noise,
    Weapon,
    GenericSpell,
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
            if let (Some(index), Some(class)) = (index, class) {
                let spells = [self.own_spells, self.other_spells, true][index];
                let enabled = match class {
                    BattleClass::Death => index == 0 || self.creature_death,
                    BattleClass::Noise => index == 0 || self.creature_noises,
                    BattleClass::Weapon => [
                        self.own_weapons,
                        self.other_weapons,
                        self.battle_filters[2][0],
                    ][index],
                    BattleClass::GenericSpell => spells,
                    BattleClass::Attack | BattleClass::Healing | BattleClass::Support => {
                        spells && self.battle_filters[index][class as usize]
                    }
                };
                if !enabled {
                    return 0.0;
                }
            }
        }
        self.channel_gain(source, channel)
    }
}
