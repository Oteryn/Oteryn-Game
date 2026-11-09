//! Screenshot settings dialog. Child controls and callback state stay on their UI thread.
#![allow(
    unsafe_code,
    reason = "Win32 handles and callback storage stay on their creating thread"
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
        Controls::EM_SETLIMITTEXT, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*,
    },
};

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == 1801 && w >> 16 == BN_CLICKED as usize {
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
            wide("Screenshots").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
            350,
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
        let label = control(
            hwnd,
            "STATIC",
            "Save folder (absolute path; blank uses the default folder)",
            0,
            [16, 16, 660, 24],
            0,
        );
        let folder = control(
            hwnd,
            "EDIT",
            "",
            1802,
            [16, 48, 660, 28],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(folder, EM_SETLIMITTEXT, 1024, 0);
        let game = control(
            hwnd,
            "BUTTON",
            "Game map only (otherwise the entire client render surface)",
            1803,
            [16, 90, 660, 28],
            BS_AUTOCHECKBOX as u32,
        );
        let apply = control(hwnd, "BUTTON", "Apply", 1801, [516, 130, 160, 28], 0);
        let info = control(
            hwnd,
            "STATIC",
            "Print Screen saves a BMP capture. Only this client's rendered pixels are captured. Visible player information may appear in captures. Ctrl+Shift+C configures automatic capture events.",
            0,
            [16, 172, 660, 56],
            0,
        );
        let feedback = control(hwnd, "STATIC", "", 0, [16, 240, 660, 60], 0);
        if [label, folder, game, apply, info, feedback]
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
            if held
                .as_ref()
                .is_some_and(|old: &(Option<[u8; 16]>, _)| old.0 != state.character_scope())
            {
                *pending = false;
            }
            if std::mem::take(&mut *pending) {
                let mut prefs = state.preferences.clone();
                prefs.screenshots.directory = text(folder);
                prefs.screenshots.game_only = SendMessageW(game, BM_GETCHECK, 0, 0) == 1;
                if prefs.screenshots.valid() {
                    state.apply(prefs);
                } else {
                    state.feedback = "Enter an absolute folder path of at most 1024 bytes without control characters, or leave it blank.".into();
                }
            }
            let snapshot = (
                state.character_scope(),
                state.preferences.screenshots.clone(),
            );
            if held.as_ref() != Some(&snapshot) {
                SetWindowTextW(folder, wide(&snapshot.1.directory).as_ptr());
                SendMessageW(game, BM_SETCHECK, usize::from(snapshot.1.game_only), 0);
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
