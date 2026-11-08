//! Native character window controls, created on the window thread.
use super::*;

pub(super) struct Widgets {
    pub(super) edits: [HWND; 3],
    pub(super) output: HWND,
    pub(super) heading: HWND,
    pub(super) history: HWND,
    pub(super) detail: HWND,
    pub(super) achievement_browser: HWND,
    pub(super) status_label: HWND,
    pub(super) cards: Vec<HWND>,
    pub(super) stamina_bar: HWND,
    pub(super) training_bar: HWND,
}

pub(super) unsafe fn create(hwnd: HWND) -> Widgets {
    unsafe {
        let fields = [
            ("Page offset", "0"),
            ("Page size (1-50)", "20"),
            ("Target UUID (blank = self)", ""),
        ];
        let mut edits = [null_mut(); 3];
        for (i, (label, value)) in fields.iter().enumerate() {
            let y = 10 + i as i32 * 32;
            control(hwnd, "STATIC", label, 301 + i, [10, y, 235, 25], 0);
            edits[i] = control(
                hwnd,
                "EDIT",
                value,
                201 + i,
                [250, y, 440, 25],
                WS_BORDER | WS_TABSTOP,
            );
        }
        for (i, label) in [
            "General stats",
            "Recent deaths",
            "Recent PvP kills",
            "Achievements",
            "Appearances",
            "Store summary",
            "Inspection",
            "Offence",
            "Defence",
            "Misc",
            "Badges",
        ]
        .iter()
        .enumerate()
        {
            control(
                hwnd,
                "BUTTON",
                label,
                101 + i,
                [
                    10 + (i % 4) as i32 * 175,
                    240 + (i / 4) as i32 * 35,
                    165,
                    28,
                ],
                WS_TABSTOP,
            );
        }
        for (id, label, x) in [(112, "Previous page", 10), (113, "Next page", 190)] {
            control(hwnd, "BUTTON", label, id, [x, 112, 165, 28], WS_TABSTOP);
        }
        let output = control(
            hwnd,
            "EDIT",
            "",
            400,
            [10, 355, 690, 240],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        for (id, label, x) in [
            (114, "All PvP results", 10),
            (115, "Unjustified", 190),
            (116, "Guild war", 370),
        ] {
            control(hwnd, "BUTTON", label, id, [x, 155, 165, 28], WS_TABSTOP);
        }
        let achievement_browser = control(
            hwnd,
            "BUTTON",
            "Browse achievements",
            140,
            [545, 155, 165, 28],
            WS_TABSTOP,
        );
        let heading = control(hwnd, "STATIC", "", 401, [10, 355, 690, 22], 0);
        let history = control(
            hwnd,
            "LISTBOX",
            "",
            402,
            [10, 380, 690, 155],
            WS_BORDER | WS_VSCROLL | WS_HSCROLL | WS_TABSTOP | LBS_USETABSTOPS as u32,
        );
        let detail = control(
            hwnd,
            "EDIT",
            "",
            403,
            [10, 545, 690, 50],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let tabs: [i32; 2] = [100, 180];
        SendMessageW(history, LB_SETTABSTOPS, tabs.len(), tabs.as_ptr() as isize);
        SendMessageW(history, LB_SETHORIZONTALEXTENT, 1800, 0);
        let status_label = control(hwnd, "STATIC", "", 472, [10, 192, 690, 34], 0);
        let common = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_PROGRESS_CLASS,
        };
        InitCommonControlsEx(&common);
        let cards: Vec<_> = (0..14)
            .map(|i| {
                control(
                    hwnd,
                    "STATIC",
                    "",
                    450 + i,
                    [
                        10 + (i % 4) as i32 * 175,
                        355 + (i / 4) as i32 * 52,
                        165,
                        48,
                    ],
                    WS_BORDER,
                )
            })
            .collect();
        let stamina_bar = control(hwnd, "msctls_progress32", "", 470, [10, 575, 335, 18], 0);
        let training_bar = control(hwnd, "msctls_progress32", "", 471, [365, 575, 335, 18], 0);
        for (id, label, x) in [
            (145, "All badges", 10),
            (146, "Earned badges", 185),
            (147, "Locked badges", 360),
            (148, "Hidden locally", 535),
        ] {
            control(hwnd, "BUTTON", label, id, [x, 615, 165, 28], WS_TABSTOP);
        }
        control(hwnd, "STATIC", "Badge ID", 151, [10, 655, 90, 25], 0);
        let badge_id = control(
            hwnd,
            "EDIT",
            "",
            149,
            [105, 655, 90, 25],
            WS_BORDER | WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            "Hide / show locally",
            150,
            [210, 655, 200, 25],
            WS_TABSTOP,
        );
        Widgets {
            edits,
            output,
            heading,
            history,
            detail,
            achievement_browser,
            status_label,
            cards,
            stamina_bar,
            training_bar,
        }
    }
}
