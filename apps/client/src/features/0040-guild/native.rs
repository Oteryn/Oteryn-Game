//! Minimal Windows guild controls; all handles remain on their creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::guild::{Intent, panel_text, request};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let a = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !a.is_null() {
                (*a).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "�").encode_utf16().chain(Some(0)).collect()
}
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.guild.open {
            return;
        }
        m.guild.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("guild-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}
fn run(weak: WeakMailbox, commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Guild")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            680,
            560,
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
        let list = control(
            hwnd,
            "LISTBOX",
            "",
            1,
            [10, 10, 300, 390],
            WS_BORDER | WS_VSCROLL,
        );
        let state = control(
            hwnd,
            "EDIT",
            "",
            2,
            [320, 10, 330, 390],
            WS_BORDER | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        for (id, label, x, y) in [
            (10, tr!("Refresh"), 10, 415),
            (11, tr!("Invitations"), 120, 415),
            (12, tr!("Roster"), 250, 415),
            (13, tr!("More"), 360, 415),
            (14, tr!("Join selected"), 10, 455),
            (15, tr!("Leave"), 140, 455),
            (16, tr!("Kick selected"), 250, 455),
            (17, tr!("Revoke selected"), 390, 455),
            (18, tr!("Found"), 200, 495),
            (19, tr!("Invite"), 330, 495),
            (20, tr!("Set rank"), 540, 455),
            (21, tr!("Guild chat"), 490, 415),
        ] {
            control(hwnd, "BUTTON", label, id, [x, y, 120, 30], WS_TABSTOP);
        }
        let name = control(
            hwnd,
            "EDIT",
            "",
            3,
            [10, 495, 180, 30],
            WS_BORDER | WS_TABSTOP,
        );
        let rank = control(
            hwnd,
            "EDIT",
            "1",
            4,
            [520, 495, 40, 30],
            WS_BORDER | WS_TABSTOP,
        );
        let mut held = None;
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
                if id == 21 {
                    drop(mailbox);
                    crate::play::feature_0036_chat_guild_room::input::open(&shared, &commands);
                    mailbox = lock(&shared);
                    continue;
                }
                let row = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0)).ok();
                let intent = match id {
                    10 => Some(Intent::RefreshOwn),
                    11 => Some(Intent::RefreshInvitations),
                    12 => Some(Intent::RefreshRoster),
                    13 => Some(Intent::MoreRoster),
                    14 => row
                        .and_then(|r| r.checked_sub(mailbox.guild.roster.len()))
                        .and_then(|r| mailbox.guild.invitations.get(r))
                        .map(|i| Intent::Join(i.guild)),
                    15 => Some(Intent::Leave),
                    16 => row
                        .and_then(|r| mailbox.guild.roster.get(r))
                        .map(|m| Intent::Kick(m.character)),
                    17 => row
                        .and_then(|r| mailbox.guild.roster.get(r))
                        .map(|m| Intent::RevokeInvite(m.character)),
                    18 => Some(Intent::Found(control_text(name))),
                    19 => Some(Intent::Invite(control_text(name))),
                    20 => row
                        .and_then(|r| mailbox.guild.roster.get(r))
                        .and_then(|m| control_text(rank).parse().ok().map(|level| (m, level)))
                        .map(|(m, level)| Intent::SetRank {
                            character: m.character,
                            level,
                        }),
                    _ => None,
                };
                if let Some(intent) = intent
                    && let Some(command) = request(&mut mailbox.guild, intent)
                    && commands.send(PlayCommand::Guild(command)).is_err()
                {
                    mailbox.guild.pending = false;
                    mailbox.guild.feedback = Some(tr!("The session has ended").into());
                }
            }
            EnableWindow(
                GetDlgItem(hwnd, 21),
                i32::from(oteryn_client_ui::chat_guild_room::available(&mailbox.chat)),
            );
            if held.as_ref() != Some(&mailbox.guild) {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for m in &mailbox.guild.roster {
                    SendMessageW(list, LB_ADDSTRING, 0, wide(&m.name).as_ptr() as isize);
                }
                for i in &mailbox.guild.invitations {
                    SendMessageW(list, LB_ADDSTRING, 0, wide(&i.name).as_ptr() as isize);
                }
                SetWindowTextW(state, wide(&panel_text(&mailbox.guild)).as_ptr());
                held = Some(mailbox.guild.clone());
            }
            drop(mailbox);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
unsafe fn control_text(hwnd: HWND) -> String {
    let mut text = [0_u16; 64];
    unsafe {
        let length = GetWindowTextW(hwnd, text.as_mut_ptr(), text.len() as i32);
        String::from_utf16_lossy(&text[..usize::try_from(length).unwrap_or_default()])
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(m) = weak.upgrade() {
        lock(&m).guild.open = false;
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    b: [i32; 4],
    style: u32,
) -> HWND {
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
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
