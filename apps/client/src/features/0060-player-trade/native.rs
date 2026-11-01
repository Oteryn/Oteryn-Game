//! Minimal native projection; exact partner and item handles are supplied by world interaction.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::player_trade::{Intent, panel_text};
use std::{
    ptr::{null, null_mut},
    sync::Weak,
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::*,
};
const OFFER: usize = 5;
const ACCEPT: usize = 1;
const CANCEL: usize = 2;
const LOOK_OWN: usize = 3;
const LOOK_COUNTER: usize = 4;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !p.is_null() {
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
        if m.player_trade.open {
            return;
        }
        m.player_trade.open = true;
    }
    let weak = std::sync::Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("player-trade".into())
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
            wide("Player trade").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            560,
            420,
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
            10,
            [10, 10, 520, 220],
            WS_BORDER | WS_VSCROLL,
        );
        let offer = control(
            hwnd,
            "BUTTON",
            "Offer selected",
            OFFER,
            [10, 275, 150, 30],
            WS_TABSTOP,
        );
        let accept = control(
            hwnd,
            "BUTTON",
            "Accept",
            ACCEPT,
            [10, 240, 120, 30],
            WS_TABSTOP,
        );
        let cancel = control(
            hwnd,
            "BUTTON",
            "Cancel",
            CANCEL,
            [140, 240, 120, 30],
            WS_TABSTOP,
        );
        let own = control(
            hwnd,
            "BUTTON",
            "Look at yours",
            LOOK_OWN,
            [270, 240, 125, 30],
            WS_TABSTOP,
        );
        let counter = control(
            hwnd,
            "BUTTON",
            "Look at theirs",
            LOOK_COUNTER,
            [405, 240, 125, 30],
            WS_TABSTOP,
        );
        if [list, offer, accept, cancel, own, counter]
            .iter()
            .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut held = String::new();
        let mut message: MSG = std::mem::zeroed();
        loop {
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
            for action in std::mem::take(&mut actions.0) {
                let intent = match action {
                    OFFER => mailbox
                        .player_trade
                        .offer
                        .map(|(partner, item)| Intent::Offer { partner, item })
                        .or_else(|| {
                            mailbox.player_trade.feedback =
                                Some("Select a player and backpack item in the world view");
                            None
                        }),
                    ACCEPT => Some(Intent::Accept),
                    CANCEL => Some(Intent::Cancel),
                    LOOK_OWN => Some(Intent::LookOwn),
                    LOOK_COUNTER => Some(Intent::LookCounter),
                    _ => None,
                };
                if let Some(intent) = intent
                    && let Some(command) =
                        oteryn_client_ui::player_trade::request(&mut mailbox.player_trade, intent)
                {
                    let _ = commands.send(crate::combat_input::PlayCommand::PlayerTrade(command));
                }
            }
            let text = panel_text(&mailbox.player_trade);
            if text != held {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for line in text.split("\r\n") {
                    SendMessageW(list, LB_ADDSTRING, 0, wide(line).as_ptr() as isize);
                }
                held = text;
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
        lock(&m).player_trade.open = false;
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
