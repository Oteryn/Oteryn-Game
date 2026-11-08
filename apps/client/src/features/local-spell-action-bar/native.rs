//! Child control on the shell thread; its parent window owns the native lifetime.
// SAFETY: only this module calls Win32 for a child of winit's live window, on its thread.
#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, MoveWindow, SW_HIDE, SW_SHOW, SetWindowTextW, ShowWindow,
    WS_CHILD, WS_VISIBLE,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

pub(super) struct Hud(HWND);

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

impl Hud {
    pub(super) fn new(window: &Window) -> Option<Self> {
        let RawWindowHandle::Win32(handle) = window.window_handle().ok()?.as_raw() else {
            return None;
        };
        // A STATIC control displays the spell status, HP/MP bars and numbers without taking
        // keyboard focus from gameplay. Win32 copies both strings during creation.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE,
                8,
                8,
                900,
                40,
                handle.hwnd.get() as HWND,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        (!hwnd.is_null()).then_some(Self(hwnd))
    }

    pub(super) fn update(&self, text: Option<&str>, bounds: [u16; 4]) {
        unsafe {
            MoveWindow(
                self.0,
                i32::from(bounds[0]),
                i32::from(bounds[1]),
                i32::from(bounds[2]),
                i32::from(bounds[3]),
                1,
            );
            SetWindowTextW(self.0, wide(text.unwrap_or("")).as_ptr());
            ShowWindow(self.0, if text.is_some() { SW_SHOW } else { SW_HIDE });
        }
    }
}

impl Drop for Hud {
    fn drop(&mut self) {
        // DestroyWindow is also safe if the parent already destroyed its child.
        unsafe {
            DestroyWindow(self.0);
        }
    }
}

impl super::Application {
    pub(super) fn update_hud(&mut self) {
        if let (Some(play), Some(window)) = (&mut self.play, &self.window) {
            let audio = self.audio.status(&mut play.view);
            let movement = play.link.item_movement_status();
            let status = format!(
                "{movement} — {} — {audio} — Ctrl+arrows: Turn; Ctrl+F: Follow; Esc: Stop — {}",
                play.view.hud_status(),
                play.link.movement_feedback().unwrap_or("")
            );
            if let Some(hud) = &self.hud {
                hud.update(
                    play.view
                        .preferences()
                        .status_panel_visible
                        .then_some(status.as_str()),
                    play.view.preferences().scaled_hud_bounds(),
                );
            }
            if status != play.status {
                window.set_title(&format!(
                    "Oteryn — {status} — Ctrl+G: Guild — Ctrl+Shift+A: achievements"
                ));
                play.status = status;
            }
        }
        if self.play.is_none()
            && let Some(hud) = &self.hud
        {
            hud.update(None, [8, 8, 900, 40]);
        }
    }
}
