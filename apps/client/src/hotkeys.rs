//! Persisted hotkey profiles. Profiles contain only local input intent; gameplay
//! commands still pass through the existing typed client/session boundaries.

use crate::{action_bar::ActionBarPreferences, settings_catalog::ShortcutBinding};
use oteryn_input_actions::KeyCode;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io};

pub const MAX_HOTKEY_PROFILES: usize = 32;
pub const MAX_CUSTOM_HOTKEYS: usize = 256;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotkeyPair {
    pub first: Option<ShortcutBinding>,
    pub second: Option<ShortcutBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CustomHotkeyAction {
    Text {
        text: String,
        send_automatically: bool,
    },
    Spell {
        /// Stable spell catalogue key; an empty live catalogue cannot create one.
        spell_key: String,
        parameter: String,
    },
    Object {
        item_definition_ref: u32,
        use_mode: ObjectUseMode,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObjectUseMode {
    OnSelf,
    OnTarget,
    Crosshair,
    AtCursor,
    EquipUnequip,
    #[default]
    Use,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomHotkey {
    pub action: CustomHotkeyAction,
    pub chat_on: HotkeyPair,
    pub chat_off: HotkeyPair,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotkeyProfile {
    pub name: String,
    pub general_chat_on: BTreeMap<String, HotkeyPair>,
    pub general_chat_off: BTreeMap<String, HotkeyPair>,
    pub custom: Vec<CustomHotkey>,
    pub action_bar: ActionBarPreferences,
}

impl Default for HotkeyProfile {
    fn default() -> Self {
        Self {
            name: "Default".into(),
            general_chat_on: BTreeMap::new(),
            general_chat_off: BTreeMap::new(),
            custom: Vec::new(),
            action_bar: ActionBarPreferences::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HotkeyPreferences {
    pub selected: usize,
    pub auto_switch: bool,
    pub profiles: Vec<HotkeyProfile>,
}

impl Default for HotkeyPreferences {
    fn default() -> Self {
        Self {
            selected: 0,
            auto_switch: false,
            profiles: vec![HotkeyProfile::default()],
        }
    }
}

impl HotkeyPreferences {
    pub fn active(&self) -> Option<&HotkeyProfile> {
        self.profiles.get(self.selected)
    }

    pub fn active_mut(&mut self) -> Option<&mut HotkeyProfile> {
        self.profiles.get_mut(self.selected)
    }

    pub fn validate(&self, movement_keys: &[u16]) -> io::Result<()> {
        if self.profiles.is_empty()
            || self.profiles.len() > MAX_HOTKEY_PROFILES
            || self.selected >= self.profiles.len()
        {
            return Err(invalid());
        }
        let mut names = Vec::new();
        for profile in &self.profiles {
            let normalized = profile.name.trim().to_lowercase();
            if normalized.is_empty()
                || profile.name.len() > 48
                || profile.name.chars().any(char::is_control)
                || names.contains(&normalized)
                || profile.custom.len() > MAX_CUSTOM_HOTKEYS
            {
                return Err(invalid());
            }
            names.push(normalized);
            profile.action_bar.validate(movement_keys)?;
            validate_general(&profile.general_chat_on)?;
            validate_general(&profile.general_chat_off)?;
            validate_custom(&profile.custom)?;
            validate_profile_conflicts(profile)?;
        }
        Ok(())
    }
}

fn validate_profile_conflicts(profile: &HotkeyProfile) -> io::Result<()> {
    let mut chat_on = Vec::new();
    let mut chat_off = Vec::new();
    for pair in profile.general_chat_on.values() {
        append_pair(&mut chat_on, *pair)?;
    }
    for pair in profile.general_chat_off.values() {
        append_pair(&mut chat_off, *pair)?;
    }
    for action in &profile.custom {
        append_pair(&mut chat_on, action.chat_on)?;
        append_pair(&mut chat_off, action.chat_off)?;
    }
    for shortcut in profile.action_bar.all_shortcuts().flatten() {
        let binding = ShortcutBinding {
            key: shortcut.key,
            shift: shortcut.modifiers & 1 != 0,
            ctrl: shortcut.modifiers & 2 != 0,
            alt: shortcut.modifiers & 4 != 0,
            meta: shortcut.modifiers & 8 != 0,
        };
        append_binding(&mut chat_on, binding)?;
        append_binding(&mut chat_off, binding)?;
    }
    Ok(())
}

fn append_pair(seen: &mut Vec<ShortcutBinding>, pair: HotkeyPair) -> io::Result<()> {
    for binding in [pair.first, pair.second].into_iter().flatten() {
        append_binding(seen, binding)?;
    }
    Ok(())
}

fn append_binding(seen: &mut Vec<ShortcutBinding>, binding: ShortcutBinding) -> io::Result<()> {
    if seen.contains(&binding) {
        return Err(invalid());
    }
    seen.push(binding);
    Ok(())
}

fn validate_general(bindings: &BTreeMap<String, HotkeyPair>) -> io::Result<()> {
    if bindings.len() > 512 {
        return Err(invalid());
    }
    let mut seen = Vec::new();
    for (action, pair) in bindings {
        if action.is_empty() || action.len() > 96 || action.chars().any(char::is_control) {
            return Err(invalid());
        }
        validate_pair(*pair)?;
        for binding in [pair.first, pair.second].into_iter().flatten() {
            if seen.contains(&binding) {
                return Err(invalid());
            }
            seen.push(binding);
        }
    }
    Ok(())
}

fn validate_custom(actions: &[CustomHotkey]) -> io::Result<()> {
    let mut chat_on = Vec::new();
    let mut chat_off = Vec::new();
    for action in actions {
        validate_pair(action.chat_on)?;
        validate_pair(action.chat_off)?;
        for (pair, seen) in [
            (action.chat_on, &mut chat_on),
            (action.chat_off, &mut chat_off),
        ] {
            for binding in [pair.first, pair.second].into_iter().flatten() {
                if seen.contains(&binding) {
                    return Err(invalid());
                }
                seen.push(binding);
            }
        }
        match &action.action {
            CustomHotkeyAction::Text { text, .. } => {
                if text.is_empty() || text.len() > 256 || text.chars().any(char::is_control) {
                    return Err(invalid());
                }
            }
            CustomHotkeyAction::Spell {
                spell_key,
                parameter,
            } => {
                if spell_key.is_empty()
                    || spell_key.len() > 128
                    || parameter.len() > 128
                    || spell_key.chars().any(char::is_control)
                    || parameter.chars().any(char::is_control)
                {
                    return Err(invalid());
                }
            }
            CustomHotkeyAction::Object {
                item_definition_ref,
                ..
            } if *item_definition_ref == 0 => return Err(invalid()),
            CustomHotkeyAction::Object { .. } => {}
        }
    }
    Ok(())
}

fn validate_pair(pair: HotkeyPair) -> io::Result<()> {
    for binding in [pair.first, pair.second].into_iter().flatten() {
        KeyCode::new(binding.key).map_err(|_| invalid())?;
        if matches!(binding.key, 40 | 41 | 67) {
            return Err(invalid());
        }
    }
    if pair.first.is_some() && pair.first == pair.second {
        return Err(invalid());
    }
    Ok(())
}

fn invalid() -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, "invalid hotkey preferences")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_round_trip_and_reject_duplicate_names() -> Result<(), Box<dyn std::error::Error>> {
        let mut profiles = HotkeyPreferences::default();
        profiles.profiles.push(HotkeyProfile {
            name: "Knight".into(),
            ..HotkeyProfile::default()
        });
        profiles.validate(&[82, 79, 81, 80])?;
        assert_eq!(
            serde_json::from_slice::<HotkeyPreferences>(&serde_json::to_vec(&profiles)?)?,
            profiles
        );
        profiles.profiles[1].name = " default ".into();
        assert!(profiles.validate(&[82, 79, 81, 80]).is_err());
        Ok(())
    }

    #[test]
    fn conflicts_are_scoped_by_chat_context_and_span_all_action_families()
    -> Result<(), Box<dyn std::error::Error>> {
        let binding = ShortcutBinding {
            key: 4,
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
        };
        let mut preferences = HotkeyPreferences::default();
        let profile = preferences.active_mut().ok_or("default profile")?;
        profile.general_chat_on.insert(
            "movement.north".into(),
            HotkeyPair {
                first: Some(binding),
                second: None,
            },
        );
        profile.general_chat_off.insert(
            "movement.south".into(),
            HotkeyPair {
                first: Some(binding),
                second: None,
            },
        );
        assert!(preferences.validate(&[82, 79, 81, 80]).is_ok());
        preferences
            .active_mut()
            .ok_or("default profile")?
            .custom
            .push(CustomHotkey {
                action: CustomHotkeyAction::Text {
                    text: "hello".into(),
                    send_automatically: false,
                },
                chat_on: HotkeyPair {
                    first: Some(binding),
                    second: None,
                },
                chat_off: HotkeyPair::default(),
            });
        assert!(preferences.validate(&[82, 79, 81, 80]).is_err());
        Ok(())
    }
}
