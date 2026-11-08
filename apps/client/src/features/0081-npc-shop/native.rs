//! Native controls use mailbox/channel handles, never an owning temporary PlayLink.
#![allow(unsafe_code)]
use crate::{
    combat_input::PlayCommand,
    play::mailbox::{Mailbox, SharedMailbox, lock},
};
use oteryn_client_ui::npc_shop::Quote;
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::*,
};
fn wide(value: &str) -> Vec<u16> {
    value
        .replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata is owned by this window's thread through destruction.
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && w >> 16 == BN_CLICKED as usize && (*actions).0.len() < 16 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(in crate::play) fn open_window(
    shared: &SharedMailbox,
    commands: &UnboundedSender<PlayCommand>,
) {
    {
        let mut mailbox = lock(shared);
        if !mailbox.npc_shop.open || mailbox.npc_shop_window || mailbox.dialogue_shutdown {
            return;
        }
        mailbox.npc_shop_window = true;
    }
    let weak = Arc::downgrade(shared);
    let fallback = weak.clone();
    let commands = commands.clone();
    if std::thread::Builder::new()
        .name("npc-shop".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&fallback);
    }
}
fn close(weak: &Weak<Mutex<Mailbox>>) {
    if let Some(shared) = weak.upgrade() {
        let mut mailbox = lock(&shared);
        mailbox.npc_shop_window = false;
        mailbox.npc_shop.open = false;
    }
}
fn run(weak: Weak<Mutex<Mailbox>>, commands: UnboundedSender<PlayCommand>) {
    // SAFETY: HWNDs and userdata remain on their creating thread until destroyed.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("NPC shop").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            760,
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
            100,
            (10, 10, 720, 280),
            WS_BORDER | WS_VSCROLL | LBS_NOTIFY as u32,
        );
        control(hwnd, "STATIC", "Offers (1–100):", 0, (10, 300, 130, 25), 0);
        let amount = control(
            hwnd,
            "EDIT",
            "1",
            101,
            (140, 300, 110, 28),
            WS_BORDER | ES_NUMBER as u32,
        );
        SendMessageW(amount, EM_SETLIMITTEXT, 10, 0);
        let look = control(hwnd, "BUTTON", "Look", 103, (270, 335, 130, 28), 0);
        let sell_all = control(hwnd, "BUTTON", "Sell all", 104, (410, 335, 130, 28), 0);
        let trade = control(hwnd, "BUTTON", "Buy / Sell", 102, (270, 300, 130, 30), 0);
        let total = control(hwnd, "STATIC", "", 0, (410, 300, 320, 35), 0);
        let status = control(
            hwnd,
            "EDIT",
            "",
            0,
            (10, 370, 720, 130),
            WS_BORDER | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        if [list, amount, look, sell_all, trade, total, status]
            .iter()
            .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let filter = control(hwnd, "EDIT", "", 105, (10, 335, 140, 28), WS_BORDER);
        SendMessageW(filter, EM_SETLIMITTEXT, 80, 0);
        let side = control(
            hwnd,
            "BUTTON",
            "All / Buy / Sell",
            106,
            (155, 335, 110, 28),
            0,
        );
        let mut side_filter = None;
        let mut rows: Vec<Quote> = Vec::new();
        let mut labels = Vec::new();
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let mailbox = lock(&shared);
            if mailbox.dialogue_shutdown || !mailbox.npc_shop.open {
                break;
            }
            let shop = &mailbox.npc_shop.live;
            let selected = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                .ok()
                .and_then(|i| rows.get(i))
                .copied();
            let mut buffer = [0u16; 12];
            let len = GetWindowTextW(amount, buffer.as_mut_ptr(), 12);
            let quantity = oteryn_client_ui::npc_shop::parse_amount(&String::from_utf16_lossy(
                &buffer[..len.max(0) as usize],
            ));
            // Resolve actions against the identities actually displayed before any redraw.
            let actions = std::mem::take(&mut actions.0);
            let clicked = actions.contains(&102);
            let looked = actions.contains(&103);
            let sold_all = actions.contains(&104);
            if actions.contains(&106) {
                use oteryn_client_ui::npc_shop::NpcTradeSide;
                side_filter = match side_filter {
                    None => Some(NpcTradeSide::Buy),
                    Some(NpcTradeSide::Buy) => Some(NpcTradeSide::Sell),
                    Some(NpcTradeSide::Sell) => None,
                };
                SetWindowTextW(side, wide(&format!("{:?}", side_filter)).as_ptr());
            }
            let mut query = [0u16; 81];
            let query_len = GetWindowTextW(filter, query.as_mut_ptr(), 81).max(0) as usize;
            let live_rows =
                shop.filtered_quotes(side_filter, &String::from_utf16_lossy(&query[..query_len]));
            let live_labels: Vec<String> = live_rows.iter().map(|q| shop.quote_text(*q)).collect();
            if live_rows != rows || live_labels != labels {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for label in &live_labels {
                    SendMessageW(list, LB_ADDSTRING, 0, wide(label).as_ptr() as isize);
                }
                if let Some(index) =
                    selected.and_then(|key| live_rows.iter().position(|q| *q == key))
                {
                    SendMessageW(list, LB_SETCURSEL, index, 0);
                }
                rows = live_rows;
                labels = live_labels;
            }
            let valid = selected.filter(|key| rows.contains(key));
            SetWindowTextW(
                total,
                wide(&match (valid, quantity) {
                    (Some(key), Some(n)) => shop.total(key, n),
                    _ => "Select a quote and positive amount".into(),
                })
                .as_ptr(),
            );
            SetWindowTextW(
                status,
                wide(
                    &format!(
                        "{}\n{}",
                        shop.text(),
                        oteryn_client_ui::npc_shop::panel_text(&mailbox.npc_shop)
                    )
                    .replace('\n', "\r\n"),
                )
                .as_ptr(),
            );
            windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                trade,
                i32::from(
                    shop.selected
                        && !shop.pending
                        && !mailbox.npc_shop.pending
                        && valid.is_some()
                        && valid
                            .zip(quantity)
                            .is_some_and(|(key, n)| shop.valid_amount(key, n)),
                ),
            );
            windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                look,
                i32::from(
                    mailbox.npc_shop.selected
                        && !shop.pending
                        && !mailbox.npc_shop.pending
                        && valid.is_some(),
                ),
            );
            windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                sell_all,
                i32::from(
                    mailbox.npc_shop.selected
                        && !shop.pending
                        && !mailbox.npc_shop.pending
                        && valid.is_some_and(|key| {
                            key.side == oteryn_client_ui::npc_shop::NpcTradeSide::Sell
                                && key.count == 1
                        }),
                ),
            );
            drop(mailbox);
            if sold_all && let Some(key) = valid {
                super::super::input::offer_request(&shared, &commands, key, true);
            }
            if looked && let Some(key) = valid {
                super::super::input::offer_request(&shared, &commands, key, false);
            }
            if clicked && let (Some(key), Some(n)) = (valid, quantity) {
                super::super::input::trade_request(&shared, &commands, key, n);
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    bounds: (i32, i32, i32, i32),
    style: u32,
) -> HWND {
    // SAFETY: temporary strings are valid for this synchronous call.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | style,
            bounds.0,
            bounds.1,
            bounds.2,
            bounds.3,
            parent,
            id as *mut _,
            null_mut(),
            null(),
        )
    }
}
