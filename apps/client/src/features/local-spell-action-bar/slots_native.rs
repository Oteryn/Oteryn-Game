//! Clickable spell row attached to the gameplay window's native thread.
#![allow(unsafe_code)] // Child HWNDs are created and destroyed on the shell thread.
use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::*,
};

pub(super) struct SpellRow {
    buttons: Vec<HWND>,
    pressed: Vec<bool>,
    parent: HWND,
}
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
impl SpellRow {
    pub(super) fn new(parent: HWND) -> Option<Self> {
        let mut row = Self {
            buttons: Vec::new(),
            pressed: vec![false; oteryn_client_runtime::spell::ACTION_BAR_SLOTS],
            parent,
        };
        for _ in 0..oteryn_client_runtime::spell::ACTION_BAR_SLOTS {
            let button = unsafe {
                CreateWindowExW(
                    0,
                    wide("BUTTON").as_ptr(),
                    wide("").as_ptr(),
                    WS_CHILD | BS_MULTILINE as u32,
                    0,
                    0,
                    1,
                    1,
                    parent,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null(),
                )
            };
            if button.is_null() {
                return None;
            }
            row.buttons.push(button);
        }
        Some(row)
    }
    pub(super) fn update(&mut self, labels: Option<&[String]>) -> Vec<usize> {
        let mut clicked = Vec::new();
        unsafe {
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(self.parent, &mut rect);
            let width = (rect.right - 180).max(120) / self.buttons.len() as i32;
            for (slot, button) in self.buttons.iter().enumerate() {
                let label = labels.and_then(|labels| labels.get(slot));
                ShowWindow(*button, if label.is_some() { SW_SHOWNA } else { SW_HIDE });
                MoveWindow(
                    *button,
                    8 + slot as i32 * width,
                    (rect.bottom - 62).max(0),
                    width - 2,
                    54,
                    1,
                );
                if let Some(label) = label {
                    SetWindowTextW(*button, wide(label).as_ptr());
                }
                let down = label.is_some()
                    && SendMessageW(*button, BM_GETSTATE, 0, 0) & BST_PUSHED as isize != 0;
                if down && !self.pressed[slot] {
                    clicked.push(slot + 1);
                }
                self.pressed[slot] = down;
            }
        }
        clicked
    }
}
impl Drop for SpellRow {
    fn drop(&mut self) {
        for button in &self.buttons {
            unsafe {
                DestroyWindow(*button);
            }
        }
    }
}
