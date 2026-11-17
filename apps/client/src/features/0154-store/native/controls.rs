//! Win32 controls owned by the Store window thread.
#![allow(
    unsafe_code,
    reason = "All controls and GDI contexts stay on their creating thread"
)]
use std::ptr::{null, null_mut};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::WindowsAndMessaging::*};
pub(super) fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(super) unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    r: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            r[0],
            r[1],
            r[2],
            r[3],
            parent,
            id as HMENU,
            null_mut(),
            null(),
        )
    }
}
pub(super) unsafe fn text(hwnd: HWND) -> String {
    unsafe {
        let mut b = [0u16; 257];
        let n = GetWindowTextW(hwnd, b.as_mut_ptr(), 257);
        String::from_utf16_lossy(&b[..n.max(0) as usize])
    }
}
pub(super) unsafe fn paint(hwnd: HWND, pixels: &[u8]) {
    unsafe {
        let dc = GetDC(hwnd);
        if dc.is_null() {
            return;
        }
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = 128;
        info.bmiHeader.biHeight = -128;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        StretchDIBits(
            dc,
            38,
            0,
            180,
            180,
            0,
            0,
            128,
            128,
            pixels.as_ptr().cast(),
            &info,
            DIB_RGB_COLORS,
            SRCCOPY,
        );
        ReleaseDC(hwnd, dc);
    }
}
