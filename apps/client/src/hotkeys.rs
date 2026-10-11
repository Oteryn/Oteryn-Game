//! Persisted hotkey profiles. Profiles contain only local input intent; gameplay
//! commands still pass through the existing typed client/session boundaries.

use crate::{action_bar::ActionBarPreferences, settings_catalog::ShortcutBinding};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, NormalizedInputEvent};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
};

pub const MAX_HOTKEY_PROFILES: usize = 32;
pub const MAX_CUSTOM_HOTKEYS: usize = 256;
const MOVEMENT_ACTIONS: [&str; 4] = [
    "movement.north",
    "movement.east",
    "movement.south",
    "movement.west",
];

const SHIFT: u8 = 1;
const CONTROL: u8 = 2;
const ALT: u8 = 4;

const DEFAULT_GENERAL_CHAT_OFF: [(&str, HotkeyPair); 49] = [
    ("battle.next", default_pair((44, 0), None)),
    ("battle.previous", default_pair((44, CONTROL), None)),
    ("chat.close", default_pair((8, CONTROL), None)),
    ("chat.help", default_pair((23, CONTROL), None)),
    ("chat.list", default_pair((18, CONTROL), None)),
    ("chat.local", default_pair((7, ALT), None)),
    ("chat.mode.temporary", default_pair((40, 0), Some((88, 0)))),
    ("chat.next", default_pair((43, 0), None)),
    ("chat.previous", default_pair((43, SHIFT), None)),
    ("chat.server_messages", default_pair((16, CONTROL), None)),
    ("chat.text.copy", default_pair((6, CONTROL), None)),
    ("client.bug_report", default_pair((29, CONTROL), None)),
    ("client.change_character", default_pair((10, CONTROL), None)),
    ("client.clear_message", default_pair((26, ALT), None)),
    ("client.fps", default_pair((65, ALT), None)),
    (
        "client.fullscreen",
        default_pair((9, CONTROL), Some((40, ALT))),
    ),
    ("client.hotkeys", default_pair((14, CONTROL), None)),
    ("client.ignore", default_pair((12, CONTROL), None)),
    (
        "client.logout",
        default_pair((15, CONTROL), Some((20, CONTROL))),
    ),
    ("client.mount", default_pair((21, CONTROL), None)),
    ("client.names", default_pair((17, CONTROL), None)),
    ("client.next_profile", default_pair((13, CONTROL), None)),
    ("client.stop", default_pair((41, 0), None)),
    ("container.manual_sort", default_pair((22, SHIFT), None)),
    ("dialog.cyclopedia.map", default_pair((16, 0), None)),
    ("dialog.lenshelp", default_pair((11, CONTROL), None)),
    ("dialog.prey", default_pair((28, CONTROL), None)),
    ("dialog.quest_log", default_pair((24, CONTROL), None)),
    ("loot.nearby_corpses", default_pair((20, ALT), None)),
    ("minimap.east", default_pair((79, ALT), None)),
    ("minimap.floor_down", default_pair((78, ALT), None)),
    ("minimap.floor_up", default_pair((75, ALT), None)),
    ("minimap.north", default_pair((82, ALT), None)),
    ("minimap.south", default_pair((81, ALT), None)),
    ("minimap.west", default_pair((80, ALT), None)),
    ("minimap.zoom_in", default_pair((77, ALT), None)),
    ("minimap.zoom_out", default_pair((74, ALT), None)),
    ("movement.east", default_pair((7, 0), None)),
    ("movement.north", default_pair((26, 0), None)),
    ("movement.north_east", default_pair((8, 0), None)),
    ("movement.north_west", default_pair((20, 0), None)),
    ("movement.south", default_pair((22, 0), None)),
    ("movement.south_east", default_pair((6, 0), None)),
    ("movement.south_west", default_pair((29, 0), None)),
    ("movement.west", default_pair((4, 0), None)),
    ("window.battle", default_pair((5, CONTROL), None)),
    ("window.skills", default_pair((22, ALT), None)),
    ("window.spell_list", default_pair((15, ALT), None)),
    ("window.vip", default_pair((19, CONTROL), None)),
];

const DEFAULT_GENERAL_CHAT_ON: [(&str, HotkeyPair); 38] = [
    ("chat.close", default_pair((8, CONTROL), None)),
    ("chat.help", default_pair((23, CONTROL), None)),
    ("chat.list", default_pair((18, CONTROL), None)),
    ("chat.local", default_pair((7, ALT), None)),
    ("chat.next", default_pair((43, 0), None)),
    ("chat.previous", default_pair((43, SHIFT), None)),
    ("chat.send", default_pair((40, 0), Some((88, 0)))),
    ("chat.server_messages", default_pair((16, CONTROL), None)),
    ("chat.text.copy", default_pair((6, CONTROL), None)),
    ("chat.text.select_all", default_pair((4, CONTROL), None)),
    ("client.bug_report", default_pair((29, CONTROL), None)),
    ("client.change_character", default_pair((10, CONTROL), None)),
    ("client.clear_message", default_pair((26, ALT), None)),
    ("client.fps", default_pair((65, ALT), None)),
    (
        "client.fullscreen",
        default_pair((9, CONTROL), Some((40, ALT))),
    ),
    ("client.hotkeys", default_pair((14, CONTROL), None)),
    ("client.ignore", default_pair((12, CONTROL), None)),
    (
        "client.logout",
        default_pair((15, CONTROL), Some((20, CONTROL))),
    ),
    ("client.mount", default_pair((21, CONTROL), None)),
    ("client.names", default_pair((17, CONTROL), None)),
    ("client.next_profile", default_pair((13, CONTROL), None)),
    ("client.stop", default_pair((41, 0), None)),
    ("dialog.lenshelp", default_pair((11, CONTROL), None)),
    ("dialog.prey", default_pair((28, CONTROL), None)),
    ("dialog.quest_log", default_pair((24, CONTROL), None)),
    ("loot.nearby_corpses", default_pair((20, ALT), None)),
    ("minimap.east", default_pair((79, ALT), None)),
    ("minimap.floor_down", default_pair((78, ALT), None)),
    ("minimap.floor_up", default_pair((75, ALT), None)),
    ("minimap.north", default_pair((82, ALT), None)),
    ("minimap.south", default_pair((81, ALT), None)),
    ("minimap.west", default_pair((80, ALT), None)),
    ("minimap.zoom_in", default_pair((77, ALT), None)),
    ("minimap.zoom_out", default_pair((74, ALT), None)),
    ("window.battle", default_pair((5, CONTROL), None)),
    ("window.skills", default_pair((22, ALT), None)),
    ("window.spell_list", default_pair((15, ALT), None)),
    ("window.vip", default_pair((19, CONTROL), None)),
];

/// A profile binding that has a concrete client consumer. Unknown general
/// actions remain persisted reference inventory, but are never reported as
/// executable until their owning client/domain path exists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ResolvedHotkey {
    General(String),
    Custom(CustomHotkeyAction),
}

/// Physical keys whose initial press was claimed by the native hotkey router.
/// Repeats and the matching release remain hidden from egui while every event
/// still reaches the gameplay routers for held movement and cleanup.
#[derive(Debug, Default)]
pub struct ClaimedKeyLifecycle {
    keys: BTreeSet<u16>,
}

impl ClaimedKeyLifecycle {
    /// Observe one normalized native event batch and report whether its source
    /// keyboard event must stay out of egui.
    pub fn observe(&mut self, events: &[NormalizedInputEvent], action_claimed: bool) -> bool {
        let mut suppress = action_claimed;
        for event in events {
            match event {
                NormalizedInputEvent::Key { code, state, .. } => match state {
                    ButtonState::Pressed => {
                        if action_claimed {
                            self.keys.insert(code.get());
                        }
                        suppress |= self.keys.contains(&code.get());
                    }
                    ButtonState::Released => {
                        suppress |= self.keys.remove(&code.get());
                    }
                },
                NormalizedInputEvent::FocusChanged { focused } if !focused => self.keys.clear(),
                NormalizedInputEvent::DeviceLost => self.keys.clear(),
                _ => {}
            }
        }
        suppress
    }
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
            | "chat.mode.off"
            | "chat.mode.on"
            | "chat.mode.temporary"
            | "chat.send"
            | "dialog.cyclopedia.map"
            | "dialog.prey"
            | "dialog.quest_log"
            | "window.vip"
            | "window.battle"
            | "window.skills"
            | "window.spell_list"
            | "client.options"
            | "client.hotkeys"
            | "client.next_profile"
            | "client.previous_profile"
            | "client.stop"
            | "client.fullscreen"
            | "client.fps"
            | "client.screenshot"
            | "minimap.zoom_out"
            | "minimap.zoom_in"
            | "minimap.north"
            | "minimap.east"
            | "minimap.south"
            | "minimap.west"
            | "minimap.center"
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
        Self {
            name: "Default".into(),
            general_chat_on: default_general_bindings(&DEFAULT_GENERAL_CHAT_ON),
            general_chat_off: default_general_bindings(&DEFAULT_GENERAL_CHAT_OFF),
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

    /// Version-two profiles shipped only Escape as the untouched default in
    /// both chat contexts. Upgrade that exact map, independently per context,
    /// while preserving every customized or explicitly cleared map verbatim.
    pub(crate) fn migrate_v2_reference_defaults(&mut self) {
        for profile in &mut self.profiles {
            if is_v2_single_stop_default(&profile.general_chat_on) {
                profile.general_chat_on = default_general_bindings(&DEFAULT_GENERAL_CHAT_ON);
            }
            if is_v2_single_stop_default(&profile.general_chat_off) {
                profile.general_chat_off = default_general_bindings(&DEFAULT_GENERAL_CHAT_OFF);
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

    /// Legacy cardinal keys remain reserved only for directions the selected
    /// profile does not own. Invalid selections conservatively reserve all of
    /// them until normal settings validation rejects the profile set.
    #[must_use]
    pub fn active_movement_fallback_keys(&self, movement_keys: &[u16]) -> Vec<u16> {
        self.active().map_or_else(
            || movement_keys.to_vec(),
            |profile| movement_fallback_keys(profile, movement_keys),
        )
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
                if general_action_supported(action)
                    && pair_matches(*pair, event, is_movement_action(action))
                {
                    resolved.push(ResolvedHotkey::General(action.clone()));
                }
            }
            for custom in &profile.custom {
                let pair = if chat_on {
                    custom.chat_on
                } else {
                    custom.chat_off
                };
                if pair_matches(pair, event, false) {
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
            let fallback_movement_keys = movement_fallback_keys(profile, movement_keys);
            profile.action_bar.validate(&fallback_movement_keys)?;
            if profile
                .action_bar
                .all_shortcuts()
                .flatten()
                .any(|shortcut| matches!(shortcut.key, 40 | 88))
            {
                return Err(invalid());
            }
            validate_general(&profile.general_chat_on, true)?;
            validate_general(&profile.general_chat_off, false)?;
            validate_custom(&profile.custom)?;
            validate_profile_conflicts(profile, movement_keys)?;
        }
        Ok(())
    }
}

const fn default_binding(key: u16, modifiers: u8) -> ShortcutBinding {
    ShortcutBinding {
        key,
        ctrl: modifiers & CONTROL != 0,
        alt: modifiers & ALT != 0,
        shift: modifiers & SHIFT != 0,
        meta: false,
    }
}

const fn default_pair(first: (u16, u8), second: Option<(u16, u8)>) -> HotkeyPair {
    HotkeyPair {
        first: Some(default_binding(first.0, first.1)),
        second: match second {
            Some(second) => Some(default_binding(second.0, second.1)),
            None => None,
        },
    }
}

fn default_general_bindings(bindings: &[(&str, HotkeyPair)]) -> BTreeMap<String, HotkeyPair> {
    bindings
        .iter()
        .map(|(action, pair)| ((*action).into(), *pair))
        .collect()
}

fn is_v2_single_stop_default(bindings: &BTreeMap<String, HotkeyPair>) -> bool {
    bindings.len() == 1 && bindings.get("client.stop") == Some(&default_pair((41, 0), None))
}

fn movement_fallback_keys(profile: &HotkeyProfile, movement_keys: &[u16]) -> Vec<u16> {
    MOVEMENT_ACTIONS
        .into_iter()
        .zip(movement_keys.iter().copied())
        .filter_map(|(action, key)| (!profile.general_chat_off.contains_key(action)).then_some(key))
        .collect()
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

fn pair_matches(pair: HotkeyPair, event: &NormalizedInputEvent, allow_repeat: bool) -> bool {
    [pair.first, pair.second]
        .into_iter()
        .flatten()
        .any(|binding| binding_matches(binding, event, allow_repeat))
}

fn binding_matches(
    binding: ShortcutBinding,
    event: &NormalizedInputEvent,
    allow_repeat: bool,
) -> bool {
    let NormalizedInputEvent::Key {
        code,
        state: ButtonState::Pressed,
        modifiers,
        repeat,
    } = event
    else {
        return false;
    };
    (!*repeat || allow_repeat)
        && code.get() == binding.key
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

fn validate_general(bindings: &BTreeMap<String, HotkeyPair>, chat_on: bool) -> io::Result<()> {
    if bindings.len() > 512 {
        return Err(invalid());
    }
    let mut seen = Vec::new();
    for (action, pair) in bindings {
        if action.is_empty() || action.len() > 96 || action.chars().any(char::is_control) {
            return Err(invalid());
        }
        validate_general_pair(action, *pair, chat_on)?;
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
        if matches!(binding.key, 40 | 41 | 88) {
            return Err(invalid());
        }
    }
    if pair.first.is_some() && pair.first == pair.second {
        return Err(invalid());
    }
    Ok(())
}

fn validate_general_pair(action: &str, pair: HotkeyPair, chat_on: bool) -> io::Result<()> {
    for binding in [pair.first, pair.second].into_iter().flatten() {
        KeyCode::new(binding.key).map_err(|_| invalid())?;
        if (matches!(binding.key, 40 | 88) && !reserved_enter_chord(action, binding, chat_on))
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

fn reserved_enter_chord(action: &str, binding: ShortcutBinding, chat_on: bool) -> bool {
    let unmodified = !has_modifiers(binding);
    (unmodified
        && ((chat_on && action == "chat.send") || (!chat_on && action == "chat.mode.temporary")))
        || (binding.key == 40
            && action == "client.fullscreen"
            && binding.alt
            && !binding.ctrl
            && !binding.shift
            && !binding.meta)
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

    fn binding_snapshot(binding: Option<ShortcutBinding>) -> String {
        let Some(binding) = binding else {
            return "-".into();
        };
        let mut modifiers = String::new();
        if binding.ctrl {
            modifiers.push('C');
        }
        if binding.alt {
            modifiers.push('A');
        }
        if binding.shift {
            modifiers.push('S');
        }
        if binding.meta {
            modifiers.push('M');
        }
        format!("{}:{modifiers}", binding.key)
    }

    fn map_snapshot(bindings: &BTreeMap<String, HotkeyPair>) -> String {
        bindings
            .iter()
            .map(|(action, pair)| {
                format!(
                    "{action}={},{}",
                    binding_snapshot(pair.first),
                    binding_snapshot(pair.second)
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn chord_count(bindings: &BTreeMap<String, HotkeyPair>) -> usize {
        bindings
            .values()
            .map(|pair| usize::from(pair.first.is_some()) + usize::from(pair.second.is_some()))
            .sum()
    }

    #[test]
    fn reference_defaults_match_the_exact_general_hotkey_matrix() {
        let profile = HotkeyProfile::default();
        assert_eq!(profile.general_chat_off.len(), 49);
        assert_eq!(chord_count(&profile.general_chat_off), 52);
        assert_eq!(profile.general_chat_on.len(), 38);
        assert_eq!(chord_count(&profile.general_chat_on), 41);
        assert_eq!(profile.action_bar, ActionBarPreferences::default());

        assert_eq!(
            map_snapshot(&profile.general_chat_off),
            r#"battle.next=44:,-
battle.previous=44:C,-
chat.close=8:C,-
chat.help=23:C,-
chat.list=18:C,-
chat.local=7:A,-
chat.mode.temporary=40:,88:
chat.next=43:,-
chat.previous=43:S,-
chat.server_messages=16:C,-
chat.text.copy=6:C,-
client.bug_report=29:C,-
client.change_character=10:C,-
client.clear_message=26:A,-
client.fps=65:A,-
client.fullscreen=9:C,40:A
client.hotkeys=14:C,-
client.ignore=12:C,-
client.logout=15:C,20:C
client.mount=21:C,-
client.names=17:C,-
client.next_profile=13:C,-
client.stop=41:,-
container.manual_sort=22:S,-
dialog.cyclopedia.map=16:,-
dialog.lenshelp=11:C,-
dialog.prey=28:C,-
dialog.quest_log=24:C,-
loot.nearby_corpses=20:A,-
minimap.east=79:A,-
minimap.floor_down=78:A,-
minimap.floor_up=75:A,-
minimap.north=82:A,-
minimap.south=81:A,-
minimap.west=80:A,-
minimap.zoom_in=77:A,-
minimap.zoom_out=74:A,-
movement.east=7:,-
movement.north=26:,-
movement.north_east=8:,-
movement.north_west=20:,-
movement.south=22:,-
movement.south_east=6:,-
movement.south_west=29:,-
movement.west=4:,-
window.battle=5:C,-
window.skills=22:A,-
window.spell_list=15:A,-
window.vip=19:C,-"#
        );
        assert_eq!(
            map_snapshot(&profile.general_chat_on),
            r#"chat.close=8:C,-
chat.help=23:C,-
chat.list=18:C,-
chat.local=7:A,-
chat.next=43:,-
chat.previous=43:S,-
chat.send=40:,88:
chat.server_messages=16:C,-
chat.text.copy=6:C,-
chat.text.select_all=4:C,-
client.bug_report=29:C,-
client.change_character=10:C,-
client.clear_message=26:A,-
client.fps=65:A,-
client.fullscreen=9:C,40:A
client.hotkeys=14:C,-
client.ignore=12:C,-
client.logout=15:C,20:C
client.mount=21:C,-
client.names=17:C,-
client.next_profile=13:C,-
client.stop=41:,-
dialog.lenshelp=11:C,-
dialog.prey=28:C,-
dialog.quest_log=24:C,-
loot.nearby_corpses=20:A,-
minimap.east=79:A,-
minimap.floor_down=78:A,-
minimap.floor_up=75:A,-
minimap.north=82:A,-
minimap.south=81:A,-
minimap.west=80:A,-
minimap.zoom_in=77:A,-
minimap.zoom_out=74:A,-
window.battle=5:C,-
window.skills=22:A,-
window.spell_list=15:A,-
window.vip=19:C,-"#
        );

        for bindings in [&profile.general_chat_off, &profile.general_chat_on] {
            assert_eq!(
                bindings["client.fullscreen"],
                default_pair((9, CONTROL), Some((40, ALT)))
            );
            assert_eq!(
                bindings["client.logout"],
                default_pair((15, CONTROL), Some((20, CONTROL)))
            );
            assert_eq!(bindings["chat.previous"], default_pair((43, SHIFT), None));
        }
        assert_eq!(
            profile.general_chat_off["chat.mode.temporary"],
            default_pair((40, 0), Some((88, 0)))
        );
        assert_eq!(
            profile.general_chat_on["chat.send"],
            default_pair((40, 0), Some((88, 0)))
        );
    }

    #[test]
    fn enter_chords_are_reserved_for_their_exact_actions_and_contexts() {
        let pair = |binding| HotkeyPair {
            first: Some(binding),
            second: None,
        };
        for (action, binding, chat_on) in [
            ("chat.send", default_binding(40, 0), true),
            ("chat.send", default_binding(88, 0), true),
            ("chat.mode.temporary", default_binding(40, 0), false),
            ("chat.mode.temporary", default_binding(88, 0), false),
            ("client.fullscreen", default_binding(40, ALT), false),
            ("client.fullscreen", default_binding(40, ALT), true),
        ] {
            assert!(validate_general_pair(action, pair(binding), chat_on).is_ok());
        }
        for (action, binding, chat_on) in [
            ("client.options", default_binding(40, 0), false),
            ("chat.send", default_binding(40, 0), false),
            ("chat.send", default_binding(40, CONTROL), true),
            ("chat.mode.temporary", default_binding(88, 0), true),
            ("client.fullscreen", default_binding(40, 0), true),
            ("client.fullscreen", default_binding(88, ALT), true),
        ] {
            assert!(validate_general_pair(action, pair(binding), chat_on).is_err());
        }
        assert!(validate_pair(pair(default_binding(88, 0))).is_err());

        let mut action_bar_enter = HotkeyPreferences::default();
        action_bar_enter.profiles[0].action_bar.shortcuts[12] =
            Some(crate::action_bar::SlotShortcut {
                key: 88,
                modifiers: 0,
            });
        assert!(action_bar_enter.validate(&[82, 79, 81, 80]).is_err());
        assert!(
            HotkeyPreferences::default()
                .validate(&[82, 79, 81, 80])
                .is_ok(),
            "the exact reference Enter chords must be valid"
        );
    }

    #[test]
    fn newly_consumed_reference_actions_resolve_only_in_their_exact_contexts()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut preferences = HotkeyPreferences::default();
        for key in [40, 88] {
            assert_eq!(
                preferences.resolve(&[pressed(key, false)?], true),
                vec![ResolvedHotkey::General("chat.send".into())]
            );
            assert_eq!(
                preferences.resolve(&[pressed(key, false)?], false),
                vec![ResolvedHotkey::General("chat.mode.temporary".into())]
            );
        }
        assert_eq!(
            preferences.resolve(&[pressed(16, false)?], false),
            vec![ResolvedHotkey::General("dialog.cyclopedia.map".into())]
        );
        let alt_l = NormalizedInputEvent::Key {
            code: KeyCode::new(15)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Alt),
            repeat: false,
        };
        for chat_on in [false, true] {
            assert_eq!(
                preferences.resolve(std::slice::from_ref(&alt_l), chat_on),
                vec![ResolvedHotkey::General("window.spell_list".into())]
            );
        }
        assert!(general_action_supported("minimap.center"));
        assert!(general_action_supported("client.previous_profile"));
        assert!(general_action_supported("chat.mode.on"));
        assert!(general_action_supported("chat.mode.off"));
        assert!(general_action_supported("chat.mode.temporary"));
        let screenshot = HotkeyPair {
            first: Some(binding(30, true)),
            second: None,
        };
        let profile = preferences.active_mut().ok_or("default profile")?;
        profile
            .general_chat_off
            .insert("client.screenshot".into(), screenshot);
        profile
            .general_chat_on
            .insert("client.screenshot".into(), screenshot);
        for chat_on in [false, true] {
            assert_eq!(
                preferences.resolve(&[pressed(30, true)?], chat_on),
                vec![ResolvedHotkey::General("client.screenshot".into())]
            );
        }
        assert!(!general_action_supported("minimap.floor_up"));
        Ok(())
    }

    #[test]
    fn movement_repeats_but_one_shot_and_custom_hotkeys_do_not()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut preferences = HotkeyPreferences::default();
        let profile = preferences.active_mut().ok_or("default profile")?;
        profile.custom.push(CustomHotkey {
            action: CustomHotkeyAction::Text {
                text: "hello".into(),
                send_automatically: false,
            },
            chat_on: HotkeyPair::default(),
            chat_off: HotkeyPair {
                first: Some(binding(30, false)),
                second: None,
            },
        });

        let repeated =
            |key, modifiers| -> Result<NormalizedInputEvent, Box<dyn std::error::Error>> {
                Ok(NormalizedInputEvent::Key {
                    code: KeyCode::new(key)?,
                    state: ButtonState::Pressed,
                    modifiers,
                    repeat: true,
                })
            };
        assert_eq!(
            preferences.resolve(&[repeated(26, Modifiers::NONE)?], false),
            vec![ResolvedHotkey::General("movement.north".into())]
        );
        assert!(
            preferences
                .resolve(&[repeated(18, Modifiers::one(Modifier::Control))?], false)
                .is_empty()
        );
        assert!(
            preferences
                .resolve(&[repeated(30, Modifiers::NONE)?], false)
                .is_empty()
        );
        Ok(())
    }

    #[test]
    fn claimed_keyboard_lifecycle_suppresses_repeats_and_release_then_clears()
    -> Result<(), Box<dyn std::error::Error>> {
        let key = |state, repeat| -> Result<NormalizedInputEvent, Box<dyn std::error::Error>> {
            Ok(NormalizedInputEvent::Key {
                code: KeyCode::new(40)?,
                state,
                modifiers: Modifiers::NONE,
                repeat,
            })
        };
        let mut claimed = ClaimedKeyLifecycle::default();
        assert!(claimed.observe(&[key(ButtonState::Pressed, false)?], true));
        assert_eq!(claimed.keys, BTreeSet::from([40]));
        assert!(claimed.observe(&[key(ButtonState::Pressed, true)?], false));
        assert!(claimed.observe(&[key(ButtonState::Released, false)?], false));
        assert!(claimed.keys.is_empty());
        assert!(!claimed.observe(&[key(ButtonState::Released, false)?], false));

        assert!(claimed.observe(&[key(ButtonState::Pressed, false)?], true));
        let _ = claimed.observe(
            &[NormalizedInputEvent::FocusChanged { focused: false }],
            false,
        );
        assert!(claimed.keys.is_empty());
        Ok(())
    }

    #[test]
    fn migration_does_not_backfill_general_defaults_or_intentional_clears() {
        let mut preferences = HotkeyPreferences::default();
        let profile = &mut preferences.profiles[0];
        profile.general_chat_on = BTreeMap::from([("chat.send".into(), HotkeyPair::default())]);
        profile.general_chat_off = BTreeMap::from([
            ("chat.mode.temporary".into(), HotkeyPair::default()),
            ("client.stop".into(), HotkeyPair::default()),
        ]);
        let expected_on = profile.general_chat_on.clone();
        let expected_off = profile.general_chat_off.clone();

        preferences.migrate_v1_reference_defaults(&[82, 79, 81, 80]);

        assert_eq!(preferences.profiles[0].general_chat_on, expected_on);
        assert_eq!(preferences.profiles[0].general_chat_off, expected_off);
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
        profile.general_chat_on.clear();
        profile.general_chat_off.clear();
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
        profile.general_chat_on.clear();
        profile.general_chat_off.clear();
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
        profile.general_chat_off.clear();
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
            .clear();
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
        assert!(profile.general_chat_on.is_empty());
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
            .clear();
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
