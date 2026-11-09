#![allow(unsafe_code, reason = "Win32 controls stay on their owning thread")]
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::*,
    UI::{Controls::EM_SETLIMITTEXT, WindowsAndMessaging::*},
};
pub(super) fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "�").encode_utf16().chain(Some(0)).collect()
}
pub(super) unsafe fn control(
    parent: HWND,
    class: &str,
    label: &'static str,
    id: usize,
    r: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(oteryn_client_ui::localization::tr(label)).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            r[0],
            r[1],
            r[2],
            r[3],
            parent,
            id as HMENU,
            null_mut(),
            null(),
        )
    }
}
pub(super) unsafe fn read(h: HWND, max: usize) -> String {
    unsafe {
        let count = (GetWindowTextLengthW(h).max(0) as usize).min(max);
        let mut out = vec![0u16; count + 1];
        let n = GetWindowTextW(h, out.as_mut_ptr(), out.len() as i32).max(0) as usize;
        String::from_utf16_lossy(&out[..n])
    }
}
pub(super) unsafe fn set(h: HWND, value: &str) {
    unsafe {
        SetWindowTextW(h, wide(value).as_ptr());
    }
}

pub(super) struct Widgets {
    pub choice: HWND,
    pub info: HWND,
    pub edits: [HWND; 3],
    pub navigation: [HWND; 3],
    pub buttons: [HWND; 6],
}
pub(super) unsafe fn build(h: HWND) -> Option<Widgets> {
    unsafe {
        control(h, "STATIC", "House", 0, [12, 12, 110, 22], 0);
        let choice = control(
            h,
            "COMBOBOX",
            "",
            50,
            [125, 10, 680, 250],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP,
        );
        let info = control(
            h,
            "EDIT",
            "",
            51,
            [12, 50, 793, 280],
            WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        let labels = ["Recipient name", "Date (UTC)", "Gold price"];
        let mut edits = [null_mut(); 3];
        for i in 0..3 {
            control(
                h,
                "STATIC",
                labels[i],
                0,
                [12, 345 + i as i32 * 43, 160, 26],
                0,
            );
            edits[i] = control(
                h,
                "EDIT",
                "",
                60 + i,
                [175, 342 + i as i32 * 43, 630, 28],
                WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
            );
            SendMessageW(edits[i], EM_SETLIMITTEXT, [29, 40, 15][i], 0);
        }
        control(
            h,
            "STATIC",
            "Use YYYY-MM-DD HH:MM:SS UTC. Choose a date 24 hours to 30 days ahead.",
            0,
            [12, 478, 793, 25],
            0,
        );
        let mut navigation = [null_mut(); 3];
        for (i, label) in ["Refresh", "Previous page", "Next page"].iter().enumerate() {
            navigation[i] = control(
                h,
                "BUTTON",
                label,
                100 + i,
                [12 + i as i32 * 265, 515, 255, 30],
                WS_TABSTOP,
            );
        }
        let mut buttons = [null_mut(); 6];
        for (i, label) in [
            "Schedule move-out",
            "Cancel move-out",
            "Offer transfer",
            "Cancel transfer",
            "Accept transfer",
            "Reject transfer",
        ]
        .iter()
        .enumerate()
        {
            buttons[i] = control(
                h,
                "BUTTON",
                label,
                201 + i,
                [
                    12 + (i % 3) as i32 * 265,
                    560 + (i / 3) as i32 * 42,
                    255,
                    32,
                ],
                WS_TABSTOP,
            );
        }
        if choice.is_null() || info.is_null() || edits.iter().any(|h| h.is_null()) {
            return None;
        }
        Some(Widgets {
            choice,
            info,
            edits,
            navigation,
            buttons,
        })
    }
}
