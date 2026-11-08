//! Native local-chat panel. HWND access remains on its creating thread.
#![allow(unsafe_code)]
#[path = "native_controls.rs"]
mod panel_controls;
use crate::combat_input::PlayCommand;
use crate::play::feature_0080_chat_private_channel::view::Controls as PrivateControls;
use crate::play::{
    PlayLink,
    mailbox::{SharedMailbox, lock},
};
use oteryn_client_ui::chat::{Mode, panel_text};
use panel_controls::PanelControls;
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

unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let note = (w >> 16) & 0xffff;
            if !actions.is_null()
                && ((matches!(
                    id,
                    SEND | FILTER
                        | CHAT_MODE
                        | SAVE
                        | APPEND
                        | IGNORE
                        | ALLOW
                        | BLOCK
                        | CLEAR_LISTS
                ) && note == BN_CLICKED as usize
                    || id == TAB && note == LBN_SELCHANGE as usize)
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
        let Some(PanelControls {
            rooms,
            message,
            recipient,
            transcript,
            tabs,
            log_path,
        }) = PanelControls::new(hwnd)
        else {
            return;
        };
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
