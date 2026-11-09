//! Arc controls reuse the validated character preference store.
#![allow(
    unsafe_code,
    reason = "Win32 controls and callback state stay on this dialog thread"
)]
use super::{Shared, control, lock, text, wide};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Controls::{BST_CHECKED, EM_SETLIMITTEXT},
        Input::KeyboardAndMouse::EnableWindow,
        WindowsAndMessaging::*,
    },
};
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == 956 && w >> 16 == BN_CLICKED as usize {
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
            wide("Health and mana arcs").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            660,
            400,
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
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let health = control(
            hwnd,
            "BUTTON",
            "Health arc",
            951,
            [16, 18, 280, 28],
            BS_AUTOCHECKBOX as u32,
        );
        let mana = control(
            hwnd,
            "BUTTON",
            "Mana arc",
            952,
            [330, 18, 280, 28],
            BS_AUTOCHECKBOX as u32,
        );
        let labels = control(
            hwnd,
            "STATIC",
            "Horizontal distance (0–120 px)\r\n\r\nArc radius (20–120 px)\r\n\r\nOpacity (0–100%)",
            0,
            [16, 70, 380, 150],
            0,
        );
        let distance = control(
            hwnd,
            "EDIT",
            "",
            953,
            [430, 66, 180, 28],
            WS_BORDER | ES_NUMBER as u32,
        );
        let size = control(
            hwnd,
            "EDIT",
            "",
            954,
            [430, 110, 180, 28],
            WS_BORDER | ES_NUMBER as u32,
        );
        let opacity = control(
            hwnd,
            "EDIT",
            "",
            955,
            [430, 154, 180, 28],
            WS_BORDER | ES_NUMBER as u32,
        );
        for field in [distance, size, opacity] {
            SendMessageW(field, EM_SETLIMITTEXT, 3, 0);
        }
        let apply = control(hwnd, "BUTTON", tr!("Apply"), 956, [430, 208, 180, 30], 0);
        let help = control(
            hwnd,
            "STATIC",
            "The arcs follow the player, camera and world zoom. Gray is the track; colored segments use server health/mana. Unknown vitals hide the arcs. The overlay passes mouse input through to the game.",
            0,
            [16, 253, 595, 52],
            0,
        );
        let feedback = control(hwnd, "STATIC", "", 0, [16, 315, 595, 50], 0);
        if [
            health, mana, labels, distance, size, opacity, apply, help, feedback,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            EnableWindow(parent, 1);
            return;
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
            let snapshot = (state.character_scope(), state.preferences.arcs);
            if held
                .as_ref()
                .is_some_and(|old: &(Option<[u8; 16]>, _)| old.0 != snapshot.0)
            {
                *pending = false;
            }
            if std::mem::take(&mut *pending) {
                match (
                    text(distance).parse::<u16>(),
                    text(size).parse::<u16>(),
                    text(opacity).parse::<u8>(),
                ) {
                    (Ok(distance), Ok(size), Ok(opacity)) => {
                        let mut prefs = state.preferences.clone();
                        prefs.arcs = oteryn_client::play::ArcPreferences {
                            health: SendMessageW(health, BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
                            mana: SendMessageW(mana, BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
                            distance,
                            size,
                            opacity,
                        };
                        if prefs.arcs.valid() {
                            state.apply(prefs);
                        } else {
                            state.feedback = "Choose values in the displayed ranges".into();
                        }
                    }
                    _ => state.feedback = "Enter whole numbers in the displayed ranges".into(),
                }
            }
            let snapshot = (state.character_scope(), state.preferences.arcs);
            if held.as_ref() != Some(&snapshot) {
                for (field, value) in [
                    (distance, snapshot.1.distance),
                    (size, snapshot.1.size),
                    (opacity, u16::from(snapshot.1.opacity)),
                ] {
                    SetWindowTextW(field, wide(&value.to_string()).as_ptr());
                }
                SendMessageW(health, BM_SETCHECK, usize::from(snapshot.1.health), 0);
                SendMessageW(mana, BM_SETCHECK, usize::from(snapshot.1.mana), 0);
                held = Some(snapshot);
            }
            SetWindowTextW(
                feedback,
                wide(&format!("{}\r\n{}", state.scope_label(), state.feedback)).as_ptr(),
            );
            drop(state);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        EnableWindow(parent, 1);
        SetForegroundWindow(parent);
    }
}
