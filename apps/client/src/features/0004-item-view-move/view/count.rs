//! Count prompt on its own native UI thread, with explicit Move and Cancel actions.
// SAFETY: creation, userdata, message dispatch and destruction stay on the creating thread.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use std::{
    ptr::{null, null_mut},
    sync::Arc,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{Controls::EM_SETLIMITTEXT, WindowsAndMessaging::*},
};
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if message == WM_COMMAND && matches!(w & 0xffff, 101 | 102) {
            let action = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut usize;
            if !action.is_null() {
                *action = w & 0xffff;
            }
            return 0;
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}
pub(super) fn open(link: &PlayLink, maximum: u32) {
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("item-count".into())
        .spawn(move || {
            unsafe {
                let hwnd = CreateWindowExW(
                    0,
                    wide("STATIC").as_ptr(),
                    wide(&lfmt!("Move stack: count 1–{maximum}", maximum = maximum)).as_ptr(),
                    WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    340,
                    155,
                    null_mut(),
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if !hwnd.is_null() {
                    let mut action = Box::new(0_usize);
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *action as *mut usize) as isize);
                    SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
                    let edit = CreateWindowExW(
                        0,
                        wide("EDIT").as_ptr(),
                        wide(&maximum.to_string()).as_ptr(),
                        WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP | ES_NUMBER as u32,
                        10,
                        10,
                        295,
                        25,
                        hwnd,
                        null_mut(),
                        null_mut(),
                        null(),
                    );
                    SendMessageW(edit, EM_SETLIMITTEXT, 3, 0);
                    for (id, label, x) in [(101_usize, tr!("Move"), 10), (102, tr!("Cancel"), 165)]
                    {
                        CreateWindowExW(
                            0,
                            wide("BUTTON").as_ptr(),
                            wide(label).as_ptr(),
                            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
                            x,
                            50,
                            140,
                            30,
                            hwnd,
                            id as _,
                            null_mut(),
                            null(),
                        );
                    }
                    let mut message: MSG = std::mem::zeroed();
                    while IsWindow(hwnd) != 0 {
                        while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                            if IsDialogMessageW(hwnd, &message) == 0 {
                                TranslateMessage(&message);
                                DispatchMessageW(&message);
                            }
                        }
                        let Some(mailbox) = weak.upgrade() else {
                            break;
                        };
                        let mut mailbox = lock(&mailbox);
                        if mailbox.inventory_shutdown || mailbox.item_movement.prompt.is_none() {
                            break;
                        }
                        match std::mem::take(&mut *action) {
                            101 => {
                                let mut buffer = [0_u16; 16];
                                let len =
                                    GetWindowTextW(edit, buffer.as_mut_ptr(), buffer.len() as i32);
                                if let Ok(count) =
                                    String::from_utf16_lossy(&buffer[..len.max(0) as usize])
                                        .parse::<u32>()
                                {
                                    let request = mailbox.item_movement.confirm(count);
                                    super::super::super::input::send_move(
                                        &mut mailbox,
                                        &commands,
                                        request,
                                    );
                                }
                                if mailbox.item_movement.prompt.is_none() {
                                    break;
                                }
                                SetWindowTextW(
                                    hwnd,
                                    wide(tr!("Invalid count: enter 1 to the stack size")).as_ptr(),
                                );
                            }
                            102 => {
                                mailbox.item_movement.cancel();
                                break;
                            }
                            _ => {}
                        }
                        drop(mailbox);
                        std::thread::sleep(Duration::from_millis(20));
                    }
                    if IsWindow(hwnd) != 0 {
                        DestroyWindow(hwnd);
                    }
                }
            }
            if let Some(mailbox) = weak.upgrade() {
                let mut mailbox = lock(&mailbox);
                if mailbox.item_movement.prompt.is_some() {
                    mailbox.item_movement.cancel();
                }
                mailbox.item_drag_native.prompt_open = false;
            }
        })
        .is_err()
        && let Some(mailbox) = failed.upgrade()
    {
        let mut mailbox = lock(&mailbox);
        mailbox.item_movement.cancel();
        mailbox.item_drag_native.prompt_open = false;
    }
}
