//! Native text child on the Watch window's own UI thread.
#![allow(
    unsafe_code,
    reason = "Win32 child control is created, updated and destroyed on its parent UI thread"
)]
use windows_sys::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        CreateWindowExW, DestroyWindow, SetWindowTextW, WS_CHILD, WS_VISIBLE,
    },
};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};
pub(super) struct Hud(HWND);
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
impl Hud {
    pub fn new(window: &Window) -> Option<Self> {
        let RawWindowHandle::Win32(handle) = window.window_handle().ok()?.as_raw() else {
            return None;
        };
        let hwnd = unsafe {
            CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE,
                8,
                455,
                560,
                235,
                handle.hwnd.get() as HWND,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        (!hwnd.is_null()).then_some(Self(hwnd))
    }
    pub fn update(&self, text: &str) {
        unsafe {
            SetWindowTextW(self.0, wide(text).as_ptr());
        }
    }
}
impl Drop for Hud {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.0);
        }
    }
}
