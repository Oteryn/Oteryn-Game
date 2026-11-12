//! Windows tracker editor; HWNDs remain on their creating thread.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::cyclopedia_tracker::{Intent, panel_text};
use std::{
    ptr::{null, null_mut},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::*,
    UI::{Controls::BST_CHECKED, WindowsAndMessaging::*},
};
#[path = "widget.rs"]
mod widget;

fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.cyclopedia_tracker.open {
            return;
        }
        m.cyclopedia_tracker.open = true
    }
    let weak = std::sync::Arc::downgrade(&link.mailbox);
    let commands = link.commands.clone();
    let _ = std::thread::Builder::new()
        .name("cyclopedia-tracker-panel".into())
        .spawn(move || unsafe {
            let h = CreateWindowExW(
                0,
                w("STATIC").as_ptr(),
                w(tr!("Cyclopedia tracker")).as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                740,
                430,
                null_mut(),
                null_mut(),
                null_mut(),
                null(),
            );
            if h.is_null() {
                if let Some(s) = weak.upgrade() {
                    lock(&s).cyclopedia_tracker.open = false;
                }
                return;
            }
            let mut actions = Box::<widget::Actions>::default();
            SetWindowLongPtrW(
                h,
                GWLP_USERDATA,
                (&mut *actions as *mut widget::Actions) as isize,
            );
            SetWindowLongPtrW(h, GWLP_WNDPROC, widget::proc as *const () as isize);
            let key = ctl(
                h,
                "EDIT",
                "oteryn:creature.",
                101,
                [10, 10, 390, 25],
                WS_BORDER,
            );
            let boss = ctl(
                h,
                "BUTTON",
                tr!("Bosstiary"),
                102,
                [410, 10, 100, 25],
                BS_AUTOCHECKBOX as u32,
            );
            for (i, (t, id)) in [
                (tr!("Add"), 103),
                (tr!("Remove"), 104),
                (tr!("Refresh"), 105),
            ]
            .iter()
            .enumerate()
            {
                ctl(h, "BUTTON", t, *id, [10 + i as i32 * 110, 45, 100, 28], 0);
            }
            ctl(
                h,
                "BUTTON",
                tr!("Bestiary widget"),
                107,
                [340, 45, 160, 28],
                0,
            );
            ctl(
                h,
                "BUTTON",
                tr!("Bosstiary widget"),
                108,
                [510, 45, 160, 28],
                0,
            );
            let out = ctl(
                h,
                "EDIT",
                "",
                106,
                [10, 85, 570, 295],
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
            );
            let mut msg: MSG = std::mem::zeroed();
            loop {
                while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                if IsWindow(h) == 0 {
                    break;
                }
                let Some(s) = weak.upgrade() else { break };
                for id in actions.0.drain(..) {
                    if matches!(id, 107 | 108) {
                        widget::open(weak.clone(), commands.clone(), id == 108);
                    }
                    if matches!(id, 103..=105) {
                        let key_text = text(key);
                        let boss_checked =
                            SendMessageW(boss, BM_GETCHECK, 0, 0) == BST_CHECKED as isize;
                        let mut m = lock(&s);
                        if m.cyclopedia_tracker.pending {
                            continue;
                        }
                        m.cyclopedia_tracker.creature_key = key_text;
                        m.cyclopedia_tracker.boss = boss_checked;
                        let intent = match id {
                            103 => Intent::Add,
                            104 => Intent::Remove,
                            _ => Intent::Read,
                        };
                        super::super::input::enqueue(&mut m, &commands, intent);
                    }
                }
                let m = lock(&s);
                SetWindowTextW(out, w(&panel_text(&m.cyclopedia_tracker)).as_ptr());
                drop(m);
                std::thread::sleep(Duration::from_millis(100));
            }
            if let Some(s) = weak.upgrade() {
                lock(&s).cyclopedia_tracker.open = false
            }
            DestroyWindow(h);
        });
}
unsafe fn ctl(h: HWND, c: &str, t: &str, id: usize, b: [i32; 4], x: u32) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            w(c).as_ptr(),
            w(t).as_ptr(),
            WS_CHILD | WS_VISIBLE | x,
            b[0],
            b[1],
            b[2],
            b[3],
            h,
            id as _,
            null_mut(),
            null(),
        )
    }
}
unsafe fn text(h: HWND) -> String {
    unsafe {
        let n = GetWindowTextLengthW(h);
        let mut v = vec![0; usize::try_from(n).unwrap_or(0) + 1];
        let n = GetWindowTextW(h, v.as_mut_ptr(), n + 1);
        String::from_utf16_lossy(&v[..usize::try_from(n).unwrap_or(0)])
    }
}
