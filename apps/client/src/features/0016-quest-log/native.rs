//! Windows system controls provide a readable panel without a new rendering dependency.
//! Handles and the window-procedure context stay on this thread. The context outlives HWND.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::quest_log::{
    QuestLogQuery, QuestLogView, QuestPanel, panel_text, request, tracking_query,
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
                    || (matches!(id, REFRESH | TRACK) && notification == BN_CLICKED as usize))
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
            760,
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
            10,
            220,
            220,
            LBS_NOTIFY as u32 | WS_VSCROLL | WS_TABSTOP | WS_BORDER,
        );
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
        if [quests, missions, description, refresh, track]
            .iter()
            .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut held = None;
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
            for id in std::mem::take(&mut actions.0) {
                let query = match id {
                    REFRESH => Some(QuestLogQuery::List),
                    QUESTS => {
                        let row = SendMessageW(quests, LB_GETCURSEL, 0, 0);
                        usize::try_from(row)
                            .ok()
                            .and_then(|i| mailbox.quest.state.list.get(i))
                            .map(|q| QuestLogQuery::Quest {
                                quest_index: q.quest_index,
                            })
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
            let panel = &mailbox.quest;
            if held.as_ref() != Some(panel) {
                if held
                    .as_ref()
                    .is_none_or(|old: &QuestPanel| old.state.list != panel.state.list)
                {
                    SendMessageW(quests, LB_RESETCONTENT, 0, 0);
                    for q in &panel.state.list {
                        let label = format!(
                            "Quest {}{}",
                            q.quest_index,
                            if q.completed { " (completed)" } else { "" }
                        );
                        SendMessageW(quests, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                    }
                }
                if held
                    .as_ref()
                    .is_none_or(|old| old.state.log.view != panel.state.log.view)
                {
                    SendMessageW(missions, LB_RESETCONTENT, 0, 0);
                    if let Some(QuestLogView::Line(line)) = &panel.state.log.view {
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
