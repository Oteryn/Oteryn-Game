use super::*;
pub(super) unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    // SAFETY: userdata points to Actions on this thread until HWND destruction.
    unsafe {
        if msg == WM_SYSKEYDOWN && w == 0x42 && (l & (1 << 29)) != 0 {
            DestroyWindow(hwnd);
            return 0;
        }
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let notification = (w >> 16) & 0xffff;
            if !actions.is_null()
                && (*actions).0.len() < 16
                && (((200..=203).contains(&id) && notification == LBN_SELCHANGE as usize)
                    || (id == 300 && notification == CBN_SELCHANGE as usize)
                    || (((101..=108).contains(&id) || (307..=309).contains(&id))
                        && notification == BN_CLICKED as usize))
            {
                (*actions).0.push(id);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
