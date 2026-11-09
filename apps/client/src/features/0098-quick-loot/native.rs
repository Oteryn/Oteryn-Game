//! CP02 native controls. Each window owns all its HWNDs and closes with its session.
#![allow(
    unsafe_code,
    reason = "Win32 HWND and userdata access stays on the creating UI thread"
)]
#[path = "native/controls.rs"]
mod controls;
#[path = "native/specs.rs"]
mod specs;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use controls::{control, proc, wide};
use specs::specs;
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Controls::EM_SETLIMITTEXT, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*,
    },
};
#[derive(Default)]
struct Actions(Vec<usize>);
fn set_open(state: &mut oteryn_client_ui::quick_loot::Discovery, panel: u32, open: bool) -> bool {
    let current = match panel {
        98 => &mut state.loot.open,
        132 => &mut state.inspection.open,
        232 => &mut state.teams.open,
        285 => &mut state.history.open,
        _ => return false,
    };
    let previous = *current;
    *current = open;
    previous
}
pub(in crate::play) fn open_window(link: &PlayLink, panel: u32) {
    {
        let mut mailbox = lock(&link.mailbox);
        if set_open(&mut mailbox.discovery, panel, true) {
            return;
        }
        if panel == 285 {
            mailbox.store.open = true;
        }
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name(format!("discovery-{panel}"))
        .spawn(move || run(weak, commands, panel))
        .is_err()
        && let Some(shared) = failed.upgrade()
    {
        let mut mailbox = lock(&shared);
        set_open(&mut mailbox.discovery, panel, false);
        if panel == 285 {
            mailbox.store.open = false;
        }
    }
}
fn run(
    weak: Weak<Mutex<Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    panel: u32,
) {
    // SAFETY: controls copy UTF-16 strings; userdata outlives every message and is cleared before destruction.
    unsafe {
        let title = match panel {
            98 => "Quick loot — Ctrl+Alt+Q",
            132 => "Inspection — Ctrl+Alt+I",
            232 => "Team Finder — Ctrl+Alt+T",
            _ => "Store — Ctrl+Alt+H",
        };
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(title).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            850,
            if panel == 232 { 950 } else { 860 },
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if !hwnd.is_null() {
            window(hwnd, &weak, &commands, panel);
        }
    }
    if let Some(shared) = weak.upgrade() {
        let mut mailbox = lock(&shared);
        set_open(&mut mailbox.discovery, panel, false);
        if panel == 285 {
            mailbox.store.open = false;
        }
    }
}
#[path = "native/paint.rs"]
mod paint;
use paint::window;
