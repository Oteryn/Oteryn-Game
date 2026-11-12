//! One native worker per connection; weak ownership closes both surfaces on disconnect.
#![allow(
    unsafe_code,
    reason = "Win32 handles and buffers stay on their creating thread"
)]
use super::super::state::{Owned, State};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Weak},
    time::{Duration, Instant},
};
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(in crate::play) fn open(owned: Weak<Owned>) -> bool {
    std::thread::Builder::new()
        .name("game-event-notices".into())
        .spawn(move || run(owned))
        .is_ok()
}
fn window(title: &str, y: i32) -> (HWND, HWND) {
    unsafe {
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            wide("STATIC").as_ptr(),
            wide(title).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION,
            20,
            y,
            660,
            160,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        let body = CreateWindowExW(
            0,
            wide("EDIT").as_ptr(),
            null(),
            WS_CHILD | WS_VISIBLE | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
            8,
            8,
            630,
            112,
            hwnd,
            null_mut(),
            null_mut(),
            null(),
        );
        (hwnd, body)
    }
}
fn run(owned: Weak<Owned>) {
    unsafe {
        let surfaces = [
            window(
                "Oteryn — Game notices · Ctrl+Alt+Shift+F10: toggle · F12: dismiss",
                80,
            ),
            window(
                "Oteryn — Event alert · Ctrl+Alt+Shift+F11: toggle · F12: dismiss",
                260,
            ),
        ];
        if surfaces
            .iter()
            .any(|(window, body)| window.is_null() || body.is_null())
        {
            for (window, _) in surfaces {
                if !window.is_null() {
                    DestroyWindow(window);
                }
            }
            return;
        }
        let mut previous = [String::new(), String::new()];
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            let Some(shared) = owned.upgrade() else { break };
            let state = State(Arc::clone(&shared));
            for (index, (window, body)) in surfaces.iter().enumerate() {
                let text = state.text_at(index == 1, Instant::now());
                if text != previous[index] {
                    SetWindowTextW(*body, wide(&text).as_ptr());
                    ShowWindow(
                        *window,
                        if text.is_empty() {
                            SW_HIDE
                        } else {
                            SW_SHOWNOACTIVATE
                        },
                    );
                    previous[index] = text;
                }
            }
            drop(state);
            drop(shared);
            std::thread::sleep(Duration::from_millis(50));
        }
        for (window, _) in surfaces {
            DestroyWindow(window);
        }
    }
}
