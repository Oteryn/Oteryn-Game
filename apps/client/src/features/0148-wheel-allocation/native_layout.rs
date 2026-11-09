//! Existing Wheel control construction, kept separate from event handling.
use super::*;
#[derive(Clone, Copy)]
pub(super) struct Controls {
    pub(super) slots: HWND,
    pub(super) points: HWND,
    pub(super) gems: HWND,
    pub(super) modifiers: HWND,
    pub(super) quality: HWND,
    pub(super) lock_button: HWND,
    pub(super) domain: HWND,
    pub(super) description: HWND,
}
pub(super) unsafe fn create(hwnd: HWND) -> Option<Controls> {
    unsafe {
        let list_style = LBS_NOTIFY as u32 | WS_VSCROLL | WS_TABSTOP | WS_BORDER;
        let combo_style = CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP;
        let title = control(
            hwnd,
            "STATIC",
            "Wheel points, perks and Gem Atelier",
            120,
            [10, 10, 380, 22],
            0,
        );
        control(
            hwnd,
            "BUTTON",
            "Wheel diagram",
            WHEEL_DIAGRAM,
            [410, 6, 180, 28],
            WS_TABSTOP,
        );
        control(hwnd, "BUTTON", "Visual Gem Atelier", VISUAL_ATELIER, [600, 6, 180, 28], WS_TABSTOP);
        let presets = control(
            hwnd,
            "BUTTON",
            tr!("Build presets"),
            PRESETS,
            [790, 6, 170, 28],
            WS_TABSTOP,
        );
        let slots = control(hwnd, "LISTBOX", "", SLOTS, [10, 40, 300, 330], list_style);
        let points = control(
            hwnd,
            "EDIT",
            "0",
            POINTS,
            [10, 380, 120, 26],
            ES_NUMBER as u32 | WS_TABSTOP | WS_BORDER,
        );
        let apply = control(
            hwnd,
            "BUTTON",
            tr!("Set slot points"),
            APPLY,
            [140, 380, 170, 28],
            WS_TABSTOP,
        );
        let save = control(
            hwnd,
            "BUTTON",
            tr!("Save Wheel"),
            SAVE,
            [10, 414, 145, 28],
            WS_TABSTOP,
        );
        let reset = control(
            hwnd,
            "BUTTON",
            tr!("Reset all points"),
            RESET,
            [165, 414, 145, 28],
            WS_TABSTOP,
        );
        let gems = control(hwnd, "LISTBOX", "", GEMS, [330, 40, 630, 230], list_style);
        let modifiers = control(
            hwnd,
            "LISTBOX",
            "",
            MODIFIERS,
            [330, 280, 340, 160],
            list_style,
        );
        let quality = control(
            hwnd,
            "COMBOBOX",
            "",
            QUALITY,
            [690, 280, 270, 200],
            combo_style,
        );
        let reveal = control(
            hwnd,
            "BUTTON",
            tr!("Reveal gem"),
            REVEAL,
            [690, 312, 270, 28],
            WS_TABSTOP,
        );
        let lock_button = control(
            hwnd,
            "BUTTON",
            tr!("Lock / Unlock"),
            LOCK,
            [690, 346, 130, 28],
            WS_TABSTOP,
        );
        let switch = control(
            hwnd,
            "BUTTON",
            tr!("Switch domain"),
            SWITCH,
            [830, 346, 130, 28],
            WS_TABSTOP,
        );
        let dismantle = control(
            hwnd,
            "BUTTON",
            tr!("Dismantle selected gem"),
            DISMANTLE,
            [690, 380, 270, 28],
            WS_TABSTOP,
        );
        let grade = control(
            hwnd,
            "BUTTON",
            tr!("Upgrade selected modifier"),
            GRADE,
            [690, 414, 270, 28],
            WS_TABSTOP,
        );
        let domain = control(
            hwnd,
            "COMBOBOX",
            "",
            DOMAIN,
            [10, 455, 300, 200],
            combo_style,
        );
        let socket = control(
            hwnd,
            "BUTTON",
            tr!("Socket selected gem"),
            SOCKET,
            [330, 455, 165, 28],
            WS_TABSTOP,
        );
        let unsocket = control(
            hwnd,
            "BUTTON",
            tr!("Clear chosen vessel"),
            UNSOCKET,
            [505, 455, 165, 28],
            WS_TABSTOP,
        );
        let refresh = control(
            hwnd,
            "BUTTON",
            tr!("Refresh"),
            REFRESH,
            [690, 455, 130, 28],
            WS_TABSTOP,
        );
        let retry = control(
            hwnd,
            "BUTTON",
            tr!("Retry last request"),
            RETRY,
            [830, 455, 130, 28],
            WS_TABSTOP,
        );
        let description = control(
            hwnd,
            "EDIT",
            "",
            119,
            [10, 500, 950, 190],
            ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32
                | WS_VSCROLL
                | WS_BORDER
                | WS_TABSTOP,
        );
        if [
            title,
            slots,
            points,
            apply,
            save,
            reset,
            gems,
            modifiers,
            quality,
            reveal,
            lock_button,
            switch,
            dismantle,
            grade,
            presets,
            domain,
            socket,
            unsocket,
            refresh,
            retry,
            description,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            return None;
        }
        for name in [tr!("Lesser gem"), tr!("Regular gem"), tr!("Greater gem")] {
            SendMessageW(quality, CB_ADDSTRING, 0, wide(name).as_ptr() as isize);
        }
        for value in 0..4 {
            SendMessageW(
                domain,
                CB_ADDSTRING,
                0,
                wide(&format!("{} vessel", domain_name(value))).as_ptr() as isize,
            );
        }
        Some(Controls {
            slots,
            points,
            gems,
            modifiers,
            quality,
            domain,
            lock_button,
            description,
        })
    }
}
