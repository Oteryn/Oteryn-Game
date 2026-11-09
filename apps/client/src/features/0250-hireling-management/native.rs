//! Native selection of owned hirelings, entitled outfits and their four colors.
#![allow(unsafe_code)] // All HWNDs and callback state stay on their creating thread.
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::hireling_management::{Intent, outfits, request};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && matches!((w >> 16) & 0xffff, 0 | 1) {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 32 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut m = lock(&link.mailbox);
        if m.hireling_management.open {
            return;
        }
        m.hireling_management.open = true;
    }
    link.hireling_management_request(Intent::List);
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("hireling-management-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&rollback);
    }
}
fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Hireling management")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            620,
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
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let identities = control(hwnd, "COMBOBOX", "", 101, [10, 10, 280, 240]);
        let looks = control(hwnd, "COMBOBOX", "", 102, [300, 10, 280, 240]);
        control(
            hwnd,
            "STATIC",
            tr!("Colors: head / body / legs / feet (0-255)"),
            103,
            [10, 45, 400, 24],
        );
        let colors = [0, 1, 2, 3].map(|i| {
            control(
                hwnd,
                "EDIT",
                "0",
                110 + i as usize,
                [10 + i * 80, 75, 70, 26],
            )
        });
        control(hwnd, "BUTTON", tr!("Save outfit"), 120, [10, 110, 140, 30]);
        control(
            hwnd,
            "BUTTON",
            tr!("Return to lamp"),
            121,
            [160, 110, 140, 30],
        );
        control(hwnd, "BUTTON", tr!("Refresh"), 122, [310, 110, 140, 30]);
        control(
            hwnd,
            "STATIC",
            "New hireling name (3–18 letters/spaces)",
            125,
            [10, 150, 380, 24],
        );
        let name = control(hwnd, "EDIT", "", 124, [10, 180, 250, 26]);
        for (id, label, x, y) in [
            (130, "Load prices", 10, 220),
            (131, "Rename", 160, 220),
            (132, "Change sex", 310, 220),
            (133, "Unlock Banker", 10, 260),
            (134, "Unlock Cook", 160, 260),
            (135, "Unlock Steward", 310, 260),
            (136, "Unlock Trader", 460, 260),
        ] {
            control(hwnd, "BUTTON", label, id, [x, y, 140, 30]);
        }
        let state = control(hwnd, "EDIT", "", 123, [10, 310, 580, 300]);
        let mut held_entries = Vec::new();
        let mut held_identity = None;
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
            let Some(shared) = weak.upgrade() else { break };
            let mut mailbox = lock(&shared);
            if mailbox.hireling_shutdown {
                break;
            }
            for action in std::mem::take(&mut actions.0) {
                if action == 130 {
                    super::super::input::configuration::prices(&mut mailbox, &commands);
                    continue;
                }
                if (131..=136).contains(&action) {
                    let mut text = [0u16; 64];
                    let n = GetWindowTextW(name, text.as_mut_ptr(), 64);
                    let name = String::from_utf16_lossy(&text[..n.max(0) as usize]);
                    let review =
                        super::super::input::configuration::review(&mailbox, action, &name);
                    match review {
                        Ok(review) => {
                            drop(mailbox);
                            let confirmed = MessageBoxW(
                                hwnd,
                                wide(&review.text()).as_ptr(),
                                wide("Confirm hireling configuration").as_ptr(),
                                MB_YESNO | MB_DEFBUTTON2,
                            ) == IDYES;
                            mailbox = lock(&shared);
                            super::super::input::configuration::submit(
                                &mut mailbox,
                                &commands,
                                &review,
                                confirmed,
                            );
                        }
                        Err(message) => mailbox.hireling_management.feedback = Some(message),
                    }
                    continue;
                }
                if mailbox.store.pending {
                    continue;
                }
                let p = &mut mailbox.hireling_management;
                let intent = match action {
                    101 if p.pending.is_none() => {
                        let at = SendMessageW(identities, CB_GETCURSEL, 0, 0);
                        if let Ok(at) = usize::try_from(at) {
                            oteryn_client_ui::hireling_management::select(p, at);
                        }
                        None
                    }
                    120 => {
                        let values = colors.map(|hwnd| read_color(hwnd));
                        let at = SendMessageW(looks, CB_GETCURSEL, 0, 0);
                        let look = p
                            .entries
                            .iter()
                            .find(|e| Some(e.hireling) == p.identity)
                            .and_then(|e| {
                                usize::try_from(at)
                                    .ok()
                                    .and_then(|i| outfits(e).get(i).map(|(_, look)| *look))
                            });
                        match (look, values) {
                            (Some(look_type), [Some(a), Some(b), Some(c), Some(d)]) => {
                                Some(Intent::Outfit {
                                    look_type,
                                    colors: [a, b, c, d],
                                })
                            }
                            _ => {
                                p.feedback = Some(tr!(
                                    "Select a hireling and outfit; enter four colors from 0 to 255"
                                ));
                                None
                            }
                        }
                    }
                    121 => Some(Intent::Return),
                    122 => Some(Intent::List),
                    _ => None,
                };
                if let Some(intent) = intent
                    && let Some(command) = request(p, intent)
                    && commands.send(PlayCommand::Feature(command)).is_err()
                {
                    p.pending = None;
                    p.feedback = Some(tr!("The session has ended"));
                }
            }
            let p = &mailbox.hireling_management;
            let entries_changed = p.entries != held_entries;
            if entries_changed {
                SendMessageW(identities, CB_RESETCONTENT, 0, 0);
                for e in &p.entries {
                    SendMessageW(identities, CB_ADDSTRING, 0, wide(&e.name).as_ptr() as isize);
                }
                if let Some(at) = p
                    .entries
                    .iter()
                    .position(|e| Some(e.hireling) == p.identity)
                {
                    SendMessageW(identities, CB_SETCURSEL, at, 0);
                }
                held_entries = p.entries.clone();
            }
            if entries_changed || p.identity != held_identity {
                SendMessageW(looks, CB_RESETCONTENT, 0, 0);
                if let Some(e) = p.entries.iter().find(|e| Some(e.hireling) == p.identity) {
                    for (i, (name, look)) in outfits(e).iter().enumerate() {
                        SendMessageW(looks, CB_ADDSTRING, 0, wide(name).as_ptr() as isize);
                        if *look == e.look_type {
                            SendMessageW(looks, CB_SETCURSEL, i, 0);
                        }
                    }
                    for (hwnd, color) in colors.iter().zip(e.colors) {
                        SetWindowTextW(*hwnd, wide(&color.to_string()).as_ptr());
                    }
                }
                held_identity = p.identity;
            }
            let text = format!(
                "{}\r\n{}",
                oteryn_client_ui::hireling_management::panel_text(&mailbox.hireling_management),
                oteryn_client_ui::store::panel_text(&mailbox.store)
            );
            if text != held {
                SetWindowTextW(state, wide(&text).as_ptr());
                held = text;
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            let _ = DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(m) = weak.upgrade() {
        lock(&m).hireling_management.open = false;
    }
}
unsafe fn control(parent: HWND, class: &str, text: &str, id: usize, b: [i32; 4]) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD
                | WS_VISIBLE
                | WS_BORDER
                | match class {
                    "COMBOBOX" => CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                    "EDIT" if id == 123 => {
                        ES_MULTILINE as u32
                            | ES_AUTOVSCROLL as u32
                            | ES_READONLY as u32
                            | WS_VSCROLL
                    }
                    _ => 0,
                },
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

unsafe fn read_color(hwnd: HWND) -> Option<u8> {
    unsafe {
        if !(1..=3).contains(&GetWindowTextLengthW(hwnd)) {
            return None;
        }
        let mut value = [0u16; 4];
        let n = GetWindowTextW(hwnd, value.as_mut_ptr(), 4);
        String::from_utf16(value.get(..usize::try_from(n).ok()?)?)
            .ok()?
            .parse()
            .ok()
    }
}
