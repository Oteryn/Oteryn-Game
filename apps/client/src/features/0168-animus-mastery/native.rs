//! Native controls use the same thread-owned HWND lifecycle as the Quest Log window.
// This module contains only the Windows FFI adapter; every unsafe operation is
// confined to its window thread and documented at the call boundary.
#![allow(unsafe_code)]

use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::animus_mastery::{AnimusPanel, Intent, panel_text, request};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[derive(Default)]
struct Actions(Vec<usize>);

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: the creating thread installs userdata pointing to its live Actions.
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
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.animus.open {
            return;
        }
        mailbox.animus.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("animus-mastery".into())
        .spawn(move || run(weak, commands))
        .is_err()
        && let Some(mailbox) = failed.upgrade()
    {
        lock(&mailbox).animus.open = false;
    }
}

fn run(
    weak: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    // SAFETY: HWND and userdata access stays on this creating thread. Actions
    // remains alive through DestroyWindow, and controls copy their strings.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Animus Mastery — Ctrl+A").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            640,
            600,
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
        let list = control(
            hwnd,
            "EDIT",
            "",
            (1, 10, 10, 600, 480),
            WS_BORDER | WS_VSCROLL | WS_TABSTOP | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        let refresh = control(hwnd, "BUTTON", "Refresh", (2, 10, 505, 180, 32), WS_TABSTOP);
        let previous = control(
            hwnd,
            "BUTTON",
            "Previous",
            (3, 220, 505, 180, 32),
            WS_TABSTOP,
        );
        let next = control(hwnd, "BUTTON", "Next", (4, 430, 505, 180, 32), WS_TABSTOP);
        if [list, refresh, previous, next]
            .iter()
            .any(|handle| handle.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut held: Option<AnimusPanel> = None;
        loop {
            let mut message: MSG = std::mem::zeroed();
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
            if mailbox.quest_shutdown || commands.is_closed() {
                break;
            }
            for id in std::mem::take(&mut actions.0) {
                let intent = match id {
                    2 => Some(Intent::Refresh),
                    3 => Some(Intent::Previous),
                    4 => Some(Intent::Next),
                    _ => None,
                };
                if let Some(intent) = intent
                    && let Some(payload) = request(&mut mailbox.animus, intent)
                    && commands.send(PlayCommand::Animus(payload)).is_err()
                {
                    mailbox.animus.pending = None;
                    mailbox.animus.error = Some("Session ended");
                }
            }
            let panel = &mailbox.animus;
            if held.as_ref() != Some(panel) {
                SetWindowTextW(list, wide(&panel_text(panel)).as_ptr());
                let ready = panel.selected && panel.pending.is_none();
                EnableWindow(refresh, i32::from(ready));
                EnableWindow(previous, i32::from(ready && panel.has_previous()));
                EnableWindow(next, i32::from(ready && panel.has_next()));
                held = Some(panel.clone());
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

fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).animus.open = false;
    }
}

unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    layout: (usize, i32, i32, i32, i32),
    style: u32,
) -> HWND {
    let (id, x, y, width, height) = layout;
    // SAFETY: parent belongs to this thread and class/text live through the call.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            x,
            y,
            width,
            height,
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
