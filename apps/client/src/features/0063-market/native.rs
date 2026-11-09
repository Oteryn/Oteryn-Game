//! Native market form, paged server lists and trade controls.
#![allow(unsafe_code)]
use crate::play::mailbox::{SharedMailbox, lock};
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
            [10, 85, 680, 280],
            WS_BORDER | WS_VSCROLL | WS_TABSTOP | LBS_NOTIFY as u32,
        );
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
        let status = control(hwnd, "STATIC", "", 9, [10, 455, 680, 110], 0);
        control(
            hwnd,
            "BUTTON",
            "Side: all / buy / sell",
            13,
            [490, 415, 190, 28],
            WS_TABSTOP,
        );
        let mut displayed_rows: Vec<usize> = Vec::new();
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
                    2 => Some(Intent::Browse),
                    3 => Some(Intent::OwnOffers),
                    4 => Some(Intent::History),
                    5 => Some(Intent::CreateOffer),
                    6 => Some(Intent::CancelOffer),
                    7 => Some(Intent::AcceptOffer),
                    8 => Some(Intent::Refresh),
                    10 => Some(Intent::PreviousPage),
                    11 => Some(Intent::NextPage),
                    13 => {
                        m.market.side_filter = (m.market.side_filter + 1) % 3;
                        m.market.selected_offer = None;
                        None
                    }
                    12 => {
                        SetWindowTextW(item, wide("").as_ptr());
                        Some(Intent::Wares)
                    }
                    1 => {
                        m.market.selected_offer =
                            usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                                .ok()
                                .and_then(|i| displayed_rows.get(i))
                                .and_then(|i| m.market.rows.get(*i))
                                .map(|r| r.offer);
                        if let Some(row) = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                            .ok()
                            .and_then(|i| displayed_rows.get(i))
                            .and_then(|i| m.market.rows.get(*i))
                        {
                            SetWindowTextW(item, wide(&row.definition.to_string()).as_ptr());
                            if row.price > 0 {
                                SetWindowTextW(price, wide(&row.price.to_string()).as_ptr());
                            }
                        }
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
                displayed_rows = oteryn_client_ui::market::visible_rows(&m.market);
                for index in &displayed_rows {
                    let row = &m.market.rows[*index];
                    SendMessageW(
                        list,
                        LB_ADDSTRING,
                        0,
                        wide(&row_text(row)).as_ptr() as isize,
                    );
                }
                if let Some(index) = displayed_rows
                    .iter()
                    .position(|index| Some(m.market.rows[*index].offer) == m.market.selected_offer)
                {
                    SendMessageW(list, LB_SETCURSEL, index, 0);
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

unsafe fn read(hwnd: HWND) -> String {
    let mut buffer = [0u16; 64];
    let length = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}
