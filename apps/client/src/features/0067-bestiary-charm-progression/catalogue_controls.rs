//! Window-thread control and bitmap construction.
use super::*;
use windows_sys::Win32::UI::Controls::{
    ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX, InitCommonControlsEx, PBM_SETPOS, PBM_SETRANGE32,
};

pub(super) struct EchoProgress {
    label: HWND,
    bar: HWND,
}

impl EchoProgress {
    pub(super) unsafe fn new(parent: HWND) -> Self {
        // SAFETY: common controls and both child handles are created on the window thread.
        unsafe {
            let common = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_PROGRESS_CLASS,
            };
            InitCommonControlsEx(&common);
            Self {
                label: control(parent, "STATIC", "", 97, [340, 105, 330, 24], 0),
                bar: control(parent, "msctls_progress32", "", 98, [340, 135, 330, 20], 0),
            }
        }
    }

    pub(super) unsafe fn refresh(&self, progress: Option<(u8, u8)>) {
        // SAFETY: these child controls remain owned by the same live catalogue window.
        unsafe {
            let visible = if progress.is_some() { SW_SHOW } else { SW_HIDE };
            ShowWindow(self.label, visible);
            ShowWindow(self.bar, visible);
            if let Some((current, goal)) = progress {
                SetWindowTextW(
                    self.label,
                    wide(&format!("Echo Warden: {current}/{goal} first defeat")).as_ptr(),
                );
                SendMessageW(self.bar, PBM_SETRANGE32, 0, isize::from(goal));
                SendMessageW(self.bar, PBM_SETPOS, usize::from(current), 0);
            }
        }
    }
}

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
