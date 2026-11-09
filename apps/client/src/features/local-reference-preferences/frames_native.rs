//! Frame-limit editor; the game window continues while this settings child is open.
#![allow(
    unsafe_code,
    reason = "Win32 controls and callback data remain on this dialog thread"
)]
use super::{Settings, Shared, control, lock, text, wide};
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
impl Settings {
    pub(in crate::windows_shell) fn frame_preferences(
        &self,
    ) -> oteryn_client::play::FramePreferences {
        let state = lock(&self.shared);
        let mut frames = state.preferences.frames;
        if frames == oteryn_client::play::FramePreferences::default() {
            frames.foreground = state.preferences.foreground_fps;
            frames.background = state.preferences.background_fps;
        }
        frames
    }
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == 651 && w >> 16 == BN_CLICKED as usize {
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
            wide("Frame rate and indicators").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            660,
            500,
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
        let labels = control(
            hwnd,
            "STATIC",
            "Foreground limit (1–500 FPS)\r\n\r\nBackground limit (1–500 FPS)",
            0,
            [16, 20, 340, 85],
            0,
        );
        let foreground = control(
            hwnd,
            "EDIT",
            "",
            648,
            [380, 16, 220, 28],
            WS_BORDER | ES_NUMBER as u32,
        );
        let background = control(
            hwnd,
            "EDIT",
            "",
            649,
            [380, 60, 220, 28],
            WS_BORDER | ES_NUMBER as u32,
        );
        for field in [foreground, background] {
            SendMessageW(field, EM_SETLIMITTEXT, 3, 0);
        }
        let fps = control(
            hwnd,
            "BUTTON",
            "Show measured FPS",
            652,
            [16, 115, 280, 28],
            BS_AUTOCHECKBOX as u32,
        );
        let latency = control(
            hwnd,
            "BUTTON",
            "Show server RTT estimate",
            653,
            [315, 115, 300, 28],
            BS_AUTOCHECKBOX as u32,
        );
        let apply = control(hwnd, "BUTTON", tr!("Apply"), 651, [450, 160, 150, 30], 0);
        let info = control(
            hwnd,
            "STATIC",
            "Limits apply when the game window gains or loses focus. VSync can lower the achieved rate.\r\nThe title shows completed frames and the server's smoothed round-trip estimate; unavailable data is labelled.",
            0,
            [16, 202, 600, 55],
            0,
        );
        let feedback = control(hwnd, "STATIC", "", 0, [16, 265, 600, 50], 0);
        if [
            labels, foreground, background, fps, latency, apply, info, feedback,
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
            let snapshot = (state.character_scope(), state.preferences.frames);
            if held
                .as_ref()
                .is_some_and(|old: &(Option<[u8; 16]>, _)| old.0 != snapshot.0)
            {
                *pending = false;
            }
            if std::mem::take(&mut *pending) {
                match (
                    text(foreground).parse::<u16>(),
                    text(background).parse::<u16>(),
                ) {
                    (Ok(foreground), Ok(background)) => {
                        let mut prefs = state.preferences.clone();
                        prefs.frames = oteryn_client::play::FramePreferences {
                            foreground,
                            background,
                            show_fps: SendMessageW(fps, BM_GETCHECK, 0, 0) == BST_CHECKED as isize,
                            show_latency: SendMessageW(latency, BM_GETCHECK, 0, 0)
                                == BST_CHECKED as isize,
                        };
                        if prefs.frames.valid() {
                            prefs.foreground_fps = prefs.frames.foreground;
                            prefs.background_fps = prefs.frames.background;
                            state.apply(prefs);
                        } else {
                            state.feedback = "Choose frame limits from 1 to 500".into();
                        }
                    }
                    _ => state.feedback = "Enter whole-number frame limits from 1 to 500".into(),
                }
            }
            let snapshot = (state.character_scope(), state.preferences.frames);
            if held.as_ref() != Some(&snapshot) {
                SetWindowTextW(
                    foreground,
                    wide(&snapshot.1.foreground.to_string()).as_ptr(),
                );
                SetWindowTextW(
                    background,
                    wide(&snapshot.1.background.to_string()).as_ptr(),
                );
                SendMessageW(fps, BM_SETCHECK, usize::from(snapshot.1.show_fps), 0);
                SendMessageW(
                    latency,
                    BM_SETCHECK,
                    usize::from(snapshot.1.show_latency),
                    0,
                );
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
