//! Gem Atelier controls; callbacks and HWND/renderer lifecycle remain in the owner.
use super::*;
pub(super) struct Controls {
    pub(super) surface: HWND,
    pub(super) list: HWND,
    pub(super) mods: HWND,
    pub(super) body: HWND,
    pub(super) quality: HWND,
    pub(super) vessel: HWND,
    pub(super) buttons: [HWND; 12],
}
pub(super) unsafe fn create(hwnd: HWND) -> Option<Controls> {
    unsafe {
        let surface = control(hwnd, "STATIC", "", 1, [12, 12, 320, 64], 0);
        control(
            hwnd,
            "STATIC",
            "Selected | Gem Vessels 1-12 (Replace Vessel with 483; otherwise the first four are draft vessels)",
            2,
            [12, 80, 500, 24],
            0,
        );
        let list = control(
            hwnd,
            "LISTBOX",
            "",
            10,
            [12, 112, 440, 245],
            WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32,
        );
        let mods = control(
            hwnd,
            "LISTBOX",
            "",
            11,
            [12, 370, 440, 245],
            WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32,
        );
        let body = control(
            hwnd,
            "EDIT",
            "",
            12,
            [470, 12, 640, 603],
            WS_BORDER
                | WS_VSCROLL
                | WS_TABSTOP
                | ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32,
        );
        let quality = control(
            hwnd,
            "COMBOBOX",
            "",
            20,
            [12, 630, 150, 200],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        );
        let vessel = control(
            hwnd,
            "COMBOBOX",
            "",
            21,
            [175, 630, 150, 200],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        );
        for value in ["Lesser", "Regular", "Greater"] {
            SendMessageW(quality, CB_ADDSTRING, 0, wide(value).as_ptr() as isize);
        }
        for d in 0..12 {
            SendMessageW(
                vessel,
                CB_ADDSTRING,
                0,
                wide(&vessel_name(d)).as_ptr() as isize,
            );
        }
        let buttons = [
            ("Reveal", 100),
            ("Dismantle", 101),
            ("Reshape mod", 102),
            ("Upgrade grade", 103),
            ("Lock / unlock", 104),
            ("Replace Vessel", 105),
            ("Clear vessel", 106),
            ("Save vessels", 107),
            ("Refresh", 108),
            ("Confirm reviewed change", 109),
            ("Cancel review", 110),
            ("Retry exact request", 111),
        ];
        let buttons = buttons.map(|(text, id)| {
            control(
                hwnd,
                "BUTTON",
                text,
                id,
                [
                    12 + ((id - 100) % 6) as i32 * 182,
                    670 + ((id - 100) / 6) as i32 * 42,
                    174,
                    32,
                ],
                WS_TABSTOP,
            )
        });
        if [surface, list, mods, body, quality, vessel]
            .iter()
            .chain(buttons.iter())
            .any(|h| h.is_null())
        {
            return None;
        }
        Some(Controls {
            surface,
            list,
            mods,
            body,
            quality,
            vessel,
            buttons,
        })
    }
}
