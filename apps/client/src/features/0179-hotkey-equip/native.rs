//! Native hotkey equip configuration presentation.
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
use oteryn_client_ui::hotkey_equip::{self, HotkeyEquipPanel};

pub(super) fn submit(panel: &mut HotkeyEquipPanel, input: &str) -> Option<ServerMechanicCommand> {
    if panel.pending || !panel.selected {
        return None;
    }
    let Ok(definition) = input.trim().parse::<u32>() else {
        panel.feedback = Some("Enter a positive item definition number".into());
        return None;
    };
    panel.item_definition = definition;
    hotkey_equip::control_request(panel)
}

pub(super) fn panel_text(panel: &HotkeyEquipPanel) -> String {
    if !panel.selected {
        return "Hotkey equip unavailable in this session".into();
    }
    if panel.pending {
        return "Pending equip…".into();
    }
    if let Some(feedback) = &panel.feedback {
        return feedback.clone();
    }
    panel.outcome.as_ref().map_or_else(
        || "Enter an item definition to equip".into(),
        |outcome| format!("Equip result: {outcome:?}"),
    )
}

#[cfg(windows)]
pub(in crate::play) use windows::open_window;

#[cfg(windows)]
mod windows {
    // Win32 handles and callback userdata stay on the creating thread.
    #![allow(unsafe_code)]
    use super::*;
    use crate::play::{PlayLink, mailbox::lock};
    use std::ptr::{null, null_mut};
    use windows_sys::Win32::Foundation::*;
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
    use windows_sys::Win32::UI::WindowsAndMessaging::*;

    fn wide(text: &str) -> Vec<u16> {
        text.replace('\0', "\u{fffd}")
            .encode_utf16()
            .chain(Some(0))
            .collect()
    }

    unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
        unsafe {
            if msg == WM_COMMAND {
                let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Vec<usize>;
                if !actions.is_null() && (*actions).len() < 32 {
                    (*actions).push(w & 0xffff);
                }
                return 0;
            }
            DefWindowProcW(hwnd, msg, w, l)
        }
    }

    unsafe fn control(
        parent: HWND,
        class: &str,
        text: &str,
        id: usize,
        y: i32,
        height: i32,
        style: u32,
    ) -> HWND {
        unsafe {
            CreateWindowExW(
                0,
                wide(class).as_ptr(),
                wide(text).as_ptr(),
                WS_CHILD | WS_VISIBLE | style,
                10,
                y,
                440,
                height,
                parent,
                id as _,
                null_mut(),
                null(),
            )
        }
    }

    pub(in crate::play) fn open_window(link: &PlayLink) {
        {
            let mut mailbox = lock(&link.mailbox);
            if mailbox.hotkey_equip.open {
                return;
            }
            mailbox.hotkey_equip.open = true;
        }
        let weak = std::sync::Arc::downgrade(&link.mailbox);
        let rollback = weak.clone();
        let commands = link.commands.clone();
        if std::thread::Builder::new()
            .name("hotkey_equip-panel".into())
            .spawn(move || {
                unsafe {
                    let hwnd = CreateWindowExW(
                        0,
                        wide("STATIC").as_ptr(),
                        wide("Hotkey equip").as_ptr(),
                        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                        CW_USEDEFAULT,
                        CW_USEDEFAULT,
                        480,
                        420,
                        null_mut(),
                        null_mut(),
                        null_mut(),
                        null(),
                    );
                    if !hwnd.is_null() {
                        let mut actions: Vec<usize> = Vec::new();
                        SetWindowLongPtrW(hwnd, GWLP_USERDATA, &mut actions as *mut _ as isize);
                        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
                        control(hwnd, "STATIC", "Item definition", 0, 10, 25, 0);
                        let input = control(
                            hwnd,
                            "EDIT",
                            "",
                            1,
                            40,
                            30,
                            WS_BORDER | ES_AUTOHSCROLL as u32 | ES_NUMBER as u32,
                        );
                        let equip = control(hwnd, "BUTTON", "Equip", 2, 85, 35, 0);
                        let status = control(hwnd, "STATIC", "", 0, 310, 65, 0);
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
                            for action in std::mem::take(&mut actions) {
                                let panel = &mut mailbox.hotkey_equip;
                                let command = {
                                    if action == 2 {
                                        let mut text = [0u16; 32];
                                        let len = GetWindowTextW(
                                            input,
                                            text.as_mut_ptr(),
                                            text.len() as i32,
                                        );
                                        submit(
                                            panel,
                                            &String::from_utf16_lossy(&text[..len as usize]),
                                        )
                                    } else {
                                        None
                                    }
                                };
                                if let Some(command) = command
                                    && commands
                                        .send(crate::combat_input::PlayCommand::ServerMechanic(
                                            command,
                                        ))
                                        .is_err()
                                {
                                    mailbox.hotkey_equip.pending = false;
                                    mailbox.hotkey_equip.feedback =
                                        Some("The session has ended".into());
                                }
                            }
                            let panel = &mut mailbox.hotkey_equip;
                            EnableWindow(equip, i32::from(panel.selected && !panel.pending));
                            SetWindowTextW(status, wide(&panel_text(panel)).as_ptr());
                            drop(mailbox);
                            std::thread::sleep(std::time::Duration::from_millis(100));
                        }
                        if IsWindow(hwnd) != 0 {
                            DestroyWindow(hwnd);
                        }
                    }
                }
                if let Some(shared) = weak.upgrade() {
                    lock(&shared).hotkey_equip.open = false;
                }
            })
            .is_err()
            && let Some(shared) = rollback.upgrade()
        {
            lock(&shared).hotkey_equip.open = false;
        }
    }
}
