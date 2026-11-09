//! Conflict checks include both named chat contexts and custom actions.
use super::*;

pub(super) fn validate_map(
    map: &std::collections::BTreeMap<String, String>,
) -> Result<(), PreferenceError> {
    for (id, value) in map {
        if !REGISTERED_BINDINGS
            .iter()
            .any(|(known, default)| known == id && !default.starts_with("MOUSE"))
        {
            return Err(PreferenceError);
        }
        let chord = parse(value)?;
        if RESERVED_CHORDS
            .iter()
            .any(|reserved| parse(reserved).is_ok_and(|r| r == chord))
        {
            return Err(PreferenceError);
        }
    }
    Ok(())
}

pub(super) fn validate_effective(
    primary: &std::collections::BTreeMap<String, String>,
    secondary: &std::collections::BTreeMap<String, String>,
) -> Result<(), PreferenceError> {
    validate_map(primary)?;
    validate_map(secondary)?;
    let mut used = Vec::new();
    for &(id, default) in REGISTERED_BINDINGS {
        if default.starts_with("MOUSE") {
            continue;
        }
        let chord = parse(primary.get(id).map_or(default, String::as_str))?;
        if used.contains(&chord) {
            return Err(PreferenceError);
        }
        used.push(chord);
        if let Some(value) = secondary.get(id) {
            let chord = parse(value)?;
            if used.contains(&chord) {
                return Err(PreferenceError);
            }
            used.push(chord);
        }
    }
    Ok(())
}

pub(in crate::play) fn validate_custom(
    preferences: &ReferencePreferences,
    primary: &std::collections::BTreeMap<String, String>,
    secondary: &std::collections::BTreeMap<String, String>,
) -> Result<(), PreferenceError> {
    if preferences.custom_hotkeys.len() > 64 {
        return Err(PreferenceError);
    }
    let mut used = Vec::new();
    for (value, action) in &preferences.custom_hotkeys {
        let chord = parse(value)?;
        if !action.valid()
            || used.contains(&chord)
            || RESERVED_CHORDS
                .iter()
                .any(|key| parse(key).is_ok_and(|r| r == chord))
        {
            return Err(PreferenceError);
        }
        used.push(chord);
    }
    for &(id, default) in REGISTERED_BINDINGS {
        if default.starts_with("MOUSE") {
            continue;
        }
        if used.contains(&parse(primary.get(id).map_or(default, String::as_str))?) {
            return Err(PreferenceError);
        }
    }
    for chord in secondary.values() {
        if used.contains(&parse(chord)?) {
            return Err(PreferenceError);
        }
    }
    Ok(())
}
