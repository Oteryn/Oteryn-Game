//! A table of the server's complete custody page, including unopened carried bags.
#![allow(unsafe_code)]
use super::controls::{control, read_text, wide};
use crate::{
    combat_input::PlayCommand,
    play::mailbox::{Mailbox, lock},
};
use oteryn_client_runtime::cyclopedia_character::items::Location;
use oteryn_client_ui::cyclopedia_character::{Intent, request};
use std::{
    ptr::{null, null_mut},
    sync::{Mutex, Weak},
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{Controls::EM_SETLIMITTEXT, WindowsAndMessaging::*},
};
type Shared = Weak<Mutex<Mailbox>>;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(h: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let a = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Actions;
            if !a.is_null() && (*a).0.len() < 16 {
                (*a).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(h, msg, w, l)
    }
}
pub(in crate::play) fn open(weak: Shared, commands: UnboundedSender<PlayCommand>) {
    let Some(shared) = weak.upgrade() else {
        return;
    };
    let generation = {
        let mut m = lock(&shared);
        let p = &mut m.cyclopedia_character;
        if !p.selected || p.items.open {
            return;
        }
        p.items.open = true;
        p.items.page = None;
        p.profile_generation
    };
    let failed = weak.clone();
    if std::thread::Builder::new()
        .name("character-item-summary".into())
        .spawn(move || run(weak, commands, generation))
        .is_err()
    {
        close(&failed, generation);
    }
}
fn run(weak: Shared, commands: UnboundedSender<PlayCommand>, generation: u64) {
    unsafe {
        // SAFETY: window handles and action storage remain on their creating thread.
        let h = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Your item summary").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            900,
            650,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if h.is_null() {
            close(&weak, generation);
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(h, GWLP_USERDATA, (&mut *actions as *mut Actions) as isize);
        SetWindowLongPtrW(h, GWLP_WNDPROC, proc as *const () as isize);
        let filter = control(
            h,
            "COMBOBOX",
            "",
            1,
            [10, 10, 200, 180],
            CBS_DROPDOWNLIST as u32,
        );
        for label in ["All locations", "Inventory", "Depot", "Stash", "Inboxes"] {
            SendMessageW(filter, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(filter, CB_SETCURSEL, 0, 0);
        let search = control(
            h,
            "EDIT",
            "",
            2,
            [225, 10, 640, 26],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(search, EM_SETLIMITTEXT, 128, 0);
        let header = control(
            h,
            "STATIC",
            "Definition                               Inventory     Depot     Stash     Inboxes",
            10,
            [10, 60, 860, 24],
            0,
        );
        let list = control(
            h,
            "LISTBOX",
            "",
            11,
            [10, 88, 860, 330],
            WS_BORDER | WS_VSCROLL | WS_HSCROLL | LBS_NOTIFY as u32 | LBS_USETABSTOPS as u32,
        );
        let tabs = [190_i32, 235, 280, 325];
        SendMessageW(list, LB_SETTABSTOPS, tabs.len(), tabs.as_ptr() as isize);
        SendMessageW(list, LB_SETHORIZONTALEXTENT, 1800, 0);
        let font = GetStockObject(ANSI_FIXED_FONT);
        SendMessageW(header, WM_SETFONT, font as usize, 1);
        SendMessageW(list, WM_SETFONT, font as usize, 1);
        let detail = control(
            h,
            "EDIT",
            "Select a row for its complete definition and quantities",
            12,
            [10, 430, 860, 75],
            WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        for (id, label, x) in [
            (3, "Previous page", 10),
            (4, "Next page", 180),
            (5, "Refresh", 350),
        ] {
            control(h, "BUTTON", label, id, [x, 525, 155, 28], WS_TABSTOP);
        }
        let status = control(h, "STATIC", "", 13, [10, 565, 860, 35], 0);
        let mut initialized = false;
        let mut painted = None;
        let mut shown = Vec::new();
        loop {
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if IsWindow(h) == 0 || commands.is_closed() {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let (items, pending, feedback) = {
                let mut m = lock(&shared);
                if m.cyclopedia_character_shutdown {
                    break;
                }
                let p = &mut m.cyclopedia_character;
                if p.profile_generation != generation || !p.selected {
                    break;
                }
                p.items.search = read_text(search);
                p.items.location = match SendMessageW(filter, CB_GETCURSEL, 0, 0) {
                    1 => Some(Location::Inventory),
                    2 => Some(Location::Depot),
                    3 => Some(Location::Stash),
                    4 => Some(Location::Inbox),
                    _ => None,
                };
                let mut intents: Vec<_> = actions
                    .0
                    .drain(..)
                    .filter_map(|id| match id {
                        3 => Some(Intent::ItemsPrevious),
                        4 => Some(Intent::ItemsNext),
                        5 => Some(Intent::Items),
                        _ => None,
                    })
                    .collect();
                if !initialized && !p.pending {
                    intents.push(Intent::Items);
                    initialized = true;
                }
                for intent in intents {
                    if let Some(command) = request(p, intent)
                        && commands.send(PlayCommand::Feature(command)).is_err()
                    {
                        p.pending = false;
                        p.items.pending_offset = None;
                        p.feedback = Some("The session has ended");
                    }
                }
                (p.items.clone(), p.pending, p.feedback)
            };
            let signature = (items.page.clone(), items.location, items.search.clone());
            if painted.as_ref() != Some(&signature) {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                shown = items.rows().into_iter().cloned().collect::<Vec<_>>();
                for row in &shown {
                    let q = row.quantities;
                    let text =
                        format!("{}\t{}\t{}\t{}\t{}", row.definition, q[0], q[1], q[2], q[3]);
                    SendMessageW(list, LB_ADDSTRING, 0, wide(&text).as_ptr() as isize);
                }
                painted = Some(signature);
            }
            let chosen = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                .ok()
                .and_then(|i| shown.get(i));
            let text = chosen.map_or_else(
                || "Select a row for its complete definition and quantities".into(),
                |r| {
                    format!(
                        "{}\r\nInventory: {} · Depot: {} · Stash: {} · Inboxes: {}",
                        r.definition,
                        r.quantities[0],
                        r.quantities[1],
                        r.quantities[2],
                        r.quantities[3]
                    )
                },
            );
            SetWindowTextW(detail, wide(&text).as_ptr());
            let page = items.page.as_ref().map_or(String::new(), |p| {
                format!(
                    "{}–{} of {} definitions · Filters apply to this server page",
                    p.offset + u16::from(!p.rows.is_empty()),
                    usize::from(p.offset) + p.rows.len(),
                    p.total
                )
            });
            SetWindowTextW(
                status,
                wide(&format!(
                    "{page}\r\n{}",
                    if pending {
                        "Loading"
                    } else {
                        feedback.unwrap_or("")
                    }
                ))
                .as_ptr(),
            );
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        DestroyWindow(h);
        close(&weak, generation);
    }
}
fn close(weak: &Shared, generation: u64) {
    if let Some(shared) = weak.upgrade() {
        let mut m = lock(&shared);
        if m.cyclopedia_character.profile_generation == generation {
            m.cyclopedia_character.items.open = false;
        }
    }
}
