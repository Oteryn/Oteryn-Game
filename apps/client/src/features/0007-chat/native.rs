//! Native local-chat panel. HWND access remains on its creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use super::console;
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
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetFocus, SetFocus};
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

#[path = "native_helpers.rs"]
mod helpers;
use helpers::*;
#[path = "native_console.rs"]
mod console_controls;

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
        .spawn(move || dialog::run(weak, commands))
        .is_err()
    {
        close(&rollback);
    }
}

fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() {
        console::close(&mut lock(&mailbox));
    }
}

#[path = "chat_dialog.rs"]
mod dialog;

#[path = "native_mirror.rs"]
mod mirror;
