//! Child control on the shell thread; its parent window owns the native lifetime.
// SAFETY: only this module calls Win32 for a child of winit's live window, on its thread.
#![allow(unsafe_code)]

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DestroyWindow, ES_MULTILINE, ES_READONLY, MoveWindow, SW_HIDE, SW_SHOW,
    SendMessageW, SetWindowTextW, ShowWindow, WS_CHILD, WS_VISIBLE, WS_VSCROLL,
};
use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
use winit::window::Window;

use windows_sys::Win32::UI::Controls::{EM_GETFIRSTVISIBLELINE, EM_LINESCROLL};
pub(super) struct Hud {
    hwnd: HWND,
    text: std::cell::RefCell<String>,
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

impl Hud {
    pub(super) fn new(window: &Window) -> Option<Self> {
        let RawWindowHandle::Win32(handle) = window.window_handle().ok()?.as_raw() else {
            return None;
        };
        // The read-only multiline HUD keeps long condition lists scrollable.
        // Win32 copies both strings during creation.
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                wide("EDIT").as_ptr(),
                wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
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
        (!hwnd.is_null()).then_some(Self {
            hwnd,
            text: Default::default(),
        })
    }

    pub(super) fn update(&self, text: Option<&str>, bounds: [u16; 4]) {
        unsafe {
            MoveWindow(
                self.hwnd,
                i32::from(bounds[0]),
                i32::from(bounds[1]),
                i32::from(bounds[2]),
                i32::from(bounds[3]),
                1,
            );
            let visible = text.is_some();
            let text = text.unwrap_or("");
            let mut last = self.text.borrow_mut();
            if last.as_str() != text {
                let first = SendMessageW(self.hwnd, EM_GETFIRSTVISIBLELINE, 0, 0);
                SetWindowTextW(self.hwnd, wide(text).as_ptr());
                SendMessageW(self.hwnd, EM_LINESCROLL, 0, first);
                text.clone_into(&mut last);
            }
            ShowWindow(self.hwnd, if visible { SW_SHOW } else { SW_HIDE });
        }
    }
}

impl Drop for Hud {
    fn drop(&mut self) {
        // DestroyWindow is also safe if the parent already destroyed its child.
        unsafe {
            DestroyWindow(self.hwnd);
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
                hud.update(Some(&status), play.view.preferences().scaled_hud_bounds());
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
