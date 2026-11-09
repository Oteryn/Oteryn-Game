//! Profile editor runs on the settings window thread and uses the validated settings owner.
#![allow(
    unsafe_code,
    reason = "Win32 controls and their boxed callback data stay on this UI thread"
)]
use super::{Shared, control, lock, text, wide};
use oteryn_client::play::preference_bindings;
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};

#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn panel_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let notice = w >> 16;
            if !actions.is_null()
                && (*actions).0.len() < 16
                && ((matches!(id, 501 | 506 | 510) && notice == CBN_SELCHANGE as usize)
                    || (matches!(id, 503..=505 | 512 | 514 | 515) && notice == BN_CLICKED as usize))
            {
                (*actions).0.push(id);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

pub(super) fn open(shared: Arc<Mutex<Shared>>, parent: HWND) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Hotkey profiles").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            920,
            580,
            parent,
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        EnableWindow(parent, 0);
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, panel_proc as *const () as isize);
        let help = control(
            hwnd,
            "STATIC",
            "The active profile applies immediately. Choose a chat context to edit its keys.\r\nThe chat-on map activates while the chat window is open.",
            0,
            [16, 12, 870, 48],
            0,
        );
        let profiles = control(
            hwnd,
            "COMBOBOX",
            "",
            501,
            [16, 75, 240, 220],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let name = control(
            hwnd,
            "EDIT",
            "",
            502,
            [270, 75, 220, 28],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        let add = control(hwnd, "BUTTON", "Copy as new", 503, [510, 75, 115, 28], 0);
        let rename = control(hwnd, "BUTTON", "Rename", 504, [640, 75, 110, 28], 0);
        let remove = control(hwnd, "BUTTON", "Delete", 505, [765, 75, 115, 28], 0);
        let context = control(
            hwnd,
            "COMBOBOX",
            "",
            506,
            [16, 125, 240, 150],
            CBS_DROPDOWNLIST as u32,
        );
        for label in ["Chat off", "Chat on"] {
            SendMessageW(context, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        let actions_list = control(
            hwnd,
            "COMBOBOX",
            "",
            510,
            [270, 125, 610, 320],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let primary_label = control(hwnd, "STATIC", "Primary key", 0, [16, 185, 180, 24], 0);
        let primary = control(
            hwnd,
            "EDIT",
            "",
            511,
            [210, 185, 440, 28],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        let bind_primary = control(hwnd, "BUTTON", "Set primary", 512, [680, 185, 200, 28], 0);
        let secondary_label = control(
            hwnd,
            "STATIC",
            "Secondary (blank clears)",
            0,
            [16, 235, 190, 24],
            0,
        );
        let secondary = control(
            hwnd,
            "EDIT",
            "",
            513,
            [210, 235, 440, 28],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        let bind_secondary = control(hwnd, "BUTTON", "Set secondary", 514, [680, 235, 200, 28], 0);
        let feedback = control(hwnd, "STATIC", "", 0, [16, 300, 860, 130], 0);
        let done = control(hwnd, "BUTTON", "Done", 515, [680, 455, 200, 30], 0);
        if [
            help,
            profiles,
            name,
            add,
            rename,
            remove,
            context,
            actions_list,
            primary_label,
            primary,
            bind_primary,
            secondary_label,
            secondary,
            bind_secondary,
            feedback,
            done,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            EnableWindow(parent, 1);
            return;
        }
        let bindings: Vec<_> = preference_bindings()
            .unwrap_or_default()
            .into_iter()
            .filter(|b| !b.default_chord.starts_with("MOUSE"))
            .collect();
        for binding in &bindings {
            let label = binding
                .id
                .split_once('-')
                .map_or(binding.id, |(_, name)| name)
                .trim_end_matches("-open")
                .replace('-', " ");
            SendMessageW(
                actions_list,
                CB_ADDSTRING,
                0,
                wide(&label).as_ptr() as isize,
            );
        }
        SendMessageW(actions_list, CB_SETCURSEL, 0, 0);
        let mut names = Vec::<String>::new();
        let mut held = None;
        let mut held_scope = None;
        let mut selected = -1;
        loop {
            let mut message: MSG = std::mem::zeroed();
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let mut state = lock(&shared);
            if state.shutdown {
                break;
            }
            if held_scope.is_some_and(|old| old != state.character_scope()) {
                actions.0.clear();
                held = None;
            }
            for action in std::mem::take(&mut actions.0) {
                if action == 515 {
                    DestroyWindow(hwnd);
                    break;
                }
                if action == 510 {
                    selected = -1;
                    continue;
                }
                let mut candidate = state.preferences.clone();
                let result = match action {
                    501 => {
                        let index = SendMessageW(profiles, CB_GETCURSEL, 0, 0);
                        names.get(index.max(0) as usize).map_or(
                            Err("Choose a profile".to_owned()),
                            |name| {
                                candidate
                                    .select_hotkey_profile(name)
                                    .map_err(|e| e.to_string())
                            },
                        )
                    }
                    503 => candidate
                        .add_hotkey_profile(text(name).trim())
                        .map_err(|e| e.to_string()),
                    504 => {
                        let old = candidate.hotkey_contexts.selected.clone();
                        candidate
                            .rename_hotkey_profile(&old, text(name).trim())
                            .map_err(|e| e.to_string())
                    }
                    505 => {
                        let old = candidate.hotkey_contexts.selected.clone();
                        candidate
                            .remove_hotkey_profile(&old)
                            .map_err(|e| e.to_string())
                    }
                    506 => candidate
                        .select_hotkey_context(SendMessageW(context, CB_GETCURSEL, 0, 0) == 1)
                        .map_err(|e| e.to_string()),
                    512 | 514 => {
                        let index = SendMessageW(actions_list, CB_GETCURSEL, 0, 0);
                        bindings.get(index.max(0) as usize).map_or(
                            Err("Choose an action".to_owned()),
                            |binding| {
                                if action == 512 {
                                    candidate.rebind(binding.id, &text(primary))
                                } else {
                                    candidate.rebind_secondary(binding.id, &text(secondary))
                                }
                                .map_err(|e| e.to_string())
                            },
                        )
                    }
                    _ => continue,
                };
                match result {
                    Ok(()) => state.apply(candidate),
                    Err(error) => state.feedback = error,
                }
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let changed =
                held.as_ref()
                    .is_none_or(|old: &oteryn_client::play::ReferencePreferences| {
                        old.hotkey_contexts != state.preferences.hotkey_contexts
                            || old.keybindings != state.preferences.keybindings
                            || old.secondary_keybindings != state.preferences.secondary_keybindings
                    });
            if changed {
                names = state
                    .preferences
                    .hotkey_contexts
                    .presets
                    .keys()
                    .cloned()
                    .collect();
                SendMessageW(profiles, CB_RESETCONTENT, 0, 0);
                for name in &names {
                    SendMessageW(profiles, CB_ADDSTRING, 0, wide(name).as_ptr() as isize);
                }
                let active = names
                    .iter()
                    .position(|n| n == &state.preferences.hotkey_contexts.selected)
                    .unwrap_or(0);
                SendMessageW(profiles, CB_SETCURSEL, active, 0);
                SendMessageW(
                    context,
                    CB_SETCURSEL,
                    usize::from(state.preferences.hotkey_contexts.chat_on),
                    0,
                );
                SetWindowTextW(
                    name,
                    wide(&state.preferences.hotkey_contexts.selected).as_ptr(),
                );
                held = Some(state.preferences.clone());
                selected = -1;
            }
            let index = SendMessageW(actions_list, CB_GETCURSEL, 0, 0);
            if index != selected {
                selected = index;
                if let Some(binding) = bindings.get(index.max(0) as usize) {
                    SetWindowTextW(
                        primary,
                        wide(state.preferences.binding_chord(binding)).as_ptr(),
                    );
                    SetWindowTextW(
                        secondary,
                        wide(state.preferences.binding_secondary(binding).unwrap_or("")).as_ptr(),
                    );
                }
            }
            SetWindowTextW(
                feedback,
                wide(&format!("{}\r\n{}", state.scope_label(), state.feedback)).as_ptr(),
            );
            held_scope = Some(state.character_scope());
            drop(state);
            std::thread::sleep(Duration::from_millis(20));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        EnableWindow(parent, 1);
    }
}
