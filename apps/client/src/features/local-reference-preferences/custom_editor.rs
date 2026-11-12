//! Custom action assignment on the settings dialog's owning thread.
use super::*;
use oteryn_client::play::{CustomAction, ObjectTarget};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;

pub(super) fn profile_action(action: usize, parent: HWND, shared: &Arc<Mutex<Shared>>, name: &str) {
    if action == 234 {
        run(parent, shared);
        return;
    }
    let mut state = lock(shared);
    if action == 232 {
        if let Err(error) = state.load_custom_hotkey_profile(name) {
            state.feedback = error.to_string();
        }
        return;
    }
    let mut preferences = state.preferences.clone();
    let result = match action {
        231 => preferences.save_hotkey_profile(name),
        _ => {
            preferences.hotkey_profiles.remove(name);
            preferences.custom_hotkey_profiles.remove(name);
            preferences.bar_hotkey_profiles.remove(name);
            preferences.bar_action_profiles.remove(name);
            Ok(())
        }
    };
    match result {
        Ok(()) => state.apply(preferences),
        Err(error) => state.feedback = error.to_string(),
    }
}

fn kind(action: &CustomAction) -> usize {
    match action {
        CustomAction::ActionBar { .. } => 8,
        CustomAction::Text { send, .. } => usize::from(*send),
        CustomAction::Object { target, .. } => match target {
            ObjectTarget::Use => 2,
            ObjectTarget::Cursor => 3,
            ObjectTarget::SelfActor => 4,
            ObjectTarget::CurrentTarget => 5,
            ObjectTarget::Equip => 6,
        },
        CustomAction::Passive { .. }
        | CustomAction::PassiveStatus { .. }
        | CustomAction::Utility { .. } => 7,
    }
}
fn source_matches(left: &CustomAction, right: &CustomAction) -> bool {
    match (left, right) {
        (
            CustomAction::Object {
                definition: a,
                sub_type: b,
                ..
            },
            CustomAction::Object {
                definition: c,
                sub_type: d,
                ..
            },
        ) => a == c && b == d,
        _ => left == right,
    }
}
fn action_from(kind: usize, text: String, source: Option<&CustomAction>) -> Option<CustomAction> {
    if kind < 2 {
        return Some(CustomAction::Text {
            text,
            send: kind == 1,
        });
    }
    match (kind, source?) {
        (
            2..=6,
            CustomAction::Object {
                definition,
                sub_type,
                ..
            },
        ) => Some(CustomAction::Object {
            definition: *definition,
            sub_type: *sub_type,
            target: [
                ObjectTarget::Use,
                ObjectTarget::Cursor,
                ObjectTarget::SelfActor,
                ObjectTarget::CurrentTarget,
                ObjectTarget::Equip,
            ][kind - 2],
        }),
        (8, CustomAction::ActionBar { slot }) => Some(CustomAction::ActionBar { slot: *slot }),
        (7, CustomAction::PassiveStatus { ability }) => {
            Some(CustomAction::PassiveStatus { ability: *ability })
        }
        (7, CustomAction::Utility { action }) => Some(CustomAction::Utility { action: *action }),
        (7, CustomAction::Passive { cast_index }) => Some(CustomAction::Passive {
            cast_index: *cast_index,
        }),
        _ => None,
    }
}
fn run(parent: HWND, shared: &Arc<Mutex<Shared>>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Custom Hotkeys — object / text / cooldown").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            750,
            500,
            parent,
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, panel_proc as *const () as isize);
        let existing = control(
            hwnd,
            "COMBOBOX",
            "",
            0,
            [10, 15, 710, 220],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        control(
            hwnd,
            "STATIC",
            "Key chord (for example F9 or ALT+9)",
            0,
            [10, 55, 600, 24],
            0,
        );
        let chord = control(
            hwnd,
            "EDIT",
            "F9",
            0,
            [10, 85, 180, 26],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        let types = control(
            hwnd,
            "COMBOBOX",
            "",
            0,
            [210, 85, 510, 250],
            CBS_DROPDOWNLIST as u32,
        );
        for label in [
            "Text: paste into composer",
            "Text: send immediately",
            "Object: use",
            "Object: choose target with cursor",
            "Object: use on self",
            "Object: use on current target",
            "Object: equip / unequip",
            "Passive ability: status / cooldown (read only)",
            "Action bar slot",
        ] {
            SendMessageW(types, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(types, CB_SETCURSEL, 0, 0);
        control(
            hwnd,
            "STATIC",
            "Text (up to 1,020 UTF-8 bytes)",
            0,
            [10, 125, 710, 24],
            0,
        );
        let content = control(
            hwnd,
            "EDIT",
            "",
            0,
            [10, 155, 710, 26],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(content, EM_SETLIMITTEXT, 1020, 0);
        control(
            hwnd,
            "STATIC",
            "Object / ability from this session's inventory or catalogue",
            0,
            [10, 200, 710, 24],
            0,
        );
        let sources = control(
            hwnd,
            "COMBOBOX",
            "",
            0,
            [10, 230, 710, 200],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        control(hwnd, "BUTTON", "Save action", 103, [10, 280, 180, 30], 0);
        control(hwnd, "BUTTON", "Delete action", 106, [210, 280, 180, 30], 0);
        control(hwnd, "BUTTON", "New action", 110, [410, 280, 180, 30], 0);
        let feedback = control(hwnd, "STATIC", "", 0, [10, 330, 710, 90], 0);
        if [existing, chord, types, content, sources, feedback]
            .iter()
            .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            return;
        }
        EnableWindow(parent, 0);
        let mut held = None;
        let mut selected = -1;
        let mut old_key: Option<String> = None;
        let mut choices: Vec<(String, CustomAction)> = Vec::new();
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if IsWindow(hwnd) == 0 || IsWindow(parent) == 0 {
                break;
            }
            let mut state = lock(shared);
            if state.shutdown {
                break;
            }
            if choices != state.custom_choices {
                let previous = usize::try_from(SendMessageW(sources, CB_GETCURSEL, 0, 0))
                    .ok()
                    .and_then(|i| choices.get(i))
                    .map(|(_, action)| action.clone());
                choices.clone_from(&state.custom_choices);
                SendMessageW(sources, CB_RESETCONTENT, 0, 0);
                for (label, _) in &choices {
                    SendMessageW(sources, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
                }
                let index = previous
                    .and_then(|action| {
                        choices
                            .iter()
                            .position(|(_, item)| source_matches(&action, item))
                    })
                    .unwrap_or(0);
                SendMessageW(sources, CB_SETCURSEL, index, 0);
            }
            if held.as_ref() != Some(&state.preferences) {
                SendMessageW(existing, CB_RESETCONTENT, 0, 0);
                for key in state.preferences.custom_hotkeys.keys() {
                    SendMessageW(existing, CB_ADDSTRING, 0, wide(key).as_ptr() as isize);
                }
                let index = old_key.as_ref().and_then(|key| {
                    state
                        .preferences
                        .custom_hotkeys
                        .keys()
                        .position(|value| value == key)
                });
                SendMessageW(existing, CB_SETCURSEL, index.unwrap_or(usize::MAX), 0);
                selected = -1;
                held = Some(state.preferences.clone());
            }
            let index = SendMessageW(existing, CB_GETCURSEL, 0, 0);
            if index != selected && index >= 0 {
                selected = index;
                if let Some((key, action)) =
                    state.preferences.custom_hotkeys.iter().nth(index as usize)
                {
                    old_key = Some(key.clone());
                    SetWindowTextW(chord, wide(key).as_ptr());
                    SendMessageW(types, CB_SETCURSEL, kind(action), 0);
                    SetWindowTextW(
                        content,
                        wide(match action {
                            CustomAction::Text { text, .. } => text,
                            _ => "",
                        })
                        .as_ptr(),
                    );
                    let source = choices
                        .iter()
                        .position(|(_, candidate)| source_matches(action, candidate));
                    SendMessageW(sources, CB_SETCURSEL, source.unwrap_or(usize::MAX), 0);
                }
            }
            for id in std::mem::take(&mut actions.0) {
                if id == 110 {
                    old_key = None;
                    selected = -1;
                    SendMessageW(existing, CB_SETCURSEL, usize::MAX, 0);
                    SetWindowTextW(chord, wide("").as_ptr());
                    SetWindowTextW(content, wide("").as_ptr());
                    continue;
                }
                let key = text(chord).trim().to_ascii_uppercase();
                let mut preferences = state.preferences.clone();
                if id == 106 {
                    preferences
                        .custom_hotkeys
                        .remove(old_key.as_ref().unwrap_or(&key));
                    state.apply(preferences);
                    old_key = None;
                    continue;
                }
                if id != 103 {
                    continue;
                }
                let source = usize::try_from(SendMessageW(sources, CB_GETCURSEL, 0, 0))
                    .ok()
                    .and_then(|i| choices.get(i))
                    .map(|(_, action)| action);
                let Some(action) = usize::try_from(SendMessageW(types, CB_GETCURSEL, 0, 0))
                    .ok()
                    .and_then(|kind| action_from(kind, editor_text(content), source))
                else {
                    state.feedback =
                        "Select an inventory item for Object, or a passive/cooldown entry for Passive"
                            .into();
                    continue;
                };
                if let Some(old) = &old_key {
                    preferences.custom_hotkeys.remove(old);
                }
                match preferences.assign_custom_hotkey(&key, action) {
                    Ok(()) => {
                        state.apply(preferences);
                        old_key = Some(key);
                    }
                    Err(error) => {
                        state.feedback =
                            format!("{error}. Check text length and choose an unused chord.")
                    }
                }
            }
            SetWindowTextW(feedback, wide(&state.feedback).as_ptr());
            drop(state);
            std::thread::sleep(Duration::from_millis(20));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        EnableWindow(parent, 1);
    }
}
unsafe fn editor_text(hwnd: HWND) -> String {
    unsafe {
        let mut buffer = [0u16; 1021];
        let count = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        String::from_utf16_lossy(&buffer[..count.max(0) as usize])
    }
}
