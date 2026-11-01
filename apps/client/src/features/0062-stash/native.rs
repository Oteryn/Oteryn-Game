//! Windows controls for the stash panel; all HWND access remains on its creating thread.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::stash::{Intent, panel_text, row_label};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Weak},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::*,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};
const LIST: usize = 101;
const AMOUNT: usize = 102;
const WITHDRAW: usize = 103;
const PREVIOUS: usize = 104;
const NEXT: usize = 105;
const REFRESH: usize = 106;
const ITEM: usize = 108;
const STOW: usize = 109;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
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
fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.stash.open {
            return;
        }
        m.stash.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("stash-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
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
            wide("Stash — Ctrl+S").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            620,
            560,
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
            LIST,
            [10, 10, 580, 300],
            LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        );
        let amount = control(
            hwnd,
            "EDIT",
            "1",
            AMOUNT,
            [10, 320, 90, 28],
            ES_NUMBER as u32 | WS_BORDER | WS_TABSTOP,
        );
        let withdraw = control(
            hwnd,
            "BUTTON",
            "Withdraw",
            WITHDRAW,
            [110, 320, 120, 28],
            WS_TABSTOP,
        );
        let previous = control(
            hwnd,
            "BUTTON",
            "Previous",
            PREVIOUS,
            [10, 358, 100, 28],
            WS_TABSTOP,
        );
        let next = control(
            hwnd,
            "BUTTON",
            "Next",
            NEXT,
            [120, 358, 100, 28],
            WS_TABSTOP,
        );
        let refresh = control(
            hwnd,
            "BUTTON",
            "Refresh",
            REFRESH,
            [230, 358, 100, 28],
            WS_TABSTOP,
        );
        let item = control(
            hwnd,
            "EDIT",
            "",
            ITEM,
            [340, 320, 120, 28],
            ES_NUMBER as u32 | WS_BORDER | WS_TABSTOP,
        );
        let stow = control(
            hwnd,
            "BUTTON",
            "Stow item handle",
            STOW,
            [470, 320, 120, 28],
            WS_TABSTOP,
        );
        let status = control(
            hwnd,
            "EDIT",
            "",
            107,
            [10, 396, 580, 115],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        if [
            list, amount, withdraw, previous, next, refresh, item, stow, status,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
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
            let mut mailbox = lock(&shared);
            for id in std::mem::take(&mut actions.0) {
                let intent = match id {
                    PREVIOUS => Some(Intent::Previous),
                    NEXT => Some(Intent::Next),
                    REFRESH => Some(Intent::Refresh),
                    STOW => {
                        let mut buffer = [0_u16; 21];
                        let length = GetWindowTextW(item, buffer.as_mut_ptr(), 21);
                        String::from_utf16_lossy(&buffer[..length as usize])
                            .parse()
                            .ok()
                            .and_then(oteryn_client_runtime::stash::ItemHandle::new)
                            .map(Intent::Stow)
                    }
                    WITHDRAW => {
                        let row = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0)).ok();
                        let mut b = [0u16; 11];
                        let n = GetWindowTextW(amount, b.as_mut_ptr(), 11);
                        let amount = String::from_utf16_lossy(&b[..n as usize]).parse().ok();
                        row.zip(amount)
                            .map(|(row, amount)| Intent::Withdraw { row, amount })
                    }
                    _ => None,
                };
                if let Some(intent) = intent
                    && let Some(command) =
                        oteryn_client_ui::stash::request(&mut mailbox.stash, intent)
                    && commands
                        .send(crate::combat_input::PlayCommand::Stash(command))
                        .is_err()
                {
                    mailbox.stash.pending = false;
                    mailbox.stash.feedback = Some("The session has ended".into());
                }
            }
            if held.as_ref() != Some(&mailbox.stash) {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for row in &mailbox.stash.rows {
                    SendMessageW(
                        list,
                        LB_ADDSTRING,
                        0,
                        wide(&row_label(row)).as_ptr() as isize,
                    );
                }
                SetWindowTextW(status, wide(&panel_text(&mailbox.stash)).as_ptr());
                let ready = mailbox.stash.selected && !mailbox.stash.pending;
                EnableWindow(withdraw, i32::from(ready && !mailbox.stash.rows.is_empty()));
                EnableWindow(previous, i32::from(ready && mailbox.stash.page > 0));
                EnableWindow(next, i32::from(ready && mailbox.stash.has_more));
                EnableWindow(refresh, i32::from(ready));
                EnableWindow(stow, i32::from(ready));
                held = Some(mailbox.stash.clone());
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(m) = weak.upgrade() {
        lock(&m).stash.open = false;
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    r: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            r[0],
            r[1],
            r[2],
            r[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
