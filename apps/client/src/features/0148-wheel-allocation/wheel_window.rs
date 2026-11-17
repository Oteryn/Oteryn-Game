//! Creating-thread graph hit and Win32 userdata/control helpers.
use super::*;
#[derive(Default)]
pub(super) struct Events {
    pub(super) close: bool,
    pub(super) actions: Vec<(usize, usize)>,
    pub(super) clicks: Vec<usize>,
}
pub(super) unsafe extern "system" fn procedure(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        let e = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Events;
        if msg == WM_CLOSE {
            if !e.is_null() {
                (*e).close = true;
            }
            return 0;
        }
        if msg == WM_COMMAND && !e.is_null() {
            if (*e).actions.len() < 32 {
                (*e).actions.push((w & 65535, (w >> 16) & 65535));
            }
            return 0;
        }
        DefWindowProcW(h, msg, w, l)
    }
}
pub(super) unsafe extern "system" fn graph_procedure(
    h: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    unsafe {
        if msg == WM_LBUTTONUP {
            let e = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Events;
            let x = (l as u16) as i16 as i32;
            let y = ((l >> 16) as u16) as i16 as i32;
            if !e.is_null()
                && (*e).clicks.len() < 32
                && let Some(i) = diagram::graph::hit(x, y)
            {
                (*e).clicks.push(i);
            }
            return 0;
        }
        DefWindowProcW(h, msg, w, l)
    }
}
pub(super) fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "�").encode_utf16().chain(Some(0)).collect()
}
pub(super) unsafe fn control(
    h: HWND,
    c: &str,
    t: &str,
    id: usize,
    b: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(c).as_ptr(),
            wide(t).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            b[0],
            b[1],
            b[2],
            b[3],
            h,
            id as _,
            null_mut(),
            null(),
        )
    }
}
