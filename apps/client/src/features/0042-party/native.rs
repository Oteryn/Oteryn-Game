//! Minimal native party controls; every HWND remains on its creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::party::{Intent, PartyPanel, panel_text, request};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const MEMBERS: usize = 101;
const INVITATIONS: usize = 102;
const JOIN: usize = 103;
const LEAVE: usize = 104;
const PASS: usize = 105;
const KICK: usize = 106;
const SHARED: usize = 107;
const REFRESH: usize = 108;

#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata points to the live Actions box owned by this window thread.
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 16 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(value: &str) -> Vec<u16> {
    value
        .replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.party.open {
            return;
        }
        mailbox.party.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("party-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
        && let Some(mailbox) = failed.upgrade()
    {
        lock(&mailbox).party.open = false;
    }
}

fn run(
    weak: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    // SAFETY: creation, message handling, and destruction of every control stay on this thread.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Party").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            650,
            500,
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
        let list = LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_TABSTOP;
        let members = control(hwnd, "LISTBOX", "", MEMBERS, [10, 10, 300, 220], list);
        let invitations = control(hwnd, "LISTBOX", "", INVITATIONS, [330, 10, 300, 220], list);
        for (id, label, x) in [
            (JOIN, "Join", 10),
            (LEAVE, "Leave", 110),
            (PASS, "Pass leadership", 210),
            (KICK, "Kick", 360),
            (SHARED, "Toggle shared XP", 460),
            (REFRESH, "Refresh", 10),
        ] {
            control(
                hwnd,
                "BUTTON",
                label,
                id,
                [
                    x,
                    if id == REFRESH { 280 } else { 240 },
                    if id == REFRESH { 100 } else { 140 },
                    30,
                ],
                WS_TABSTOP,
            );
        }
        let status = control(hwnd, "STATIC", "", 109, [120, 280, 510, 160], 0);
        let mut held: Option<PartyPanel> = None;
        let mut message: MSG = std::mem::zeroed();
        loop {
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
                if id == MEMBERS && (message.wParam >> 16) & 0xffff == LBN_SELCHANGE as usize {
                    let row = usize::try_from(SendMessageW(members, LB_GETCURSEL, 0, 0)).ok();
                    mailbox.party.selected_member = row
                        .and_then(|row| mailbox.party.state.as_ref()?.members.get(row))
                        .map(|entry| entry.handle);
                    continue;
                }
                if id == INVITATIONS && (message.wParam >> 16) & 0xffff == LBN_SELCHANGE as usize {
                    let row = usize::try_from(SendMessageW(invitations, LB_GETCURSEL, 0, 0)).ok();
                    mailbox.party.selected_invitation = row
                        .and_then(|row| mailbox.party.state.as_ref()?.invitations.get(row))
                        .map(|entry| entry.handle);
                    continue;
                }
                let intent = match id {
                    JOIN => Some(Intent::Join),
                    LEAVE => Some(Intent::Leave),
                    PASS => Some(Intent::PassLeadership),
                    KICK => Some(Intent::Kick),
                    SHARED => Some(Intent::ToggleSharedExperience),
                    REFRESH => Some(Intent::Refresh),
                    _ => None,
                };
                if let Some(intent) = intent
                    && let Some(command) = request(&mut mailbox.party, intent)
                    && commands.send(PlayCommand::Party(command)).is_err()
                {
                    mailbox.party.pending = None;
                    mailbox.party.feedback = Some("The session has ended");
                }
            }
            if held.as_ref() != Some(&mailbox.party) {
                SendMessageW(members, LB_RESETCONTENT, 0, 0);
                SendMessageW(invitations, LB_RESETCONTENT, 0, 0);
                if let Some(state) = &mailbox.party.state {
                    for entry in &state.members {
                        SendMessageW(
                            members,
                            LB_ADDSTRING,
                            0,
                            wide(&entry.name).as_ptr() as isize,
                        );
                    }
                    for entry in &state.invitations {
                        SendMessageW(
                            invitations,
                            LB_ADDSTRING,
                            0,
                            wide(&entry.name).as_ptr() as isize,
                        );
                    }
                }
                SetWindowTextW(status, wide(&panel_text(&mailbox.party)).as_ptr());
                held = Some(mailbox.party.clone());
            }
            drop(mailbox);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        DestroyWindow(hwnd);
        close(&weak);
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).party.open = false;
    }
}
