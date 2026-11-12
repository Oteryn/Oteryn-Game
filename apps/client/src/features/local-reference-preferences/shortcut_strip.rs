//! Docked shortcut controls owned by the game window's UI thread.
#![allow(
    unsafe_code,
    reason = "Child HWNDs and their boxed callback state share one owner thread"
)]
use oteryn_client::hud_error::{HudInitError, HudPart};
use oteryn_client::play::SHORTCUTS;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    UI::WindowsAndMessaging::*,
};
#[derive(Default)]
struct Actions(Vec<usize>);
pub(super) struct ShortcutStrip {
    root: HWND,
    parent: HWND,
    buttons: Vec<HWND>,
    actions: Box<Actions>,
    displayed: Vec<String>,
}
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w >> 16 == BN_CLICKED as usize {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 16 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
impl ShortcutStrip {
    pub(super) fn new(parent: HWND) -> Result<Self, HudInitError> {
        unsafe {
            let root = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("").as_ptr(),
                WS_CHILD,
                0,
                0,
                1,
                1,
                parent,
                null_mut(),
                null_mut(),
                null(),
            );
            if root.is_null() {
                return Err(super::last_error(HudPart::ShortcutRoot));
            }
            let mut strip = Self {
                root,
                parent,
                buttons: Vec::new(),
                actions: Box::default(),
                displayed: Vec::new(),
            };
            SetWindowLongPtrW(
                root,
                GWLP_USERDATA,
                (&mut *strip.actions as *mut Actions) as isize,
            );
            SetWindowLongPtrW(root, GWLP_WNDPROC, proc as *const () as isize);
            for (i, (_, label)) in SHORTCUTS.iter().enumerate() {
                let button = CreateWindowExW(
                    0,
                    wide("BUTTON").as_ptr(),
                    wide(label).as_ptr(),
                    WS_CHILD | WS_TABSTOP,
                    0,
                    0,
                    100,
                    26,
                    root,
                    (i + 1) as *mut _,
                    null_mut(),
                    null(),
                );
                if button.is_null() {
                    return Err(super::last_error(HudPart::ShortcutButton));
                }
                strip.buttons.push(button);
            }
            Ok(strip)
        }
    }
    pub(super) fn update(&mut self, ids: Option<&[String]>) -> Vec<String> {
        unsafe {
            let Some(ids) = ids else {
                ShowWindow(self.root, SW_HIDE);
                self.actions.0.clear();
                self.displayed.clear();
                return Vec::new();
            };
            // Discard clicks from the old layout if import or a character switch changed it.
            let actions = if self.displayed == ids {
                std::mem::take(&mut self.actions.0)
            } else {
                self.actions.0.clear();
                Vec::new()
            };
            self.displayed = ids.to_vec();
            let mut rect: RECT = std::mem::zeroed();
            if GetClientRect(self.parent, &mut rect) == 0 {
                return Vec::new();
            }
            let width = rect.right.max(1);
            let columns = (width / 100).max(1) as usize;
            let rows = ids.len().div_ceil(columns) as i32;
            MoveWindow(
                self.root,
                0,
                (rect.bottom - rows * 28).max(0),
                width,
                rows * 28,
                1,
            );
            for button in &self.buttons {
                ShowWindow(*button, SW_HIDE);
            }
            for (i, id) in ids.iter().enumerate() {
                if let Some(index) = SHORTCUTS.iter().position(|(known, _)| known == id) {
                    let button = self.buttons[index];
                    MoveWindow(
                        button,
                        (i % columns) as i32 * 100,
                        (i / columns) as i32 * 28,
                        100.min(width),
                        26,
                        1,
                    );
                    ShowWindow(button, SW_SHOW);
                }
            }
            ShowWindow(self.root, if ids.is_empty() { SW_HIDE } else { SW_SHOW });
            actions
                .into_iter()
                .filter_map(|index| index.checked_sub(1).and_then(|i| SHORTCUTS.get(i)))
                .filter(|(id, _)| ids.iter().any(|shown| shown == id))
                .map(|(id, _)| (*id).to_owned())
                .collect()
        }
    }
}
impl Drop for ShortcutStrip {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.root);
        }
    }
}
