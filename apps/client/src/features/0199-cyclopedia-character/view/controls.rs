//! Shared helpers for this Character window’s thread-owned controls.
#![allow(unsafe_code)]
use std::ptr::{null, null_mut};
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};
pub(super) fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(super) fn control(
    parent: HWND,
    class: &str,
    label: &str,
    id: usize,
    rect: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            parent,
            id as *mut _,
            null_mut(),
            null(),
        )
    }
}
pub(super) fn read_text(hwnd: HWND) -> String {
    // SAFETY: hwnd is an edit owned by this thread; the buffer includes its terminator.
    unsafe {
        let len = GetWindowTextLengthW(hwnd).max(0);
        let mut buffer = vec![0; len as usize + 1];
        let read = GetWindowTextW(hwnd, buffer.as_mut_ptr(), len + 1).max(0);
        String::from_utf16_lossy(&buffer[..read as usize])
    }
}

/// Table handles share one initialization for history, statistics and profile rows.
pub(super) fn tables(hwnd: HWND) -> [HWND; 4] {
    unsafe {
        let heading = control(hwnd, "STATIC", "", 401, [10, 355, 690, 22], 0);
        let history = control(
            hwnd,
            "LISTBOX",
            "",
            402,
            [10, 380, 690, 155],
            WS_BORDER | WS_VSCROLL | WS_HSCROLL | WS_TABSTOP | LBS_USETABSTOPS as u32,
        );
        let detail = control(
            hwnd,
            "EDIT",
            "",
            403,
            [10, 545, 690, 50],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let tabs: [i32; 2] = [100, 180];
        SendMessageW(history, LB_SETTABSTOPS, tabs.len(), tabs.as_ptr() as isize);
        SendMessageW(history, LB_SETHORIZONTALEXTENT, 1800, 0);
        let status_label = control(hwnd, "STATIC", "", 472, [10, 192, 690, 34], 0);
        [heading, history, detail, status_label]
    }
}
