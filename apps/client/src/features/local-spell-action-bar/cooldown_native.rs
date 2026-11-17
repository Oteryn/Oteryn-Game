//! Console-owned cooldown icons; all HWND and GDI access stays on the dialog thread.
#![allow(
    unsafe_code,
    reason = "Win32 child and its paint data stay on the console thread"
)]
use oteryn_client_ui::spell_action_bar::CooldownIcon;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::*,
    UI::{
        Controls::{SetScrollPos, SetScrollRange},
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};

pub(super) struct CooldownBar {
    hwnd: HWND,
    state: Box<PaintState>,
}
struct PaintState {
    icons: Vec<CooldownIcon>,
}
unsafe extern "system" fn paint(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    unsafe {
        if msg == WM_HSCROLL {
            let state = &*(data as *const PaintState);
            let current = GetScrollPos(hwnd, SB_HORZ);
            let max = state.icons.len().saturating_sub(26) as i32;
            let next = match w & 0xffff {
                0 => current - 1,
                1 => current + 1,
                2 => current - 26,
                3 => current + 26,
                4 | 5 => (w >> 16) as i32,
                6 => 0,
                7 => max,
                _ => current,
            }
            .clamp(0, max);
            SetScrollPos(hwnd, SB_HORZ, next, 1);
            InvalidateRect(hwnd, null(), 1);
            return 0;
        }
        if msg != WM_PAINT {
            return DefSubclassProc(hwnd, msg, w, l);
        }
        let icons = &(*(data as *const PaintState)).icons;
        let mut ps: PAINTSTRUCT = std::mem::zeroed();
        let dc = BeginPaint(hwnd, &mut ps);
        let mut bounds: RECT = std::mem::zeroed();
        GetClientRect(hwnd, &mut bounds);
        FillRect(dc, &bounds, GetStockObject(BLACK_BRUSH) as _);
        SetBkMode(dc, TRANSPARENT as i32);
        for (index, icon) in icons.iter().enumerate() {
            let x = (index as i32 - GetScrollPos(hwnd, SB_HORZ)) * 40;
            let rect = RECT {
                left: x,
                top: 0,
                right: x + 38,
                bottom: 30,
            };
            let brush = CreateSolidBrush(if icon.group && icon.remaining_ms == 0 {
                0x404040
            } else {
                0x805030
            });
            FillRect(dc, &rect, brush);
            DeleteObject(brush as _);
            SetTextColor(
                dc,
                if icon.group && icon.remaining_ms == 0 {
                    0x909090
                } else {
                    0xffffff
                },
            );
            let label = icon
                .label
                .chars()
                .take(5)
                .collect::<String>()
                .encode_utf16()
                .collect::<Vec<_>>();
            TextOutW(dc, x + 2, 5, label.as_ptr(), label.len() as i32);
            let line = RECT {
                left: x,
                top: 27,
                right: x + i32::from(icon.overlay_per_mille) * 38 / 1000,
                bottom: 30,
            };
            FillRect(dc, &line, GetStockObject(WHITE_BRUSH) as _);
        }
        EndPaint(hwnd, &ps);
        0
    }
}
impl CooldownBar {
    pub(super) unsafe fn new(parent: HWND) -> Self {
        unsafe {
            let class: Vec<_> = "STATIC\0".encode_utf16().collect();
            let hwnd = CreateWindowExW(
                0,
                class.as_ptr(),
                null(),
                WS_CHILD | WS_HSCROLL,
                10,
                6,
                1070,
                48,
                parent,
                null_mut(),
                null_mut(),
                null(),
            );
            let state = Box::new(PaintState { icons: Vec::new() });
            if !hwnd.is_null() {
                SetWindowSubclass(
                    hwnd,
                    Some(paint),
                    1,
                    (&*state as *const PaintState) as usize,
                );
            }
            Self { hwnd, state }
        }
    }
    pub(super) unsafe fn update(&mut self, icons: Vec<CooldownIcon>, enabled: bool) {
        unsafe {
            if self.hwnd.is_null() {
                return;
            }
            if self.state.icons != icons {
                self.state.icons = icons;
                SetScrollRange(
                    self.hwnd,
                    SB_HORZ,
                    0,
                    self.state.icons.len().saturating_sub(26) as i32,
                    1,
                );
                InvalidateRect(self.hwnd, null(), 1);
            }
            ShowWindow(self.hwnd, if enabled { SW_SHOWNA } else { SW_HIDE });
        }
    }
}
impl Drop for CooldownBar {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.hwnd) != 0 {
                RemoveWindowSubclass(self.hwnd, Some(paint), 1);
                DestroyWindow(self.hwnd);
            }
        }
    }
}
