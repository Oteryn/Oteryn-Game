//! Native hotkey equip configuration presentation.
use oteryn_client_runtime::server_mechanic::ServerMechanicCommand;
use oteryn_client_ui::hotkey_equip::{self, HotkeyEquipPanel};

pub(in crate::play) fn submit(
    panel: &mut HotkeyEquipPanel,
    input: &str,
) -> Option<ServerMechanicCommand> {
    if panel.pending || !panel.selected {
        return None;
    }
    let Ok(definition) = input.trim().parse::<u32>() else {
        panel.feedback = Some(tr!("Enter a positive item definition number").into());
        return None;
    };
    panel.item_definition = definition;
    hotkey_equip::control_request(panel)
}

pub(in crate::play) fn panel_text(panel: &HotkeyEquipPanel) -> String {
    if !panel.selected {
        return tr!("Hotkey equip unavailable in this session").into();
    }
    if panel.pending {
        return tr!("Pending equip…").into();
    }
    if let Some(feedback) = &panel.feedback {
        return feedback.clone();
    }
    panel.outcome.as_ref().map_or_else(
        || tr!("Enter an item definition to equip").into(),
        |outcome| lfmt!("Equip result: {outcome:?}", outcome = outcome),
    )
}

#[cfg(windows)]
pub(in crate::play) use windows::open_window;

#[cfg(windows)]
mod windows {
    //! Native item assignment; HWNDs remain on the thread that creates them.
    #![allow(unsafe_code)] // Windows control FFI, following the stash panel's ownership pattern.
    use super::panel_text;
    use crate::play::{PlayLink, mailbox::lock};
    use oteryn_client_ui::hotkey_equip::{ITEM_HOTKEY_SLOTS, assign, control_request};
    use std::{
        ptr::{null, null_mut},
        sync::{Arc, Weak},
        time::Duration,
    };
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
    use windows_sys::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
    const ASSIGN: usize = 103;
    const REMOVE: usize = 104;
    const USE: usize = 105;
    const SUBMIT: usize = 202;
    const SMART: usize = 203;
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
        text.replace('\0', "\u{fffd}")
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
            if mailbox.hotkey_equip.open {
                return;
            }
            mailbox.hotkey_equip.open = true;
        }
        let weak = Arc::downgrade(&link.mailbox);
        let failed = weak.clone();
        let commands = link.commands.clone();
        if std::thread::Builder::new()
            .name("item-hotkeys".into())
            .spawn(move || run(weak, commands))
            .is_err()
        {
            close(&failed);
        }
    }
    type Mailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;
    fn run(
        weak: Mailbox,
        commands: tokio::sync::mpsc::UnboundedSender<crate::combat_input::PlayCommand>,
    ) {
        unsafe {
            let hwnd = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide(tr!("Item hotkeys — Ctrl+Shift+E")).as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                620,
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
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
            let keys = control(
                hwnd,
                "LISTBOX",
                "",
                101,
                [10, 40, 280, 230],
                LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_TABSTOP,
            );
            let items = control(
                hwnd,
                "LISTBOX",
                "",
                102,
                [300, 40, 280, 230],
                LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_TABSTOP,
            );
            let labels = control(
                hwnd,
                "STATIC",
                tr!("Choose a key                         Choose a carried or equipped item"),
                106,
                [10, 10, 580, 25],
                0,
            );
            let assign_button = control(
                hwnd,
                "BUTTON",
                tr!("Assign item"),
                ASSIGN,
                [10, 280, 170, 30],
                WS_TABSTOP,
            );
            let remove = control(
                hwnd,
                "BUTTON",
                tr!("Clear key"),
                REMOVE,
                [190, 280, 170, 30],
                WS_TABSTOP,
            );
            let use_button = control(
                hwnd,
                "BUTTON",
                tr!("Use key"),
                USE,
                [370, 280, 170, 30],
                WS_TABSTOP,
            );
            let smart = control(
                hwnd,
                "BUTTON",
                "Smart equip (applies at next login)",
                SMART,
                [10, 320, 550, 28],
                BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
            );
            let status = control(
                hwnd,
                "EDIT",
                "",
                107,
                [10, 420, 570, 130],
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
            );
            let input = control(
                hwnd,
                "EDIT",
                "",
                201,
                [10, 360, 280, 30],
                WS_BORDER | ES_NUMBER as u32,
            );
            let submit = control(
                hwnd,
                "BUTTON",
                tr!("Equip definition"),
                SUBMIT,
                [300, 360, 240, 30],
                WS_TABSTOP,
            );
            if [
                input,
                smart,
                submit,
                keys,
                items,
                labels,
                assign_button,
                remove,
                use_button,
                status,
            ]
            .iter()
            .any(|handle| handle.is_null())
            {
                DestroyWindow(hwnd);
                close(&weak);
                return;
            }
            let mut desired: Option<oteryn_client_ui::hotkey_equip::HotkeyEquipPanel> = None;
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
                let panel = &mut mailbox.hotkey_equip;
                for action in std::mem::take(&mut actions.0) {
                    let slot = usize::try_from(SendMessageW(keys, LB_GETCURSEL, 0, 0)).unwrap_or(0);
                    if slot >= ITEM_HOTKEY_SLOTS {
                        continue;
                    }
                    match action {
                        SMART => {
                            panel.smart = SendMessageW(smart, BM_GETCHECK, 0, 0) == 1;
                            super::super::edited(panel);
                        }
                        SUBMIT => {
                            let mut text = [0_u16; 32];
                            let len = GetWindowTextW(input, text.as_mut_ptr(), 32);
                            if let Some(command) = super::submit(
                                panel,
                                &String::from_utf16_lossy(&text[..len as usize]),
                            ) && commands
                                .send(crate::combat_input::PlayCommand::ServerMechanic(command))
                                .is_err()
                            {
                                panel.pending = false;
                                panel.feedback = Some(tr!("The session has ended").into());
                            }
                        }
                        ASSIGN => {
                            if let Ok(row) =
                                usize::try_from(SendMessageW(items, LB_GETCURSEL, 0, 0))
                                && let Some(&definition) =
                                    desired.as_ref().and_then(|shown| shown.items.get(row))
                                && assign(panel, slot, definition)
                            {
                                super::super::edited(panel);
                            }
                        }
                        REMOVE => {
                            if assign(panel, slot, 0) {
                                super::super::edited(panel);
                            }
                        }
                        USE => {
                            if let Some(definition) = panel.bindings[slot] {
                                panel.item_definition = definition.get();
                                if let Some(command) = control_request(panel)
                                    && commands
                                        .send(crate::combat_input::PlayCommand::ServerMechanic(
                                            command,
                                        ))
                                        .is_err()
                                {
                                    panel.pending = false;
                                    panel.feedback = Some(tr!("The session has ended").into());
                                }
                            }
                        }
                        _ => {}
                    }
                }
                EnableWindow(submit, i32::from(panel.selected && !panel.pending));
                EnableWindow(use_button, i32::from(panel.selected && !panel.pending));
                if desired.as_ref() != Some(panel) {
                    let key_row = SendMessageW(keys, LB_GETCURSEL, 0, 0);
                    let item_row = SendMessageW(items, LB_GETCURSEL, 0, 0);
                    SendMessageW(keys, LB_RESETCONTENT, 0, 0);
                    for (slot, definition) in panel.bindings.iter().enumerate() {
                        let label = format!(
                            "F{}: {}",
                            slot + 1,
                            definition.map_or_else(
                                || tr!("Unassigned").into(),
                                |value| lfmt!("Item {value}", value = value)
                            )
                        );
                        SendMessageW(keys, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                    }
                    SendMessageW(items, LB_RESETCONTENT, 0, 0);
                    for definition in &panel.items {
                        SendMessageW(
                            items,
                            LB_ADDSTRING,
                            0,
                            wide(&lfmt!("Item {definition}", definition = definition)).as_ptr()
                                as isize,
                        );
                    }
                    SendMessageW(keys, LB_SETCURSEL, usize::try_from(key_row).unwrap_or(0), 0);
                    SendMessageW(
                        items,
                        LB_SETCURSEL,
                        usize::try_from(item_row).unwrap_or(0),
                        0,
                    );
                    SendMessageW(smart, BM_SETCHECK, usize::from(panel.smart), 0);
                    SetWindowTextW(status, wide(&format!(
                        "{}\r\nSession source selection: {}.\r\nSmart equip searches carried containers and prefers a started copy. Mode changes apply on your next login.",
                        panel_text(panel), if panel.smart_active { "smart" } else { "direct inventory" }
                    )).as_ptr());
                    desired = Some(panel.clone());
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
    fn close(weak: &Mailbox) {
        if let Some(shared) = weak.upgrade() {
            lock(&shared).hotkey_equip.open = false;
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
}
