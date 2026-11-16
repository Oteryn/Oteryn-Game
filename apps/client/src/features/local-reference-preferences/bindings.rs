//! Local key overrides translate into the existing production feature inputs.
use super::{PreferenceError, ReferencePreferences};
use oteryn_input_actions::{KeyCode, Modifiers, NormalizedInputEvent};

include!(concat!(env!("OUT_DIR"), "/preference_bindings.rs"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceBinding {
    pub id: &'static str,
    pub default_chord: &'static str,
}

pub fn preference_bindings() -> Result<Vec<PreferenceBinding>, PreferenceError> {
    let mut bindings: Vec<_> = REGISTERED_BINDINGS
        .iter()
        .map(|&(id, default_chord)| PreferenceBinding {
            id,
            default_chord: if unbound_by_default(id) {
                ""
            } else {
                default_chord
            },
        })
        .collect();
    bindings.extend(super::bar_hotkeys::extra_bindings());
    Ok(bindings)
}

/// Features whose input chord belongs to a Global channel, help or console command by default;
/// the player can still bind them, and the feature keeps reading its own chord internally.
/// Global opens the Market only from the depot locker, and its Ctrl+M toggles server messages.
/// Prey, VIP and skills open through Global's own window actions (Ctrl+Y, Ctrl+P, Alt+S), Ctrl+K
/// shows the hotkey options, and Global gives the boss slots no chord.
const UNBOUND_BY_DEFAULT: &[&str] = &[
    "0009-highscores-open",
    "0043-vip-open",
    "0060-player-trade-open",
    "0063-market-open",
    "0072-task-board-open",
    "0140-prey-open",
    "0147-exiva-settings-open",
    "0225-boss-difficulty-open",
    "0274-loyalty-skills-open",
];

fn unbound_by_default(id: &str) -> bool {
    UNBOUND_BY_DEFAULT.contains(&id)
}

/// The key an id answers to: the player's choice, else its default unless it has none.
fn effective_chord<'a>(
    map: &'a std::collections::BTreeMap<String, String>,
    id: &str,
    default: &'a str,
) -> Option<&'a str> {
    map.get(id)
        .map(String::as_str)
        .or_else(|| (!unbound_by_default(id)).then_some(default))
}

#[path = "chords.rs"]
mod chords;
use chords::{Chord, parse};
#[path = "binding_validation.rs"]
mod binding_validation;
pub(super) use binding_validation::validate_custom;
use binding_validation::{validate_effective, validate_map};

#[cfg(test)]
#[path = "tests/input_audit.rs"]
mod input_audit;

impl ReferencePreferences {
    pub fn set_action_bar_hotkeys(&mut self, chords: &[&str; 4]) -> Result<(), PreferenceError> {
        let mut candidate = self.clone();
        for (slot, chord) in chords.iter().enumerate() {
            parse(chord)?;
            candidate.keybindings.insert(
                format!("local-spell-action-bar-spell-{}", slot + 1),
                chord.trim().to_ascii_uppercase(),
            );
        }
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }

    /// Chat-on keeps letters available for text; chat-off uses WASD movement.
    /// Apply all four together so a failed preset never leaves a half-edited map.
    pub fn general_hotkey_preset(&mut self, chat_on: bool) -> Result<(), PreferenceError> {
        let mut candidate = self.clone();
        for (direction, chord) in [("north", "W"), ("east", "D"), ("south", "S"), ("west", "A")] {
            let id = format!("0013-paced-movement-move-{direction}");
            if chat_on {
                candidate.keybindings.remove(&id);
            } else {
                candidate.keybindings.insert(id, chord.into());
            }
        }
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }

    pub fn binding_chord<'a>(&'a self, binding: &'a PreferenceBinding) -> &'a str {
        if let Some(slot) = super::bar_hotkeys::extra_slot(binding.id) {
            return self.bar_slot_chord(slot);
        }
        self.keybindings
            .get(binding.id)
            .map_or(binding.default_chord, String::as_str)
    }

    pub fn rebind(&mut self, id: &str, chord: &str) -> Result<(), PreferenceError> {
        if let Some(slot) = super::bar_hotkeys::extra_slot(id) {
            return self.rebind_bar_slot(slot, chord);
        }
        parse(chord)?;
        let mut candidate = self.clone();
        candidate
            .keybindings
            .insert(id.to_owned(), chord.trim().to_ascii_uppercase());
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }

    pub fn binding_secondary<'a>(&'a self, binding: &PreferenceBinding) -> Option<&'a str> {
        self.secondary_keybindings
            .get(binding.id)
            .map(String::as_str)
    }

    pub fn rebind_secondary(&mut self, id: &str, chord: &str) -> Result<(), PreferenceError> {
        let mut candidate = self.clone();
        if chord.trim().is_empty() {
            candidate.secondary_keybindings.remove(id);
        } else {
            parse(chord)?;
            candidate
                .secondary_keybindings
                .insert(id.to_owned(), chord.trim().to_ascii_uppercase());
        }
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }

    pub(super) fn validate_bindings(&self) -> Result<(), PreferenceError> {
        if !self.hotkey_contexts.valid_shape() {
            return Err(PreferenceError);
        }
        validate_effective(&self.keybindings, &self.secondary_keybindings)?;
        self.validate_custom_chords(&self.keybindings, &self.secondary_keybindings)?;
        for (name, preset) in &self.hotkey_contexts.presets {
            for (chat_on, bindings) in [(true, &preset.chat_on), (false, &preset.chat_off)] {
                validate_map(&bindings.primary)?;
                validate_map(&bindings.secondary)?;
                let (primary, secondary) = if name == &self.hotkey_contexts.selected
                    && chat_on == self.hotkey_contexts.chat_on
                {
                    (&self.keybindings, &self.secondary_keybindings)
                } else {
                    (&bindings.primary, &bindings.secondary)
                };
                validate_effective(primary, secondary)?;
                self.validate_custom_chords(primary, secondary)?;
            }
        }
        if !self.action_keys.valid(self) {
            return Err(PreferenceError);
        }
        Ok(())
    }

    /// Translate once, without recursively mapping swapped bindings. Release/repeat events
    /// follow the same mapping so the spell router does not retain a held default key.
    pub fn route_input(
        &self,
        events: &[NormalizedInputEvent],
    ) -> Result<Vec<NormalizedInputEvent>, PreferenceError> {
        self.route_input_chat(events, self.hotkey_contexts.chat_on)
    }

    pub fn route_input_chat(
        &self,
        events: &[NormalizedInputEvent],
        chat_on: bool,
    ) -> Result<Vec<NormalizedInputEvent>, PreferenceError> {
        let (primary, secondary) = self.binding_maps(chat_on)?;
        let mut routed = Vec::new();
        for event in events {
            let NormalizedInputEvent::Key {
                code,
                modifiers,
                state,
                repeat,
            } = event
            else {
                routed.push(event.clone());
                continue;
            };
            let chord = Chord(*code, *modifiers);
            let mut target = None;
            let mut suppressed = false;
            for &(id, default) in REGISTERED_BINDINGS {
                if default.starts_with("MOUSE") {
                    continue;
                }
                let original = parse(default)?;
                let effective = effective_chord(primary, id, default)
                    .map(parse)
                    .transpose()?;
                let alternate = secondary.get(id).map(|s| parse(s)).transpose()?;
                let movement_key = match id {
                    "0013-paced-movement-move-north" => Some(26),
                    "0013-paced-movement-move-east" => Some(7),
                    "0013-paced-movement-move-south" => Some(22),
                    "0013-paced-movement-move-west" => Some(4),
                    _ => None,
                };
                let preserve_arrow = effective.is_some_and(|Chord(code, modifiers)| {
                    movement_key == Some(code.get()) && modifiers == Modifiers::NONE
                });
                if effective == Some(chord)
                    || alternate == Some(chord)
                    || (preserve_arrow && original == chord)
                {
                    target = Some(original);
                    break;
                }
                // Some legacy handlers accept additional modifiers (logout, movement,
                // store, forge, equip and browse-field). Retire those variants as well;
                // another registered effective chord still wins above.
                suppressed |= original.0 == chord.0
                    && chord.1.bits() & original.1.bits() == original.1.bits()
                    && effective != Some(original)
                    && alternate != Some(original)
                    && !preserve_arrow;
            }
            if let Some(Chord(code, modifiers)) = target {
                routed.push(NormalizedInputEvent::Key {
                    code,
                    modifiers,
                    state: *state,
                    repeat: *repeat,
                });
            } else if !suppressed {
                routed.push(event.clone());
            }
        }
        Ok(routed)
    }
}

/// Retains a press's translation until release, including across a preference edit or
/// an early modifier release. Focus/device loss clears both this map and downstream routers.
#[derive(Default)]
pub struct PreferenceInput {
    actions: Vec<super::CustomAction>,
    held_actions: std::collections::BTreeMap<KeyCode, super::CustomAction>,
    held: std::collections::BTreeMap<KeyCode, Option<(KeyCode, Modifiers)>>,
}
impl PreferenceInput {
    pub fn take_custom_actions(&mut self) -> Vec<super::CustomAction> {
        std::mem::take(&mut self.actions)
    }
    pub fn route(
        &mut self,
        preferences: &ReferencePreferences,
        events: &[NormalizedInputEvent],
    ) -> Result<Vec<NormalizedInputEvent>, PreferenceError> {
        self.route_chat(preferences, events, preferences.hotkey_contexts.chat_on)
    }

    pub fn route_chat(
        &mut self,
        preferences: &ReferencePreferences,
        events: &[NormalizedInputEvent],
        chat_on: bool,
    ) -> Result<Vec<NormalizedInputEvent>, PreferenceError> {
        use oteryn_input_actions::ButtonState;
        self.actions.clear();
        let mut output = Vec::new();
        for event in events {
            match event {
                NormalizedInputEvent::Key {
                    code,
                    state,
                    repeat,
                    modifiers,
                } => {
                    if let Some(action) = self.held_actions.get(code) {
                        if *state == ButtonState::Released {
                            self.held_actions.remove(code);
                            self.held.remove(code);
                        } else if *repeat
                            && matches!(
                                action,
                                super::CustomAction::Object { .. }
                                    | super::CustomAction::ActionBar { .. }
                                    | super::CustomAction::Text { send: true, .. }
                            )
                            && self.actions.len() < 16
                        {
                            self.actions.push(action.clone());
                        }
                        continue;
                    }
                    if !self.held.contains_key(code)
                        && let Some(action) = super::bar_hotkeys::routed_action(
                            preferences,
                            *code,
                            *modifiers,
                            chat_on,
                        )
                    {
                        if *state == ButtonState::Pressed {
                            self.held.insert(*code, None);
                            self.held_actions.insert(*code, action.clone());
                            if !repeat && self.actions.len() < 16 {
                                self.actions.push(action);
                            }
                        }
                        continue;
                    }
                    let translated = match self.held.get(code) {
                        Some(translation) => *translation,
                        None => preferences
                            .route_input_chat(std::slice::from_ref(event), chat_on)?
                            .first()
                            .and_then(|e| match e {
                                NormalizedInputEvent::Key {
                                    code, modifiers, ..
                                } => Some((*code, *modifiers)),
                                _ => None,
                            }),
                    };
                    if *state == ButtonState::Released {
                        self.held.remove(code);
                    } else {
                        self.held.insert(*code, translated);
                    }
                    if let Some((code, modifiers)) = translated {
                        output.push(NormalizedInputEvent::Key {
                            code,
                            modifiers,
                            state: *state,
                            repeat: *repeat,
                        });
                    }
                }
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::DeviceLost => {
                    self.held.clear();
                    self.held_actions.clear();
                    self.actions.clear();
                    output.push(event.clone());
                }
                _ => output.push(event.clone()),
            }
        }
        Ok(output)
    }
}

/// Generate one complete key gesture for a menu selection, honoring local rebinding.
pub(super) fn window_events(
    preferences: &ReferencePreferences,
    id: &str,
) -> Vec<NormalizedInputEvent> {
    let Some(binding) = REGISTERED_BINDINGS.iter().find(|(key, _)| *key == id) else {
        return Vec::new();
    };
    let Some(Ok(Chord(code, modifiers))) =
        effective_chord(&preferences.keybindings, id, binding.1).map(parse)
    else {
        return Vec::new();
    };
    [
        oteryn_input_actions::ButtonState::Pressed,
        oteryn_input_actions::ButtonState::Released,
    ]
    .into_iter()
    .map(|state| NormalizedInputEvent::Key {
        code,
        modifiers,
        state,
        repeat: false,
    })
    .collect()
}

pub(crate) fn parse_action_chord(value: &str) -> Option<(KeyCode, Modifiers)> {
    parse(value)
        .ok()
        .map(|Chord(code, modifiers)| (code, modifiers))
}
