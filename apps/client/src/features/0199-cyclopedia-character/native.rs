//! Native controls; parsing and intent selection are shared with Linux tests.
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::cyclopedia_character::{CyclopediaCharacterPanel, Intent, request};

pub(in crate::play) fn display(p: &CyclopediaCharacterPanel) -> String {
    let badges = oteryn_client_ui::cyclopedia_character::badge_text(p);
    format!(
        "{}\r\n{}\r\n{}",
        if p.pending {
            "Waiting for the server"
        } else if !p.selected {
            "Feature unavailable in this session"
        } else {
            "Ready"
        },
        p.feedback.unwrap_or(""),
        if p.command_kind == 387 && p.badges.is_some() {
            badges.as_str()
        } else {
            p.result.as_deref().unwrap_or("")
        }
    )
}

pub(in crate::play) fn submit(
    p: &mut CyclopediaCharacterPanel,
    id: usize,
    values: [&str; 3],
) -> Option<FeatureCommand> {
    if p.pending || !p.selected {
        return None;
    }
    if matches!(id, 114..=116) {
        use oteryn_client_runtime::cyclopedia_character::KillStatus;
        p.kill_filter = match id {
            115 => Some(KillStatus::Unjustified),
            116 => Some(KillStatus::GuildWar),
            _ => None,
        };
        return None;
    }
    if matches!(id, 145..=148) {
        use oteryn_client_ui::cyclopedia_character::BadgeFilter;
        p.badge_filter = match id {
            146 => BadgeFilter::Earned,
            147 => BadgeFilter::Locked,
            148 => BadgeFilter::Hidden,
            _ => BadgeFilter::All,
        };
        return None;
    }
    if id == 111 {
        return request(p, Intent::Badges);
    }
    let tab = if matches!(id, 112 | 113) {
        Some(p.tab)
    } else {
        id.checked_sub(100)
            .and_then(|v| u8::try_from(v).ok())
            .and_then(oteryn_client_runtime::cyclopedia_character::CharacterTab::from_id)
    }?;
    let parsed = (|| {
        let offset = values[0].trim().parse::<u16>().ok()?;
        let limit = values[1].trim().parse::<u8>().ok()?;
        oteryn_client_runtime::character_info::PageRequest::new(offset, limit)?;
        let target = if values[2].trim().is_empty() {
            None
        } else {
            Some(parse_target(values[2].trim())?)
        };
        Some((offset, limit, target))
    })();
    let Some((offset, limit, target)) = parsed else {
        p.feedback = Some("Enter a valid offset, page size 1-50 and target UUID");
        return None;
    };
    p.tab = tab;
    p.offset = offset;
    p.limit = limit;
    p.change_target(target);
    request(
        p,
        match id {
            112 => Intent::Previous,
            113 => Intent::Next,
            _ => Intent::Tab,
        },
    )
}

fn parse_target(text: &str) -> Option<[u8; 16]> {
    if text.len() != 32
        && (text.len() != 36 || ![8, 13, 18, 23].iter().all(|i| text.as_bytes()[*i] == b'-'))
    {
        return None;
    }
    let compact: String = text.chars().filter(|c| *c != '-').collect();
    if compact.len() != 32 || !compact.is_ascii() {
        return None;
    }
    let mut target = [0; 16];
    for (i, byte) in target.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&compact[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(target)
}

#[cfg(windows)]
pub(in crate::play) use window::open_window;
#[cfg(windows)]
mod window {
    // HWND operations stay on the creating thread, following the daily reward panel.
    #![allow(unsafe_code)]
    use super::controls::{control, read_text, wide};
    use super::*;
    use crate::combat_input::PlayCommand;
    use crate::play::{PlayLink, mailbox::lock};
    use std::ptr::{null, null_mut};
    use std::sync::{Arc, Weak};
    use std::time::Duration;
    use tokio::sync::mpsc::UnboundedSender;
    use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows_sys::Win32::UI::{
        Controls::{
            ICC_PROGRESS_CLASS, INITCOMMONCONTROLSEX, InitCommonControlsEx, PBM_SETPOS,
            PBM_SETRANGE32,
        },
        Input::KeyboardAndMouse::EnableWindow,
        WindowsAndMessaging::*,
    };
    type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
    #[derive(Default)]
    struct Actions(Vec<usize>);
    unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        unsafe {
            if msg == WM_COMMAND {
                let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
                let id = w & 0xffff;
                let note = (w >> 16) & 0xffff;
                if !actions.is_null()
                    && (*actions).0.len() < 32
                    && matches!(id, 101..=117 | 140)
                    && note == BN_CLICKED as usize
                {
                    (*actions).0.push(id);
                }
                return 0;
            }
            DefWindowProcW(hwnd, msg, w, l)
        }
    }
    pub(in crate::play) fn open_window(link: &PlayLink) {
        {
            let mut m = lock(&link.mailbox);
            if m.cyclopedia_character.open {
                return;
            }
            m.cyclopedia_character.open = true;
        }
        link.cyclopedia_character_request(Intent::Tab);
        let weak = Arc::downgrade(&link.mailbox);
        let rollback = weak.clone();
        let commands = link.commands.clone();
        if std::thread::Builder::new()
            .name("0199-cyclopedia-character-panel".into())
            .spawn(move || run(weak, commands))
            .is_err()
        {
            close(&rollback);
        }
    }
    struct CharacterControls {
        edits: [HWND; 3],
        output: HWND,
        achievement_browser: HWND,
        heading: HWND,
        history: HWND,
        detail: HWND,
        status_label: HWND,
        cards: Vec<HWND>,
        stamina_bar: HWND,
        training_bar: HWND,
        badge_id: HWND,
    }

    unsafe fn create_charactercontrols(hwnd: HWND) -> CharacterControls {
        // SAFETY: called on the window's creating thread; Win32 copies control labels.
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
            for (id, label, x) in [
                (112, "Previous page", 10),
                (113, "Next page", 190),
                (117, "Analytics", 370),
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

            CharacterControls {
                edits,
                output,
                achievement_browser,
                heading,
                history,
                detail,
                status_label,
                cards,
                stamina_bar,
                training_bar,
                badge_id,
            }
        }
    }

    include!("native/run.rs");
    fn close(weak: &WeakMailbox) {
        if let Some(shared) = weak.upgrade() {
            lock(&shared).cyclopedia_character.open = false;
        }
    }
}

#[cfg(windows)]
#[path = "view/controls.rs"]
mod controls;

#[cfg(windows)]
#[path = "view/misc_layout.rs"]
mod misc_layout;
