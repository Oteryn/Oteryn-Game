//! Persisted hotkey profiles. Profiles contain only local input intent; gameplay
//! commands still pass through the existing typed client/session boundaries.

use crate::{action_bar::ActionBarPreferences, settings_catalog::ShortcutBinding};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, NormalizedInputEvent};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io};

pub const MAX_HOTKEY_PROFILES: usize = 32;
pub const MAX_CUSTOM_HOTKEYS: usize = 256;
const MOVEMENT_ACTIONS: [&str; 4] = [
    "movement.north",
    "movement.east",
    "movement.south",
    "movement.west",
];

/// A profile binding that has a concrete client consumer. Unknown general
/// actions remain persisted reference inventory, but are never reported as
/// executable until their owning client/domain path exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedHotkey {
    General(String),
    Custom(CustomHotkeyAction),
}

/// General actions currently connected to real local behavior.
#[must_use]
pub fn general_action_supported(action: &str) -> bool {
    matches!(
        action,
        "movement.north"
            | "movement.east"
            | "movement.south"
            | "movement.west"
            | "battle.first"
            | "battle.next"
            | "battle.previous"
            | "chat.close"
            | "chat.next"
            | "chat.list"
            | "chat.help"
            | "chat.previous"
            | "chat.local"
            | "dialog.prey"
            | "dialog.quest_log"
            | "window.vip"
            | "window.battle"
            | "window.skills"
            | "client.options"
            | "client.hotkeys"
            | "client.next_profile"
            | "client.stop"
            | "client.fullscreen"
            | "client.fps"
            | "minimap.zoom_out"
            | "minimap.zoom_in"
            | "minimap.north"
            | "minimap.east"
            | "minimap.south"
            | "minimap.west"
            | "action.bottom.1"
            | "action.bottom.2"
            | "action.bottom.3"
            | "action.bottom.all"
            | "action.left.1"
            | "action.left.2"
            | "action.left.3"
            | "action.left.all"
            | "action.right.1"
            | "action.right.2"
            | "action.right.3"
            | "action.right.all"
    )
}

/// Movement bindings are profile-owned and, unlike other hotkeys, must be a
/// single unmodified physical key in the reference client.
#[must_use]
pub fn is_movement_action(action: &str) -> bool {
    action.starts_with("movement.")
}

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
        let stop = default_stop_pair();
        Self {
            name: "Default".into(),
            general_chat_on: BTreeMap::from([("client.stop".into(), stop)]),
            general_chat_off: BTreeMap::from([("client.stop".into(), stop)]),
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
    pub(crate) fn migrate_v1_reference_defaults(&mut self, movement_keys: &[u16; 4]) {
        for profile in &mut self.profiles {
            migrate_v1_movement_bindings(profile, true, movement_keys);
            migrate_v1_movement_bindings(profile, false, movement_keys);
            if !profile_uses_reference_action_chord(profile) {
                profile.action_bar.migrate_v1_default_shortcuts();
            }
        }
    }

    pub fn active(&self) -> Option<&HotkeyProfile> {
        self.profiles.get(self.selected)
    }

    pub fn active_mut(&mut self) -> Option<&mut HotkeyProfile> {
        self.profiles.get_mut(self.selected)
    }

    /// Presence, including an explicitly cleared pair, disables the matching
    /// legacy direction without affecting the other three directions.
    #[must_use]
    pub fn profile_owns_movement_action(&self, action: &str) -> bool {
        self.active()
            .is_some_and(|profile| profile.general_chat_off.contains_key(action))
    }

    /// Resolve pressed, non-repeating physical keys in the active chat context.
    /// Validation guarantees that one chord cannot name two actions in the same
    /// context, including action-bar and custom-action families.
    #[must_use]
    pub fn resolve(&self, events: &[NormalizedInputEvent], chat_on: bool) -> Vec<ResolvedHotkey> {
        let Some(profile) = self.active() else {
            return Vec::new();
        };
        let general = if chat_on {
            &profile.general_chat_on
        } else {
            &profile.general_chat_off
        };
        let mut resolved = Vec::new();
        for event in events {
            for (action, pair) in general {
                if general_action_supported(action) && pair_matches(*pair, event) {
                    resolved.push(ResolvedHotkey::General(action.clone()));
                }
            }
            for custom in &profile.custom {
                let pair = if chat_on {
                    custom.chat_on
                } else {
                    custom.chat_off
                };
                if pair_matches(pair, event) {
                    resolved.push(ResolvedHotkey::Custom(custom.action.clone()));
                }
            }
        }
        resolved
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
            validate_profile_conflicts(profile, movement_keys)?;
        }
        Ok(())
    }
}

fn default_stop_pair() -> HotkeyPair {
    HotkeyPair {
        first: Some(ShortcutBinding {
            key: 41,
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        }),
        second: None,
    }
}

fn migrate_v1_movement_bindings(
    profile: &mut HotkeyProfile,
    chat_on: bool,
    movement_keys: &[u16; 4],
) {
    let mut used = Vec::new();
    let bindings = if chat_on {
        &profile.general_chat_on
    } else {
        &profile.general_chat_off
    };
    for pair in bindings.values() {
        for binding in [pair.first, pair.second].into_iter().flatten() {
            if !has_modifiers(binding) {
                used.push(binding);
            }
        }
    }
    for custom in &profile.custom {
        let pair = if chat_on {
            custom.chat_on
        } else {
            custom.chat_off
        };
        used.extend(
            [pair.first, pair.second]
                .into_iter()
                .flatten()
                .filter(|binding| !has_modifiers(*binding)),
        );
    }
    used.extend(
        profile
            .action_bar
            .shortcuts_for_context(chat_on)
            .flatten()
            .filter(|shortcut| shortcut.modifiers == 0)
            .map(|shortcut| ShortcutBinding {
                key: shortcut.key,
                shift: false,
                ctrl: false,
                alt: false,
                meta: false,
            }),
    );
    if !chat_on {
        used.extend(
            MOVEMENT_ACTIONS
                .into_iter()
                .zip(movement_keys.iter().copied())
                .filter(|(action, _)| !profile.general_chat_off.contains_key(*action))
                .map(|(_, key)| unmodified_binding(key)),
        );
    }

    let bindings = if chat_on {
        &mut profile.general_chat_on
    } else {
        &mut profile.general_chat_off
    };
    for (action, pair) in bindings {
        if !is_movement_action(action) {
            continue;
        }
        for slot in [&mut pair.first, &mut pair.second] {
            let Some(binding) = *slot else { continue };
            if !has_modifiers(binding) {
                continue;
            }
            let candidate = ShortcutBinding {
                key: binding.key,
                shift: false,
                ctrl: false,
                alt: false,
                meta: false,
            };
            if used.contains(&candidate) {
                *slot = None;
            } else {
                *slot = Some(candidate);
                used.push(candidate);
            }
        }
    }
}

fn profile_uses_reference_action_chord(profile: &HotkeyProfile) -> bool {
    let target =
        |binding: ShortcutBinding| !has_modifiers(binding) && (58..=69).contains(&binding.key);
    profile
        .general_chat_on
        .values()
        .chain(profile.general_chat_off.values())
        .any(|pair| [pair.first, pair.second].into_iter().flatten().any(target))
        || profile.custom.iter().any(|custom| {
            [custom.chat_on, custom.chat_off]
                .into_iter()
                .any(|pair| [pair.first, pair.second].into_iter().flatten().any(target))
        })
}

fn has_modifiers(binding: ShortcutBinding) -> bool {
    binding.ctrl || binding.alt || binding.shift || binding.meta
}

fn pair_matches(pair: HotkeyPair, event: &NormalizedInputEvent) -> bool {
    [pair.first, pair.second]
        .into_iter()
        .flatten()
        .any(|binding| binding_matches(binding, event))
}

fn binding_matches(binding: ShortcutBinding, event: &NormalizedInputEvent) -> bool {
    let NormalizedInputEvent::Key {
        code,
        state: ButtonState::Pressed,
        modifiers,
        repeat: false,
    } = event
    else {
        return false;
    };
    code.get() == binding.key
        && modifiers.contains(Modifier::Control) == binding.ctrl
        && modifiers.contains(Modifier::Alt) == binding.alt
        && modifiers.contains(Modifier::Shift) == binding.shift
        && modifiers.contains(Modifier::Super) == binding.meta
}

fn validate_profile_conflicts(profile: &HotkeyProfile, movement_keys: &[u16]) -> io::Result<()> {
    let movement_keys: &[u16; 4] = movement_keys.try_into().map_err(|_| invalid())?;
    let mut chat_on = Vec::new();
    let mut chat_off = Vec::new();
    for pair in profile.general_chat_on.values() {
        append_pair(&mut chat_on, *pair)?;
    }
    for pair in profile.general_chat_off.values() {
        append_pair(&mut chat_off, *pair)?;
    }
    for (action, key) in MOVEMENT_ACTIONS
        .into_iter()
        .zip(movement_keys.iter().copied())
    {
        if !profile.general_chat_off.contains_key(action) {
            append_binding(&mut chat_off, unmodified_binding(key))?;
        }
    }
    for action in &profile.custom {
        append_pair(&mut chat_on, action.chat_on)?;
        append_pair(&mut chat_off, action.chat_off)?;
    }
    for (shortcuts, seen) in [
        (profile.action_bar.shortcuts_for_context(true), &mut chat_on),
        (
            profile.action_bar.shortcuts_for_context(false),
            &mut chat_off,
        ),
    ] {
        for shortcut in shortcuts.flatten() {
            let binding = ShortcutBinding {
                key: shortcut.key,
                shift: shortcut.modifiers & 1 != 0,
                ctrl: shortcut.modifiers & 2 != 0,
                alt: shortcut.modifiers & 4 != 0,
                meta: shortcut.modifiers & 8 != 0,
            };
            append_binding(seen, binding)?;
        }
    }
    Ok(())
}

fn unmodified_binding(key: u16) -> ShortcutBinding {
    ShortcutBinding {
        key,
        shift: false,
        ctrl: false,
        alt: false,
        meta: false,
    }
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
        validate_general_pair(action, *pair)?;
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
        if matches!(binding.key, 40 | 41) {
            return Err(invalid());
        }
    }
    if pair.first.is_some() && pair.first == pair.second {
        return Err(invalid());
    }
    Ok(())
}

fn validate_general_pair(action: &str, pair: HotkeyPair) -> io::Result<()> {
    for binding in [pair.first, pair.second].into_iter().flatten() {
        KeyCode::new(binding.key).map_err(|_| invalid())?;
        if binding.key == 40
            || (binding.key == 41 && action != "client.stop")
            || (is_movement_action(action)
                && (binding.ctrl || binding.alt || binding.shift || binding.meta))
        {
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
    use oteryn_input_actions::{Modifier, Modifiers};

    fn binding(key: u16, ctrl: bool) -> ShortcutBinding {
        ShortcutBinding {
            key,
            ctrl,
            alt: false,
            shift: false,
            meta: false,
        }
    }

    fn pressed(key: u16, ctrl: bool) -> Result<NormalizedInputEvent, Box<dyn std::error::Error>> {
        Ok(NormalizedInputEvent::Key {
            code: KeyCode::new(key)?,
            state: ButtonState::Pressed,
            modifiers: if ctrl {
                Modifiers::one(Modifier::Control)
            } else {
                Modifiers::NONE
            },
            repeat: false,
        })
    }

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
            ctrl: false,
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

    #[test]
    fn resolver_obeys_chat_context_modifiers_and_real_consumer_boundary()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut preferences = HotkeyPreferences::default();
        let profile = preferences.active_mut().ok_or("default profile")?;
        profile.general_chat_off.insert(
            "movement.north".into(),
            HotkeyPair {
                first: Some(binding(26, false)),
                second: None,
            },
        );
        profile.general_chat_on.insert(
            "client.options".into(),
            HotkeyPair {
                first: Some(binding(18, true)),
                second: None,
            },
        );
        profile.general_chat_off.insert(
            "chat.next".into(),
            HotkeyPair {
                first: Some(binding(21, false)),
                second: None,
            },
        );
        // A catalogue-only action is retained but cannot masquerade as executable.
        profile.general_chat_off.insert(
            "dialog.lenshelp".into(),
            HotkeyPair {
                first: Some(binding(19, false)),
                second: None,
            },
        );
        profile.custom.push(CustomHotkey {
            action: CustomHotkeyAction::Text {
                text: "hello".into(),
                send_automatically: true,
            },
            chat_on: HotkeyPair::default(),
            chat_off: HotkeyPair {
                first: Some(binding(20, false)),
                second: None,
            },
        });
        assert_eq!(
            preferences.resolve(&[pressed(26, false)?], false),
            vec![ResolvedHotkey::General("movement.north".into())]
        );
        assert!(preferences.resolve(&[pressed(26, false)?], true).is_empty());
        assert!(preferences.resolve(&[pressed(18, false)?], true).is_empty());
        assert_eq!(
            preferences.resolve(&[pressed(18, true)?], true),
            vec![ResolvedHotkey::General("client.options".into())]
        );
        assert_eq!(
            preferences.resolve(&[pressed(21, false)?], false),
            vec![ResolvedHotkey::General("chat.next".into())]
        );
        assert!(
            preferences
                .resolve(&[pressed(19, false)?], false)
                .is_empty()
        );
        assert_eq!(
            preferences.resolve(&[pressed(20, false)?], false),
            vec![ResolvedHotkey::Custom(CustomHotkeyAction::Text {
                text: "hello".into(),
                send_automatically: true,
            })]
        );
        Ok(())
    }

    #[test]
    fn movement_is_unmodified_and_an_explicit_clear_owns_the_profile()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut preferences = HotkeyPreferences::default();
        let profile = preferences.active_mut().ok_or("default profile")?;
        profile.general_chat_off.insert(
            "movement.north".into(),
            HotkeyPair {
                first: Some(binding(26, true)),
                second: None,
            },
        );
        assert!(preferences.validate(&[82, 79, 81, 80]).is_err());
        preferences
            .active_mut()
            .ok_or("default profile")?
            .general_chat_off
            .insert("movement.north".into(), HotkeyPair::default());
        assert!(preferences.validate(&[82, 79, 81, 80]).is_ok());
        assert!(preferences.profile_owns_movement_action("movement.north"));
        assert!(!preferences.profile_owns_movement_action("movement.east"));
        Ok(())
    }

    #[test]
    fn effective_movement_fallbacks_reject_partial_collision_but_allow_an_owned_swap()
    -> Result<(), Box<dyn std::error::Error>> {
        let movement_keys = [82, 79, 81, 80];
        let mut preferences = HotkeyPreferences::default();
        preferences
            .active_mut()
            .ok_or("default profile")?
            .general_chat_off
            .insert(
                "movement.north".into(),
                HotkeyPair {
                    first: Some(binding(79, false)),
                    second: None,
                },
            );
        assert!(preferences.validate(&movement_keys).is_err());

        preferences
            .active_mut()
            .ok_or("default profile")?
            .general_chat_off
            .insert(
                "movement.east".into(),
                HotkeyPair {
                    first: Some(binding(82, false)),
                    second: None,
                },
            );
        preferences.validate(&movement_keys)?;
        Ok(())
    }

    #[test]
    fn escape_is_exclusive_to_stop_all_and_f10_is_assignable()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut preferences = HotkeyPreferences::default();
        let escape_event = pressed(41, false)?;
        for chat_on in [false, true] {
            assert_eq!(
                preferences.resolve(std::slice::from_ref(&escape_event), chat_on),
                vec![ResolvedHotkey::General("client.stop".into())]
            );
        }
        let profile = preferences.active_mut().ok_or("default profile")?;
        profile.general_chat_off.insert(
            "client.stop".into(),
            HotkeyPair {
                first: Some(binding(41, false)),
                second: None,
            },
        );
        profile.general_chat_off.insert(
            "client.options".into(),
            HotkeyPair {
                first: Some(binding(67, false)),
                second: None,
            },
        );
        profile.action_bar.shortcuts[9] = None;
        assert!(preferences.validate(&[82, 79, 81, 80]).is_ok());
        preferences
            .active_mut()
            .ok_or("default profile")?
            .general_chat_off
            .get_mut("client.options")
            .ok_or("options binding")?
            .first = Some(binding(41, false));
        assert!(preferences.validate(&[82, 79, 81, 80]).is_err());
        Ok(())
    }

    #[test]
    fn v1_migration_normalizes_movement_and_skips_conflicting_action_defaults()
    -> Result<(), Box<dyn std::error::Error>> {
        let legacy = [30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 68, 69];
        let mut preferences = HotkeyPreferences::default();
        let profile = preferences.active_mut().ok_or("default profile")?;
        profile.general_chat_on.clear();
        profile.general_chat_off.clear();
        profile.general_chat_off.insert(
            "movement.north".into(),
            HotkeyPair {
                first: Some(binding(26, true)),
                second: None,
            },
        );
        profile.general_chat_off.insert(
            "client.options".into(),
            HotkeyPair {
                first: Some(binding(58, false)),
                second: None,
            },
        );
        for (shortcut, key) in profile.action_bar.shortcuts.iter_mut().zip(legacy) {
            *shortcut = Some(crate::action_bar::SlotShortcut { key, modifiers: 0 });
        }
        profile.action_bar.chat_on_shortcuts.fill(None);

        preferences.migrate_v1_reference_defaults(&[82, 79, 81, 80]);
        let profile = preferences.active().ok_or("default profile")?;
        assert_eq!(
            profile.general_chat_off["movement.north"].first,
            Some(binding(26, false))
        );
        assert_eq!(
            profile.action_bar.shortcuts[0].map(|shortcut| shortcut.key),
            Some(30)
        );
        assert!(!profile.general_chat_on.contains_key("client.stop"));
        preferences.validate(&[82, 79, 81, 80])?;
        Ok(())
    }

    #[test]
    fn v1_movement_collision_becomes_an_explicit_clear() -> Result<(), Box<dyn std::error::Error>> {
        let mut preferences = HotkeyPreferences::default();
        let profile = preferences.active_mut().ok_or("default profile")?;
        profile.general_chat_off.insert(
            "client.options".into(),
            HotkeyPair {
                first: Some(binding(26, false)),
                second: None,
            },
        );
        profile.general_chat_off.insert(
            "movement.north".into(),
            HotkeyPair {
                first: Some(binding(26, true)),
                second: None,
            },
        );
        preferences.migrate_v1_reference_defaults(&[82, 79, 81, 80]);
        assert_eq!(
            preferences
                .active()
                .ok_or("default profile")?
                .general_chat_off["movement.north"],
            HotkeyPair::default()
        );
        preferences.validate(&[82, 79, 81, 80])?;
        Ok(())
    }

    #[test]
    fn v1_action_defaults_do_not_overwrite_chat_on_or_custom_f_key_chords()
    -> Result<(), Box<dyn std::error::Error>> {
        let legacy = [30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 68, 69];
        for custom in [false, true] {
            let mut preferences = HotkeyPreferences::default();
            let profile = preferences.active_mut().ok_or("default profile")?;
            for (shortcut, key) in profile.action_bar.shortcuts.iter_mut().zip(legacy) {
                *shortcut = Some(crate::action_bar::SlotShortcut { key, modifiers: 0 });
            }
            profile.action_bar.chat_on_shortcuts.fill(None);
            if custom {
                profile.custom.push(CustomHotkey {
                    action: CustomHotkeyAction::Text {
                        text: "existing".into(),
                        send_automatically: false,
                    },
                    chat_on: HotkeyPair {
                        first: Some(binding(58, false)),
                        second: None,
                    },
                    chat_off: HotkeyPair::default(),
                });
            } else {
                profile.general_chat_on.insert(
                    "client.options".into(),
                    HotkeyPair {
                        first: Some(binding(58, false)),
                        second: None,
                    },
                );
            }

            preferences.migrate_v1_reference_defaults(&[82, 79, 81, 80]);
            let profile = preferences.active().ok_or("default profile")?;
            assert_eq!(
                profile.action_bar.shortcuts[0].map(|shortcut| shortcut.key),
                Some(30)
            );
            assert_eq!(profile.action_bar.chat_on_shortcuts[0], None);
            preferences.validate(&[82, 79, 81, 80])?;
        }
        Ok(())
    }

    #[test]
    fn v1_movement_normalization_does_not_take_an_unowned_direction_fallback()
    -> Result<(), Box<dyn std::error::Error>> {
        let movement_keys = [82, 79, 81, 80];
        let mut preferences = HotkeyPreferences::default();
        preferences
            .active_mut()
            .ok_or("default profile")?
            .general_chat_off
            .insert(
                "movement.north".into(),
                HotkeyPair {
                    first: Some(binding(79, true)),
                    second: None,
                },
            );

        preferences.migrate_v1_reference_defaults(&movement_keys);
        assert_eq!(
            preferences
                .active()
                .ok_or("default profile")?
                .general_chat_off["movement.north"],
            HotkeyPair::default()
        );
        assert!(!preferences.profile_owns_movement_action("movement.east"));
        preferences.validate(&movement_keys)?;
        Ok(())
    }
}
