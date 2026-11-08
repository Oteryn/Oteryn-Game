//! Native local-chat panel. HWND access remains on its creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::feature_0080_chat_private_channel::view::Controls as PrivateControls;
use crate::play::{
    PlayLink,
    mailbox::{SharedMailbox, lock},
};
use oteryn_client_ui::chat::{Mode, panel_text};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const MODES: usize = 101;
const MESSAGE: usize = 102;
const RECIPIENT: usize = 103;
const RECIPIENT_LABEL: usize = 104;
const SEND: usize = 105;
const IGNORE: usize = 113;
const ALLOW: usize = 114;
const BLOCK: usize = 115;
const CLEAR_LISTS: usize = 116;
const LOG_PATH: usize = 110;
const SAVE: usize = 111;
const APPEND: usize = 112;
const TAB: usize = 107;
const FILTER: usize = 108;
const CHAT_MODE: usize = 109;
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
                && ((matches!(
                    id,
                    SEND | FILTER
                        | CHAT_MODE
                        | SAVE
                        | APPEND
                        | IGNORE
                        | ALLOW
                        | BLOCK
                        | CLEAR_LISTS
                ) && note == BN_CLICKED as usize
                    || id == TAB && note == LBN_SELCHANGE as usize)
                    || PrivateControls::accepts(id, note))
            {
                (*actions).0.push(id);
            }
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
    open_shared(&link.mailbox, &link.commands);
}

pub(in crate::play) fn open_shared(
    mailbox: &SharedMailbox,
    commands: &UnboundedSender<PlayCommand>,
) {
    {
        let mut mailbox = lock(mailbox);
        if mailbox.chat.open {
            return;
        }
        mailbox.chat.open = true;
    }
    let weak = Arc::downgrade(mailbox);
    let rollback = weak.clone();
    let commands = commands.clone();
    if std::thread::Builder::new()
        .name("local-chat-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&rollback);
    }
}

include!("native/run.rs");

fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).chat.open = false;
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
