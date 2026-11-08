//! Native VIP management controls; every HWND remains on its creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::vip::{Intent, request};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata points to the live Actions box owned by this window thread.
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 16 {
                (*actions).0.push(w);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
fn wide(value: &str) -> Vec<u16> {
    value
        .replace('\0', "�")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.vip.open {
            return;
        }
        mailbox.vip.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("vip-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
        && let Some(mailbox) = failed.upgrade()
    {
        lock(&mailbox).vip.open = false;
    }
}

fn run(
    weak: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    // SAFETY: creation, message handling, and destruction of every control stay on this thread.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("VIP management — Ctrl+V").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            650,
            720,
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
        let list = LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_TABSTOP;
        control(
            hwnd,
            "STATIC",
            "VIP entries                 Groups                      Consent invitations",
            200,
            [10, 5, 620, 20],
            0,
        );
        let entries = control(hwnd, "LISTBOX", "", 101, [10, 30, 205, 160], list);
        let groups = control(
            hwnd,
            "LISTBOX",
            "",
            102,
            [225, 30, 180, 160],
            list | LBS_EXTENDEDSEL as u32,
        );
        let consents = control(hwnd, "LISTBOX", "", 103, [415, 30, 210, 160], list);
        control(
            hwnd,
            "STATIC",
            "Character / group name or description",
            201,
            [10, 195, 300, 20],
            0,
        );
        let text = control(
            hwnd,
            "EDIT",
            "",
            104,
            [10, 220, 300, 25],
            WS_BORDER | WS_TABSTOP,
        );
        control(
            hwnd,
            "STATIC",
            "Icon (0–19); select groups above to assign",
            202,
            [320, 195, 300, 20],
            0,
        );
        let icon = control(
            hwnd,
            "EDIT",
            "0",
            105,
            [320, 220, 50, 25],
            WS_BORDER | WS_TABSTOP,
        );
        let notify = control(
            hwnd,
            "BUTTON",
            "Notify",
            107,
            [545, 220, 80, 25],
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        for (index, label) in [
            "Add",
            "Remove",
            "Save edit",
            "Add group",
            "Rename group",
            "Delete group",
            "Accept",
            "Reject",
            "Revoke",
            "Withdraw",
            "Show status",
            "Hide status",
            "Refresh",
            "Next page",
        ]
        .iter()
        .enumerate()
        {
            control(
                hwnd,
                "BUTTON",
                label,
                110 + index,
                [
                    10 + (index % 5) as i32 * 123,
                    260 + (index / 5) as i32 * 35,
                    120,
                    30,
                ],
                WS_TABSTOP,
            );
        }
        let status = control(hwnd, "STATIC", "", 203, [10, 370, 615, 75], 0);
        for (id, title, x) in [
            (124, "All contacts", 10),
            (125, "Friends", 220),
            (126, "Invitations", 430),
        ] {
            control(hwnd, "BUTTON", title, id, [x, 455, 195, 28], WS_TABSTOP);
        }
        let friends = control(
            hwnd,
            "EDIT",
            "",
            204,
            [10, 490, 615, 140],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL | WS_BORDER,
        );
        let mut held: Option<oteryn_client_ui::vip::VipPanel> = None;
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
            if commands.is_closed() || !mailbox.vip.open {
                break;
            }
            let view = super::management_view(mailbox.vip.view.as_deref()).unwrap_or_default();
            for action in std::mem::take(&mut actions.0) {
                let id = action & 0xffff;
                if (124..=126).contains(&id) {
                    mailbox.vip.friends_filter = match id {
                        125 => oteryn_client_ui::vip::FriendsFilter::Confirmed,
                        126 => oteryn_client_ui::vip::FriendsFilter::Invitations,
                        _ => oteryn_client_ui::vip::FriendsFilter::All,
                    };
                    continue;
                }
                if id == 101 && action >> 16 == LBN_SELCHANGE as usize {
                    let index = SendMessageW(entries, LB_GETCURSEL, 0, 0);
                    if let Ok(index) = usize::try_from(index)
                        && let Some(row) = mailbox
                            .vip
                            .view
                            .as_deref()
                            .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
                            .and_then(|value| value["entries"].get(index).cloned())
                    {
                        SetWindowTextW(
                            text,
                            wide(row["description"].as_str().unwrap_or("")).as_ptr(),
                        );
                        SetWindowTextW(
                            icon,
                            wide(&row["icon"].as_u64().unwrap_or(0).to_string()).as_ptr(),
                        );
                        SendMessageW(notify, BM_SETCHECK, usize::from(row["notify"] == true), 0);
                        for (index, group) in view.groups.iter().enumerate() {
                            let selected = row["groups"].as_array().is_some_and(|values| {
                                values
                                    .iter()
                                    .any(|value| value.as_u64() == Some(group.number))
                            });
                            SendMessageW(groups, LB_SETSEL, usize::from(selected), index as isize);
                        }
                    }
                    continue;
                }
                let selected = |hwnd, rows: &[super::Row]| {
                    usize::try_from(SendMessageW(hwnd, LB_GETCURSEL, 0, 0))
                        .ok()
                        .and_then(|index| rows.get(index))
                        .map(|row| row.number)
                };
                let entry = selected(entries, &view.entries);
                let group = view
                    .groups
                    .iter()
                    .enumerate()
                    .find(|(index, _)| SendMessageW(groups, LB_GETSEL, *index, 0) > 0)
                    .map(|(_, row)| row.number);
                let consent = selected(consents, &view.consents);
                let value = read(text);
                let intent = match id {
                    110 => Some(Intent::Add(value)),
                    111 => entry.map(Intent::Remove),
                    112 => {
                        let metadata = view
                            .groups
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| SendMessageW(groups, LB_GETSEL, *index, 0) > 0)
                            .map(|(_, row)| u8::try_from(row.number))
                            .collect::<Result<Vec<_>, _>>();
                        match (entry, read(icon).parse::<u8>(), metadata) {
                            (Some(number), Ok(icon), Ok(groups)) => Some(Intent::Edit {
                                number,
                                text: value,
                                icon,
                                notify: SendMessageW(notify, BM_GETCHECK, 0, 0) == 1,
                                groups,
                            }),
                            _ => {
                                mailbox.vip.feedback =
                                    Some("Select an entry and enter a numeric icon".into());
                                None
                            }
                        }
                    }
                    113 => Some(Intent::GroupAdd(value)),
                    114 => group.map(|number| Intent::GroupRename(number, value)),
                    115 => group.map(Intent::GroupRemove),
                    116 => consent.map(Intent::Accept),
                    117 => consent.map(Intent::Reject),
                    118 => consent.map(Intent::Revoke),
                    119 => entry.map(Intent::Withdraw),
                    120 => Some(Intent::Status(1)),
                    121 => Some(Intent::Status(0)),
                    122 => Some(Intent::Query(0)),
                    123 if view.next_entry != 0 => Some(Intent::Query(view.next_entry)),
                    _ => None,
                };
                if let Some(intent) = intent
                    && let Some(command) = request(&mut mailbox.vip, intent)
                    && commands.send(PlayCommand::Vip(command)).is_err()
                {
                    mailbox.vip.pending = false;
                    mailbox.vip.feedback = Some("The session has ended".into());
                }
            }
            if held.as_ref() != Some(&mailbox.vip) {
                if held.as_ref().map(|panel| &panel.view) != Some(&mailbox.vip.view) {
                    for (hwnd, rows) in [
                        (entries, &view.entries),
                        (groups, &view.groups),
                        (consents, &view.consents),
                    ] {
                        SendMessageW(hwnd, LB_RESETCONTENT, 0, 0);
                        for row in rows {
                            SendMessageW(hwnd, LB_ADDSTRING, 0, wide(&row.label).as_ptr() as isize);
                        }
                    }
                }
                SetWindowTextW(
                    friends,
                    wide(&oteryn_client_ui::vip::friends_text(&mailbox.vip)).as_ptr(),
                );
                let visibility = mailbox
                    .vip
                    .view
                    .as_deref()
                    .and_then(|text| serde_json::from_str::<serde_json::Value>(text).ok())
                    .map(|value| {
                        if value["shown"] == false {
                            "Status hidden. "
                        } else {
                            "Status shown. "
                        }
                    })
                    .unwrap_or("");
                let feedback = mailbox
                    .vip
                    .feedback
                    .as_deref()
                    .unwrap_or(if mailbox.vip.pending {
                        "Loading…"
                    } else if !mailbox.vip.selected {
                        "VIP unavailable in this session"
                    } else {
                        "Select a named row to manage it. Refresh returns to the first page."
                    });
                SetWindowTextW(status, wide(&format!("{visibility}{feedback}")).as_ptr());
                for id in 110..124 {
                    EnableWindow(
                        GetDlgItem(hwnd, id),
                        i32::from(mailbox.vip.selected && !mailbox.vip.pending),
                    );
                }
                held = Some(mailbox.vip.clone());
            }
            drop(mailbox);
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        DestroyWindow(hwnd);
        close(&weak);
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).vip.open = false;
    }
}

unsafe fn read(hwnd: HWND) -> String {
    // SAFETY: the edit belongs to this thread; the buffer includes room for the terminator.
    unsafe {
        let mut buffer = vec![0u16; GetWindowTextLengthW(hwnd).max(0) as usize + 1];
        let length = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }
}
