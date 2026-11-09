//! Native VIP-presence panel. HWND access remains on its creating thread.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::vip_presence::{VipPresencePanel, panel_text};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.vip_presence.open {
            return;
        }
        mailbox.vip_presence.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    if std::thread::Builder::new()
        .name("vip-presence-panel".into())
        .spawn(move || run(weak))
        .is_err()
    {
        close(&rollback);
    }
}
fn run(weak: WeakMailbox) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("VIP List")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            440,
            520,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            close(&weak);
            return;
        }
        let list = CreateWindowExW(
            0,
            wide("EDIT").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD
                | WS_VISIBLE
                | WS_BORDER
                | WS_VSCROLL
                | ES_MULTILINE as u32
                | ES_READONLY as u32,
            10,
            10,
            400,
            450,
            hwnd,
            101 as _,
            null_mut(),
            null(),
        );
        let mut held: Option<VipPresencePanel> = None;
        let mut msg: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let mailbox = lock(&shared);
            if mailbox.chat_public_shutdown {
                break;
            }
            if held.as_ref() != Some(&mailbox.vip_presence) {
                SetWindowTextW(list, wide(&panel_text(&mailbox.vip_presence)).as_ptr());
                held = Some(mailbox.vip_presence.clone());
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            let _ = DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).vip_presence.open = false;
    }
}
