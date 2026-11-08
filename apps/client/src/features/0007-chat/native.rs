//! Native local-chat panel. HWND access remains on its creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::feature_0080_chat_private_channel::view::Controls as PrivateControls;
use crate::play::{
    PlayLink,
    mailbox::{SharedMailbox, lock},
};
use oteryn_client_ui::chat::{ComposerMode, Mode, panel_text};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const MODES: usize = 101;
const MESSAGE: usize = 102;
const RECIPIENT: usize = 103;
const RECIPIENT_LABEL: usize = 104;
const SEND: usize = 105;
const TABS: usize = 110;
const FILTER: usize = 111;
const HIDE_YELLS: usize = 112;
const INPUT_MODE: usize = 113;
const LOG_PATH: usize = 115;
const LOG_SAVE: usize = 116;
const LOG_APPEND: usize = 117;
const IGNORE_NAMES: usize = 120;
const WHITELIST: usize = 121;
const BLACKLIST: usize = 122;
const IGNORE_PRIVATE: usize = 123;
const IGNORE_YELLS: usize = 124;
const RULES_APPLY: usize = 125;
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;

#[derive(Default)]
struct Actions(Vec<usize>);

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let note = (w >> 16) & 0xffff;
            if !actions.is_null()
                && ((matches!(
                    id,
                    SEND | HIDE_YELLS | INPUT_MODE | LOG_SAVE | LOG_APPEND | RULES_APPLY
                ) && note == BN_CLICKED as usize)
                    || (id == TABS && note == LBN_SELCHANGE as usize)
                    || PrivateControls::accepts(id, note))
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
    open_shared(&link.mailbox, &link.commands);
}

pub(in crate::play) fn open_shared(
    mailbox: &SharedMailbox,
    commands: &UnboundedSender<PlayCommand>,
) {
    {
        let mut mailbox = lock(mailbox);
        if mailbox.chat.open {
            return;
        }
        mailbox.chat.open = true;
    }
    let weak = Arc::downgrade(mailbox);
    let rollback = weak.clone();
    let commands = commands.clone();
    if std::thread::Builder::new()
        .name("local-chat-panel".into())
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
            wide("Chat console (Ctrl+Shift+C)").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1100,
            820,
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
        let rooms = control(
            hwnd,
            "LISTBOX",
            "",
            MODES,
            [10, 10, 230, 180],
            LBS_NOTIFY as u32 | WS_BORDER,
        );
        for mode in ["Say", "Whisper", "Yell", "Private", "Guild"] {
            SendMessageW(rooms, LB_ADDSTRING, 0, wide(mode).as_ptr() as isize);
        }
        SendMessageW(rooms, LB_SETCURSEL, 0, 0);
        let message = control(
            hwnd,
            "EDIT",
            "",
            MESSAGE,
            [250, 10, 430, 28],
            WS_BORDER | WS_TABSTOP,
        );
        let recipient = control(
            hwnd,
            "EDIT",
            "",
            RECIPIENT,
            [250, 48, 270, 28],
            WS_BORDER | WS_TABSTOP,
        );
        control(
            hwnd,
            "STATIC",
            "Private recipient",
            RECIPIENT_LABEL,
            [10, 200, 230, 28],
            0,
        );
        control(hwnd, "BUTTON", "Send", SEND, [540, 48, 100, 28], WS_TABSTOP);
        let transcript = control(
            hwnd,
            "EDIT",
            "",
            106,
            [250, 86, 430, 380],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let tabs_control = control(
            hwnd,
            "LISTBOX",
            "",
            TABS,
            [10, 240, 230, 180],
            LBS_NOTIFY as u32 | WS_BORDER,
        );
        control(
            hwnd,
            "STATIC",
            "Filter this tab (speaker or message)",
            114,
            [10, 425, 230, 24],
            0,
        );
        let filter = control(
            hwnd,
            "EDIT",
            "",
            FILTER,
            [10, 450, 230, 28],
            WS_BORDER | WS_TABSTOP,
        );
        let hide_yells = control(
            hwnd,
            "BUTTON",
            "Hide yells in this tab",
            HIDE_YELLS,
            [10, 490, 230, 28],
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        let input_mode = control(
            hwnd,
            "BUTTON",
            "Input On / Off / Once",
            INPUT_MODE,
            [10, 530, 230, 28],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "STATIC",
            "Channel log file",
            118,
            [250, 480, 430, 24],
            0,
        );
        let log_path = control(
            hwnd,
            "EDIT",
            "channel.log",
            LOG_PATH,
            [250, 505, 430, 28],
            WS_BORDER | WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            "Save (replace file)",
            LOG_SAVE,
            [250, 545, 210, 28],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            "Append to file",
            LOG_APPEND,
            [470, 545, 210, 28],
            WS_TABSTOP,
        );
        let mut communication_fields = Vec::new();
        for (x, label, id) in [
            (10, "Ignored names (comma separated)", IGNORE_NAMES),
            (370, "Always allow these names", WHITELIST),
            (730, "Blocked names", BLACKLIST),
        ] {
            control(hwnd, "STATIC", label, id + 10, [x, 595, 340, 24], 0);
            communication_fields.push(control(
                hwnd,
                "EDIT",
                "",
                id,
                [x, 620, 340, 28],
                WS_BORDER | WS_TABSTOP,
            ));
        }
        let ignore_private = control(
            hwnd,
            "BUTTON",
            "Ignore private messages",
            IGNORE_PRIVATE,
            [10, 660, 340, 28],
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        let ignore_yells = control(
            hwnd,
            "BUTTON",
            "Ignore yells",
            IGNORE_YELLS,
            [370, 660, 340, 28],
            BS_AUTOCHECKBOX as u32 | WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            "Apply communication lists",
            RULES_APPLY,
            [730, 660, 340, 28],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "STATIC",
            "Allowed names override ignore and blocked lists. History is retained.",
            139,
            [10, 710, 1060, 28],
            0,
        );
        let mut tabs = Vec::new();
        let mut preferences_initialized = false;
        let mut private = PrivateControls::new(hwnd);
        let mut held = String::new();
        let mut msg: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let mut mailbox = lock(&shared);
            if mailbox.chat_public_shutdown {
                break;
            }
            if preferences_initialized {
                mailbox.chat.console.filter_mut().contains = read_text(filter);
            } else {
                let preferences = mailbox.chat.console.filter_mut();
                SetWindowTextW(filter, wide(&preferences.contains).as_ptr());
                SendMessageW(
                    hide_yells,
                    BM_SETCHECK,
                    usize::from(preferences.hide_yells),
                    0,
                );
                let rules = &mailbox.chat.console.communication;
                for (control, names) in communication_fields.iter().zip([
                    &rules.ignored,
                    &rules.whitelist,
                    &rules.blacklist,
                ]) {
                    SetWindowTextW(
                        *control,
                        wide(
                            &names
                                .iter()
                                .map(String::as_str)
                                .collect::<Vec<_>>()
                                .join(", "),
                        )
                        .as_ptr(),
                    );
                }
                SendMessageW(
                    ignore_private,
                    BM_SETCHECK,
                    usize::from(rules.ignore_private),
                    0,
                );
                SendMessageW(
                    ignore_yells,
                    BM_SETCHECK,
                    usize::from(rules.ignore_yells),
                    0,
                );
                preferences_initialized = true;
            }
            let current_tabs = mailbox.chat.console.tabs(&mailbox.chat.lines);
            if tabs != current_tabs {
                tabs = current_tabs;
                SendMessageW(tabs_control, LB_RESETCONTENT, 0, 0);
                for tab in &tabs {
                    SendMessageW(
                        tabs_control,
                        LB_ADDSTRING,
                        0,
                        wide(&tab.label()).as_ptr() as isize,
                    );
                }
                let selected = tabs
                    .iter()
                    .position(|tab| *tab == mailbox.chat.console.tab)
                    .unwrap_or(0);
                SendMessageW(tabs_control, LB_SETCURSEL, selected, 0);
            }
            mailbox.chat.text = read_text(message);
            super::super::super::feature_0233_player_typing::input::composer(
                &mut mailbox,
                &commands,
                GetForegroundWindow() == hwnd && GetFocus() == message,
            );
            if mailbox.chat.guild_open {
                SendMessageW(rooms, LB_SETCURSEL, 4, 0);
                mailbox.chat.guild_open = false;
            }
            let guild_mode = SendMessageW(rooms, LB_GETCURSEL, 0, 0) == 4;
            let enabled = mailbox.chat.console.mode != ComposerMode::Off
                && (!guild_mode || oteryn_client_ui::chat_guild_room::available(&mailbox.chat));
            EnableWindow(message, i32::from(enabled));
            EnableWindow(GetDlgItem(hwnd, SEND as i32), i32::from(enabled));
            for action in std::mem::take(&mut actions.0) {
                if private.action(action, &mut mailbox, &commands) {
                    continue;
                }
                if action == RULES_APPLY {
                    let values: Vec<_> = communication_fields
                        .iter()
                        .map(|control| read_text(*control))
                        .collect();
                    match oteryn_client_ui::chat::CommunicationRules::parse(
                        &values[0],
                        &values[1],
                        &values[2],
                        SendMessageW(ignore_private, BM_GETCHECK, 0, 0) != 0,
                        SendMessageW(ignore_yells, BM_GETCHECK, 0, 0) != 0,
                    ) {
                        Ok(rules) => {
                            super::super::input::apply_communication(&mut mailbox, rules);
                            mailbox.chat.feedback = Some("Communication lists applied".into());
                        }
                        Err(error) => mailbox.chat.feedback = Some(error.into()),
                    }
                    continue;
                }
                if matches!(action, LOG_SAVE | LOG_APPEND) {
                    let path = read_text(log_path);
                    let transcript = panel_text(&mailbox.chat);
                    mailbox.chat.feedback = Some(
                        match super::super::input::write_log(
                            std::path::Path::new(&path),
                            action == LOG_APPEND,
                            &transcript,
                        ) {
                            Ok(()) => "Channel log written".into(),
                            Err(error) => format!("Could not write channel log: {error}"),
                        },
                    );
                    continue;
                }
                if action == TABS {
                    let selected = SendMessageW(tabs_control, LB_GETCURSEL, 0, 0);
                    if let Ok(index) = usize::try_from(selected)
                        && let Some(tab) = tabs.get(index)
                    {
                        mailbox.chat.console.tab = *tab;
                        let preferences = mailbox.chat.console.filter_mut();
                        SetWindowTextW(filter, wide(&preferences.contains).as_ptr());
                        SendMessageW(
                            hide_yells,
                            BM_SETCHECK,
                            usize::from(preferences.hide_yells),
                            0,
                        );
                    }
                    continue;
                }
                if action == HIDE_YELLS {
                    mailbox.chat.console.filter_mut().hide_yells =
                        SendMessageW(hide_yells, BM_GETCHECK, 0, 0) != 0;
                    continue;
                }
                if action == INPUT_MODE {
                    mailbox.chat.console.mode = match mailbox.chat.console.mode {
                        ComposerMode::On => ComposerMode::Off,
                        ComposerMode::Off => ComposerMode::Temporary,
                        ComposerMode::Temporary => ComposerMode::On,
                    };
                    continue;
                }
                if action != SEND {
                    continue;
                }
                let mode = match SendMessageW(rooms, LB_GETCURSEL, 0, 0) {
                    1 => Mode::Whisper,
                    2 => Mode::Yell,
                    3 => Mode::Private,
                    4 => Mode::Guild,
                    _ => Mode::Say,
                };
                mailbox.chat.text = read_text(message);
                super::super::input::enqueue(&mut mailbox, &commands, mode, &read_text(recipient));
                if mailbox.chat.pending {
                    SetWindowTextW(message, wide("").as_ptr());
                }
            }
            SetWindowTextW(
                input_mode,
                wide(&format!(
                    "Input {:?} (click to cycle)",
                    mailbox.chat.console.mode
                ))
                .as_ptr(),
            );
            private.render(&mailbox.chat_private);
            let text = format!(
                "{}\n{}",
                panel_text(&mailbox.chat),
                oteryn_client_ui::player_typing::panel_text(&mailbox.player_typing)
            );
            if held != text {
                SetWindowTextW(transcript, wide(&text).as_ptr());
                held = text;
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            let _ = DestroyWindow(hwnd);
        }
        if let Some(shared) = weak.upgrade() {
            super::super::super::feature_0233_player_typing::input::composer(
                &mut lock(&shared),
                &commands,
                false,
            );
        }
        close(&weak);
    }
}

fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).chat.open = false;
    }
}

unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    bounds: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            bounds[0],
            bounds[1],
            bounds[2],
            bounds[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}

unsafe fn read_text(hwnd: HWND) -> String {
    unsafe {
        let length = GetWindowTextLengthW(hwnd);
        let mut value = vec![0; usize::try_from(length).unwrap_or(0) + 1];
        let read = GetWindowTextW(hwnd, value.as_mut_ptr(), length + 1);
        String::from_utf16_lossy(&value[..usize::try_from(read).unwrap_or(0)])
    }
}
