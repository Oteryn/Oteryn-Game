//! Windows system controls provide a readable panel without a new rendering dependency.
//! Handles and the window-procedure context stay on this thread. The context outlives HWND.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::quest_log::{
    QUEST_SORT_LABELS, QuestLogQuery, QuestLogView, QuestPanel, QuestSort, panel_text,
    quest_row_query, quest_rows, request, tracking_query,
};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const QUESTS: usize = 101;
const MISSIONS: usize = 102;
const REFRESH: usize = 103;
const TRACK: usize = 104;
const AUTO_ADD: usize = 106;
const AUTO_REMOVE: usize = 107;
const FILTER: usize = 108;

#[derive(Default)]
struct Actions(Vec<usize>);

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: only our thread installs this procedure and sets userdata to its live Actions.
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let notification = (w >> 16) & 0xffff;
            if !actions.is_null()
                && (*actions).0.len() < 16
                && ((matches!(id, QUESTS | MISSIONS) && notification == LBN_SELCHANGE as usize)
                    || (matches!(id, REFRESH | TRACK | AUTO_ADD | AUTO_REMOVE | FILTER)
                        && notification == BN_CLICKED as usize))
            {
                (*actions).0.push(id);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

fn wide(text: &str) -> Vec<u16> {
    // Wire text may contain NUL; display a replacement rather than silently truncating it.
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.quest.open {
            return;
        }
        mailbox.quest.open = true;
    }
    let mailbox = Arc::downgrade(&link.mailbox);
    let commands = link.commands.clone();
    let weak = mailbox.clone();
    if std::thread::Builder::new()
        .name("quest-log-panel".into())
        .spawn(move || run(mailbox, commands))
        .is_err()
        && let Some(mailbox) = weak.upgrade()
    {
        lock(&mailbox).quest.open = false;
    }
}

fn run(
    weak: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: UnboundedSender<PlayCommand>,
) {
    // SAFETY: all HWND operations and message dispatch happen on their creating thread.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Quest Log — select a quest, then a mission").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1080,
            580,
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
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
        let quests = control(
            hwnd,
            "LISTBOX",
            "",
            QUESTS,
            10,
            94,
            220,
            136,
            LBS_NOTIFY as u32 | WS_VSCROLL | WS_TABSTOP | WS_BORDER,
        );
        control(
            hwnd,
            "STATIC",
            "Find quest number or status",
            112,
            10,
            10,
            220,
            20,
            0,
        );
        let list_search = control(
            hwnd,
            "EDIT",
            "",
            113,
            10,
            32,
            220,
            24,
            WS_BORDER | WS_TABSTOP,
        );
        SendMessageW(
            list_search,
            windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT,
            128,
            0,
        );
        let list_sort = control(
            hwnd,
            "COMBOBOX",
            "",
            114,
            10,
            62,
            220,
            200,
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP,
        );
        for label in QUEST_SORT_LABELS {
            SendMessageW(list_sort, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        let missions = control(
            hwnd,
            "LISTBOX",
            "",
            MISSIONS,
            240,
            10,
            490,
            220,
            LBS_NOTIFY as u32 | WS_VSCROLL | WS_TABSTOP | WS_BORDER,
        );
        let description = control(
            hwnd,
            "EDIT",
            "",
            105,
            10,
            270,
            720,
            260,
            ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32
                | WS_VSCROLL
                | WS_BORDER
                | WS_TABSTOP,
        );
        let refresh = control(
            hwnd,
            "BUTTON",
            "Quest list / Refresh",
            REFRESH,
            10,
            235,
            220,
            28,
            WS_TABSTOP,
        );
        let track = control(
            hwnd,
            "BUTTON",
            "Track quest",
            TRACK,
            240,
            235,
            220,
            28,
            WS_TABSTOP,
        );
        if [
            quests,
            missions,
            description,
            refresh,
            track,
            list_search,
            list_sort,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let auto_add = control(
            hwnd,
            "BUTTON",
            "Auto-add: off",
            AUTO_ADD,
            750,
            10,
            290,
            28,
            WS_TABSTOP,
        );
        let auto_remove = control(
            hwnd,
            "BUTTON",
            "Auto-remove: off",
            AUTO_REMOVE,
            750,
            45,
            290,
            28,
            WS_TABSTOP,
        );
        let filter = control(
            hwnd,
            "BUTTON",
            "Tracker: all quests",
            FILTER,
            750,
            80,
            290,
            28,
            WS_TABSTOP,
        );
        control(
            hwnd,
            "STATIC",
            "Search tracked missions",
            109,
            750,
            115,
            290,
            22,
            0,
        );
        let search = control(
            hwnd,
            "EDIT",
            "",
            110,
            750,
            140,
            290,
            28,
            WS_BORDER | WS_TABSTOP,
        );
        SendMessageW(
            search,
            windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT,
            128,
            0,
        );
        let tracker = control(
            hwnd,
            "EDIT",
            "",
            111,
            750,
            180,
            290,
            350,
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL | WS_BORDER,
        );
        let mut held = None;
        let mut last_search = None;
        let mut shown_quests = Vec::new();
        let mut options_initialized = false;
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(mailbox) = weak.upgrade() else {
                break;
            };
            let mut mailbox = lock(&mailbox);
            if mailbox.quest_shutdown {
                break;
            }
            if !options_initialized {
                SetWindowTextW(list_search, wide(&mailbox.quest.list_search).as_ptr());
                SendMessageW(
                    list_sort,
                    CB_SETCURSEL,
                    mailbox.quest.list_sort.selection(),
                    0,
                );
                options_initialized = true;
            }
            for id in std::mem::take(&mut actions.0) {
                let query = match id {
                    AUTO_ADD => {
                        let p = &mut mailbox.quest;
                        p.tracker.auto_add = !p.tracker.auto_add;
                        None
                    }
                    AUTO_REMOVE => {
                        mailbox.quest.tracker.auto_remove = !mailbox.quest.tracker.auto_remove;
                        None
                    }
                    FILTER => {
                        use oteryn_client_ui::quest_log::TrackerFilter as F;
                        mailbox.quest.tracker.filter = match mailbox.quest.tracker.filter {
                            F::All => F::Active,
                            F::Active => F::Completed,
                            F::Completed => F::All,
                        };
                        None
                    }
                    REFRESH => Some(QuestLogQuery::List),
                    QUESTS => {
                        let row = SendMessageW(quests, LB_GETCURSEL, 0, 0);
                        usize::try_from(row)
                            .ok()
                            .and_then(|i| quest_row_query(&mailbox.quest, &shown_quests, i))
                    }
                    MISSIONS => {
                        let row = SendMessageW(missions, LB_GETCURSEL, 0, 0);
                        mailbox.quest.mission = match &mailbox.quest.state.log.view {
                            Some(QuestLogView::Line(line)) => usize::try_from(row)
                                .ok()
                                .and_then(|i| line.missions.get(i))
                                .map(|m| m.mission_index),
                            _ => None,
                        };
                        None
                    }
                    TRACK => tracking_query(&mailbox.quest),
                    _ => None,
                };
                if let Some(query) = query
                    && !mailbox.quest_pending
                    && let Ok(payload) = request(&mut mailbox.quest, &query)
                {
                    mailbox.quest_pending = commands.send(PlayCommand::QuestLog(payload)).is_ok();
                }
            }
            let mut list_text = [0u16; 129];
            let len = GetWindowTextW(list_search, list_text.as_mut_ptr(), 129).max(0) as usize;
            mailbox.quest.list_search = String::from_utf16_lossy(&list_text[..len]);
            mailbox.quest.list_sort =
                QuestSort::from_selection(SendMessageW(list_sort, CB_GETCURSEL, 0, 0));
            let mut search_text = [0u16; 129];
            let len = GetWindowTextW(search, search_text.as_mut_ptr(), 129);
            let typed = String::from_utf16_lossy(&search_text[..len.max(0) as usize]);
            if let Some(value) = oteryn_client_ui::quest_log::sync_tracker_search(
                &mut mailbox.quest,
                &mut last_search,
                typed,
            ) {
                SetWindowTextW(search, wide(&value).as_ptr());
            }
            if !mailbox.quest_pending
                && let Some(query) = oteryn_client_ui::quest_log::tracker_query(
                    &mut mailbox.quest,
                    std::time::Instant::now(),
                )
                && let Ok(payload) = request(&mut mailbox.quest, &query)
            {
                mailbox.quest_pending = commands.send(PlayCommand::QuestLog(payload)).is_ok();
            }
            let panel = &mailbox.quest;
            SetWindowTextW(
                auto_add,
                wide(if panel.tracker.auto_add {
                    "Auto-add: on"
                } else {
                    "Auto-add: off"
                })
                .as_ptr(),
            );
            SetWindowTextW(
                auto_remove,
                wide(if panel.tracker.auto_remove {
                    "Auto-remove: on"
                } else {
                    "Auto-remove: off"
                })
                .as_ptr(),
            );
            SetWindowTextW(
                filter,
                wide(&format!("Tracker: {:?}", panel.tracker.filter)).as_ptr(),
            );
            if held.as_ref() != Some(panel) {
                let text = oteryn_client_ui::quest_log::tracker_lines(panel)
                    .iter()
                    .map(|line| {
                        format!(
                            "Quest {}{}\r\n{}",
                            line.quest_index,
                            if line.completed { " (completed)" } else { "" },
                            line.missions
                                .iter()
                                .map(|m| format!(
                                    "{}{}: {}",
                                    m.name,
                                    if m.done { " (done)" } else { "" },
                                    m.text
                                ))
                                .collect::<Vec<_>>()
                                .join("\r\n")
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\r\n\r\n");
                SetWindowTextW(tracker, wide(&text).as_ptr());
                if held.as_ref().is_none_or(|old: &QuestPanel| {
                    old.state.list != panel.state.list
                        || old.list_search != panel.list_search
                        || old.list_sort != panel.list_sort
                }) {
                    let selected = usize::try_from(SendMessageW(quests, LB_GETCURSEL, 0, 0))
                        .ok()
                        .and_then(|i| shown_quests.get(i))
                        .copied();
                    SendMessageW(quests, LB_RESETCONTENT, 0, 0);
                    let rows = quest_rows(panel);
                    shown_quests = rows.iter().map(|q| q.quest_index).collect();
                    for q in rows {
                        let label = format!(
                            "Quest {}{}",
                            q.quest_index,
                            if q.completed { " (completed)" } else { "" }
                        );
                        SendMessageW(quests, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                    }
                    if let Some(row) =
                        selected.and_then(|id| shown_quests.iter().position(|q| *q == id))
                    {
                        SendMessageW(quests, LB_SETCURSEL, row, 0);
                    }
                }
                if held
                    .as_ref()
                    .is_none_or(|old| old.state.log.view != panel.state.log.view)
                {
                    SendMessageW(missions, LB_RESETCONTENT, 0, 0);
                    if let Some(QuestLogView::Line(line)) = &panel.state.log.view {
                        let selected = shown_quests.iter().position(|id| *id == line.quest_index);
                        SendMessageW(
                            quests,
                            LB_SETCURSEL,
                            selected.map_or(usize::MAX, |row| row),
                            0,
                        );
                        for m in &line.missions {
                            SendMessageW(
                                missions,
                                LB_ADDSTRING,
                                0,
                                wide(&format!(
                                    "{}{}",
                                    m.name,
                                    if m.done { " (completed)" } else { "" }
                                ))
                                .as_ptr() as isize,
                            );
                        }
                    }
                }
                SetWindowTextW(description, wide(&panel_text(panel)).as_ptr());
                let tracked = matches!(&panel.state.log.view, Some(QuestLogView::Line(line)) if panel.state.log.tracked.iter().any(|q| q.quest_index == line.quest_index));
                SetWindowTextW(
                    track,
                    wide(if tracked {
                        "Stop tracking quest"
                    } else {
                        "Track quest"
                    })
                    .as_ptr(),
                );
                held = Some(panel.clone());
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        // No procedure can run after destruction; the userdata box can now be dropped.
        close(&weak);
    }
}

fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).quest.open = false;
    }
}

#[allow(clippy::too_many_arguments)]
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    style: u32,
) -> HWND {
    // SAFETY: parent is live on this thread; system class strings live for the call.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            x,
            y,
            width,
            height,
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
