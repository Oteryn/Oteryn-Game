use super::*;
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
                            | VISUAL_ATELIER
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
