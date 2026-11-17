//! Store layout controls; all handles are created and consumed on one UI thread.
#![allow(unsafe_code, reason = "Native controls stay on their owning thread")]
use super::controls::{control, wide};
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};
pub(super) struct Layout {
    pub(super) category: HWND,
    pub(super) search: HWND,
    pub(super) sort: HWND,
    pub(super) refresh: HWND,
    pub(super) offers: HWND,
    pub(super) image: HWND,
    pub(super) detail: HWND,
    pub(super) buy: HWND,
    pub(super) previous: HWND,
    pub(super) next: HWND,
    pub(super) name: HWND,
    pub(super) sex: HWND,
    pub(super) identity: HWND,
    pub(super) balance: HWND,
    pub(super) status: HWND,
}
pub(super) unsafe fn create(hwnd: HWND) -> Layout {
    unsafe {
        let category = control(
            hwnd,
            "LISTBOX",
            "",
            1,
            [12, 60, 200, 500],
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        let search = control(
            hwnd,
            "EDIT",
            "",
            10,
            [230, 14, 340, 28],
            WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
        );
        let sort = control(
            hwnd,
            "COMBOBOX",
            "",
            11,
            [580, 14, 190, 180],
            WS_TABSTOP | CBS_DROPDOWNLIST as u32,
        );
        for label in [
            "Popularity",
            "Name A–Z",
            "Name Z–A",
            "Price low–high",
            "Price high–low",
        ] {
            SendMessageW(sort, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(sort, CB_SETCURSEL, 0, 0);
        let refresh = control(
            hwnd,
            "BUTTON",
            "Search / refresh",
            2,
            [790, 14, 220, 28],
            WS_TABSTOP,
        );
        let offers = control(
            hwnd,
            "LISTBOX",
            "",
            3,
            [230, 60, 480, 390],
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32 | WS_TABSTOP,
        );
        let image = control(
            hwnd,
            "STATIC",
            "Select a product",
            30,
            [738, 65, 256, 180],
            WS_BORDER,
        );
        control(
            hwnd,
            "BUTTON",
            "Male / female",
            15,
            [730, 255, 130, 28],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            "Turn preview",
            16,
            [870, 255, 140, 28],
            WS_TABSTOP,
        );
        let detail = control(
            hwnd,
            "EDIT",
            "",
            20,
            [730, 300, 280, 110],
            WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        control(
            hwnd,
            "BUTTON",
            "Toggle Cosmetic Tokens / coins",
            17,
            [730, 415, 280, 28],
            WS_TABSTOP,
        );
        let buy = control(
            hwnd,
            "BUTTON",
            "Buy selected product…",
            4,
            [730, 450, 280, 35],
            WS_TABSTOP,
        );
        let previous = control(
            hwnd,
            "BUTTON",
            "Previous page",
            5,
            [230, 465, 230, 30],
            WS_TABSTOP,
        );
        let next = control(
            hwnd,
            "BUTTON",
            "Next page",
            6,
            [480, 465, 230, 30],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "STATIC",
            "Hireling name / sex / target UUID",
            0,
            [730, 500, 280, 24],
            0,
        );
        let name = control(hwnd, "EDIT", "", 21, [730, 530, 180, 26], WS_BORDER);
        let sex = control(hwnd, "EDIT", "1", 22, [920, 530, 90, 26], WS_BORDER);
        let identity = control(hwnd, "EDIT", "", 23, [730, 566, 280, 26], WS_BORDER);
        let balance = control(hwnd, "STATIC", "", 0, [12, 610, 200, 46], 0);
        for (id, label, x) in [(24, "Coin history (web)", 12), (25, "Coin transfer (web)", 112)] {
            control(hwnd, "BUTTON", label, id, [x, 660, 98, 40], WS_TABSTOP | BS_MULTILINE as u32);
        }
        let status = control(
            hwnd,
            "EDIT",
            "",
            0,
            [230, 610, 780, 78],
            WS_BORDER | ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL,
        );
        for (id, label, x) in [
            (12, "Purchase history", 12),
            (13, "Coin history", 220),
            (14, "Gift coins", 428),
            (9, tr!("Get coins (sandbox)"), 636),
        ] {
            control(hwnd, "BUTTON", label, id, [x, 705, 194, 28], WS_TABSTOP);
        }

        control(
            hwnd,
            "BUTTON",
            "Classic Store",
            15,
            [844, 705, 170, 28],
            WS_TABSTOP,
        );
        Layout {
            category,
            search,
            sort,
            refresh,
            offers,
            image,
            detail,
            buy,
            previous,
            next,
            name,
            sex,
            identity,
            balance,
            status,
        }
    }
}
