//! Native controls for the achievement browser.
use super::*;
pub(super) unsafe fn ctl(h: HWND, c: &str, t: &str, id: usize, r: [i32; 4], s: u32) -> HWND {
    // SAFETY: the caller owns the live parent; Win32 copies the temporary strings.
    unsafe {
        CreateWindowExW(
            0,
            wide(c).as_ptr(),
            wide(t).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | s,
            r[0],
            r[1],
            r[2],
            r[3],
            h,
            id as _,
            null_mut(),
            null(),
        )
    }
}
pub(super) unsafe fn text(h: HWND) -> String {
    // SAFETY: this thread owns the live edit control and the bounded output buffer.
    unsafe {
        let n = GetWindowTextLengthW(h).max(0);
        let mut b = vec![0; n as usize + 1];
        let n = GetWindowTextW(h, b.as_mut_ptr(), n + 1).max(0) as usize;
        String::from_utf16_lossy(&b[..n])
    }
}
