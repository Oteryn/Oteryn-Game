//! Native controls; parsing and intent selection are shared with Linux tests.
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::cyclopedia_character::{CyclopediaCharacterPanel, Intent, request};

pub(in crate::play) fn display(p: &CyclopediaCharacterPanel) -> String {
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
        p.result.as_deref().unwrap_or("")
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
    fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("Character cyclopedia").as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                740,
                650,
                null_mut(),
                null_mut(),
                null_mut(),
                null(),
            );
            if hwnd.is_null() {
                close(&weak);
                return;
            }
            let mut actions = Box::<Actions>::default();
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                (&mut *actions as *mut Actions) as isize,
            );
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
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
            let mut held_history = Vec::new();
            let mut held_combat = Vec::new();
            let mut held_store = String::new();
            let mut painted_tab = None;
            let mut message: MSG = std::mem::zeroed();
            loop {
                while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                if IsWindow(hwnd) == 0 {
                    break;
                }
                let Some(shared) = weak.upgrade() else { break };
                let mut mailbox = lock(&shared);
                if mailbox.cyclopedia_character_shutdown {
                    break;
                }
                let mut analytics = false;
                let mut browse_achievements = false;
                for action in std::mem::take(&mut actions.0) {
                    if action == 117 {
                        analytics = true;
                        continue;
                    }
                    if action == 140 {
                        browse_achievements = true;
                        continue;
                    }
                    let values = edits.map(read_text);
                    if let Some(command) = submit(
                        &mut mailbox.cyclopedia_character,
                        action,
                        values.each_ref().map(String::as_str),
                    ) && commands.send(PlayCommand::Feature(command)).is_err()
                    {
                        mailbox.cyclopedia_character.pending = false;
                        mailbox.cyclopedia_character.feedback = Some("The session has ended");
                    }
                    SetWindowTextW(
                        edits[0],
                        wide(&mailbox.cyclopedia_character.offset.to_string()).as_ptr(),
                    );
                }
                SetWindowTextW(
                    output,
                    wide(&display(&mailbox.cyclopedia_character)).as_ptr(),
                );
                let p = &mut mailbox.cyclopedia_character;
                if !p.pending
                    && p.misc_resources_due
                    && let Some(command) = request(p, Intent::MiscResources)
                    && commands.send(PlayCommand::Feature(command)).is_err()
                {
                    p.pending = false;
                    p.feedback = Some("The session has ended");
                }
                let panel = &mailbox.cyclopedia_character;
                EnableWindow(
                    achievement_browser,
                    i32::from(panel.selected && !panel.pending),
                );
                ShowWindow(
                    achievement_browser,
                    if panel.tab
                        == oteryn_client_runtime::cyclopedia_character::CharacterTab::Achievements
                    {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
                let status = if panel.pending {
                    "Waiting for the server"
                } else {
                    panel.feedback.unwrap_or("")
                };
                SetWindowTextW(status_label, wide(status).as_ptr());
                let table = panel.history_tab == Some(panel.tab);
                let pvp = panel.tab
                    == oteryn_client_runtime::cyclopedia_character::CharacterTab::RecentPvpKills;
                for id in 114..=116 {
                    ShowWindow(GetDlgItem(hwnd, id), if pvp { SW_SHOW } else { SW_HIDE });
                }
                let overview = panel.general.is_some()
                    && panel.tab
                        == oteryn_client_runtime::cyclopedia_character::CharacterTab::GeneralStats;
                let combat = panel.combat.as_ref().is_some_and(|s| s.tab() == panel.tab);
                let store = panel.store.is_some()
                    && panel.tab
                        == oteryn_client_runtime::cyclopedia_character::CharacterTab::StoreSummary;
                let misc =
                    panel.tab == oteryn_client_runtime::cyclopedia_character::CharacterTab::Misc;
                super::misc_layout::layout(
                    &cards,
                    [stamina_bar, training_bar],
                    heading,
                    detail,
                    misc,
                );
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
                        for (handle, (value, max)) in
                            [stamina_bar, training_bar].into_iter().zip(bars)
                        {
                            SendMessageW(handle, PBM_SETRANGE32, 0, max as isize);
                            SendMessageW(handle, PBM_SETPOS, usize::from(value), 0);
                        }
                    }
                }
                for handle in [heading, detail] {
                    ShowWindow(handle, if table || combat { SW_SHOW } else { SW_HIDE });
                }
                ShowWindow(
                    history,
                    if table || combat || store {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
                SetWindowPos(
                    history,
                    null_mut(),
                    10,
                    if store { 415 } else { 380 },
                    if misc { 400 } else { 690 },
                    if store { 180 } else { 155 },
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                if painted_tab != Some(panel.tab) {
                    SendMessageW(history, LB_RESETCONTENT, 0, 0);
                    held_history.clear();
                    held_combat.clear();
                    held_store.clear();
                    painted_tab = Some(panel.tab);
                }
                if table {
                    let note = if panel.pending {
                        "Loading"
                    } else {
                        panel.feedback.unwrap_or("")
                    };
                    use oteryn_client_runtime::cyclopedia_character::KillStatus;
                    let filter = match panel.kill_filter {
                        None => "All results",
                        Some(KillStatus::GuildWar) => "Guild war",
                        Some(KillStatus::Unjustified) => "Unjustified",
                    };
                    let columns = format!(
                        "Time (server Unix ms)     Result     Description | {filter} | {note}"
                    );
                    SetWindowTextW(heading, wide(&columns).as_ptr());
                    let rows: Vec<_> = oteryn_client_ui::cyclopedia_character::history_rows(panel)
                        .into_iter()
                        .cloned()
                        .collect();
                    if held_history != rows {
                        SendMessageW(history, LB_RESETCONTENT, 0, 0);
                        for row in &rows {
                            let kind = match row.status {
                                Some(KillStatus::Unjustified) => "Unjustified",
                                Some(KillStatus::GuildWar) => "Guild war",
                                None => "Death",
                            };
                            let label = format!("{}\t{}\t{}", row.at, kind, row.description);
                            SendMessageW(history, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                        }
                        held_history = rows;
                    }
                    let selected = usize::try_from(SendMessageW(history, LB_GETCURSEL, 0, 0)).ok();
                    let description = selected
                        .and_then(|i| held_history.get(i))
                        .map_or("Select a row to read its complete description", |r| {
                            r.description.as_str()
                        });
                    SetWindowTextW(detail, wide(description).as_ptr());
                }
                if combat {
                    SetWindowTextW(heading, wide("Statistic                    Value                    Source — select a row for details").as_ptr());
                    let rows = oteryn_client_ui::cyclopedia_character::combat_rows(panel);
                    if held_combat != rows {
                        SendMessageW(history, LB_RESETCONTENT, 0, 0);
                        for row in &rows {
                            let label = format!("{}\t{}\t{}", row.label, row.value, row.source);
                            SendMessageW(history, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                        }
                        held_combat = rows;
                    }
                    let selected = usize::try_from(SendMessageW(history, LB_GETCURSEL, 0, 0)).ok();
                    let description = selected.and_then(|i| held_combat.get(i)).map_or_else(
                        || "Select a statistic to inspect its contributions".into(),
                        |row| oteryn_client_ui::cyclopedia_character::combat_detail(panel, row.key),
                    );
                    SetWindowTextW(detail, wide(&description).as_ptr());
                }
                if store {
                    let blessings = oteryn_client_ui::cyclopedia_character::store_blessings(panel);
                    if held_store != blessings {
                        SendMessageW(history, LB_RESETCONTENT, 0, 0);
                        for blessing in blessings.lines() {
                            SendMessageW(
                                history,
                                LB_ADDSTRING,
                                0,
                                wide(blessing).as_ptr() as isize,
                            );
                        }
                        held_store = blessings;
                    }
                }
                drop(mailbox);
                if analytics {
                    crate::play::feature_0010_analyser::view::open_shared(&shared, &commands);
                }
                if browse_achievements {
                    crate::play::feature_local_account_achievements::view::open_character(
                        weak.clone(),
                        commands.clone(),
                    );
                }
                std::thread::sleep(Duration::from_millis(25));
            }
            DestroyWindow(hwnd);
            close(&weak);
            drop(actions);
        }
    }
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
