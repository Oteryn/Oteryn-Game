//! Native console window helpers; buffers and HWNDs remain on the UI thread.
#![allow(unsafe_code)]
use super::*;
pub(super) unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_ACTIVATEAPP {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() {
                console::native_activation(&mut (*actions).keyboard_input, w != 0);
            }
            return 0;
        }
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let note = (w >> 16) & 0xffff;
            if !actions.is_null()
                && ((matches!(
                    id,
                    SEND | FILTER
                        | CHAT_MODE
                        | SAVE
                        | APPEND
                        | IGNORE
                        | ALLOW
                        | BLOCK
                        | CLEAR_LISTS
                ) && note == BN_CLICKED as usize
                    || id == TAB && note == LBN_SELCHANGE as usize)
                    || PrivateControls::accepts(id, note)
                    || channel_dialog::accepts(id, note)
                    || console_controls::accepts(id, note))
            {
                (*actions).pending.push(id);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

pub(super) fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
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
        let length = GetWindowTextLengthW(hwnd);
        let mut value = vec![0; usize::try_from(length).unwrap_or(0) + 1];
        let read = GetWindowTextW(hwnd, value.as_mut_ptr(), length + 1);
        String::from_utf16_lossy(&value[..usize::try_from(read).unwrap_or(0)])
    }
}
