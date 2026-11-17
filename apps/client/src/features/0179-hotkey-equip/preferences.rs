//! Saved item hotkeys and the server-negotiated carried-source policy.
use crate::play::{PlayLink, PreferenceStore, SettingsState, mailbox::lock};
use serde::{Deserialize, Serialize};
use std::{num::NonZeroU32, path::Path};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ItemHotkeys {
    pub bindings: [Option<NonZeroU32>; oteryn_client_ui::hotkey_equip::ITEM_HOTKEY_SLOTS],
    pub smart: bool,
}
impl Default for ItemHotkeys {
    fn default() -> Self {
        Self {
            bindings: [None; oteryn_client_ui::hotkey_equip::ITEM_HOTKEY_SLOTS],
            smart: true,
        }
    }
}

pub(crate) fn supported_capabilities(character: [u8; 16], base: Option<&Path>) -> Vec<u32> {
    let smart = base
        .and_then(|base| {
            let selected = PreferenceStore::character_path(base, character);
            PreferenceStore::load(if selected.exists() {
                selected
            } else {
                base.to_owned()
            })
            .ok()
        })
        .is_none_or(|store| store.preferences().item_hotkeys.smart);
    oteryn_session::CLIENT_SUPPORTED_CAPABILITIES
        .iter()
        .copied()
        .filter(|capability| smart || *capability != 180)
        .collect()
}

pub(in crate::play) fn edited(panel: &mut oteryn_client_ui::hotkey_equip::HotkeyEquipPanel) {
    panel.edit = crate::play::feature_0017_attack::view::next_edit();
    panel.dirty = true;
}

impl PlayLink {
    pub fn set_smart_equip(&self, smart: bool) {
        let mut mailbox = lock(&self.mailbox);
        mailbox.hotkey_equip.smart = smart;
        edited(&mut mailbox.hotkey_equip);
    }
}
impl SettingsState {
    pub fn sync_item_hotkeys(&mut self, link: &PlayLink) {
        let mut mailbox = lock(&link.mailbox);
        let panel = &mut mailbox.hotkey_equip;
        if panel.dirty && panel.edit > self.item_hotkeys_edit {
            let edit = panel.edit;
            let mut preferences = self.preferences.clone();
            preferences.item_hotkeys = ItemHotkeys {
                bindings: panel.bindings,
                smart: panel.smart,
            };
            panel.dirty = false;
            drop(mailbox);
            self.apply(preferences);
            self.item_hotkeys_edit = edit;
        } else {
            panel.dirty = false;
            panel.bindings = self.preferences.item_hotkeys.bindings;
            panel.smart = self.preferences.item_hotkeys.smart;
        }
    }
}
