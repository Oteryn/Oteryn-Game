//! Position existing thread-owned controls in the Misc two-column layout.
#![allow(unsafe_code)]
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};
pub(super) fn layout(cards: &[HWND], bars: [HWND; 2], heading: HWND, detail: HWND, misc: bool) {
    // SAFETY: all handles belong to the caller's live window thread.
    unsafe {
        SetWindowPos(
            heading,
            std::ptr::null_mut(),
            10,
            355,
            if misc { 400 } else { 690 },
            22,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        for (i, &h) in cards.iter().enumerate() {
            let (x, y, w, height) = if misc {
                (
                    425 + (i % 2) as i32 * 140,
                    355 + (i / 2) as i32 * 43,
                    135,
                    40,
                )
            } else {
                (
                    10 + (i % 4) as i32 * 175,
                    355 + (i / 4) as i32 * 52,
                    165,
                    48,
                )
            };
            SetWindowPos(
                h,
                std::ptr::null_mut(),
                x,
                y,
                w,
                height,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        for (i, h) in bars.into_iter().enumerate() {
            SetWindowPos(
                h,
                std::ptr::null_mut(),
                if misc {
                    425 + i as i32 * 140
                } else {
                    10 + i as i32 * 355
                },
                575,
                if misc { 130 } else { 335 },
                18,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        SetWindowPos(
            detail,
            std::ptr::null_mut(),
            10,
            545,
            if misc { 400 } else { 690 },
            50,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}
