//! Minimal native party controls; every HWND remains on its creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::party::{Intent, ListAction, ListSort, PartyPanel, request};
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, ODS_SELECTED};
#[path = "list_draw.rs"]
mod list_draw;
#[path = "portraits.rs"]
mod portraits;
use oteryn_client_ui::party_list::panel_text;
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const MEMBERS: usize = 101;
const INVITATIONS: usize = 102;
const JOIN: usize = 103;
const LEAVE: usize = 104;
const PASS: usize = 105;
const KICK: usize = 106;
const SHARED: usize = 107;
const REFRESH: usize = 108;

struct Actions {
    commands: Vec<(usize, usize)>,
    rows: Vec<(u64, oteryn_client_ui::party::party::PartyListEntity)>,
    portraits: std::cell::RefCell<Option<portraits::PreviewAssets>>,
    legacy: Vec<(u64, String)>,
}
impl Default for Actions {
    fn default() -> Self {
        Self {
            commands: Vec::new(),
            rows: Vec::new(),
            legacy: Vec::new(),
            portraits: std::cell::RefCell::new(portraits::PreviewAssets::load()),
        }
    }
}
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata points to the live Actions box owned by this window thread.
    unsafe {
        if msg == WM_DRAWITEM && l != 0 {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() {
                list_draw::draw(&*actions, &*(l as *const DRAWITEMSTRUCT));
            }
            return 1;
        }
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).commands.len() < 16 {
                (*actions).commands.push((w & 0xffff, (w >> 16) & 0xffff));
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
    if link.uses_shared_frontend() {
        return;
    }
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.party.open {
            return;
        }
        mailbox.party.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("party-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
        && let Some(mailbox) = failed.upgrade()
    {
        lock(&mailbox).party.open = false;
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
            wide(tr!("Party")).as_ptr(),
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
        let members = control(
            hwnd,
            "LISTBOX",
            "",
            MEMBERS,
            [10, 10, 300, 220],
            list | LBS_OWNERDRAWFIXED as u32 | LBS_HASSTRINGS as u32,
        );
        SendMessageW(members, LB_SETITEMHEIGHT, 0, 48);
        let invitations = control(hwnd, "LISTBOX", "", INVITATIONS, [330, 10, 300, 220], list);
        for (id, label, x) in [
            (JOIN, tr!("Accept"), 10),
            (LEAVE, tr!("Leave"), 210),
            (PASS, tr!("Pass leadership"), 310),
            (KICK, tr!("Kick"), 460),
            (SHARED, tr!("Toggle shared XP"), 120),
            (REFRESH, tr!("Refresh"), 10),
        ] {
            control(
                hwnd,
                "BUTTON",
                label,
                id,
                [
                    x,
                    if id == REFRESH || id == SHARED {
                        280
                    } else {
                        240
                    },
                    if id == PASS || id == SHARED { 140 } else { 90 },
                    30,
                ],
                WS_TABSTOP,
            );
        }
        let status = control(
            hwnd,
            "EDIT",
            "",
            109,
            [10, 520, 620, 140],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL,
        );
        for (group, label) in [
            "Players",
            "Knights",
            "Paladins",
            "Druids",
            "Sorcerers",
            "Monks",
            "Summons",
        ]
        .iter()
        .enumerate()
        {
            let check = control(
                hwnd,
                "BUTTON",
                label,
                200 + group,
                [
                    10 + (group % 4) as i32 * 150,
                    330 + (group / 4) as i32 * 32,
                    145,
                    28,
                ],
                BS_AUTOCHECKBOX as u32,
            );
            SendMessageW(check, BM_SETCHECK, 1, 0);
        }
        for (index, label) in ["Time", "Distance", "HP", "Name"].iter().enumerate() {
            control(
                hwnd,
                "BUTTON",
                label,
                210 + index,
                [10 + index as i32 * 150, 405, 140, 28],
                WS_TABSTOP,
            );
        }
        let title = control(
            hwnd,
            "EDIT",
            "Party List",
            220,
            [10, 450, 440, 28],
            WS_BORDER | WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            "Rename",
            221,
            [460, 450, 140, 28],
            WS_TABSTOP,
        );
        let mut held: Option<PartyPanel> = None;
        let mut next_refresh = std::time::Instant::now();
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
            for (id, notification) in std::mem::take(&mut actions.commands) {
                if id == MEMBERS && notification == LBN_SELCHANGE as usize {
                    let row = usize::try_from(SendMessageW(members, LB_GETCURSEL, 0, 0)).ok();
                    mailbox.party.selected_member = row.and_then(|row| {
                        if row < actions.rows.len() {
                            actions
                                .rows
                                .get(row)
                                .filter(|(_, entry)| !entry.summon)
                                .map(|(handle, _)| *handle)
                        } else {
                            actions
                                .legacy
                                .get(row - actions.rows.len())
                                .map(|(handle, _)| *handle)
                        }
                    });
                    continue;
                }
                if id == INVITATIONS && notification == LBN_SELCHANGE as usize {
                    let row = usize::try_from(SendMessageW(invitations, LB_GETCURSEL, 0, 0)).ok();
                    mailbox.party.selected_invitation = row
                        .and_then(|row| mailbox.party.state.as_ref()?.invitations.get(row))
                        .map(|entry| entry.handle);
                    continue;
                }
                if (200..207).contains(&id) {
                    mailbox.party.list_action(ListAction::ToggleGroup(id - 200));
                    continue;
                }
                if (210..214).contains(&id) {
                    mailbox.party.list_action(ListAction::Sort(
                        [
                            ListSort::Time,
                            ListSort::Distance,
                            ListSort::Health,
                            ListSort::Name,
                        ][id - 210],
                    ));
                    continue;
                }
                if id == 221 {
                    let mut buffer = [0u16; 65];
                    let count = GetWindowTextW(title, buffer.as_mut_ptr(), buffer.len() as i32);
                    mailbox
                        .party
                        .list_action(ListAction::Rename(String::from_utf16_lossy(
                            &buffer[..count.max(0) as usize],
                        )));
                    continue;
                }
                let intent = match id {
                    JOIN => Some(Intent::Join),
                    LEAVE => Some(Intent::Leave),
                    PASS => Some(Intent::PassLeadership),
                    KICK => Some(Intent::Kick),
                    SHARED => Some(Intent::ToggleSharedExperience),
                    REFRESH => Some(Intent::Refresh),
                    _ => None,
                };
                if let Some(intent) = intent
                    && let Some(command) = request(&mut mailbox.party, intent)
                    && commands.send(PlayCommand::Party(command)).is_err()
                {
                    mailbox.party.pending = None;
                    mailbox.party.feedback = Some(tr!("The session has ended"));
                }
            }
            if std::time::Instant::now() >= next_refresh && mailbox.party.pending.is_none() {
                if let Some(command) = request(&mut mailbox.party, Intent::Refresh)
                    && commands.send(PlayCommand::Party(command)).is_err()
                {
                    mailbox.party.pending = None;
                    mailbox.party.feedback = Some(tr!("The session has ended"));
                }
                next_refresh = std::time::Instant::now() + std::time::Duration::from_secs(1);
            }
            if held.as_ref() != Some(&mailbox.party) {
                let enabled = i32::from(mailbox.party.can_manage_member());
                windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                    GetDlgItem(hwnd, KICK as i32),
                    enabled,
                );
                windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                    GetDlgItem(hwnd, PASS as i32),
                    enabled,
                );
                SendMessageW(members, LB_RESETCONTENT, 0, 0);
                SendMessageW(invitations, LB_RESETCONTENT, 0, 0);
                actions.rows.clear();
                actions.legacy.clear();
                if let Some(state) = &mailbox.party.state {
                    actions.rows = mailbox
                        .party
                        .list_rows()
                        .into_iter()
                        .map(|(handle, entry)| (handle, entry.clone()))
                        .collect();
                    for (row, (handle, entry)) in actions.rows.iter().enumerate() {
                        SendMessageW(
                            members,
                            LB_ADDSTRING,
                            0,
                            wide(&entry.name).as_ptr() as isize,
                        );
                        if mailbox.party.selected_member == Some(*handle) && !entry.summon {
                            SendMessageW(members, LB_SETCURSEL, row, 0);
                        }
                    }
                    if !mailbox.party.list_hidden[0] {
                        actions.legacy = state
                            .members
                            .iter()
                            .filter(|member| member.list.is_empty())
                            .map(|member| (member.handle, member.name.clone()))
                            .collect();
                        for (offset, (handle, name)) in actions.legacy.iter().enumerate() {
                            SendMessageW(members, LB_ADDSTRING, 0, wide(name).as_ptr() as isize);
                            if mailbox.party.selected_member == Some(*handle) {
                                SendMessageW(members, LB_SETCURSEL, actions.rows.len() + offset, 0);
                            }
                        }
                    }
                    for (row, entry) in state.invitations.iter().enumerate() {
                        SendMessageW(
                            invitations,
                            LB_ADDSTRING,
                            0,
                            wide(&entry.name).as_ptr() as isize,
                        );
                        if mailbox.party.selected_invitation == Some(entry.handle) {
                            SendMessageW(invitations, LB_SETCURSEL, row, 0);
                        }
                    }
                }
                let caption = if mailbox.party.list_title.is_empty() {
                    "Party List"
                } else {
                    &mailbox.party.list_title
                };
                SetWindowTextW(hwnd, wide(caption).as_ptr());
                InvalidateRect(members, null(), 0);
                held = Some(mailbox.party.clone());
            }
            SetWindowTextW(status, wide(&panel_text(&mailbox.party)).as_ptr());
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
        lock(&mailbox).party.open = false;
    }
}

/// Runs synchronously on the play window thread; the menu never owns session authority.
pub(in crate::play) fn context_menu(link: &PlayLink, target: oteryn_session::EntityRef) {
    if link.uses_shared_frontend() {
        return;
    }
    // SAFETY: all menu/window resources are created, used, and destroyed on this thread.
    unsafe {
        let owner = GetForegroundWindow();
        if owner.is_null() {
            return;
        }
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        AppendMenuW(menu, MF_STRING, 1, wide(tr!("Invite to party")).as_ptr());
        AppendMenuW(
            menu,
            MF_STRING,
            2,
            wide(tr!("Party (Ctrl+Shift+G)")).as_ptr(),
        );
        let mut point = std::mem::zeroed();
        windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            owner,
            null(),
        );
        DestroyMenu(menu);
        match chosen {
            1 => {
                link.party_request(Intent::Invite(target));
                open_window(link);
            }
            2 => link.open_party(),
            _ => {}
        }
    }
}
