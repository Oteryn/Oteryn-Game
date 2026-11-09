//! Local projection filters retain received events so toggles can restore them.
use oteryn_session::{PresentationEventKind, PresentationSource};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GameWindowPreferences {
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
                _ => self.other_spells,
            },
            PresentationEventKind::ValueText { .. } => self.value_text,
            _ => true,
        }
    }
}
