//! Native market form, paged server lists and trade controls.
#![allow(unsafe_code)]
use crate::play::mailbox::{SharedMailbox, lock};
use oteryn_client_ui::market::{Intent, panel_text};
#[path = "view/catalogue_native.rs"]
mod catalogue;
#[path = "view/table_native.rs"]
mod table;
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
            if !p.is_null()
                && (*p).0.len() < 16
                && (w >> 16 == BN_CLICKED as usize
                    || w >> 16 == LBN_SELCHANGE as usize
                    || ((20..=22).contains(&(w & 0xffff)) && w >> 16 == EN_CHANGE as usize))
            {
                (*p).0.push(w & 0xffff);
            }
            return 0;
        }
        if msg == WM_NOTIFY
            && let Some(id) = table::selection_notification(l)
        {
            let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !p.is_null() && (*p).0.len() < 16 {
                (*p).0.push(id);
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
pub(in crate::play) fn open_window(
    mailbox: &SharedMailbox,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
) {
    {
        let mut m = lock(mailbox);
        if m.market.open || m.market_shutdown {
            return;
        }
        m.market.open = true;
    }
    let weak = Arc::downgrade(mailbox);
    let failed = weak.clone();
    let commands = commands.clone();
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
            wide(tr!("Market")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
            780,
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
        let Some(mut tables) = table::Offers::create(hwnd) else {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        };
        let item = control(
            hwnd,
            "EDIT",
            "",
            20,
            [80, 10, 100, 25],
            WS_BORDER | WS_TABSTOP,
        );
        let amount = control(
            hwnd,
            "EDIT",
            "1",
            21,
            [270, 10, 100, 25],
            WS_BORDER | WS_TABSTOP,
        );
        let price = control(
            hwnd,
            "EDIT",
            "1",
            22,
            [475, 10, 170, 25],
            WS_BORDER | WS_TABSTOP,
        );
        control(hwnd, "STATIC", tr!("Item ID"), 23, [10, 10, 65, 25], 0);
        control(hwnd, "STATIC", tr!("Amount"), 24, [195, 10, 70, 25], 0);
        control(hwnd, "STATIC", tr!("Unit price"), 25, [390, 10, 80, 25], 0);
        let buy = control(
            hwnd,
            "BUTTON",
            tr!("Buy (unchecked: sell)"),
            26,
            [10, 45, 240, 28],
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        let anonymous = control(
            hwnd,
            "BUTTON",
            tr!("Anonymous"),
            27,
            [270, 45, 150, 28],
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        for (id, label) in [
            (2, tr!("Browse")),
            (3, tr!("My offers")),
            (4, tr!("History")),
            (5, tr!("Create offer")),
            (6, tr!("Cancel offer")),
            (7, tr!("Accept offer")),
            (8, tr!("Refresh")),
        ] {
            control(
                hwnd,
                "BUTTON",
                label,
                id,
                [10 + (id as i32 - 2) * 95, 375, 90, 30],
                WS_TABSTOP,
            );
        }
        control(
            hwnd,
            "BUTTON",
            tr!("Previous page"),
            10,
            [10, 415, 150, 28],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            tr!("Next page"),
            11,
            [170, 415, 150, 28],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            tr!("Wares"),
            12,
            [330, 415, 150, 28],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            "Side: all / buy / sell",
            13,
            [490, 415, 190, 28],
            WS_TABSTOP,
        );
        let stashback = control(
            hwnd, "BUTTON", tr!("Stashback"), 6191,
            [440, 45, 230, 28], WS_TABSTOP,
        );
        let status = control(
            hwnd,
            "EDIT",
            "",
            9,
            [10, 455, 680, 220],
            WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        control(hwnd,"STATIC","Coin market and coin transactions: Ctrl+Alt+F8",26,[10,690,680,20],0);

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
            if m.market_shutdown || !m.market.open {
                break;
            }
            ShowWindow(stashback, if m.stash.return_from_market { SW_SHOW } else { SW_HIDE });
            let mut return_to_stash = false;
            let mut browse_catalogue = false;
            for id in std::mem::take(&mut actions.0) {
                if (20..=22).contains(&id) && read(item).trim().is_empty() {
                    continue;
                }
                if matches!(id, 2 | 5 | 7 | 20..=22 | 26 | 27)
                    && !oteryn_client_ui::market::edit_details(
                        &mut m.market,
                        &read(item),
                        &read(amount),
                        &read(price),
                        SendMessageW(buy, BM_GETCHECK, 0, 0) == 0,
                        SendMessageW(anonymous, BM_GETCHECK, 0, 0) != 0,
                    )
                {
                    continue;
                }
                let intent = match id {
                    6191 => {
                        return_to_stash = true;
                        None
                    }
                    2 => Some(Intent::Browse),
                    3 => Some(Intent::OwnOffers),
                    4 => Some(Intent::History),
                    5 => Some(Intent::CreateOffer),
                    6 => Some(Intent::CancelOffer),
                    7 => Some(Intent::AcceptOffer),
                    8 => Some(Intent::Refresh),
                    10 => Some(Intent::PreviousPage),
                    11 => Some(Intent::NextPage),
                    12 => {
                        browse_catalogue = true;
                        None
                    }
                    13 => {
                        m.market.side_filter = (m.market.side_filter + 1) % 3;
                        m.market.selected_offer = None;
                        None
                    }
                    1 | 14 => {
                        if let Some(row) = tables.selected(&m.market.rows, id) {
                            let (offer, definition, unit_price) =
                                (row.offer, row.definition, row.price);
                            m.market.selected_offer = Some(offer);
                            SetWindowTextW(item, wide(&definition.to_string()).as_ptr());
                            if unit_price > 0 {
                                SetWindowTextW(price, wide(&unit_price.to_string()).as_ptr());
                            }
                        }
                        None
                    }
                    _ => None,
                };
                if let Some(intent) = intent {
                    if matches!(
                        intent,
                        Intent::Browse | Intent::Refresh | Intent::NextPage | Intent::PreviousPage
                    ) && (intent == Intent::Browse
                        || m.market.view == oteryn_client_ui::market::MarketView::Browse)
                        && m.market.definition > 0
                    {
                        crate::play::feature_0063_market::protocol::details::enqueue_book(
                            &mut m, &commands, intent,
                        );
                    } else {
                        super::super::input::enqueue(&mut m, &commands, intent);
                    }
                }
            }
            if held.as_ref() != Some(&m.market) {
                tables.refresh(&m.market);
                SetWindowTextW(status, wide(&panel_text(&m.market)).as_ptr());
                held = Some(m.market.clone());
            }
            drop(m);
            if return_to_stash {
                DestroyWindow(hwnd);
                close(&weak);
                super::super::input::stashback(&shared, &commands);
                return;
            }
            if browse_catalogue {
                catalogue::open(hwnd, item, &weak, &commands);
            }
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

unsafe fn read(hwnd: HWND) -> String {
    let mut buffer = [0u16; 64];
    let length = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}
