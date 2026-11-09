//! Window-thread control and bitmap construction.
use super::*;
pub(super) unsafe fn control(
    h: HWND,
    class: &str,
    label: &str,
    id: usize,
    r: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            r[0],
            r[1],
            r[2],
            r[3],
            h,
            id as HMENU,
            null_mut(),
            null(),
        )
    }
}
pub(super) unsafe fn bitmap(rgba: &[u8]) -> HBITMAP {
    unsafe {
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = 96;
        info.bmiHeader.biHeight = -96;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let mut bits = null_mut();
        let handle = CreateDIBSection(null_mut(), &info, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
        if !handle.is_null() {
            std::ptr::copy_nonoverlapping(rgba.as_ptr(), bits as *mut u8, rgba.len());
        }
        handle
    }
}

pub(super) unsafe extern "system" fn procedure(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let a = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Actions;
            if !a.is_null() && (*a).0.len() < 16 {
                (*a).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(h, msg, w, l)
    }
}
