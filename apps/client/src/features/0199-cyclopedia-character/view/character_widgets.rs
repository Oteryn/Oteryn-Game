//! Native character window controls, created on the window thread.
use super::*;

pub(super) struct Widgets {
    pub(super) badge_id: HWND,
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
            (tr!("Page offset"), "0"),
            (tr!("Page size (1-50)"), "20"),
            (tr!("Target UUID (blank = self)"), ""),
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
            tr!("General stats"),
            tr!("Recent deaths"),
            tr!("Recent PvP kills"),
            tr!("Achievements"),
            tr!("Appearances"),
            tr!("Store summary"),
            tr!("Inspection"),
            tr!("Offence"),
            tr!("Defence"),
            tr!("Misc"),
            tr!("Badges"),
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
        for (id, label, x) in [
            (112, tr!("Previous page"), 10),
            (113, tr!("Next page"), 190),
            (117, tr!("Analytics"), 370),
            (118, "Item summary", 545),
        ] {
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
            (114, tr!("All PvP results"), 10),
            (115, tr!("Unjustified"), 190),
            (116, tr!("Guild war"), 370),
        ] {
            control(hwnd, "BUTTON", label, id, [x, 155, 165, 28], WS_TABSTOP);
        }
        control(
            hwnd,
            "BUTTON",
            "Death loss preview",
            119,
            [10, 190, 340, 28],
            WS_TABSTOP,
        );
        let achievement_browser = control(
            hwnd,
            "BUTTON",
            tr!("Browse achievements"),
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
        let badge_id=super::super::profile_table::badge_controls(hwnd);
        super::super::offence_table::controls(hwnd);
        Widgets {
            badge_id,
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

pub(super) unsafe fn render_cards(
    panel: &CyclopediaCharacterPanel,
    cards: &[HWND],
    bars: [HWND; 2],
    output: HWND,
    [table, overview, combat, store, misc]: [bool; 5],
) {
    let [stamina_bar, training_bar] = bars;
    unsafe {
        let card_rows = if misc {
            oteryn_client_ui::cyclopedia_character::misc_cards(panel)
        } else if store {
            oteryn_client_ui::cyclopedia_character::store_cards(panel)
        } else {
            oteryn_client_ui::cyclopedia_character::overview_cards(panel)
        };
        ShowWindow(
            output,
            if table || overview || combat || store {
                SW_HIDE
            } else {
                SW_SHOW
            },
        );
        for (i, handle) in cards.iter().enumerate() {
            ShowWindow(
                *handle,
                if (overview || store || misc) && i < card_rows.len() {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
        }
        for handle in [stamina_bar, training_bar] {
            ShowWindow(
                handle,
                if overview || (misc && panel.misc_general.is_some()) {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
        }
        if overview || store || misc {
            for (handle, (label, value)) in cards.iter().zip(card_rows) {
                SetWindowTextW(*handle, wide(&format!("{label}\r\n{value}")).as_ptr());
            }
            let bars = if misc {
                oteryn_client_ui::cyclopedia_character::misc_bars(panel)
            } else {
                oteryn_client_ui::cyclopedia_character::overview_bars(panel)
            };
            if let Some(bars) = bars {
                for (handle, (value, max)) in [stamina_bar, training_bar].into_iter().zip(bars) {
                    SendMessageW(handle, PBM_SETRANGE32, 0, max as isize);
                    SendMessageW(handle, PBM_SETPOS, usize::from(value), 0);
                }
            }
        }
    }
}
