//! Click-through child HUD, painted on the game window's UI thread with Oteryn geometry.
#![allow(
    unsafe_code,
    reason = "Child HWND, callback state and GDI objects remain on the shell thread"
)]
use oteryn_client::hud_error::{HudInitError, HudPart};
use oteryn_client::play::ArcLayout;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    Graphics::Gdi::*,
    UI::WindowsAndMessaging::*,
};
#[derive(Default)]
struct PaintData {
    layout: Option<ArcLayout>,
}
pub(super) struct ArcHud {
    hwnd: HWND,
    data: Box<PaintData>,
}
unsafe fn path(dc: HDC, points: &[[i32; 2]], color: u32) {
    unsafe {
        if points.len() < 2 {
            return;
        }
        let pen = CreatePen(PS_SOLID, 6, color);
        if pen.is_null() {
            return;
        }
        let old = SelectObject(dc, pen);
        MoveToEx(dc, points[0][0], points[0][1], null_mut());
        for p in &points[1..] {
            LineTo(dc, p[0], p[1]);
        }
        SelectObject(dc, old);
        DeleteObject(pen);
    }
}
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_NCHITTEST {
            return HTTRANSPARENT as LRESULT;
        }
        if msg == WM_PAINT {
            let mut paint: PAINTSTRUCT = std::mem::zeroed();
            let dc = BeginPaint(hwnd, &mut paint);
            FillRect(dc, &paint.rcPaint, GetStockObject(BLACK_BRUSH) as HBRUSH);
            let data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const PaintData;
            if !data.is_null()
                && let Some(layout) = &(*data).layout
            {
                SetBkMode(dc, TRANSPARENT as i32);
                SetTextColor(dc, 0x00ffffff);
                for (index, icon) in layout.condition_icons.iter().enumerate() {
                    let top = index as i32 * 18;
                    let rect = windows_sys::Win32::Foundation::RECT {
                        left: 0,
                        top,
                        right: 26,
                        bottom: top + 18,
                    };
                    let brush = CreateSolidBrush(0x00202020);
                    FillRect(dc, &rect, brush);
                    DeleteObject(brush);
                    let mut rect = rect;
                    let text: Vec<u16> = icon.encode_utf16().collect();
                    DrawTextW(
                        dc,
                        text.as_ptr(),
                        text.len() as i32,
                        &mut rect,
                        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                    );
                }
                for stroke in &layout.strokes {
                    path(dc, &stroke.points, 0x00484848);
                    path(
                        dc,
                        &stroke.points[..=stroke.filled_segments],
                        if stroke.health {
                            0x003030e0
                        } else {
                            0x00e08030
                        },
                    );
                }
            }
            EndPaint(hwnd, &paint);
            return 0;
        }
        if msg == WM_ERASEBKGND {
            return 1;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
impl ArcHud {
    pub(super) fn new(parent: HWND) -> Result<Self, HudInitError> {
        unsafe {
            let class: Vec<u16> = "STATIC\0".encode_utf16().collect();
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_NOACTIVATE,
                class.as_ptr(),
                null(),
                WS_CHILD | WS_DISABLED,
                0,
                0,
                1,
                1,
                parent,
                null_mut(),
                null_mut(),
                null(),
            );
            if hwnd.is_null() {
                return Err(super::last_error(HudPart::ArcWindow));
            }
            let mut data = Box::<PaintData>::default();
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *data as *mut PaintData) as isize);
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
            if SetLayeredWindowAttributes(hwnd, 0, 255, LWA_COLORKEY | LWA_ALPHA) == 0 {
                let error = super::last_error(HudPart::ArcLayer);
                DestroyWindow(hwnd);
                return Err(error);
            }
            Ok(Self { hwnd, data })
        }
    }
    pub(super) fn update(&mut self, layout: Option<ArcLayout>) {
        if self.data.layout == layout {
            return;
        }
        self.data.layout = layout;
        unsafe {
            if let Some(layout) = &self.data.layout {
                SetWindowPos(
                    self.hwnd,
                    HWND_TOP,
                    layout.origin[0],
                    layout.origin[1],
                    layout.width,
                    layout.height,
                    SWP_NOACTIVATE | SWP_NOSENDCHANGING,
                );
                SetLayeredWindowAttributes(self.hwnd, 0, layout.alpha, LWA_COLORKEY | LWA_ALPHA);
                ShowWindow(self.hwnd, SW_SHOWNA);
                InvalidateRect(self.hwnd, null(), 0);
            } else {
                ShowWindow(self.hwnd, SW_HIDE);
            }
        }
    }
}
impl Drop for ArcHud {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.hwnd) != 0
                && GetWindowLongPtrW(self.hwnd, GWLP_USERDATA)
                    == (&*self.data as *const PaintData) as isize
            {
                DestroyWindow(self.hwnd);
            }
        }
    }
}
