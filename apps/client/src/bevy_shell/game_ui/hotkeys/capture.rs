//! Physical key capture uses the same chord spelling as the preference dispatcher.
use super::*;
use bevy::input::keyboard::KeyboardInput;

fn chord(key: KeyCode, held: &ButtonInput<KeyCode>) -> Option<String> {
    if matches!(
        key,
        KeyCode::ControlLeft
            | KeyCode::ControlRight
            | KeyCode::AltLeft
            | KeyCode::AltRight
            | KeyCode::ShiftLeft
            | KeyCode::ShiftRight
            | KeyCode::SuperLeft
            | KeyCode::SuperRight
    ) {
        return None;
    }
    let mut text = String::new();
    for (left, right, name) in [
        (KeyCode::ControlLeft, KeyCode::ControlRight, "CTRL+"),
        (KeyCode::ShiftLeft, KeyCode::ShiftRight, "SHIFT+"),
        (KeyCode::AltLeft, KeyCode::AltRight, "ALT+"),
        (KeyCode::SuperLeft, KeyCode::SuperRight, "SUPER+"),
    ] {
        if held.pressed(left) || held.pressed(right) {
            text.push_str(name);
        }
    }
    let key = format!("{key:?}").to_ascii_uppercase();
    text.push_str(
        key.strip_prefix("KEY")
            .or_else(|| key.strip_prefix("DIGIT"))
            .unwrap_or(&key),
    );
    Some(text)
}

pub(super) fn capture(
    mut keys: MessageReader<KeyboardInput>,
    held: Res<ButtonInput<KeyCode>>,
    owner: NonSend<HostOwner>,
    guard: super::super::modal::ModalGuard,
    mut editors: Query<(Entity, &mut Editor)>,
    mut fields: Query<(&Field, &mut EditableText)>,
) {
    let Some((entity, mut editor)) = editors.iter_mut().next() else {
        keys.clear();
        return;
    };
    if !editor.capturing {
        keys.clear();
        return;
    }
    if !guard.allows(entity)
        || editor.generation != owner.generation()
        || owner.preferences_draft().is_none()
    {
        editor.capturing = false;
        keys.clear();
        return;
    }
    for key in keys.read() {
        if key.state != bevy::input::ButtonState::Pressed || key.repeat {
            continue;
        }
        let Some(chord) = chord(key.key_code, &held) else {
            continue;
        };
        // Escape retains the surrounding Options dialog's normal Cancel behaviour.
        if key.key_code == KeyCode::Escape {
            editor.capturing = false;
            break;
        }
        for (field, mut text) in &mut fields {
            if field.0 == 0 {
                text.pending_edits.clear();
                text.pending_paste = None;
                text.editor_mut().set_text(&chord);
            }
        }
        let mut candidate = owner.preferences_draft().cloned().unwrap_or_default();
        editor.feedback = if save(
            &mut candidate,
            editor.selected.as_deref(),
            &chord,
            CustomAction::Text {
                text: "capture validation".into(),
                send: false,
            },
        )
        .is_ok()
        {
            format!("Captured {chord}; Add / Save to assign")
        } else {
            format!("{chord}: invalid or conflicting binding; choose another key")
        };
        editor.capturing = false;
        break;
    }
}
