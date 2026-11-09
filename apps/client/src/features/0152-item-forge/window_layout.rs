//! Forge control geometry and creation order; GPU surfaces stay owned by the window loop.
use super::{OPS, icons, wide};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};
pub(super) struct Controls {
    pub operation: HWND,
    pub first: HWND,
    pub second: HWND,
    pub boost: HWND,
    pub protect: HWND,
    pub bags: HWND,
    pub open: HWND,
    pub refresh: HWND,
    pub next: HWND,
    pub quote: HWND,
    pub forge: HWND,
    pub retry: HWND,
    pub status: HWND,
    pub secondary: [HWND; 5],
}
pub(super) unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    bounds: [i32; 4],
    style: u32,
) -> HWND {
    // SAFETY: strings survive synchronous creation and parent belongs to this thread.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            bounds[0],
            bounds[1],
            bounds[2],
            bounds[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}

pub(super) unsafe fn create(hwnd: HWND) -> (Controls, [icons::Icon; 2]) {
    // SAFETY: every control and renderer is created on its owning window thread.
    unsafe {
        control(hwnd, "STATIC", "Operation", 1, [12, 12, 220, 22], 0);
        let operation = control(
            hwnd,
            "COMBOBOX",
            "",
            100,
            [12, 36, 570, 200],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        );
        for (_, label) in OPS {
            SendMessageW(operation, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        control(
            hwnd,
            "STATIC",
            "First: receiver / retained item",
            2,
            [12, 76, 570, 22],
            0,
        );
        let first = control(
            hwnd,
            "COMBOBOX",
            "",
            101,
            [12, 100, 570, 250],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        );
        control(
            hwnd,
            "STATIC",
            "Second: donor / consumed item",
            3,
            [12, 140, 570, 22],
            0,
        );
        let second = control(
            hwnd,
            "COMBOBOX",
            "",
            102,
            [12, 164, 570, 250],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        );
        let icons = [
            icons::Icon::new(hwnd, [610, 36, 128, 128]),
            icons::Icon::new(hwnd, [754, 36, 128, 128]),
        ];
        let boost = control(
            hwnd,
            "BUTTON",
            "Core: boost success",
            103,
            [12, 208, 250, 28],
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        let protect = control(
            hwnd,
            "BUTTON",
            "Core: protect failure",
            104,
            [282, 208, 250, 28],
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        let bags = control(
            hwnd,
            "COMBOBOX",
            "",
            105,
            [12, 252, 570, 220],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        );
        let open = control(
            hwnd,
            "BUTTON",
            tr!("Open selected bag"),
            106,
            [610, 252, 270, 28],
            WS_TABSTOP,
        );
        let refresh = control(
            hwnd,
            "BUTTON",
            "Refresh items",
            107,
            [12, 300, 165, 30],
            WS_TABSTOP,
        );
        let next = control(
            hwnd,
            "BUTTON",
            "More items",
            108,
            [190, 300, 165, 30],
            WS_TABSTOP,
        );
        let quote = control(
            hwnd,
            "BUTTON",
            "Preview pair",
            109,
            [370, 300, 165, 30],
            WS_TABSTOP,
        );
        let forge = control(
            hwnd,
            "BUTTON",
            "Forge selected pair",
            110,
            [550, 300, 180, 30],
            WS_TABSTOP,
        );
        let retry = control(
            hwnd,
            "BUTTON",
            "Retry exact attempt",
            111,
            [12, 344, 220, 30],
            WS_TABSTOP,
        );
        let secondary = [
            (tr!("Current dust"), 113),
            (tr!("History"), 114),
            ("Dust to slivers", 115),
            ("Slivers to cores", 116),
            ("Raise dust limit", 117),
        ]
        .map(|(label, id)| {
            control(
                hwnd,
                "BUTTON",
                label,
                id,
                [12 + (id as i32 - 113) * 174, 382, 164, 30],
                WS_TABSTOP,
            )
        });
        let status = control(
            hwnd,
            "EDIT",
            "",
            112,
            [12, 428, 870, 294],
            ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32
                | WS_VSCROLL
                | WS_BORDER
                | WS_TABSTOP,
        );
        (
            Controls {
                operation,
                first,
                second,
                boost,
                protect,
                bags,
                open,
                refresh,
                next,
                quote,
                forge,
                retry,
                secondary,
                status,
            },
            icons,
        )
    }
}
