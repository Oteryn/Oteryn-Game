//! General controls editor; local settings own validation and character persistence.
#![allow(
    unsafe_code,
    reason = "Win32 controls and callback data stay on this UI thread"
)]
use super::{Shared, control, lock, text, wide};
use oteryn_client::play::{MousePreset, MovementKeys, RotationModifier};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};
unsafe extern "system" fn panel_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == 705 && w >> 16 == BN_CLICKED as usize {
            let pending = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut bool;
            if !pending.is_null() {
                *pending = true;
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
            wide("Mouse and movement").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            690,
            440,
            parent,
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        EnableWindow(parent, 0);
        let mut pending = Box::new(false);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *pending as *mut bool) as isize);
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, panel_proc as *const () as isize);
        let labels = control(
            hwnd,
            "STATIC",
            "Mouse preset\r\n\r\nMovement keys\r\n\r\nRotation modifier\r\n\r\nKey repeat interval (0–1000 ms)",
            0,
            [16, 22, 290, 190],
            0,
        );
        let mouse = control(
            hwnd,
            "COMBOBOX",
            "",
            701,
            [320, 16, 320, 120],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let movement = control(
            hwnd,
            "COMBOBOX",
            "",
            702,
            [320, 60, 320, 120],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let rotation = control(
            hwnd,
            "COMBOBOX",
            "",
            703,
            [320, 104, 320, 140],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let delay = control(
            hwnd,
            "EDIT",
            "",
            704,
            [320, 148, 160, 28],
            WS_BORDER | ES_NUMBER as u32,
        );
        let apply = control(hwnd, "BUTTON", "Apply", 705, [490, 148, 150, 28], 0);
        let help = control(
            hwnd,
            "STATIC",
            "Left-handed swaps the world select/walk and context-menu buttons, including drag actions.\r\nWASD movement is active only while chat is closed; arrow keys remain available.\r\nThe rotation modifier plus a movement key turns instead of stepping.\r\nThe repeat interval slows held-key repeats; the first press stays immediate. Server movement pacing still applies.",
            0,
            [16, 215, 625, 110],
            0,
        );
        let feedback = control(hwnd, "STATIC", "", 0, [16, 340, 625, 50], 0);
        if [
            labels, mouse, movement, rotation, delay, apply, help, feedback,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            EnableWindow(parent, 1);
            return;
        }
        for (handle, labels) in [
            (mouse, vec!["Standard", "Left-handed"]),
            (movement, vec!["Arrows", "Arrows and WASD (chat closed)"]),
            (rotation, vec!["Control", "Shift", "Alt"]),
        ] {
            for label in labels {
                SendMessageW(handle, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            }
        }
        let mut held = None;
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 && !lock(&shared).shutdown {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            let mut state = lock(&shared);
            let snapshot = (state.character_scope(), state.preferences.controls.clone());
            // Drop a queued apply after a character switch rather than writing old fields
            // into the newly selected character's settings.
            if held
                .as_ref()
                .is_some_and(|old: &(Option<[u8; 16]>, _)| old.0 != snapshot.0)
            {
                *pending = false;
            }
            if std::mem::take(&mut *pending) {
                let mut preferences = state.preferences.clone();
                preferences.controls.mouse = if SendMessageW(mouse, CB_GETCURSEL, 0, 0) == 1 {
                    MousePreset::LeftHanded
                } else {
                    MousePreset::Standard
                };
                preferences.controls.movement = if SendMessageW(movement, CB_GETCURSEL, 0, 0) == 1 {
                    MovementKeys::ArrowsAndWasd
                } else {
                    MovementKeys::Arrows
                };
                preferences.controls.rotation = match SendMessageW(rotation, CB_GETCURSEL, 0, 0) {
                    1 => RotationModifier::Shift,
                    2 => RotationModifier::Alt,
                    _ => RotationModifier::Control,
                };
                match text(delay).parse::<u16>() {
                    Ok(delay) => {
                        preferences.controls.keyboard_delay_ms = delay;
                        state.apply(preferences);
                    }
                    Err(_) => {
                        state.feedback =
                            "Enter a repeat interval from 0 to 1000 milliseconds".into()
                    }
                }
            }
            let snapshot = (state.character_scope(), state.preferences.controls.clone());
            if held.as_ref() != Some(&snapshot) {
                let prefs = &snapshot.1;
                SendMessageW(
                    mouse,
                    CB_SETCURSEL,
                    usize::from(prefs.mouse == MousePreset::LeftHanded),
                    0,
                );
                SendMessageW(
                    movement,
                    CB_SETCURSEL,
                    usize::from(prefs.movement == MovementKeys::ArrowsAndWasd),
                    0,
                );
                SendMessageW(
                    rotation,
                    CB_SETCURSEL,
                    match prefs.rotation {
                        RotationModifier::Control => 0,
                        RotationModifier::Shift => 1,
                        RotationModifier::Alt => 2,
                    },
                    0,
                );
                SetWindowTextW(delay, wide(&prefs.keyboard_delay_ms.to_string()).as_ptr());
                held = Some(snapshot);
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
