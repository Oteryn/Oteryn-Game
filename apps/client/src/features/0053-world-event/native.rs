//! System controls stay on their creating thread; the weak mailbox follows session lifetime.
#![allow(unsafe_code)] // Win32 controls require FFI; all HWNDs are confined to this thread.
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::world_event::panel_text;
use std::ptr::{null, null_mut};
use std::sync::Arc;
use std::time::Duration;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata points to this thread's live refresh flag until HWND destruction.
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == 101 && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let refresh = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut bool;
            if !refresh.is_null() {
                *refresh = true;
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    if link.uses_shared_frontend() {
        return;
    }
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.world_event.open {
            return;
        }
        mailbox.world_event.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let recovery = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("world-event-panel".into())
        .spawn(move || {
            // SAFETY: windows, controls and message dispatch remain on this creating thread.
            unsafe {
                let hwnd = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide(tr!("World events — Ctrl+N")).as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    540,
                    240,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if !hwnd.is_null() {
                    let mut clicked = Box::new(false);
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *clicked as *mut bool) as isize);
                    SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
                    let text = CreateWindowExW(
                        0,
                        wide("EDIT").as_ptr(),
                        wide("").as_ptr(),
                        WS_CHILD | WS_VISIBLE | ES_MULTILINE as u32 | ES_READONLY as u32,
                        12,
                        12,
                        500,
                        130,
                        hwnd,
                        null_mut(),
                        null_mut(),
                        null(),
                    );
                    let refresh = CreateWindowExW(
                        0,
                        wide("BUTTON").as_ptr(),
                        wide(tr!("Refresh")).as_ptr(),
                        WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
                        12,
                        155,
                        120,
                        28,
                        hwnd,
                        101 as *mut _,
                        null_mut(),
                        null(),
                    );
                    if !text.is_null() && !refresh.is_null() {
                        let mut previous = String::new();
                        let mut message: MSG = std::mem::zeroed();
                        loop {
                            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                                TranslateMessage(&message);
                                DispatchMessageW(&message);
                            }
                            if IsWindow(hwnd) == 0 {
                                break;
                            }
                            let Some(mailbox) = weak.upgrade() else {
                                break;
                            };
                            let mut mailbox = lock(&mailbox);
                            if mailbox.world_event_shutdown || commands.is_closed() {
                                break;
                            }
                            if std::mem::take(&mut *clicked)
                                && let Some(command) =
                                    oteryn_client_ui::world_event::request(&mut mailbox.world_event)
                                && commands
                                    .send(crate::combat_input::PlayCommand::Feature(command))
                                    .is_err()
                            {
                                mailbox.world_event.pending = false;
                                mailbox.world_event.feedback = Some(tr!("The session has ended"));
                            }
                            let current = panel_text(&mailbox.world_event);
                            if current != previous {
                                SetWindowTextW(text, wide(&current).as_ptr());
                                previous = current;
                            }
                            drop(mailbox);
                            std::thread::sleep(Duration::from_millis(16));
                        }
                    }
                    if IsWindow(hwnd) != 0 {
                        DestroyWindow(hwnd);
                    }
                }
            }
            if let Some(mailbox) = weak.upgrade() {
                lock(&mailbox).world_event.open = false;
            }
        })
        .is_err()
        && let Some(mailbox) = recovery.upgrade()
    {
        lock(&mailbox).world_event.open = false;
    }
}
