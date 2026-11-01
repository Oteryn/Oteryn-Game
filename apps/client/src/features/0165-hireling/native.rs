//! Minimal Windows hireling controls; HWNDs stay on their creating thread.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::hireling::{Intent, panel_text};
use std::ptr::{null, null_mut};
use std::time::Duration;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub(super) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.hireling.open {
            return;
        }
        mailbox.hireling.open = true;
    }
    let weak = std::sync::Arc::downgrade(&link.mailbox);
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("hireling-panel".into())
        .spawn(move || unsafe {
            let hwnd = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide("Banker Hireling").as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                430,
                300,
                null_mut(),
                null_mut(),
                null_mut(),
                null(),
            );
            if hwnd.is_null() {
                return;
            }
            let list = CreateWindowExW(
                0,
                wide("LISTBOX").as_ptr(),
                null(),
                WS_CHILD | WS_VISIBLE | WS_BORDER,
                10,
                10,
                390,
                140,
                hwnd,
                null_mut(),
                null_mut(),
                null(),
            );
            let summon = button(hwnd, "Summon banker", 101, 10);
            let dismiss = button(hwnd, "Dismiss", 102, 145);
            let refresh = button(hwnd, "Refresh", 103, 280);
            let mut message: MSG = std::mem::zeroed();
            loop {
                while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if message.message == WM_COMMAND {
                        let intent = match message.wParam & 0xffff {
                            101 => Some(Intent::Summon),
                            102 => Some(Intent::Dismiss),
                            103 => Some(Intent::Refresh),
                            _ => None,
                        };
                        if let Some(intent) = intent
                            && let Some(shared) = weak.upgrade()
                        {
                            let mut mailbox = lock(&shared);
                            if let Some(command) =
                                oteryn_client_ui::hireling::request(&mut mailbox.hireling, intent)
                                && commands
                                    .send(crate::combat_input::PlayCommand::Hireling(command))
                                    .is_err()
                            {
                                mailbox.hireling.pending = None;
                                mailbox.hireling.feedback = Some("The session has ended");
                            }
                        }
                    }
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                if IsWindow(hwnd) == 0 {
                    break;
                }
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                let mailbox = lock(&shared);
                if mailbox.hireling_shutdown {
                    break;
                }
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                SendMessageW(
                    list,
                    LB_ADDSTRING,
                    0,
                    wide(&panel_text(&mailbox.hireling)).as_ptr() as isize,
                );
                drop(mailbox);
                std::thread::sleep(Duration::from_millis(100));
            }
            if IsWindow(hwnd) != 0 {
                DestroyWindow(hwnd);
            }
            if let Some(shared) = weak.upgrade() {
                lock(&shared).hireling.open = false;
            }
            let _ = (summon, dismiss, refresh);
        })
        .is_err()
    {
        lock(&link.mailbox).hireling.open = false;
    }
}

unsafe fn button(parent: HWND, text: &str, id: usize, x: i32) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE,
            x,
            165,
            125,
            30,
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
