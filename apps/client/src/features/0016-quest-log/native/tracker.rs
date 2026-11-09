//! Independent tracker window; all HWNDs and action storage belong to this thread.
#![allow(unsafe_code, reason = "Win32 UI requires thread-owned raw handles")]
use crate::combat_input::PlayCommand;
use crate::play::mailbox::{Mailbox, lock};
use oteryn_client_ui::quest_log::{TrackerFilter, request, tracker_query, tracker_text};
use std::{
    ptr::{null, null_mut},
    sync::{Mutex, Weak},
    time::{Duration, Instant},
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::*,
    UI::{Controls::EM_SETLIMITTEXT, WindowsAndMessaging::*},
};
type WeakMailbox = Weak<Mutex<Mailbox>>;
#[derive(Default)]
struct Actions(Vec<usize>);
fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
unsafe extern "system" fn proc(h: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    // SAFETY: Actions remains on this window's thread until after destruction.
    unsafe {
        if msg == WM_COMMAND
            && (wp >> 16) & 0xffff == BN_CLICKED as usize
            && matches!(wp & 0xffff, 101..=103)
        {
            let actions = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 32 {
                (*actions).0.push(wp & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(h, msg, wp, lp)
    }
}
pub(super) fn open(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
    let Some(shared) = weak.upgrade() else { return };
    {
        let mut m = lock(&shared);
        if m.quest.tracker_open {
            return;
        }
        m.quest.tracker_open = true;
    }
    let failed = weak.clone();
    if std::thread::Builder::new()
        .name("quest-tracker".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}
fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
    // SAFETY: all controls, userdata, dispatch and destruction stay on this thread.
    unsafe {
        let h = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Quest tracker — Ctrl+Alt+Shift+Q")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
            580,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if h.is_null() {
            close(&weak);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
        SetWindowLongPtrW(h, GWLP_WNDPROC, proc as *const () as isize);
        ctl(
            h,
            "STATIC",
            tr!("Search tracked missions"),
            0,
            [10, 8, 680, 20],
            0,
        );
        let search = ctl(
            h,
            "EDIT",
            "",
            104,
            [10, 30, 680, 26],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(search, EM_SETLIMITTEXT, 128, 0);
        let add = ctl(h, "BUTTON", "", 101, [10, 65, 220, 28], 0);
        let remove = ctl(h, "BUTTON", "", 102, [240, 65, 220, 28], 0);
        let filter = ctl(h, "BUTTON", "", 103, [470, 65, 220, 28], 0);
        let out = ctl(
            h,
            "EDIT",
            "",
            105,
            [10, 105, 680, 410],
            WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        if [search, add, remove, filter, out]
            .iter()
            .any(|h| h.is_null())
        {
            DestroyWindow(h);
            close(&weak);
            return;
        }
        if let Some(shared) = weak.upgrade() {
            SetWindowTextW(search, wide(&lock(&shared).quest.tracker.search).as_ptr());
        }
        let mut held = String::new();
        let mut last_search = None;
        loop {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if IsWindow(h) == 0 || commands.is_closed() {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let mut m = lock(&shared);
            if m.quest_shutdown || !m.quest.tracker_open {
                break;
            }
            let mut input = [0u16; 129];
            let count = GetWindowTextW(search, input.as_mut_ptr(), 129).max(0) as usize;
            let typed = String::from_utf16_lossy(&input[..count]);
            if let Some(value) = oteryn_client_ui::quest_log::sync_tracker_search(
                &mut m.quest,
                &mut last_search,
                typed,
            ) {
                SetWindowTextW(search, wide(&value).as_ptr());
            }
            for action in actions.0.drain(..) {
                match action {
                    101 => m.quest.tracker.auto_add = !m.quest.tracker.auto_add,
                    102 => m.quest.tracker.auto_remove = !m.quest.tracker.auto_remove,
                    103 => {
                        m.quest.tracker.filter = match m.quest.tracker.filter {
                            TrackerFilter::All => TrackerFilter::Active,
                            TrackerFilter::Active => TrackerFilter::Completed,
                            TrackerFilter::Completed => TrackerFilter::All,
                        }
                    }
                    _ => {}
                }
            }
            if !m.quest_pending
                && let Some(query) = tracker_query(&mut m.quest, Instant::now())
                && let Ok(payload) = request(&mut m.quest, &query)
            {
                m.quest_pending = commands.send(PlayCommand::QuestLog(payload)).is_ok();
                if !m.quest_pending {
                    m.quest.pending_query = None;
                    m.quest.tracker.command_failed();
                }
            }
            let p = &m.quest;
            SetWindowTextW(
                add,
                wide(if p.tracker.auto_add {
                    tr!("Auto-add: on")
                } else {
                    tr!("Auto-add: off")
                })
                .as_ptr(),
            );
            SetWindowTextW(
                remove,
                wide(if p.tracker.auto_remove {
                    tr!("Auto-remove: on")
                } else {
                    tr!("Auto-remove: off")
                })
                .as_ptr(),
            );
            SetWindowTextW(
                filter,
                wide(&lfmt!("Filter: {:?}", p.tracker.filter)).as_ptr(),
            );
            let text = tracker_text(p);
            if held != text {
                SetWindowTextW(out, wide(&text).as_ptr());
                held = text;
            }
            drop(m);
            std::thread::sleep(Duration::from_millis(100));
        }
        if IsWindow(h) != 0 {
            DestroyWindow(h);
        }
        close(&weak);
    }
}
unsafe fn ctl(h: HWND, class: &str, text: &str, id: usize, b: [i32; 4], style: u32) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            b[0],
            b[1],
            b[2],
            b[3],
            h,
            id as _,
            null_mut(),
            null(),
        )
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).quest.tracker_open = false;
    }
}
