//! Physical chord parsing shared by settings input and validation.
use super::*;
use oteryn_input_actions::Modifier;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Chord(pub(super) KeyCode, pub(super) Modifiers);

pub(super) fn parse(value: &str) -> Result<Chord, PreferenceError> {
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
            "BACKSPACE" => 42,
            "INSERT" => 73,
            "HOME" => 74,
            "PAGEUP" => 75,
            "DELETE" => 76,
            "END" => 77,
            "PAGEDOWN" => 78,
            "ARROWUP" => 82,
            "ARROWDOWN" => 81,
            "ARROWLEFT" => 80,
            "ARROWRIGHT" => 79,
            "ESCAPE" => 41,
            "ENTER" => 40,
            "KEYPADENTER" | "NUMPADENTER" => 88,
            "KEYPAD0" | "NUMPAD0" => 98,
            "KEYPAD1" | "NUMPAD1" => 89,
            "KEYPAD2" | "NUMPAD2" => 90,
            "KEYPAD3" | "NUMPAD3" => 91,
            "KEYPAD4" | "NUMPAD4" => 92,
            "KEYPAD5" | "NUMPAD5" => 93,
            "KEYPAD6" | "NUMPAD6" => 94,
            "KEYPAD7" | "NUMPAD7" => 95,
            "KEYPAD8" | "NUMPAD8" => 96,
            "KEYPAD9" | "NUMPAD9" => 97,
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

#[cfg(test)]
mod audit_tests {
    use super::*;
    #[test]
    fn global_navigation_and_keypad_keys_rebind_and_route() -> Result<(), Box<dyn std::error::Error>>
    {
        use oteryn_input_actions::ButtonState;
        for (name, physical) in [
            ("Home", 74),
            ("End", 77),
            ("Insert", 73),
            ("Delete", 76),
            ("PageUp", 75),
            ("PageDown", 78),
            ("Backspace", 42),
            ("Keypad0", 98),
            ("Keypad2", 90),
            ("Keypad4", 92),
            ("Keypad5", 93),
            ("Keypad6", 94),
            ("Keypad8", 96),
            ("Numpad0", 98),
            ("Numpad2", 90),
            ("Numpad4", 92),
            ("Numpad5", 93),
            ("Numpad6", 94),
            ("Numpad8", 96),
        ] {
            let mut preferences = ReferencePreferences::default();
            let chord = format!("SUPER+{name}");
            preferences.rebind("0009-highscores-open", &chord)?;
            let binding = preference_bindings()?
                .into_iter()
                .find(|binding| binding.id == "0009-highscores-open")
                .ok_or("binding")?;
            assert_eq!(
                preferences.binding_chord(&binding),
                chord.to_ascii_uppercase()
            );
            let event = NormalizedInputEvent::Key {
                code: KeyCode::new(physical)?,
                modifiers: Modifiers::one(Modifier::Super),
                state: ButtonState::Pressed,
                repeat: false,
            };
            assert!(
                oteryn_client_ui::highscores::open_key(&preferences.route_input(&[event])?),
                "{name}"
            );
        }
        Ok(())
    }
}
