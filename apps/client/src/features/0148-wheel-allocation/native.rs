//! Windows controls for the existing session feature; HWNDs remain on their creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::wheel_allocation::{
    Intent, WheelPanel, gem_label, modifier_label, panel_text, slot_label, vessel_name,
};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const SLOTS: usize = 101;
const POINTS: usize = 102;
const APPLY: usize = 103;
const SAVE: usize = 104;
const RESET: usize = 105;
const REFRESH: usize = 106;
const GEMS: usize = 107;
const MODIFIERS: usize = 108;
const QUALITY: usize = 109;
const VESSEL: usize = 110;
const SOCKET: usize = 111;
const UNSOCKET: usize = 112;
const REVEAL: usize = 113;
const LOCK: usize = 114;
const RESHAPE: usize = 115;
const DISMANTLE: usize = 116;
const GRADE: usize = 117;
const RETRY: usize = 118;
const PRESETS: usize = 121;
const VISUAL_ATELIER: usize = 122;
const WHEEL_DIAGRAM: usize = 123;

type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;

#[derive(Default)]
struct Actions(Vec<usize>);

#[path = "native_localized_10.rs"]
mod native_localized_10;

use native_localized_10::window_proc;

fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.wheel.open {
            return;
        }
        mailbox.wheel.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("wheel-atelier-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&rollback);
    }
}

#[path = "native_localized_2.rs"]
mod native_localized_2;

use native_localized_2::run;

fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).wheel.open = false;
    }
}

#[path = "native_localized_11.rs"]
mod native_localized_11;

use native_localized_11::control;

#[path = "native_layout.rs"]
mod layout;
#[path = "native_refresh.rs"]
mod refresh;
