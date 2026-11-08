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

struct QuestControls {
    quests: HWND,
    list_search: HWND,
    list_sort: HWND,
    missions: HWND,
    description: HWND,
    refresh: HWND,
    track: HWND,
}

unsafe fn create_questcontrols(hwnd: HWND) -> QuestControls {
    // SAFETY: called on the window's creating thread; Win32 copies control labels.
    unsafe {
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
            "Find number, status or known mission",
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
        QuestControls {
            quests,
            list_search,
            list_sort,
            missions,
            description,
            refresh,
            track,
        }
    }
}

include!("native/run.rs");

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
