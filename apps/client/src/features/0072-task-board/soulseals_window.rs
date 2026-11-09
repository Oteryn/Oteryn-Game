//! Thread-owned fight selector; the shared caller submits every request.
#![allow(
    unsafe_code,
    reason = "native controls and userdata remain on their owning thread"
)]
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use oteryn_client_ui::task_board::{
    fights::{self, FightIntent},
    soulseals_text,
};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
type Shared = Weak<Mutex<Mailbox>>;
#[derive(Default)]
struct Actions(Vec<usize>);
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
unsafe extern "system" fn procedure(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let a = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Actions;
            if !a.is_null() && (*a).0.len() < 16 && (1..=4).contains(&(w & 0xffff)) {
                (*a).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(h, msg, w, l)
    }
}
unsafe fn control(h: HWND, class: &str, label: &str, id: usize, r: [i32; 4], style: u32) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(label).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            r[0],
            r[1],
            r[2],
            r[3],
            h,
            id as HMENU,
            null_mut(),
            null(),
        )
    }
}
pub(super) fn open(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if !m.task_board.selected || m.task_board.soulseals_open {
            return;
        }
        m.task_board.soulseals_open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("soulseal-fights".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}
fn close(weak: &Shared) {
    if let Some(shared) = weak.upgrade() {
        let mut m = lock(&shared);
        m.task_board.soulseals_open = false;
        if let Some(reply) = &mut m.task_board.fights.reply {
            reply.authorization = None;
        }
    }
}
fn run(weak: Shared, commands: UnboundedSender<PlayCommand>) {
    // SAFETY: HWNDs and the stable boxed userdata belong only to this thread.
    unsafe {
        let h = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Oteryn — Soulseal fights").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            760,
            600,
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
        SetWindowLongPtrW(h, GWLP_WNDPROC, procedure as *const () as isize);
        let choice = control(
            h,
            "COMBOBOX",
            "",
            100,
            [10, 10, 720, 300],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP,
        );
        let details = control(
            h,
            "EDIT",
            "",
            101,
            [10, 60, 720, 420],
            WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        for (id, label, x, width) in [
            (1, "Refresh", 10, 120),
            (2, "Previous", 140, 120),
            (3, "Next", 270, 120),
            (4, "Enter selected fight", 400, 250),
        ] {
            control(h, "BUTTON", label, id, [x, 495, width, 32], WS_TABSTOP);
        }
        if choice.is_null() || details.is_null() {
            DestroyWindow(h);
            close(&weak);
            return;
        }
        let mut rows = Vec::<(u16, String)>::new();
        let mut history = Vec::new();
        let mut after = 0;
        loop {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(h, &msg) == 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            if IsWindow(h) == 0 || commands.is_closed() {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let p = {
                let mut m = lock(&shared);
                if !m.task_board.selected || !m.task_board.soulseals_open {
                    break;
                }
                if let Ok(index) = usize::try_from(SendMessageW(choice, CB_GETCURSEL, 0, 0))
                    && let Some((race, _)) = rows.get(index)
                {
                    fights::select(&mut m.task_board, *race);
                }
                m.task_board.clone()
            };
            for id in actions.0.drain(..) {
                if p.fights.pending.is_some() {
                    continue;
                }
                let intent = match id {
                    1 => {
                        history.clear();
                        after = 0;
                        Some(FightIntent::Catalogue { after: 0 })
                    }
                    2 => history.pop().map(|previous| {
                        after = previous;
                        FightIntent::Catalogue { after }
                    }),
                    3 => p.fights.reply.as_ref().and_then(|r| r.next).map(|next| {
                        history.push(after);
                        after = next;
                        FightIntent::Catalogue { after }
                    }),
                    4 => {
                        let selected = p.fights.reply.as_ref().and_then(|r| {
                            r.entries.iter().find(|f| Some(f.race) == p.fights.selected)
                        });
                        if selected.is_some_and(|f|MessageBoxW(h,wide(&format!("Enter a solo fight against {} for {} Soulseals? The server spends the seals only if entry commits.",f.name,f.cost)).as_ptr(),wide("Confirm Soulseal fight").as_ptr(),MB_YESNO|MB_ICONQUESTION)==IDYES) {Some(FightIntent::EnterSelected)} else {None}
                    }
                    _ => None,
                };
                if let Some(intent) = intent {
                    let mut m = lock(&shared);
                    if intent == FightIntent::EnterSelected
                        && (m.task_board.fights.selected != p.fights.selected
                            || m.task_board.fights.reply != p.fights.reply)
                    {
                        continue;
                    }
                    super::super::input::soulseals::send(&mut m, &commands, intent);
                }
            }
            let new_rows: Vec<_> = p
                .fights
                .reply
                .iter()
                .flat_map(|r| &r.entries)
                .map(|f| (f.race, format!("{} — {} Soulseals", f.name, f.cost)))
                .collect();
            if new_rows != rows {
                SendMessageW(choice, CB_RESETCONTENT, 0, 0);
                for (_, label) in &new_rows {
                    SendMessageW(choice, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
                }
                rows = new_rows;
            }
            if let Some(index) = rows
                .iter()
                .position(|(race, _)| Some(*race) == p.fights.selected)
            {
                SendMessageW(choice, CB_SETCURSEL, index, 0);
            }
            SetWindowTextW(details, wide(&soulseals_text(&p)).as_ptr());
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        DestroyWindow(h);
        close(&weak);
    }
}
