//! Player context menu and live inventory picker for the existing trade adapter.
#![allow(unsafe_code)]
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::player_trade::{Intent, status_text};
use std::{
    ptr::{null, null_mut},
    sync::Weak,
    time::Duration,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::*,
};
const OFFER: usize = 5;
const OPEN_CONTAINER: usize = 6;
const CLOSE: usize = 7;
const ACCEPT: usize = 1;
const CANCEL: usize = 2;
const LOOK_OWN: usize = 3;
const LOOK_COUNTER: usize = 4;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_CLOSE {
            let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !p.is_null() {
                (*p).0.push(CLOSE);
            }
            return 0;
        }
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
    if link.uses_shared_frontend() {
        return;
    }
    {
        let mut m = lock(&link.mailbox);
        m.player_trade.show_requested = false;
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
            wide(tr!("Player trade")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            560,
            630,
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
            [10, 10, 520, 180],
            WS_BORDER | WS_VSCROLL,
        );
        let items = control(
            hwnd,
            "LISTBOX",
            "",
            11,
            [10, 320, 520, 180],
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        let open = control(
            hwnd,
            "BUTTON",
            tr!("Open container"),
            OPEN_CONTAINER,
            [180, 510, 150, 30],
            WS_TABSTOP,
        );
        let offer = control(
            hwnd,
            "BUTTON",
            tr!("Offer selected"),
            OFFER,
            [10, 510, 150, 30],
            WS_TABSTOP,
        );
        let accept = control(
            hwnd,
            "BUTTON",
            tr!("Accept"),
            ACCEPT,
            [10, 240, 120, 30],
            WS_TABSTOP,
        );
        let cancel = control(
            hwnd,
            "BUTTON",
            tr!("Cancel"),
            CANCEL,
            [140, 240, 120, 30],
            WS_TABSTOP,
        );
        let own = control(
            hwnd,
            "BUTTON",
            tr!("Look at yours"),
            LOOK_OWN,
            [270, 240, 125, 30],
            WS_TABSTOP,
        );
        let counter = control(
            hwnd,
            "BUTTON",
            tr!("Look at theirs"),
            LOOK_COUNTER,
            [405, 240, 125, 30],
            WS_TABSTOP,
        );
        if [list, items, open, offer, accept, cancel, own, counter]
            .iter()
            .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut held = String::new();
        let mut held_rows = Vec::new();
        let mut close_requested = false;
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
            if !mailbox.player_trade.selected {
                break;
            }

            for action in std::mem::take(&mut actions.0) {
                if action == OFFER || action == OPEN_CONTAINER {
                    if oteryn_client_ui::player_trade::picker::rows(&mailbox.player_trade.items)
                        != held_rows
                    {
                        mailbox.player_trade.feedback =
                            Some(tr!("Inventory changed; choose the item again"));
                        continue;
                    }
                    let row = SendMessageW(items, LB_GETCURSEL, 0, 0);
                    if let Ok(row) = usize::try_from(row) {
                        if action == OPEN_CONTAINER {
                            super::super::input::open_container(
                                &mut mailbox.player_trade,
                                row,
                                &commands,
                            );
                            continue;
                        }
                        oteryn_client_ui::player_trade::picker::select_item(
                            &mut mailbox.player_trade,
                            row,
                        );
                    } else {
                        mailbox.player_trade.feedback =
                            Some(tr!("Choose an item from the inventory list"));
                        continue;
                    }
                }
                if action == CLOSE {
                    close_requested = true;
                    continue;
                }
                let command = if action == OFFER {
                    oteryn_client_ui::player_trade::picker::offer_selected(
                        &mut mailbox.player_trade,
                    )
                } else {
                    let intent = match action {
                        ACCEPT => Some(Intent::Accept),
                        CANCEL => Some(Intent::Cancel),
                        LOOK_OWN => Some(Intent::LookOwn),
                        LOOK_COUNTER => Some(Intent::LookCounter),
                        _ => None,
                    };
                    intent.and_then(|intent| {
                        oteryn_client_ui::player_trade::request(&mut mailbox.player_trade, intent)
                    })
                };
                if let Some(command) = command
                    && commands
                        .send(crate::combat_input::PlayCommand::PlayerTrade(command))
                        .is_err()
                {
                    mailbox.player_trade.pending = false;
                    mailbox.player_trade.feedback = Some(tr!("The session has ended"));
                }
            }
            let rows = oteryn_client_ui::player_trade::picker::rows(&mailbox.player_trade.items);
            if rows != held_rows {
                SendMessageW(items, LB_RESETCONTENT, 0, 0);
                for (source, item) in &rows {
                    let label = lfmt!(
                        "{source}: item {} x{} (subtype {})",
                        item.item_definition_ref,
                        item.count,
                        item.sub_type,
                        source = source
                    );
                    SendMessageW(items, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                }
                held_rows = rows;
            }
            let enabled = !mailbox.player_trade.pending && !mailbox.player_trade.items.pending;
            EnableWindow(offer, i32::from(enabled));
            EnableWindow(
                open,
                i32::from(enabled && mailbox.player_trade.items.can_open),
            );
            EnableWindow(
                accept,
                i32::from(
                    enabled
                        && mailbox
                            .player_trade
                            .state
                            .as_ref()
                            .is_some_and(|s| s.phase == 2 && !s.accepted[0]),
                ),
            );
            EnableWindow(cancel, i32::from(enabled));
            let text = status_text(&mailbox.player_trade);
            if text != held {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for line in text.split("\r\n") {
                    SendMessageW(list, LB_ADDSTRING, 0, wide(line).as_ptr() as isize);
                }
                held = text;
            }
            drop(mailbox);
            if close_requested && enabled {
                let mut mailbox = lock(&shared);
                if let Some(command) = oteryn_client_ui::player_trade::request(
                    &mut mailbox.player_trade,
                    Intent::Cancel,
                ) {
                    let _ = commands.send(crate::combat_input::PlayCommand::PlayerTrade(command));
                }
                break;
            }
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

pub(in crate::play) fn context_menu(link: &PlayLink, partner: oteryn_session::EntityRef) {
    if link.uses_shared_frontend() {
        return;
    }
    if !link.player_trade_panel().selected {
        return;
    }
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        AppendMenuW(
            menu,
            MF_STRING,
            1,
            wide(tr!("Trade with this player")).as_ptr(),
        );
        let mut point = std::mem::zeroed();
        windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            point.x,
            point.y,
            0,
            GetForegroundWindow(),
            null(),
        );
        DestroyMenu(menu);
        if chosen == 1 && link.player_trade_choose_partner(partner) {
            open_window(link);
        }
    }
}
