//! Battle controls and HWNDs remain on their creating thread.
#![allow(unsafe_code)]
use super::preferences::{BattleListPreferences, BattleSort, row_label, validate_lists};
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
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
#[path = "native_localized_17.rs"]
mod native_localized_17;

use native_localized_17::window_proc;
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
#[path = "native_localized_3.rs"]
mod native_localized_3;

use native_localized_3::run;
fn close(weak: &Weak<Mutex<Mailbox>>) {
    if let Some(shared) = weak.upgrade() {
        let mut mailbox = lock(&shared);
        mailbox.battle.open = false;
        mailbox.battle.native_running = false;
    }
}
#[path = "native_localized_16.rs"]
mod native_localized_16;

use native_localized_16::control;
