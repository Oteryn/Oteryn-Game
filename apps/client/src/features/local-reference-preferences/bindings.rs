//! Local key overrides translate into the existing production feature inputs.
use super::{PreferenceError, ReferencePreferences};
use oteryn_input_actions::{KeyCode, Modifier, Modifiers, NormalizedInputEvent};

include!(concat!(env!("OUT_DIR"), "/preference_bindings.rs"));

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreferenceBinding {
    pub id: &'static str,
    pub default_chord: &'static str,
}

pub fn preference_bindings() -> Result<Vec<PreferenceBinding>, PreferenceError> {
    Ok(REGISTERED_BINDINGS
        .iter()
        .map(|&(id, default_chord)| PreferenceBinding { id, default_chord })
        .collect())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Chord(KeyCode, Modifiers);

fn parse(value: &str) -> Result<Chord, PreferenceError> {
    let upper = value.trim().to_ascii_uppercase();
    let mut modifiers = Modifiers::NONE;
    let mut code = None;
    for part in upper.split('+') {
        let modifier = match part {
            "CTRL" => Some(Modifier::Control),
            "SHIFT" => Some(Modifier::Shift),
            "ALT" => Some(Modifier::Alt),
            "SUPER" => Some(Modifier::Super),
            _ => None,
        };
        if let Some(modifier) = modifier {
            if modifiers.contains(modifier) {
                return Err(PreferenceError);
            }
            modifiers = modifiers.with(modifier);
            continue;
        }
        if code.is_some() {
            return Err(PreferenceError);
        }
        let number = match part {
            "ARROWUP" => 82,
            "ARROWDOWN" => 81,
            "ARROWLEFT" => 80,
            "ARROWRIGHT" => 79,
            "NUMPAD1" | "KEYPAD1" => 89,
            "NUMPAD3" | "KEYPAD3" => 91,
            "NUMPAD7" | "KEYPAD7" => 95,
            "NUMPAD9" | "KEYPAD9" => 97,
            "ESCAPE" => 41,
            "ENTER" => 40,
            "KEYPADENTER" | "NUMPADENTER" => 88,
            "SPACE" => 44,
            "TAB" => 43,
            "0" => 39,
            key if key.len() == 1 && key.as_bytes()[0].is_ascii_uppercase() => {
                u16::from(key.as_bytes()[0] - b'A') + 4
            }
            key if key.len() == 1 && matches!(key.as_bytes()[0], b'1'..=b'9') => {
                u16::from(key.as_bytes()[0] - b'1') + 30
            }
            key if key.starts_with('F') => {
                let n = key[1..].parse::<u16>().map_err(|_| PreferenceError)?;
                if !(1..=12).contains(&n) {
                    return Err(PreferenceError);
                }
                57 + n
            }
            _ => return Err(PreferenceError),
        };
        code = Some(KeyCode::new(number).map_err(|_| PreferenceError)?);
    }
    Ok(Chord(code.ok_or(PreferenceError)?, modifiers))
}

impl ReferencePreferences {
    pub fn binding_chord<'a>(&'a self, binding: &'a PreferenceBinding) -> &'a str {
        self.keybindings
            .get(binding.id)
            .map_or(binding.default_chord, String::as_str)
    }

    pub fn rebind(&mut self, id: &str, chord: &str) -> Result<(), PreferenceError> {
        parse(chord)?;
        let mut candidate = self.clone();
        candidate
            .keybindings
            .insert(id.to_owned(), chord.trim().to_ascii_uppercase());
        candidate.validate_bindings()?;
        *self = candidate;
        Ok(())
    }

    pub(super) fn validate_bindings(&self) -> Result<(), PreferenceError> {
        if self.keybindings.keys().any(|id| {
            !REGISTERED_BINDINGS
                .iter()
                .any(|(known, default)| known == id && !default.starts_with("MOUSE"))
        }) {
            return Err(PreferenceError);
        }
        for chord in self.keybindings.values() {
            let chord = parse(chord)?;
            if RESERVED_CHORDS
                .iter()
                .any(|reserved| parse(reserved).is_ok_and(|r| r == chord))
            {
                return Err(PreferenceError);
            }
        }
        let mut used = Vec::new();
        for &(id, default) in REGISTERED_BINDINGS {
            if default.starts_with("MOUSE") {
                continue;
            }
            let chord = parse(self.keybindings.get(id).map_or(default, String::as_str))?;
            if used.contains(&chord) {
                return Err(PreferenceError);
            }
            used.push(chord);
        }
        Ok(())
    }

    /// Translate once, without recursively mapping swapped bindings. Release/repeat events
    /// follow the same mapping so the spell router does not retain a held default key.
    pub fn route_input(
        &self,
        events: &[NormalizedInputEvent],
    ) -> Result<Vec<NormalizedInputEvent>, PreferenceError> {
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
                let effective = parse(self.keybindings.get(id).map_or(default, String::as_str))?;
                if effective == chord {
                    target = Some(original);
                    break;
                }
                // Some legacy handlers accept additional modifiers (logout, movement,
                // store, forge, equip and browse-field). Retire those variants as well;
                // another registered effective chord still wins above.
                suppressed |= original.0 == chord.0
                    && chord.1.bits() & original.1.bits() == original.1.bits()
                    && effective != original;
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
    held: std::collections::BTreeMap<KeyCode, Option<(KeyCode, Modifiers)>>,
}
impl PreferenceInput {
    pub fn route(
        &mut self,
        preferences: &ReferencePreferences,
        events: &[NormalizedInputEvent],
    ) -> Result<Vec<NormalizedInputEvent>, PreferenceError> {
        use oteryn_input_actions::ButtonState;
        let mut output = Vec::new();
        for event in events {
            match event {
                NormalizedInputEvent::Key {
                    code,
                    state,
                    repeat,
                    ..
                } => {
                    let translated = match self.held.get(code) {
                        Some(translation) => *translation,
                        None => preferences
                            .route_input(std::slice::from_ref(event))?
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
                    output.push(event.clone());
                }
                _ => output.push(event.clone()),
            }
        }
        Ok(output)
    }
}

#[cfg(test)]
#[test]
fn qn6_diagonal_default_chords_parse_to_keypad_codes() -> Result<(), &'static str> {
    for (name, expected) in [
        ("NUMPAD1", 89),
        ("NUMPAD3", 91),
        ("NUMPAD7", 95),
        ("NUMPAD9", 97),
    ] {
        let Chord(code, modifiers) = parse(name).map_err(|_| "diagonal default chord")?;
        assert_eq!(code.get(), expected);
        assert_eq!(modifiers, Modifiers::NONE);
    }
    Ok(())
}
