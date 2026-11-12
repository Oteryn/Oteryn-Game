//! Native controls live on their creating thread; the session owns the data.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use oteryn_client_ui::highscores::{self, Action};
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
    // SAFETY: userdata points to Actions on this thread until the window is destroyed.
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let notification = (w >> 16) & 0xffff;
            if !actions.is_null()
                && (*actions).0.len() < 16
                && ((matches!(id, 101 | 102) && notification == CBN_SELCHANGE as usize)
                    || (matches!(id, 103..=106 | 108) && notification == BN_CLICKED as usize))
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
        if mailbox.highscores.open || mailbox.highscores_shutdown {
            return;
        }
        mailbox.highscores.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let fallback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("highscores-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&fallback);
    }
}
fn run(weak: Weak<Mutex<Mailbox>>, commands: UnboundedSender<PlayCommand>) {
    // SAFETY: every HWND operation, including destruction and dispatch, stays on this thread.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("Highscores")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            840,
            600,
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
        let category = control(
            hwnd,
            "COMBOBOX",
            "",
            101,
            10,
            230,
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let vocation = control(
            hwnd,
            "COMBOBOX",
            "",
            102,
            250,
            170,
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let refresh = control(hwnd, "BUTTON", tr!("Refresh"), 103, 430, 80, 0);
        let previous = control(hwnd, "BUTTON", tr!("Previous"), 104, 520, 80, 0);
        let next = control(hwnd, "BUTTON", tr!("Next"), 105, 610, 80, 0);
        let show_me = control(hwnd, "BUTTON", tr!("Show me"), 106, 700, 100, 0);
        let world_label = control(hwnd, "STATIC", "World ID (blank = current)", 0, 10, 220, 0);
        let world = control(hwnd, "EDIT", "", 107, 240, 460, WS_BORDER);
        let select_world = control(hwnd, "BUTTON", "Select", 108, 710, 90, 0);
        for (handle, x) in [(world_label, 10), (world, 240), (select_world, 710)] {
            SetWindowPos(handle, null_mut(), x, 50, 0, 0, SWP_NOSIZE | SWP_NOZORDER);
        }
        SendMessageW(
            world,
            windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT,
            36,
            0,
        );
        let text = CreateWindowExW(
            0,
            wide("EDIT").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD
                | WS_VISIBLE
                | WS_TABSTOP
                | WS_BORDER
                | WS_VSCROLL
                | ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32,
            10,
            95,
            790,
            455,
            hwnd,
            null_mut(),
            null_mut(),
            null(),
        );
        if [
            category,
            vocation,
            refresh,
            previous,
            next,
            show_me,
            world_label,
            world,
            select_world,
            text,
        ]
        .iter()
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        // Capability 498 adds the Loyalty row; the flag is fixed for the session.
        let loyalty = weak
            .upgrade()
            .is_some_and(|shared| lock(&shared).highscores.loyalty_selected);
        for (_, label) in oteryn_client_ui::highscores_loyalty::categories(loyalty) {
            let label = oteryn_client_ui::localization::tr(label);
            SendMessageW(category, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        for label in highscores::VOCATIONS {
            let label = oteryn_client_ui::localization::tr(label);
            SendMessageW(vocation, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        let mut held = None;
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
            if mailbox.highscores_shutdown {
                break;
            }
            let panel = &mut mailbox.highscores;
            for id in std::mem::take(&mut actions.0) {
                let action = match id {
                    101 | 102 => {
                        let c = SendMessageW(category, CB_GETCURSEL, 0, 0);
                        let v = SendMessageW(vocation, CB_GETCURSEL, 0, 0);
                        if c < 0 || v < 0 {
                            continue;
                        }
                        let Some(category_id) = usize::try_from(c)
                            .ok()
                            .and_then(|i| {
                                oteryn_client_ui::highscores_loyalty::category_for_selection(
                                    i, loyalty,
                                )
                            })
                        else {
                            continue;
                        };
                        Action::Filter {
                            category: category_id,
                            vocation: v as u32,
                        }
                    }
                    108 => {
                        let mut value = [0u16; 37];
                        let length = GetWindowTextW(world, value.as_mut_ptr(), value.len() as i32);
                        let Some(world) = highscores::parse_world(&String::from_utf16_lossy(
                            &value[..length.max(0) as usize],
                        )) else {
                            panel.feedback = Some(
                                "Enter a valid World ID, or leave blank for the current World"
                                    .into(),
                            );
                            continue;
                        };
                        Action::World(world)
                    }
                    104 => Action::Previous,
                    105 => Action::Next,
                    106 => Action::ShowMe,
                    _ => Action::Refresh,
                };
                if let Some(command) = highscores::request(panel, action)
                    && commands.send(PlayCommand::ServerMechanic(command)).is_err()
                {
                    panel.pending = false;
                    panel.feedback = Some(tr!("The session has ended").into());
                }
            }
            if held.as_ref() != Some(panel) {
                SetWindowTextW(
                    text,
                    wide(&highscores::panel_text(panel).replace('\n', "\r\n")).as_ptr(),
                );
                if let Some(index) = oteryn_client_ui::highscores_loyalty::category_selection(panel.category, loyalty) {
                    SendMessageW(category, CB_SETCURSEL, index, 0);
                }
                SendMessageW(vocation, CB_SETCURSEL, panel.vocation as usize, 0);
                let enabled = panel.selected && !panel.pending;
                if held
                    .as_ref()
                    .is_none_or(|old: &highscores::HighscoresPanel| old.world != panel.world)
                {
                    SetWindowTextW(
                        world,
                        wide(&panel.world.map(highscores::world_text).unwrap_or_default()).as_ptr(),
                    );
                }
                for h in [category, vocation, refresh, world, select_world] {
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                        h,
                        i32::from(enabled),
                    );
                }
                windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                    previous,
                    i32::from(enabled && panel.reply.as_ref().is_some_and(|r| r.page > 1)),
                );
                windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                    next,
                    i32::from(enabled && panel.reply.as_ref().is_some_and(|r| r.has_more)),
                );
                windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                    show_me,
                    i32::from(
                        enabled && panel.reply.as_ref().is_some_and(|r| r.own_page.is_some()),
                    ),
                );
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
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).highscores.open = false;
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    x: i32,
    width: i32,
    style: u32,
) -> HWND {
    // SAFETY: parent and all temporary UTF-16 strings are valid throughout this call.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | style,
            x,
            10,
            width,
            if class == "COMBOBOX" { 300 } else { 30 },
            parent,
            id as *mut _,
            null_mut(),
            null(),
        )
    }
}
