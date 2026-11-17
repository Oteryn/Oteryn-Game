//! Win32 controls remain on their creating thread.
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub(super) fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(super) unsafe extern "system" fn procedure(
    hwnd: HWND,
    message: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    unsafe {
        if message == WM_COMMAND
            && (1..=8).contains(&(w & 0xffff))
            && (w >> 16) & 0xffff == BN_CLICKED as usize
        {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (w & 0xffff) as isize);
            return 0;
        }
        if message == WM_CLOSE {
            ShowWindow(hwnd, SW_HIDE);
            return 0;
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}
pub(super) unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    bounds: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            bounds[0],
            bounds[1],
            bounds[2],
            bounds[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
pub(super) unsafe fn read_text(hwnd: HWND) -> String {
    unsafe {
        let length = GetWindowTextLengthW(hwnd).clamp(0, 4096);
        let mut value = vec![0; length as usize + 1];
        let read = GetWindowTextW(hwnd, value.as_mut_ptr(), length + 1).max(0) as usize;
        String::from_utf16_lossy(&value[..read])
    }
}
