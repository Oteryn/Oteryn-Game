//! Battle controls and HWNDs remain on their creating thread.
#![allow(unsafe_code)]
#[path = "native_dialog.rs"]
mod dialog;
use super::preferences::{BattleListPreferences, BattleSort, row_label, validate_lists};
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use dialog::run;
use oteryn_client_ui::battle::{self, Action, BattlePanel};
use oteryn_session::{ChaseMode, FightMode};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::*,
};

#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata points to Actions on this thread until HWND destruction.
    unsafe {
        if msg == WM_SYSKEYDOWN && w == 0x42 && (l & (1 << 29)) != 0 {
            DestroyWindow(hwnd);
            return 0;
        }
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let notification = (w >> 16) & 0xffff;
            if !actions.is_null()
                && (*actions).0.len() < 16
                && (((200..=203).contains(&id) && notification == LBN_SELCHANGE as usize)
                    || (id == 300 && notification == CBN_SELCHANGE as usize)
                    || (((101..=108).contains(&id) || (307..=309).contains(&id))
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
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
unsafe fn read_text(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let length = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.battle.native_running {
            return;
        }
        mailbox.battle.native_running = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let fallback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("battle-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&fallback);
    }
}
fn close(weak: &Weak<Mutex<Mailbox>>) {
    if let Some(shared) = weak.upgrade() {
        let mut mailbox = lock(&shared);
        mailbox.battle.open = false;
        mailbox.battle.native_running = false;
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: (i32, i32, i32, i32),
    style: u32,
) -> HWND {
    // SAFETY: parent and temporary UTF-16 strings remain valid throughout this call.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | style,
            rect.0,
            rect.1,
            rect.2,
            rect.3,
            parent,
            id as *mut _,
            null_mut(),
            null(),
        )
    }
}
