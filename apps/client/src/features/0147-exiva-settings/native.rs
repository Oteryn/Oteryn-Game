//! System controls follow the existing Quest Log window's thread and handle ownership.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::exiva_settings::{panel_text, request};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: our creating thread installs userdata pointing to its live Actions.
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
fn wide(s: &str) -> Vec<u16> {
    s.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if mailbox.exiva.open {
            return;
        }
        mailbox.exiva.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("exiva-permissions".into())
        .spawn(move || run(weak, commands))
        .is_err()
        && let Some(mailbox) = failed.upgrade()
    {
        lock(&mailbox).exiva.open = false;
    }
}
fn run(
    weak: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    // SAFETY: every HWND and userdata access stays on this creating thread. The
    // Actions box stays alive through DestroyWindow; controls copy their strings.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Exiva permissions — Ctrl+E").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            600,
            560,
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
            (1, 10, 10, 270, 250),
            WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        );
        let name = control(
            hwnd,
            "EDIT",
            "",
            (2, 10, 275, 270, 30),
            WS_BORDER | WS_TABSTOP,
        );
        SendMessageW(name, EM_SETLIMITTEXT, 29, 0);
        let add = control(
            hwnd,
            "BUTTON",
            "Add name",
            (3, 10, 315, 130, 32),
            WS_TABSTOP,
        );
        let remove = control(
            hwnd,
            "BUTTON",
            "Remove selected",
            (4, 150, 315, 130, 32),
            WS_TABSTOP,
        );
        let refresh = control(hwnd, "BUTTON", "Refresh", (5, 10, 355, 130, 32), WS_TABSTOP);
        let mut modes = Vec::new();
        for (index, label) in [
            "Allow everyone",
            "Allow player list",
            "Allow own guild",
            "Allow own party",
            "Allow VIP list",
        ]
        .iter()
        .enumerate()
        {
            let button = control(
                hwnd,
                "BUTTON",
                label,
                (10 + index, 300, 10 + index as i32 * 40, 260, 32),
                BS_CHECKBOX as u32 | WS_TABSTOP,
            );
            modes.push(button);
        }
        let status = control(hwnd, "STATIC", "", (6, 10, 400, 560, 100), 0);
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
            let mut mailbox = lock(&shared);
            if mailbox.quest_shutdown {
                break;
            }
            for id in std::mem::take(&mut actions.0) {
                let payload = match id {
                    3 => {
                        let mut buffer = [0u16; 30];
                        let len = GetWindowTextW(name, buffer.as_mut_ptr(), 30);
                        let mut payload = vec![6];
                        payload
                            .extend(String::from_utf16_lossy(&buffer[..len as usize]).as_bytes());
                        Some(payload)
                    }
                    4 => usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0))
                        .ok()
                        .and_then(|row| mailbox.exiva.list.as_ref()?.players.get(row))
                        .map(|(id, _)| {
                            let mut bytes = vec![2];
                            bytes.extend(id);
                            bytes
                        }),
                    5 => Some(vec![5]),
                    10..=14 => mailbox
                        .exiva
                        .list
                        .as_ref()
                        .map(|list| vec![0, list.modes ^ (1 << (id - 10))]),
                    _ => None,
                };
                if let Some(payload) = payload
                    && let Some(payload) = request(&mut mailbox.exiva, payload)
                    && commands.send(PlayCommand::Exiva(payload)).is_err()
                {
                    mailbox.exiva.pending = false;
                    mailbox.exiva.error = Some("Session ended");
                }
            }
            let panel = &mailbox.exiva;
            if held.as_ref() != Some(panel) {
                if held
                    .as_ref()
                    .is_none_or(|old: &oteryn_client_ui::exiva_settings::ExivaPanel| {
                        old.list != panel.list
                    })
                {
                    SendMessageW(list, LB_RESETCONTENT, 0, 0);
                    if let Some(list_state) = &panel.list {
                        for (_, name) in &list_state.players {
                            SendMessageW(list, LB_ADDSTRING, 0, wide(name).as_ptr() as isize);
                        }
                        for (bit, button) in modes.iter().enumerate() {
                            SendMessageW(
                                *button,
                                BM_SETCHECK,
                                usize::from(list_state.modes & (1 << bit) != 0),
                                0,
                            );
                        }
                    }
                }
                SetWindowTextW(status, wide(&panel_text(panel)).as_ptr());
                for button in [add, remove, refresh].iter().chain(modes.iter()) {
                    EnableWindow(
                        *button,
                        i32::from(panel.selected && !panel.pending && panel.list.is_some()),
                    );
                }
                // Refresh remains usable when the first read is refused.
                EnableWindow(refresh, i32::from(panel.selected && !panel.pending));
                held = Some(panel.clone());
            }
            drop(mailbox);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}
fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).exiva.open = false;
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    layout: (usize, i32, i32, i32, i32),
    style: u32,
) -> HWND {
    let (id, x, y, width, height) = layout;
    // SAFETY: parent belongs to this thread and class/text live through the call.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            x,
            y,
            width,
            height,
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
