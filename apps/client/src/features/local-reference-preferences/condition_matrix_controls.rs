//! Each condition row has independent status-bar and HUD checkboxes.
use super::*;
use oteryn_client::play::ReferencePreferences;
use windows_sys::Win32::UI::Controls::{SetScrollPos, SetScrollRange};

pub(super) struct ConditionMatrix {
    window: HWND,
    rows: Vec<[HWND; 3]>,
}
unsafe extern "system" fn matrix_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            return SendMessageW(GetParent(hwnd), msg, w, l);
        }
        if msg == WM_VSCROLL || msg == WM_MOUSEWHEEL {
            let previous = GetScrollPos(hwnd, SB_VERT);
            let target = if msg == WM_MOUSEWHEEL {
                previous - i32::from((w >> 16) as i16) / 120 * 3
            } else {
                match (w & 0xffff) as i32 {
                    SB_LINEUP => previous - 1,
                    SB_LINEDOWN => previous + 1,
                    SB_PAGEUP => previous - 5,
                    SB_PAGEDOWN => previous + 5,
                    SB_THUMBTRACK | SB_THUMBPOSITION => (w >> 16) as i32,
                    _ => previous,
                }
            };
            SetScrollPos(hwnd, SB_VERT, target, 1);
            let current = GetScrollPos(hwnd, SB_VERT);
            ScrollWindowEx(
                hwnd,
                0,
                (previous - current) * 24,
                null(),
                null(),
                null_mut(),
                null_mut(),
                SW_SCROLLCHILDREN | SW_INVALIDATE | SW_ERASE,
            );
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
impl ConditionMatrix {
    pub(super) unsafe fn new(parent: HWND) -> Option<Self> {
        unsafe {
            let window = control(
                parent,
                "STATIC",
                "",
                0,
                [10, 380, 710, 150],
                WS_BORDER | WS_VSCROLL | WS_CLIPCHILDREN,
            );
            if window.is_null() {
                return None;
            }
            SetWindowLongPtrW(window, GWLP_WNDPROC, matrix_proc as *const () as isize);
            let rows: Vec<_> = ReferencePreferences::special_condition_keys()
                .iter()
                .enumerate()
                .map(|(index, key)| {
                    let y = index as i32 * 24;
                    [
                        control(window, "STATIC", key, 0, [8, y, 280, 24], 0),
                        control(
                            window,
                            "BUTTON",
                            "Status bar",
                            300 + index * 2,
                            [300, y, 170, 24],
                            BS_AUTOCHECKBOX as u32,
                        ),
                        control(
                            window,
                            "BUTTON",
                            "HUD",
                            301 + index * 2,
                            [490, y, 150, 24],
                            BS_AUTOCHECKBOX as u32,
                        ),
                    ]
                })
                .collect();
            if rows.iter().flatten().any(|handle| handle.is_null()) {
                return None;
            }
            SetScrollRange(window, SB_VERT, 0, (rows.len() as i32 - 6).max(0), 1);
            Some(Self { window, rows })
        }
    }
    pub(super) unsafe fn show(&self, visible: bool) {
        unsafe {
            ShowWindow(self.window, if visible { SW_SHOW } else { SW_HIDE });
        }
    }
    pub(super) unsafe fn apply(
        &self,
        action: usize,
        preferences: &mut ReferencePreferences,
    ) -> bool {
        let Some(cell) = action
            .checked_sub(300)
            .filter(|cell| *cell < self.rows.len() * 2)
        else {
            return false;
        };
        let order = preferences.ordered_condition_keys();
        let key = &order[cell / 2];
        let hud = cell % 2 == 1;
        let visible = unsafe {
            SendMessageW(self.rows[cell / 2][1 + usize::from(hud)], BM_GETCHECK, 0, 0) == 1
        };
        preferences.set_condition_visible(key, hud, visible);
        true
    }
    pub(super) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            for (row, key) in self.rows.iter().zip(preferences.ordered_condition_keys()) {
                SetWindowTextW(row[0], wide(&key).as_ptr());
                for (handle, hidden) in row[1..].iter().zip([
                    &preferences.condition_status_hidden,
                    &preferences.condition_hud_hidden,
                ]) {
                    SendMessageW(*handle, BM_SETCHECK, usize::from(!hidden.contains(&key)), 0);
                }
            }
        }
    }
}
