//! Native depot and carried-item grids. HWNDs belong to this panel thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::depot::{DepotPanel, Intent, deposit_items, panel_text};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::*,
};

#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: only this thread owns userdata and handles; userdata outlives the window.
    unsafe {
        if msg == WM_COMMAND {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !ptr.is_null() && w >> 16 == BN_CLICKED as usize {
                (*ptr).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "�").encode_utf16().chain(Some(0)).collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.depot.open || m.depot_shutdown {
            return;
        }
        m.depot.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("depot-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}
fn close(weak: &Weak<Mutex<Mailbox>>) {
    if let Some(m) = weak.upgrade() {
        lock(&m).depot.open = false;
    }
}
fn run(weak: Weak<Mutex<Mailbox>>, commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>) {
    // SAFETY: all HWNDs are created, serviced and destroyed on this thread.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Depot — Ctrl+Shift+L")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            940,
            680,
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
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
        let boxes: Vec<_> = (0..17)
            .map(|i| {
                control(
                    hwnd,
                    "BUTTON",
                    &format!("{}", i + 1),
                    10 + i,
                    [10 + i as i32 * 52, 10, 50, 28],
                )
            })
            .collect();
        let mut depot = Vec::new();
        let mut carried = Vec::new();
        for i in 0..32 {
            let x = (i % 4) as i32 * 110;
            let y = (i / 4) as i32 * 48;
            depot.push(control(
                hwnd,
                "BUTTON",
                "",
                100 + i,
                [10 + x, 100 + y, 106, 44],
            ));
            carried.push(control(
                hwnd,
                "BUTTON",
                "",
                200 + i,
                [470 + x, 100 + y, 106, 44],
            ));
        }
        let controls = [
            control(hwnd, "BUTTON", tr!("Previous page"), 30, [10, 44, 140, 28]),
            control(hwnd, "BUTTON", tr!("Next page"), 31, [155, 44, 140, 28]),
            control(hwnd, "BUTTON", tr!("Refresh"), 32, [300, 44, 140, 28]),
            control(
                hwnd,
                "BUTTON",
                tr!("Carried previous"),
                33,
                [470, 44, 160, 28],
            ),
            control(hwnd, "BUTTON", tr!("Carried next"), 34, [635, 44, 160, 28]),
            control(
                hwnd,
                "BUTTON",
                tr!("Open bag: select a carried cell"),
                35,
                [470, 490, 440, 28],
            ),
        ];
        let labels = [
            control(hwnd, "STATIC", tr!("Depot items"), 301, [10, 76, 440, 20]),
            control(
                hwnd,
                "STATIC",
                tr!("Inventory / carried bags"),
                302,
                [470, 76, 440, 20],
            ),
        ];
        let stash = control(hwnd, "BUTTON", tr!("Stash items…"), 36, [10, 490, 215, 28]);
        let market = control(
            hwnd,
            "BUTTON",
            tr!("Market (Ctrl+M)"),
            37,
            [235, 490, 215, 28],
        );
        let status = control(hwnd, "STATIC", "", 300, [10, 530, 900, 100]);
        if boxes
            .iter()
            .chain(&depot)
            .chain(&carried)
            .chain(&controls)
            .chain(&labels)
            .chain([&status, &stash, &market])
            .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut held: Option<DepotPanel> = None;
        let mut carried_page = 0usize;
        let mut open_bag = false;
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
            let mut m = lock(&shared);
            if m.depot_shutdown {
                break;
            }
            for id in std::mem::take(&mut actions.0) {
                if id == 36 {
                    drop(m);
                    PlayLink::stash_open(&shared, &commands);
                    m = lock(&shared);
                    continue;
                }
                if id == 37 {
                    drop(m);
                    crate::play::feature_0063_market::input::open_from_depot(&shared, &commands);
                    m = lock(&shared);
                    continue;
                }
                if m.depot.pending || ((100..=231).contains(&id) && held.as_ref() != Some(&m.depot))
                {
                    continue;
                }
                let r = m.depot.reply.as_ref();
                let b = r.map_or(1, |r| r.box_number);
                let page = r.map_or(0, |r| r.page);
                if id == 35 {
                    open_bag = !open_bag;
                    SetWindowTextW(
                        controls[5],
                        wide(if open_bag {
                            tr!("Opening bag: select a carried cell (click to cancel)")
                        } else {
                            tr!("Open bag: select a carried cell")
                        })
                        .as_ptr(),
                    );
                    continue;
                }
                if open_bag && (200..=231).contains(&id) {
                    if let Some(item) = deposit_items(&m.depot).get(carried_page * 32 + id - 200) {
                        super::super::input::enqueue_container(&mut m, &commands, item.handle);
                    }
                    open_bag = false;
                    SetWindowTextW(
                        controls[5],
                        wide(tr!("Open bag: select a carried cell")).as_ptr(),
                    );
                    continue;
                }
                let intent = match id {
                    10..=26 => Some(Intent::Browse {
                        box_number: (id - 9) as u8,
                        page: 0,
                    }),
                    30 if page > 0 => Some(Intent::Browse {
                        box_number: b,
                        page: page - 1,
                    }),
                    31 if r.is_some_and(|r| (page + 1) * 32 < r.total) => Some(Intent::Browse {
                        box_number: b,
                        page: page + 1,
                    }),
                    32 => Some(Intent::Browse {
                        box_number: b,
                        page,
                    }),
                    33 => {
                        carried_page = carried_page.saturating_sub(1);
                        held = None;
                        None
                    }
                    34 if (carried_page + 1) * 32 < deposit_items(&m.depot).len() => {
                        carried_page += 1;
                        held = None;
                        None
                    }
                    100..=131 => Some(Intent::Withdraw(id - 100)),
                    200..=231 => deposit_items(&m.depot)
                        .get(carried_page * 32 + id - 200)
                        .map(|e| Intent::Deposit(e.handle)),
                    _ => None,
                };
                if let Some(intent) = intent {
                    super::super::input::enqueue(&mut m, &commands, intent);
                }
            }
            if held.as_ref() != Some(&m.depot) {
                let p = &m.depot;
                let items = deposit_items(p);
                carried_page = carried_page.min(items.len().saturating_sub(1) / 32);
                for i in 0..32 {
                    let entry = p.reply.as_ref().and_then(|r| r.entries.get(i));
                    let label = entry.map_or_else(String::new, |e| {
                        lfmt!("Item {}\n×{}", e.definition, e.count)
                    });
                    SetWindowTextW(depot[i], wide(&label).as_ptr());
                    EnableWindow(
                        depot[i],
                        i32::from(
                            !p.pending
                                && entry.is_some()
                                && p.reply.as_ref().is_some_and(|r| r.town.is_some()),
                        ),
                    );
                    let entry = items.get(carried_page * 32 + i);
                    let label = entry.map_or_else(String::new, |e| {
                        lfmt!("Item {}\n×{}", e.item_definition_ref, e.count)
                    });
                    SetWindowTextW(carried[i], wide(&label).as_ptr());
                    EnableWindow(
                        carried[i],
                        i32::from(
                            !p.pending
                                && entry.is_some()
                                && p.reply.as_ref().is_some_and(|r| r.town.is_some()),
                        ),
                    );
                }
                for h in boxes.iter().chain(&controls) {
                    EnableWindow(*h, i32::from(p.selected && !p.pending));
                }
                SetWindowTextW(status, wide(&panel_text(p)).as_ptr());
                held = Some(p.clone());
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
unsafe fn control(parent: HWND, class: &str, text: &str, id: usize, b: [i32; 4]) -> HWND {
    // SAFETY: parent is live and CreateWindowExW copies the strings synchronously.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_MULTILINE as u32,
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
