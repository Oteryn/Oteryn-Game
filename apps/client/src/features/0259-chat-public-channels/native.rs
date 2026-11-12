//! Native public-channel panel. HWND access remains on its creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_runtime::chat_public_channels::ChatRoom;
use oteryn_client_ui::chat_public_channels::{ChatPanel, Intent, panel_text, room_name};
use oteryn_client_ui::spell_action_bar::movement::MovementSpell;
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const ROOMS: usize = 101;
const MESSAGE: usize = 102;
const OPEN: usize = 103;
const CLOSE: usize = 104;
const SEND: usize = 105;
const LEVITATE_UP: usize = 107;
const LEVITATE_DOWN: usize = 108;
const MAGIC_ROPE: usize = 109;
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
                && ((id == ROOMS && note == LBN_SELCHANGE as usize)
                    || (matches!(
                        id,
                        OPEN | CLOSE | SEND | LEVITATE_UP | LEVITATE_DOWN | MAGIC_ROPE
                    ) && note == BN_CLICKED as usize))
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
        if mailbox.chat_public.open {
            return;
        }
        mailbox.chat_public.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("public-channels-panel".into())
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
            wide(tr!("Public Channels")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
            520,
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
            ROOMS,
            [10, 10, 230, 180],
            LBS_NOTIFY as u32 | WS_BORDER,
        );
        for room in ChatRoom::ALL {
            SendMessageW(
                rooms,
                LB_ADDSTRING,
                0,
                wide(room_name(room)).as_ptr() as isize,
            );
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
        for (x, label, id) in [
            (250, tr!("Open"), OPEN),
            (360, tr!("Close"), CLOSE),
            (470, tr!("Send"), SEND),
        ] {
            control(hwnd, "BUTTON", label, id, [x, 48, 100, 28], WS_TABSTOP);
        }
        for (y, label, id) in [
            (210, tr!("Levitate Up"), LEVITATE_UP),
            (250, tr!("Levitate Down"), LEVITATE_DOWN),
            (290, tr!("Magic Rope"), MAGIC_ROPE),
        ] {
            control(hwnd, "BUTTON", label, id, [10, y, 230, 28], WS_TABSTOP);
        }
        let transcript = control(
            hwnd,
            "EDIT",
            "",
            106,
            [250, 86, 430, 380],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let mut held: Option<ChatPanel> = None;
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
            for action in std::mem::take(&mut actions.0) {
                let movement = match action {
                    LEVITATE_UP => Some(MovementSpell::LevitateUp),
                    LEVITATE_DOWN => Some(MovementSpell::LevitateDown),
                    MAGIC_ROPE => Some(MovementSpell::MagicRope),
                    _ => None,
                };
                if let Some(spell) = movement {
                    crate::play::feature_local_movement_spells::input::enqueue(
                        &mut mailbox,
                        &commands,
                        spell,
                    );
                    continue;
                }
                if action == ROOMS {
                    if let Ok(row) = usize::try_from(SendMessageW(rooms, LB_GETCURSEL, 0, 0))
                        && let Some(room) = ChatRoom::ALL.get(row)
                    {
                        mailbox.chat_public.room = *room;
                    }
                    continue;
                }
                let room = mailbox.chat_public.room;
                let intent = match action {
                    OPEN => Intent::Open(room),
                    CLOSE => Intent::Close(room),
                    SEND => {
                        mailbox.chat_public.text = read_text(message);
                        Intent::Send(room)
                    }
                    _ => continue,
                };
                super::super::input::enqueue(&mut mailbox, &commands, intent);
            }
            if held.as_ref() != Some(&mailbox.chat_public) {
                SetWindowTextW(transcript, wide(&panel_text(&mailbox.chat_public)).as_ptr());
                held = Some(mailbox.chat_public.clone());
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            let _ = DestroyWindow(hwnd);
        }
        close(&weak);
    }
}

fn close(weak: &WeakMailbox) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).chat_public.open = false;
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
