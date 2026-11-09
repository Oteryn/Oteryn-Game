//! Native local-chat panel. HWND access remains on its creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::feature_0080_chat_private_channel::view::Controls as PrivateControls;
use crate::play::{
    PlayLink,
    mailbox::{SharedMailbox, lock},
};
use oteryn_client_ui::chat::{Mode, panel_text};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetFocus;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[path = "native_helpers.rs"]
mod helpers;
use helpers::*;
#[path = "native_console.rs"]
mod console_controls;

const MODES: usize = 101;
const MESSAGE: usize = 102;
const RECIPIENT: usize = 103;
const RECIPIENT_LABEL: usize = 104;
const SEND: usize = 105;
const IGNORE: usize = 113;
const ALLOW: usize = 114;
const BLOCK: usize = 115;
const CLEAR_LISTS: usize = 116;
const LOG_PATH: usize = 110;
const SAVE: usize = 111;
const APPEND: usize = 112;
const TAB: usize = 107;
const FILTER: usize = 108;
const CHAT_MODE: usize = 109;
type WeakMailbox = Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>;

#[derive(Default)]
struct Actions(Vec<usize>);

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
            wide("Local Chat (Ctrl+Shift+C)").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            1100,
            630,
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
        let tabs = control(
            hwnd,
            "LISTBOX",
            "",
            TAB,
            [10, 245, 230, 75],
            LBS_NOTIFY as u32 | WS_BORDER,
        );
        for tab in ["All", "Local", "Private", "Guild"] {
            SendMessageW(tabs, LB_ADDSTRING, 0, wide(tab).as_ptr() as isize);
        }
        SendMessageW(tabs, LB_SETCURSEL, 0, 0);
        control(
            hwnd,
            "BUTTON",
            "Toggle messages",
            FILTER,
            [10, 325, 230, 28],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            "Chat on/off/temporary",
            CHAT_MODE,
            [10, 360, 230, 28],
            WS_TABSTOP,
        );
        for (id, title, y) in [
            (IGNORE, "Ignore recipient", 400),
            (ALLOW, "Whitelist recipient", 435),
            (BLOCK, "Blacklist recipient", 470),
            (CLEAR_LISTS, "Remove recipient from lists", 505),
        ] {
            control(hwnd, "BUTTON", title, id, [10, y, 230, 28], WS_TABSTOP);
        }
        let log_path = control(
            hwnd,
            "EDIT",
            "chat.log",
            LOG_PATH,
            [250, 480, 300, 28],
            WS_BORDER | WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            "Save log",
            SAVE,
            [250, 515, 140, 28],
            WS_TABSTOP,
        );
        control(
            hwnd,
            "BUTTON",
            "Append log",
            APPEND,
            [400, 515, 140, 28],
            WS_TABSTOP,
        );
        let mut console = console_controls::Controls::new(hwnd);
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
            let enabled =
                !guild_mode || oteryn_client_ui::chat_guild_room::available(&mailbox.chat);
            EnableWindow(message, i32::from(enabled));
            EnableWindow(GetDlgItem(hwnd, SEND as i32), i32::from(enabled));
            for action in std::mem::take(&mut actions.0) {
                if console.action(action, &mut mailbox, &commands, transcript) {
                    continue;
                }
                if private.action(action, &mut mailbox, &commands) {
                    continue;
                }
                use oteryn_client_ui::chat::{ComposerMode, ConsoleTab};
                match action {
                    IGNORE | ALLOW | BLOCK | CLEAR_LISTS => {
                        use oteryn_client_ui::chat::CommunicationList;
                        let name = read_text(recipient);
                        let list = match action {
                            ALLOW => CommunicationList::Whitelist,
                            BLOCK => CommunicationList::Blacklist,
                            _ => CommunicationList::Ignore,
                        };
                        if action == CLEAR_LISTS {
                            for list in [
                                CommunicationList::Ignore,
                                CommunicationList::Whitelist,
                                CommunicationList::Blacklist,
                            ] {
                                mailbox.chat.console.communication.edit(list, &name, false);
                            }
                        } else {
                            mailbox.chat.console.communication.edit(list, &name, true);
                        }
                        mailbox.chat_private.communication =
                            mailbox.chat.console.communication.clone();
                        mailbox.chat_party_room.communication =
                            mailbox.chat.console.communication.clone();
                        mailbox.chat.feedback = Some(format!(
                            "Ignore: {:?}; whitelist: {:?}; blacklist: {:?}",
                            mailbox.chat.console.communication.ignored,
                            mailbox.chat.console.communication.whitelist,
                            mailbox.chat.console.communication.blacklist
                        ));
                        continue;
                    }
                    SAVE | APPEND => {
                        let path = read_text(log_path);
                        let snapshot = mailbox.chat.clone();
                        drop(mailbox);
                        let result = std::fs::OpenOptions::new()
                            .create(true)
                            .write(true)
                            .append(action == APPEND)
                            .truncate(action == SAVE)
                            .open(path)
                            .and_then(|mut file| {
                                oteryn_client_ui::chat::write_log(&snapshot, &mut file)
                            });
                        mailbox = lock(&shared);
                        mailbox.chat.feedback = Some(match result {
                            Ok(()) => "Channel log written".into(),
                            Err(error) => format!("Could not write channel log: {error}"),
                        });
                        continue;
                    }
                    TAB => {
                        mailbox
                            .chat
                            .console
                            .select(match SendMessageW(tabs, LB_GETCURSEL, 0, 0) {
                                1 => ConsoleTab::Local,
                                2 => ConsoleTab::Private,
                                3 => ConsoleTab::Guild,
                                _ => ConsoleTab::All,
                            });
                        continue;
                    }
                    FILTER => {
                        mailbox.chat.console.show_messages = !mailbox.chat.console.show_messages;
                        continue;
                    }
                    CHAT_MODE => {
                        mailbox.chat.console.mode = match mailbox.chat.console.mode {
                            ComposerMode::On => ComposerMode::Off,
                            ComposerMode::Off => ComposerMode::Temporary,
                            ComposerMode::Temporary => ComposerMode::On,
                        };
                        continue;
                    }
                    _ => {}
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
            console.tick(&mut mailbox, &commands, transcript);
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
