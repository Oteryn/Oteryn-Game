//! Minimal Windows hireling controls; HWNDs stay on their creating thread.
#![allow(unsafe_code)]
#[cfg(windows)]
use crate::play::{PlayLink, mailbox::lock};
#[cfg(windows)]
use oteryn_client_ui::hireling::panel_text;
#[cfg(windows)]
use std::ptr::{null, null_mut};
#[cfg(windows)]
use std::time::Duration;
#[cfg(windows)]
use windows_sys::Win32::Foundation::*;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(windows)]
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

#[cfg(windows)]
pub(super) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
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
                wide(tr!("Banker Hireling")).as_ptr(),
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
            if let Some(shared) = weak.upgrade() {
                if !crate::play::mailbox::native_startup_result(&mut lock(&shared).hireling.open, !hwnd.is_null()) {
                    return;
                }
            } else {
                if !hwnd.is_null() { DestroyWindow(hwnd); }
                return;
            }
            let mut actions = Vec::<usize>::new();
            if !install_notifications(hwnd, &mut actions) {
                if let Some(shared) = weak.upgrade() {
                    crate::play::mailbox::native_startup_result(&mut lock(&shared).hireling.open, false);
                }
                DestroyWindow(hwnd);
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
            let summon = button(hwnd, tr!("Summon banker"), 101, 10);
            let dismiss = button(hwnd, tr!("Dismiss"), 102, 145);
            let refresh = button(hwnd, tr!("Refresh"), 103, 280);
            let mut message: MSG = std::mem::zeroed();
            loop {
                while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                if IsWindow(hwnd) == 0 {
                    break;
                }
                let Some(shared) = crate::play::mailbox::native_mailbox(&weak, &commands) else {
                    break;
                };
                let mut mailbox = lock(&shared);
                dispatch_notifications(&mut mailbox, &commands, std::mem::take(&mut actions));
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

#[cfg(windows)]
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

#[cfg(windows)]
unsafe extern "system" fn window_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if message == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Vec<usize>;
            if let Some(actions) = actions.as_mut() {
                record_notification(actions, w);
            }
            return 0;
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}
pub(in crate::play) fn record_notification(actions: &mut Vec<usize>, w: usize) {
    let id = w & 0xffff;
    if (w >> 16) & 0xffff == 0 && matches!(id, 101..=103) && actions.len() < 16 {
        actions.push(id);
    }
}
pub(in crate::play) fn dispatch_notifications(
    mailbox: &mut crate::play::mailbox::Mailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    actions: Vec<usize>,
) {
    use oteryn_client_ui::hireling::{Intent, request};
    for action in actions {
        let intent = match action {
            101 => Intent::Summon,
            102 => Intent::Dismiss,
            103 => Intent::Refresh,
            _ => continue,
        };
        if let Some(command) = request(&mut mailbox.hireling, intent)
            && commands
                .send(crate::combat_input::PlayCommand::Hireling(command))
                .is_err()
        {
            mailbox.hireling.pending = None;
            mailbox.hireling.feedback = Some(tr!("The session has ended"));
        }
    }
}

#[cfg(windows)]
unsafe fn install_notifications(hwnd: HWND, actions: &mut Vec<usize>) -> bool {
    unsafe {
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, actions as *mut _ as isize);
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize) != 0
    }
}

#[cfg(all(test, windows))]
#[test]
fn synchronous_hireling_wm_command_records_button_notification()
-> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Hireling notification regression").as_ptr(),
            0,
            0,
            0,
            100,
            100,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return Err("create test window".into());
        }
        let mut actions = Vec::new();
        if !install_notifications(hwnd, &mut actions) {
            DestroyWindow(hwnd);
            return Err("install notification procedure".into());
        }
        SendMessageW(hwnd, WM_COMMAND, 101, 0);
        DestroyWindow(hwnd);
        assert_eq!(actions, [101]);
    }
    Ok(())
}
