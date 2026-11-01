//! Minimal native market list and action controls.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::market::{Intent, panel_text, row_text};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !p.is_null() && (*p).0.len() < 16 {
                (*p).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(value: &str) -> Vec<u16> {
    value
        .replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.market.open {
            return;
        }
        m.market.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("market-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
        && let Some(m) = failed.upgrade()
    {
        lock(&m).market.open = false;
    }
}
fn run(
    weak: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Market").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
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
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let list = control(
            hwnd,
            "LISTBOX",
            "",
            1,
            [10, 10, 680, 280],
            WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        );
        for (id, label) in [
            (2, "Browse"),
            (3, "My offers"),
            (4, "History"),
            (5, "Create offer"),
            (6, "Cancel offer"),
            (7, "Accept offer"),
            (8, "Refresh"),
        ] {
            control(
                hwnd,
                "BUTTON",
                label,
                id,
                [10 + (id as i32 - 2) * 95, 300, 90, 30],
                WS_TABSTOP,
            );
        }
        let status = control(hwnd, "STATIC", "", 9, [10, 345, 680, 120], 0);
        let mut held = None;
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
            let mut m = lock(&shared);
            if m.market_shutdown {
                break;
            }
            for id in std::mem::take(&mut actions.0) {
                let intent = match id {
                    2 => Some(Intent::Browse),
                    3 => Some(Intent::OwnOffers),
                    4 => Some(Intent::History),
                    5 => Some(Intent::CreateOffer),
                    6 => Some(Intent::CancelOffer),
                    7 => Some(Intent::AcceptOffer),
                    8 => Some(Intent::Refresh),
                    1 => {
                        m.market.selected_offer =
                            usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                                .ok()
                                .and_then(|i| m.market.rows.get(i))
                                .map(|r| r.offer);
                        None
                    }
                    _ => None,
                };
                if let Some(intent) = intent {
                    super::super::input::enqueue(&mut m, &commands, intent);
                }
            }
            if held.as_ref() != Some(&m.market) {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for row in &m.market.rows {
                    SendMessageW(
                        list,
                        LB_ADDSTRING,
                        0,
                        wide(&row_text(row)).as_ptr() as isize,
                    );
                }
                SetWindowTextW(status, wide(&panel_text(&m.market)).as_ptr());
                held = Some(m.market.clone());
            }
            drop(m);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(m) = weak.upgrade() {
        lock(&m).market.open = false;
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    b: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            b[0],
            b[1],
            b[2],
            b[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
