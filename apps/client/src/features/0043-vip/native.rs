//! Native VIP management controls; every HWND remains on its creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::vip::{Intent, request};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata points to the live Actions box owned by this window thread.
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 16 {
                (*actions).0.push(w);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(value: &str) -> Vec<u16> {
    value
        .replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.vip.open {
            return;
        }
        mailbox.vip.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("vip-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
        && let Some(mailbox) = failed.upgrade()
    {
        lock(&mailbox).vip.open = false;
    }
}

include!("native/run.rs");
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).vip.open = false;
    }
}

unsafe fn read(hwnd: HWND) -> String {
    // SAFETY: the edit belongs to this thread; the buffer includes room for the terminator.
    unsafe {
        let mut buffer = vec![0u16; GetWindowTextLengthW(hwnd).max(0) as usize + 1];
        let length = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }
}
