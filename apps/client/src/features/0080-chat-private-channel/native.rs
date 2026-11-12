//! Win32 FFI for private-channel controls, used only on the chat window's creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::mailbox::Mailbox;
use oteryn_client_ui::chat_private_channel::{ChatPanel, Intent, leave_label, panel_text};
use std::ptr::{null, null_mut};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

pub(in crate::play) struct Controls {
    channels: HWND,
    leave: HWND,
    name: HWND,
    message: HWND,
    transcript: HWND,
    ids: Vec<u16>,
    held: Option<ChatPanel>,
}

fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

impl Controls {
    pub(in crate::play) fn accepts(id: usize, note: usize) -> bool {
        (matches!(id, 201..=207) && note == BN_CLICKED as usize)
            || (id == 200 && note == LBN_SELCHANGE as usize)
    }

    pub(in crate::play) unsafe fn new(parent: HWND) -> Self {
        unsafe {
            control(
                parent,
                "STATIC",
                tr!("Private channels"),
                210,
                [700, 10, 370, 22],
                0,
            );
            let channels = control(
                parent,
                "LISTBOX",
                "",
                200,
                [700, 36, 370, 90],
                LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL,
            );
            let mut leave = null_mut();
            for (id, label, x, y) in [
                (201, tr!("Create/Open own"), 700, 132),
                (202, tr!("Refresh"), 890, 132),
                (203, tr!("Join"), 700, 166),
                (204, tr!("Leave"), 890, 166),
                (205, tr!("Invite player"), 700, 234),
                (206, tr!("Exclude player"), 890, 234),
                (207, tr!("Send"), 890, 302),
            ] {
                let button = control(parent, "BUTTON", label, id, [x, y, 180, 28], WS_TABSTOP);
                if id == 204 {
                    leave = button;
                }
            }
            let name = control(
                parent,
                "EDIT",
                "",
                208,
                [700, 200, 370, 28],
                WS_BORDER | WS_TABSTOP,
            );
            let message = control(
                parent,
                "EDIT",
                "",
                209,
                [700, 268, 370, 28],
                WS_BORDER | WS_TABSTOP,
            );
            let transcript = control(
                parent,
                "EDIT",
                "",
                211,
                [700, 340, 370, 240],
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
            );
            Self {
                channels,
                leave,
                name,
                message,
                transcript,
                ids: vec![],
                held: None,
            }
        }
    }

    pub(in crate::play) unsafe fn action(
        &mut self,
        id: usize,
        mailbox: &mut Mailbox,
        commands: &UnboundedSender<PlayCommand>,
    ) -> bool {
        unsafe {
            if id == 200 {
                if let Ok(row) = usize::try_from(SendMessageW(self.channels, LB_GETCURSEL, 0, 0)) {
                    mailbox.chat_private.channel = self.ids.get(row).copied();
                }
                return true;
            }
            let intent = match id {
                201 => Intent::Create,
                202 => Intent::List,
                203 => Intent::Join,
                204 => Intent::Leave,
                205 => Intent::Invite,
                206 => Intent::Exclude,
                207 => Intent::Send,
                _ => return false,
            };
            mailbox.chat_private.name = read_text(self.name);
            mailbox.chat_private.text = read_text(self.message);
            super::super::input::enqueue(mailbox, commands, intent);
            true
        }
    }

    pub(in crate::play) unsafe fn render(&mut self, panel: &ChatPanel) {
        unsafe {
            if self.held.as_ref() == Some(panel) {
                return;
            }
            if self.held.as_ref().map(|held| &held.list) != Some(&panel.list) {
                SendMessageW(self.channels, LB_RESETCONTENT, 0, 0);
                self.ids.clear();
                if let Some(list) = &panel.list {
                    if let Some(id) = list.own_channel {
                        self.add(id, &lfmt!("Own channel ({id})", id = id));
                    }
                    for invite in &list.invited {
                        self.add(
                            invite.channel_id,
                            &format!("{} ({})", invite.owner_name, invite.channel_id),
                        );
                    }
                }
            }
            if let Some(row) = self.ids.iter().position(|id| Some(*id) == panel.channel) {
                SendMessageW(self.channels, LB_SETCURSEL, row, 0);
            }
            SetWindowTextW(self.leave, wide(leave_label(panel)).as_ptr());
            SetWindowTextW(
                self.transcript,
                wide(&panel_text(panel).replace('\n', "\r\n")).as_ptr(),
            );
            self.held = Some(panel.clone());
        }
    }

    unsafe fn add(&mut self, id: u16, label: &str) {
        unsafe {
            SendMessageW(
                self.channels,
                LB_ADDSTRING,
                0,
                wide(label).as_ptr() as isize,
            );
        }
        self.ids.push(id);
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
