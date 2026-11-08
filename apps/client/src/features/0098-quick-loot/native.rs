//! CP02 native controls. Each window owns all its HWNDs and closes with its session.
#![allow(
    unsafe_code,
    reason = "Win32 HWND and userdata access stays on the creating UI thread"
)]
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{
        Controls::EM_SETLIMITTEXT, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*,
    },
};
#[path = "native/projection.rs"]
mod projection;
#[path = "native/specs.rs"]
mod specs;
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
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
fn set_open(state: &mut oteryn_client_ui::quick_loot::Discovery, panel: u32, open: bool) -> bool {
    let current = match panel {
        98 => &mut state.loot.open,
        132 => &mut state.inspection.open,
        232 => &mut state.teams.open,
        285 => &mut state.history.open,
        _ => return false,
    };
    let previous = *current;
    *current = open;
    previous
}
pub(in crate::play) fn open_window(link: &PlayLink, panel: u32) {
    {
        let mut mailbox = lock(&link.mailbox);
        if set_open(&mut mailbox.discovery, panel, true) {
            return;
        }
        if panel == 285 {
            mailbox.store.open = true;
        }
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name(format!("discovery-{panel}"))
        .spawn(move || run(weak, commands, panel))
        .is_err()
        && let Some(shared) = failed.upgrade()
    {
        let mut mailbox = lock(&shared);
        set_open(&mut mailbox.discovery, panel, false);
        if panel == 285 {
            mailbox.store.open = false;
        }
    }
}
fn run(
    weak: Weak<Mutex<Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    panel: u32,
) {
    // SAFETY: controls copy UTF-16 strings; userdata outlives every message and is cleared before destruction.
    unsafe {
        let title = match panel {
            98 => "Quick loot — Ctrl+Alt+Q",
            132 => "Inspection — Ctrl+Alt+I",
            232 => "Social — Team Finder — Ctrl+Alt+T",
            _ => "Store — Ctrl+Alt+H",
        };
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(title).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            850,
            if panel == 232 { 950 } else { 860 },
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if !hwnd.is_null() {
            window(hwnd, &weak, &commands, panel);
        }
    }
    if let Some(shared) = weak.upgrade() {
        let mut mailbox = lock(&shared);
        set_open(&mut mailbox.discovery, panel, false);
        if panel == 285 {
            mailbox.store.open = false;
        }
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: (i32, i32, i32, i32),
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            rect.0,
            rect.1,
            rect.2,
            rect.3,
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
unsafe fn window(
    hwnd: HWND,
    weak: &Weak<Mutex<Mailbox>>,
    commands: &tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    panel: u32,
) {
    unsafe {
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let text = control(
            hwnd,
            "EDIT",
            "",
            100,
            (10, 10, 810, 230),
            WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        let rows = control(
            hwnd,
            "LISTBOX",
            "",
            101,
            (10, 250, 810, 100),
            WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        );
        let (buttons, fields) = specs::specs(panel);
        let mut inputs = Vec::new();
        let mut labels = Vec::new();
        for (i, (label, initial)) in fields.iter().enumerate() {
            let position = if panel == 232 && i >= 10 { i - 10 } else { i };
            let x = 10 + (position % 3) as i32 * 270;
            let y = 360 + (position / 3) as i32 * 60;
            labels.push(control(hwnd, "STATIC", label, 200 + i, (x, y, 260, 25), 0));
            let input = control(
                hwnd,
                "EDIT",
                initial,
                300 + i,
                (x, y + 25, 250, 25),
                WS_BORDER | WS_TABSTOP,
            );
            SendMessageW(input, EM_SETLIMITTEXT, 36, 0);
            inputs.push(input);
        }
        let mut controls = Vec::new();
        for (i, label) in buttons.iter().enumerate() {
            controls.push(control(
                hwnd,
                "BUTTON",
                label,
                i + 1,
                (
                    10 + (i % 3) as i32 * 270,
                    (if fields.len() > 12 {
                        680
                    } else if fields.len() > 9 {
                        620
                    } else {
                        550
                    }) + (i / 3) as i32 * 42,
                    260,
                    36,
                ),
                WS_TABSTOP,
            ));
        }
        if panel == 285 {
            control(
                hwnd,
                "STATIC",
                "Store tabs: Catalogue / Purchase history",
                400,
                (10, 360, 800, 30),
                0,
            );
        }
        let mut held = String::new();
        let mut held_rows = Vec::<String>::new();
        while IsWindow(hwnd) != 0 && !commands.is_closed() {
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
            let ended = {
                let mailbox = lock(&shared);
                mailbox.quest_shutdown
            };
            if ended {
                break;
            }
            // This temporary facade borrows the live session; running PlayLink::drop
            // here would shut the session down after the first panel repaint.
            let mut link = std::mem::ManuallyDrop::new(PlayLink {
                mailbox: shared,
                commands: commands.clone(),
            });
            for action in std::mem::take(&mut actions.0) {
                let values: Vec<String> = inputs
                    .iter()
                    .map(|input| {
                        let mut value = [0u16; 37];
                        let len = GetWindowTextW(*input, value.as_mut_ptr(), 37);
                        String::from_utf16_lossy(&value[..len.max(0) as usize])
                    })
                    .collect();
                let row = usize::try_from(SendMessageW(rows, LB_GETCURSEL, 0, 0)).unwrap_or(0);
                if panel == 285
                    && (action == 3
                        || ((action == 1 || action == 2)
                            && link.store_panel().page
                                == oteryn_client_ui::store::StorePage::Catalogue))
                {
                    let command = {
                        let mut mailbox = lock(&link.mailbox);
                        if mailbox.store.category.is_empty() {
                            mailbox.store.category = "featured".into();
                        }
                        if action != 2 {
                            mailbox.store.catalogue = None;
                        }
                        oteryn_client_ui::store::catalogue(&mut mailbox.store)
                    };
                    if let Some(command) = command {
                        link.discovery_send(command);
                    }
                } else if panel == 285 && action == 5 {
                    if let [offer, from, until] = values.as_slice() {
                        link.purchase_history_filter(offer, from, until);
                    }
                } else if panel == 285 && action == 4 {
                    lock(&link.mailbox).store.page = oteryn_client_ui::store::StorePage::History;
                    link.purchase_history(false);
                } else {
                    link.discovery_action(panel, action, &values, row);
                }
            }
            let state = link.discovery_panel();
            let store = link.store_panel();
            if panel == 232 {
                for (i, (label, input)) in labels.iter().zip(&inputs).enumerate() {
                    let show = if (i < 10) == state.teams.applicants {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    };
                    ShowWindow(*label, show);
                    ShowWindow(*input, show);
                }
            }
            let (display, names, available) = projection::project(panel, &state, &store);
            if display != held {
                SetWindowTextW(text, wide(&display).as_ptr());
                held = display;
            }
            if names != held_rows {
                SendMessageW(rows, LB_RESETCONTENT, 0, 0);
                for name in &names {
                    SendMessageW(rows, LB_ADDSTRING, 0, wide(name).as_ptr() as isize);
                }
                if !names.is_empty() {
                    SendMessageW(rows, LB_SETCURSEL, 0, 0);
                }
                held_rows = names;
            }
            for (i, button) in controls.iter().enumerate() {
                let enable = if panel == 98 && (1..5).contains(&i) {
                    available && !state.loot.managed
                } else if panel == 98 && (7..=8).contains(&i) {
                    available && state.loot.managed
                } else if panel == 232 && (8..10).contains(&i) {
                    state.teams.applicants && state.leader.selected && !state.leader.pending
                } else if panel == 232 && (3..8).contains(&i) {
                    available && (i >= 5) == state.teams.applicants
                } else if panel == 232 && i >= 10 {
                    available && !state.teams.applicants
                } else if panel == 232 && i == 2 {
                    available && state.teams.next().is_some()
                } else if panel == 285 && i >= 2 {
                    store.selected || state.history.selected
                } else if panel == 285
                    && i == 0
                    && store.page == oteryn_client_ui::store::StorePage::Catalogue
                {
                    store.selected && !store.pending
                } else if panel == 285 && i == 1 {
                    match store.page {
                        oteryn_client_ui::store::StorePage::Catalogue => {
                            store.selected
                                && !store.pending
                                && store
                                    .catalogue
                                    .as_ref()
                                    .is_some_and(|p| !p.next_page_cursor.is_empty())
                        }
                        _ => {
                            available
                                && state
                                    .history
                                    .page
                                    .as_ref()
                                    .is_some_and(|p| !p.next_page_cursor.is_empty())
                        }
                    }
                } else {
                    available
                };
                EnableWindow(*button, i32::from(enable));
            }
            // Release both cloned handles without invoking the owning facade's destructor.
            std::ptr::drop_in_place(&mut link.mailbox);
            std::ptr::drop_in_place(&mut link.commands);
            MsgWaitForMultipleObjects(0, null(), 0, 50, QS_ALLINPUT);
        }
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
    }
}
