//! Native party-chat panel. HWND access remains on its creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::chat_party_room::{ChatPartyRoomPanel, panel_text, request};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const MESSAGE: usize = 101;
const SEND: usize = 102;
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == SEND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 1);
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.chat_party_room.open {
            return;
        }
        mailbox.chat_party_room.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("party-chat-panel".into())
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
            wide(tr!("Party Chat")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            620,
            460,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            close(&weak);
            return;
        }
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let transcript = control(
            hwnd,
            "EDIT",
            "",
            103,
            [10, 10, 580, 340],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let message = control(
            hwnd,
            "EDIT",
            "",
            MESSAGE,
            [10, 365, 465, 28],
            WS_BORDER | WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            tr!("Send"),
            SEND,
            [490, 365, 100, 28],
            WS_TABSTOP,
        );
        let mut held: Option<ChatPartyRoomPanel> = None;
        let mut msg: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let mut mailbox = lock(&shared);
            if mailbox.chat_public_shutdown {
                break;
            }
            if GetWindowLongPtrW(hwnd, GWLP_USERDATA) != 0 {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                mailbox.chat_party_room.text = read_text(message);
                if let Some(intent) = request(&mut mailbox.chat_party_room)
                    && commands.send(PlayCommand::ChatPublic(intent)).is_err()
                {
                    mailbox.chat_party_room.pending = false;
                    mailbox.chat_party_room.feedback = Some(tr!("The session has ended").into());
                }
            }
            if held.as_ref() != Some(&mailbox.chat_party_room) {
                SetWindowTextW(
                    transcript,
                    wide(&panel_text(&mailbox.chat_party_room)).as_ptr(),
                );
                held = Some(mailbox.chat_party_room.clone());
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            let _ = DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).chat_party_room.open = false;
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    bounds: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            bounds[0],
            bounds[1],
            bounds[2],
            bounds[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
unsafe fn read_text(hwnd: HWND) -> String {
    unsafe {
        let length = GetWindowTextLengthW(hwnd);
        let mut value = vec![0; usize::try_from(length).unwrap_or(0) + 1];
        let read = GetWindowTextW(hwnd, value.as_mut_ptr(), length + 1);
        String::from_utf16_lossy(&value[..usize::try_from(read).unwrap_or(0)])
    }
}
