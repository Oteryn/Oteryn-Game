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
            "ARROWUP" => 82,
            "ARROWDOWN" => 81,
            "ARROWLEFT" => 80,
            "ARROWRIGHT" => 79,
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
