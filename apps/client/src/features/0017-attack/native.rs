//! Battle controls and HWNDs remain on their creating thread.
#![allow(unsafe_code)]
#[path = "native_controls.rs"]
mod panel_controls;
use super::preferences::{BattleListPreferences, BattleSort, row_label, validate_lists};
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::battle::{self, Action, BattlePanel};
use oteryn_session::{ChaseMode, FightMode};
use panel_controls::PanelControls;
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::*,
};

#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata points to Actions on this thread until HWND destruction.
    unsafe {
        if msg == WM_SYSKEYDOWN && w == 0x42 && (l & (1 << 29)) != 0 {
            DestroyWindow(hwnd);
            return 0;
        }
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let notification = (w >> 16) & 0xffff;
            if !actions.is_null()
                && (*actions).0.len() < 16
                && (((200..=203).contains(&id) && notification == LBN_SELCHANGE as usize)
                    || (id == 300 && notification == CBN_SELCHANGE as usize)
                    || (((101..=108).contains(&id) || (307..=309).contains(&id))
                        && notification == BN_CLICKED as usize))
            {
                (*actions).0.push(id);
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
unsafe fn read_text(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let length = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.battle.native_running {
            return;
        }
        mailbox.battle.native_running = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let fallback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("battle-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&fallback);
    }
}
fn run(weak: Weak<Mutex<Mailbox>>, commands: UnboundedSender<PlayCommand>) {
    // SAFETY: all window operations, dispatch and destruction stay on this thread.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Battle — Alt+B").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            760,
            740,
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
        let Some(PanelControls {
            headings,
            lists,
            text,
            active,
            title,
            search,
            maximum,
            players,
            creatures,
            sort,
            controls,
        }) = PanelControls::new(hwnd, &weak)
        else {
            return;
        };
        let mut held: Option<BattlePanel> = None;
        let mut held_lists = None;
        let mut displayed: Vec<Vec<oteryn_session::WorldSpatialEntity>> = Vec::new();
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
            let mut mailbox = lock(&shared);
            if mailbox.battle.shutdown || !mailbox.battle.open {
                break;
            }
            for id in std::mem::take(&mut actions.0) {
                if id == 300 || (307..=309).contains(&id) {
                    let state = &mut mailbox.battle_lists;
                    if id == 300 {
                        state.active = (SendMessageW(active, CB_GETCURSEL, 0, 0).max(0) as usize)
                            .min(state.preferences.len() - 1);
                        held_lists = None;
                        continue;
                    }
                    let mut candidate = state.preferences.clone();
                    let mut selected = state.active.min(candidate.len() - 1);
                    match id {
                        307 => {
                            let Ok(max_health) = read_text(maximum).parse() else {
                                mailbox.battle.feedback =
                                    Some("Enter a maximum HP percentage from 0 to 100");
                                continue;
                            };
                            candidate[selected] = BattleListPreferences {
                                name: read_text(title).trim().into(),
                                search: read_text(search).trim().into(),
                                players: SendMessageW(players, BM_GETCHECK, 0, 0) == 1,
                                creatures: SendMessageW(creatures, BM_GETCHECK, 0, 0) == 1,
                                max_health,
                                sort: match SendMessageW(sort, CB_GETCURSEL, 0, 0) {
                                    1 => BattleSort::HealthAscending,
                                    2 => BattleSort::HealthDescending,
                                    3 => BattleSort::Identity,
                                    _ => BattleSort::Name,
                                },
                            };
                        }
                        308 if candidate.len() < 4 => {
                            let next = (1..=8).map(|i| format!("List {i}")).find(|name| {
                                !candidate.iter().any(|c| c.name.eq_ignore_ascii_case(name))
                            });
                            if let Some(name) = next {
                                candidate.push(BattleListPreferences {
                                    name,
                                    ..Default::default()
                                });
                                selected = candidate.len() - 1;
                            }
                        }
                        309 if candidate.len() > 1 => {
                            candidate.remove(selected);
                            selected = selected.min(candidate.len() - 1);
                        }
                        _ => continue,
                    }
                    if validate_lists(&candidate) {
                        state.preferences = candidate;
                        state.active = selected;
                        state.dirty = true;
                        state.edit = super::next_edit();
                        mailbox.battle.feedback = Some("List updated");
                    } else {
                        mailbox.battle.feedback = Some(
                            "Use unique list names (1–32 bytes), a filter up to 64 bytes and HP from 0 to 100",
                        );
                    }
                    continue;
                }
                let panel = &mut mailbox.battle;
                let action = match id {
                    200..=203 => {
                        let list_index = id - 200;
                        let index = SendMessageW(lists[list_index], LB_GETCURSEL, 0, 0);
                        let Some(row) = usize::try_from(index)
                            .ok()
                            .and_then(|i| displayed.get(list_index)?.get(i))
                        else {
                            continue;
                        };
                        Action::Attack(row.entity)
                    }
                    101 => Action::Stop,
                    102 => Action::Fight(FightMode::Offensive),
                    103 => Action::Fight(FightMode::Balanced),
                    104 => Action::Fight(FightMode::Defensive),
                    107 => Action::Chase(ChaseMode::Stand),
                    108 => Action::Chase(ChaseMode::Chase),
                    _ => continue,
                };
                if let Some(command) = battle::request(panel, action)
                    && commands.send(PlayCommand::Battle(command)).is_err()
                {
                    panel.pending = false;
                    panel.pending_command = None;
                }
            }
            let list_state = mailbox.battle_lists.clone();
            let panel = &mailbox.battle;
            if held.as_ref() != Some(panel) || held_lists.as_ref() != Some(&list_state) {
                displayed.clear();
                SendMessageW(active, CB_RESETCONTENT, 0, 0);
                for i in 0..4 {
                    let config = list_state.preferences.get(i);
                    ShowWindow(
                        headings[i],
                        if config.is_some() { SW_SHOW } else { SW_HIDE },
                    );
                    ShowWindow(lists[i], if config.is_some() { SW_SHOW } else { SW_HIDE });
                    let Some(config) = config else {
                        continue;
                    };
                    SetWindowTextW(headings[i], wide(&config.name).as_ptr());
                    SendMessageW(
                        active,
                        CB_ADDSTRING,
                        0,
                        wide(&config.name).as_ptr() as isize,
                    );
                    let rows = config.rows(&panel.rows);
                    SendMessageW(lists[i], LB_RESETCONTENT, 0, 0);
                    for row in &rows {
                        SendMessageW(
                            lists[i],
                            LB_ADDSTRING,
                            0,
                            wide(&row_label(row)).as_ptr() as isize,
                        );
                    }
                    let selected = panel
                        .combat
                        .and_then(|s| s.target)
                        .and_then(|target| rows.iter().position(|r| r.entity == target));
                    SendMessageW(lists[i], LB_SETCURSEL, selected.unwrap_or(usize::MAX), 0);
                    displayed.push(rows);
                }
                let selected = list_state.active.min(list_state.preferences.len() - 1);
                SendMessageW(active, CB_SETCURSEL, selected, 0);
                if held_lists.as_ref() != Some(&list_state) {
                    let config = &list_state.preferences[selected];
                    SetWindowTextW(title, wide(&config.name).as_ptr());
                    SetWindowTextW(search, wide(&config.search).as_ptr());
                    SetWindowTextW(maximum, wide(&config.max_health.to_string()).as_ptr());
                    SendMessageW(players, BM_SETCHECK, usize::from(config.players), 0);
                    SendMessageW(creatures, BM_SETCHECK, usize::from(config.creatures), 0);
                    SendMessageW(sort, CB_SETCURSEL, config.sort as usize, 0);
                }
                let status = panel.combat.map_or_else(
                    || "Combat capability unavailable".into(),
                    |state| {
                        format!(
                            "Fight: {:?}   Chase: {:?}\r\n{}",
                            state.modes.fight_mode,
                            state.modes.chase,
                            panel
                                .feedback
                                .unwrap_or("Select a creature or player to attack")
                        )
                    },
                );
                SetWindowTextW(text, wide(&status).as_ptr());
                for &control in &controls {
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                        control,
                        i32::from(panel.combat.is_some() && !panel.pending),
                    );
                }
                held = Some(panel.clone());
                held_lists = Some(list_state);
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
fn close(weak: &Weak<Mutex<Mailbox>>) {
    if let Some(shared) = weak.upgrade() {
        let mut mailbox = lock(&shared);
        mailbox.battle.open = false;
        mailbox.battle.native_running = false;
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
    // SAFETY: parent and temporary UTF-16 strings remain valid throughout this call.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | style,
            rect.0,
            rect.1,
            rect.2,
            rect.3,
            parent,
            id as *mut _,
            null_mut(),
            null(),
        )
    }
}
