//! Unchanged controls extracted from the Wheel window to keep its module bounded.
use super::*;
#[derive(Default)]
pub(super) struct Actions(pub(super) Vec<usize>);

pub(super) unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    // SAFETY: this thread installs userdata from a live Actions box until window destruction.
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let notification = (w >> 16) & 0xffff;
            if !actions.is_null()
                && (*actions).0.len() < 32
                && ((matches!(id, SLOTS | GEMS | MODIFIERS)
                    && notification == LBN_SELCHANGE as usize)
                    || (matches!(id, QUALITY | DOMAIN) && notification == CBN_SELCHANGE as usize)
                    || (matches!(
                        id,
                        APPLY
                            | SAVE
                            | RESET
                            | REFRESH
                            | SOCKET
                            | UNSOCKET
                            | REVEAL
                            | LOCK
                            | SWITCH
                            | DISMANTLE
                            | GRADE
                            | RETRY
                            | PRESETS
                            | WHEEL_DIAGRAM
                    ) && notification == BN_CLICKED as usize))
            {
                (*actions).0.push(id);
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

pub(super) fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).wheel.open = false;
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
    // SAFETY: parent is live here and all UTF-16 strings survive the synchronous call.
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
