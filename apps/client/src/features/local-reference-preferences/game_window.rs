//! Local projection filters retain received events so toggles can restore them.
use oteryn_session::{PresentationEventKind, PresentationSource};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GameWindowPreferences {
    pub messages: bool,
    pub attack_animation: bool,
    pub combat_frames: bool,
    pub target_highlight: bool,
    pub upper_pane_height: u16,
    pub loot_style: u8,
    pub boosted_messages: bool,
    pub hotkey_messages: bool,
    pub offline_training_messages: bool,
    pub private_messages: bool,
    pub store_messages: bool,

    pub own_spells: bool,
    pub other_spells: bool,
    pub value_text: bool,
    pub loot_messages: bool,
    pub target_frame: bool,
    pub pvp_frame: bool,
    pub loot_highlight: bool,
    pub banner: bool,
}
impl Default for GameWindowPreferences {
    fn default() -> Self {
        Self {
            messages: true,
            attack_animation: true,
            combat_frames: false,
            target_highlight: false,
            upper_pane_height: 0,
            loot_style: 0,
            boosted_messages: true,
            hotkey_messages: true,
            offline_training_messages: true,
            private_messages: true,
            store_messages: true,

            own_spells: true,
            other_spells: true,
            value_text: true,
            pvp_frame: false,
            loot_highlight: false,
            banner: true,
            target_frame: true,
            loot_messages: true,
        }
    }
}
impl GameWindowPreferences {
    pub(crate) fn visible(self, event: &PresentationEventKind) -> bool {
        match event {
            PresentationEventKind::MagicEffect { source, .. }
            | PresentationEventKind::Missile { source, .. } => match source {
                PresentationSource::Own => self.own_spells,
                PresentationSource::Others => self.other_spells,
                PresentationSource::Creatures | PresentationSource::Global => true,
            },
            PresentationEventKind::ValueText { .. } => self.value_text,
            _ => true,
        }
    }
}
