//! Native tutorial and server-save panel. HWND access remains on its creating thread.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::tutorial_notices::{NoticePanel, panel_text};
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
        if mailbox.notices.open {
            return;
        }
        mailbox.notices.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    if std::thread::Builder::new()
        .name("notices-panel".into())
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
            wide("Compendium — Guide, updates and help").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            840,
            620,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            close(&weak);
            return;
        }
        let mut tabs = Vec::new();
        for (index, label) in [
            "Notices",
            "Guide",
            "Recommendations",
            "Updates",
            "Features",
            "Help",
        ]
        .iter()
        .enumerate()
        {
            tabs.push(CreateWindowExW(
                0,
                wide("BUTTON").as_ptr(),
                wide(label).as_ptr(),
                WS_CHILD
                    | WS_VISIBLE
                    | WS_TABSTOP
                    | BS_AUTORADIOBUTTON as u32
                    | if index == 0 { WS_GROUP } else { 0 },
                10 + index as i32 * 132,
                10,
                130,
                30,
                hwnd,
                (201 + index) as _,
                null_mut(),
                null(),
            ));
        }
        if tabs.iter().any(|tab| tab.is_null()) {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        SendMessageW(tabs[0], BM_SETCHECK, 1, 0);
        let mut guide = oteryn_client_ui::guide::Guide::default();
        let mut held_tab = usize::MAX;
        let updates = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(std::path::Path::to_owned))
            .map(|path| crate::bulletin::Browser::load(&path))
            .transpose()
            .map(|browser| browser.unwrap_or_default().news())
            .unwrap_or_else(|error| format!("Updates unavailable: {error}"));
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
            50,
            800,
            510,
            hwnd,
            101 as _,
            null_mut(),
            null(),
        );
        if list.is_null() {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut held: Option<NoticePanel> = None;
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
            let tab = tabs
                .iter()
                .position(|tab| SendMessageW(*tab, BM_GETCHECK, 0, 0) == 1)
                .unwrap_or(0);
            if held.as_ref() != Some(&mailbox.notices) || held_tab != tab {
                let text = if tab == 0 {
                    panel_text(&mailbox.notices)
                } else {
                    use oteryn_client_ui::guide::Page;
                    guide.select(match tab {
                        1 => Page::Basics,
                        2 => Page::Recommendations,
                        3 => Page::Updates,
                        4 => Page::Features,
                        _ => Page::Help,
                    });
                    if tab == 3 {
                        format!("Updates\r\n\r\n{updates}")
                    } else {
                        guide.text()
                    }
                };
                SetWindowTextW(list, wide(&text).as_ptr());
                held_tab = tab;
                held = Some(mailbox.notices.clone());
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
        lock(&mailbox).notices.open = false;
    }
}
