//! Existing native target, userdata and control helpers.
use super::*;
#[derive(Clone, Copy)]
pub(super) struct Target(pub(super) NonZeroIsize);
impl HasWindowHandle for Target {
    fn window_handle(&self) -> Result<WindowHandle<'_>, HandleError> {
        Ok(unsafe {
            WindowHandle::borrow_raw(RawWindowHandle::Win32(Win32WindowHandle::new(self.0)))
        })
    }
}
impl HasDisplayHandle for Target {
    fn display_handle(&self) -> Result<DisplayHandle<'_>, HandleError> {
        Ok(unsafe {
            DisplayHandle::borrow_raw(RawDisplayHandle::Windows(WindowsDisplayHandle::new()))
        })
    }
}
#[derive(Default)]
pub(super) struct Events {
    pub(super) close: bool,
    pub(super) actions: Vec<(usize, usize)>,
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
pub(super) unsafe fn row(h: HWND, combo: bool) -> Option<usize> {
    usize::try_from(unsafe {
        SendMessageW(h, if combo { CB_GETCURSEL } else { LB_GETCURSEL }, 0, 0)
    })
    .ok()
}
