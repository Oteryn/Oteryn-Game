//! CP02 native controls. Each window owns all its HWNDs and closes with its session.
#![allow(
    unsafe_code,
    reason = "Win32 HWND and userdata access stays on the creating UI thread"
)]
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
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
#[path = "native/projection.rs"]
mod projection;
#[path = "native/specs.rs"]
mod specs;
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
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
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
            232 => "Social — Team Finder — Ctrl+Alt+T",
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
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: (i32, i32, i32, i32),
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            rect.0,
            rect.1,
            rect.2,
            rect.3,
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
#[path = "native/paint.rs"]
mod paint;
use paint::window;
