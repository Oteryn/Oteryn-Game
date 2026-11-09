use super::*;
pub(super) unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: (i32, i32, i32, i32),
    style: u32,
) -> HWND {
    // SAFETY: parent and temporary UTF-16 strings remain valid throughout this call.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | style,
            rect.0,
            rect.1,
            rect.2,
            rect.3,
            parent,
            id as *mut _,
            null_mut(),
            null(),
        )
    }
}
