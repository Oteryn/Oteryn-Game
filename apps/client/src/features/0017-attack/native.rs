//! Battle controls and HWNDs remain on their creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::battle::{self, Action, BattlePanel};
use oteryn_session::{ChaseMode, FightMode};
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
                && ((id == 100 && notification == LBN_SELCHANGE as usize)
                    || ((101..=108).contains(&id) && notification == BN_CLICKED as usize))
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
            510,
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
        let list = control(
            hwnd,
            "LISTBOX",
            "",
            100,
            (10, 10, 720, 300),
            LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_HSCROLL,
        );
        let stop = control(hwnd, "BUTTON", "Stop attack", 101, (10, 320, 130, 30), 0);
        let offensive = control(hwnd, "BUTTON", "Offensive", 102, (150, 320, 110, 30), 0);
        let balanced = control(hwnd, "BUTTON", "Balanced", 103, (270, 320, 110, 30), 0);
        let defensive = control(hwnd, "BUTTON", "Defensive", 104, (390, 320, 110, 30), 0);
        let stand = control(hwnd, "BUTTON", "Stand", 107, (510, 320, 100, 30), 0);
        let chase = control(hwnd, "BUTTON", "Chase", 108, (620, 320, 100, 30), 0);
        let text = control(hwnd, "STATIC", "", 109, (10, 365, 720, 100), 0);
        let controls = [list, stop, offensive, balanced, defensive, stand, chase];
        if controls.iter().chain([text].iter()).any(|h| h.is_null()) {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut held: Option<BattlePanel> = None;
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
            let panel = &mut mailbox.battle;
            if panel.shutdown || !panel.open {
                break;
            }
            for id in std::mem::take(&mut actions.0) {
                let action = match id {
                    100 => {
                        let index = SendMessageW(list, LB_GETCURSEL, 0, 0);
                        let Some(row) = usize::try_from(index)
                            .ok()
                            .and_then(|i| held.as_ref()?.rows.get(i))
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
            if held.as_ref() != Some(panel) {
                if held.as_ref().is_none_or(|old| old.rows != panel.rows) {
                    SendMessageW(list, LB_RESETCONTENT, 0, 0);
                    for row in &panel.rows {
                        SendMessageW(
                            list,
                            LB_ADDSTRING,
                            0,
                            wide(&battle::row_text(row)).as_ptr() as isize,
                        );
                    }
                }
                let status = panel.combat.map_or_else(
                    || "Combat capability unavailable".into(),
                    |state| {
                        let selected = state
                            .target
                            .and_then(|target| panel.rows.iter().position(|r| r.entity == target));
                        SendMessageW(list, LB_SETCURSEL, selected.unwrap_or(usize::MAX), 0);
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
                for control in controls {
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                        control,
                        i32::from(panel.combat.is_some() && !panel.pending),
                    );
                }
                held = Some(panel.clone());
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
