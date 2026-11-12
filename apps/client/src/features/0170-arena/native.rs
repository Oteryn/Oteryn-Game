//! Native arena window. HWNDs and their callback state stay on the window thread.
#![allow(unsafe_code)] // Win32 controls require FFI; pointers never cross the window thread.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_runtime::arena::ArenaAction;
use oteryn_client_ui::arena::{DROME_REWARDS, Intent, drome_intent, panel_text, request};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Weak},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};

// Crystal 201189f: data-global/startup/tables/chest.lua:2324-2343,2362-2381,2400-2419
// orders the weapon rewards club, axe, sword. Trophy: lib/quests/svargrond_arena.lua:175-190.
const LABELS: [&str; 11] = [
    "Catalogue",
    "Refresh status",
    "Enter",
    "Next pit",
    "Leave",
    "Present",
    "Supplies",
    "Club",
    "Axe",
    "Sword",
    "Trophy",
];
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 16 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.arena.open {
            return;
        }
        m.arena.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("arena-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}
fn run(
    weak: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Arenas — Ctrl+Shift+Z")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            680,
            620,
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
        let status = control(
            hwnd,
            "EDIT",
            "",
            90,
            [10, 10, 640, 280],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let rank = control(
            hwnd,
            "COMBOBOX",
            "",
            91,
            [10, 335, 220, 150],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP,
        );
        for label in [
            tr!("Greenhorn rewards"),
            tr!("Scrapper rewards"),
            tr!("Warlord rewards"),
        ] {
            SendMessageW(rank, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(rank, CB_SETCURSEL, 0, 0);
        let mut buttons: Vec<_> = LABELS
            .iter()
            .enumerate()
            .map(|(i, label)| {
                let label = oteryn_client_ui::localization::tr(label);
                let (x, y) = if i < 5 {
                    (10 + i as i32 * 128, 300)
                } else {
                    (10 + (i as i32 - 5) % 3 * 210, 375 + (i as i32 - 5) / 3 * 40)
                };
                control(
                    hwnd,
                    "BUTTON",
                    label,
                    i + 100,
                    [x, y, if i < 5 { 120 } else { 200 }, 30],
                    WS_TABSTOP,
                )
            })
            .collect();
        for (index, label) in [
            tr!("Drome status"),
            tr!("Drome enter"),
            tr!("Drome leave"),
            tr!("Lower start -5"),
            tr!("Claim potion"),
        ]
        .iter()
        .enumerate()
        {
            buttons.push(control(
                hwnd,
                "BUTTON",
                label,
                index + 200,
                [10 + index as i32 * 128, 465, 120, 30],
                WS_TABSTOP,
            ));
        }
        let reward = control(
            hwnd,
            "COMBOBOX",
            "",
            92,
            [10, 515, 440, 300],
            CBS_DROPDOWNLIST as u32 | WS_TABSTOP | WS_VSCROLL,
        );
        for label in DROME_REWARDS {
            SendMessageW(reward, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(reward, CB_SETCURSEL, 0, 0);
        buttons.push(control(
            hwnd,
            "BUTTON",
            tr!("Buy Drome reward"),
            205,
            [460, 515, 190, 30],
            WS_TABSTOP,
        ));
        if status.is_null()
            || rank.is_null()
            || reward.is_null()
            || buttons.iter().any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut rendered = String::new();
        loop {
            let mut message: MSG = std::mem::zeroed();
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let mut mailbox = lock(&shared);
            for id in std::mem::take(&mut actions.0) {
                let action = match id {
                    100 => Some(Intent::Catalogue),
                    101 => Some(Intent::Action(ArenaAction::Status)),
                    102 => Some(Intent::Action(ArenaAction::Enter)),
                    103 => Some(Intent::Action(ArenaAction::Advance)),
                    104 => Some(Intent::Action(ArenaAction::Leave)),
                    200..=205 => drome_intent(id, SendMessageW(reward, CB_GETCURSEL, 0, 0)),
                    105..=110 => {
                        let rank = SendMessageW(rank, CB_GETCURSEL, 0, 0);
                        let (category, choice) = match id {
                            105 => (1, 1),
                            106 => (2, 1),
                            107 => (3, 1),
                            108 => (3, 2),
                            109 => (3, 3),
                            _ => (4, 1),
                        };
                        u8::try_from(rank + 1).ok().map(|rank| {
                            Intent::Action(ArenaAction::ClaimReward {
                                rank,
                                category,
                                choice,
                            })
                        })
                    }
                    _ => None,
                };
                if let Some(intent) = action
                    && let Some(command) = request(&mut mailbox.arena, intent)
                    && commands
                        .send(crate::combat_input::PlayCommand::Feature(command))
                        .is_err()
                {
                    mailbox.arena.pending = None;
                    mailbox.arena.feedback = Some(tr!("The session has ended"));
                }
            }
            let text = panel_text(&mailbox.arena);
            if text != rendered {
                SetWindowTextW(status, wide(&text).as_ptr());
                rendered = text;
            }
            let ready = mailbox.arena.selected && mailbox.arena.pending.is_none();
            for button in &buttons {
                EnableWindow(*button, i32::from(ready));
            }
            EnableWindow(rank, i32::from(ready));
            EnableWindow(reward, i32::from(ready));
            drop(mailbox);
            // Native message-loop redraw cadence; gameplay timing remains server-owned.
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(m) = weak.upgrade() {
        lock(&m).arena.open = false;
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    r: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            r[0],
            r[1],
            r[2],
            r[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
