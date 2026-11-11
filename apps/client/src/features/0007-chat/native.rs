//! Native local-chat panel. HWND access remains on its creating thread.
#![allow(unsafe_code)]
use super::console;
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
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetFocus, SetFocus};
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

#[path = "native_helpers.rs"]
mod helpers;
use helpers::*;
#[path = "native_channels.rs"]
mod channel_dialog;
#[path = "native_console.rs"]
mod console_controls;

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
        .spawn(move || dialog::run(weak, commands))
        .is_err()
    {
        close(&rollback);
    }
}

fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() {
        console::close(&mut lock(&mailbox));
    }
}

#[path = "chat_dialog.rs"]
mod dialog;

#[path = "native_mirror.rs"]
mod mirror;

unsafe fn keyboard(
    msg: &MSG,
    weak: &WeakMailbox,
    commands: &UnboundedSender<PlayCommand>,
    input: &mut crate::play::PreferenceInput,
) -> bool {
    use oteryn_input_actions::{Modifier, Modifiers};
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };
    unsafe {
        if msg.message == WM_KILLFOCUS {
            let _ = input.route_chat(
                &crate::play::ReferencePreferences::default(),
                &[oteryn_input_actions::NormalizedInputEvent::FocusChanged { focused: false }],
                true,
            );
            return false;
        }
        if !matches!(
            msg.message,
            WM_KEYDOWN | WM_KEYUP | WM_SYSKEYDOWN | WM_SYSKEYUP
        ) {
            return false;
        }
        let Some(mailbox) = weak.upgrade() else {
            return false;
        };
        let mut modifiers = Modifiers::NONE;
        for (modifier, down) in [
            (Modifier::Control, GetKeyState(i32::from(VK_CONTROL)) < 0),
            (Modifier::Shift, GetKeyState(i32::from(VK_SHIFT)) < 0),
            (Modifier::Alt, GetKeyState(i32::from(VK_MENU)) < 0),
            (
                Modifier::Super,
                GetKeyState(i32::from(VK_LWIN)) < 0 || GetKeyState(i32::from(VK_RWIN)) < 0,
            ),
        ] {
            if down {
                modifiers = modifiers.with(modifier);
            }
        }
        let link = PlayLink {
            owns_session: false,
            mailbox,
            commands: commands.clone(),
        };
        console::native_key(
            &link,
            input,
            msg.wParam as u32,
            modifiers,
            matches!(msg.message, WM_KEYDOWN | WM_SYSKEYDOWN),
            msg.lParam & (1 << 30) != 0,
            msg.lParam & (1 << 24) != 0,
        )
    }
}
