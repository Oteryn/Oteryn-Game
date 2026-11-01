//! Minimal Windows Prey controls; all HWND access stays on the creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::prey::{Intent, PreyPanel, panel_text};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
const SLOTS: usize = 101;
const REFRESH: usize = 102;
const LIST: usize = 103;
const SELECT: usize = 104;
const ANY: usize = 105;
const BONUS: usize = 106;
const OPTION: usize = 107;
const CREATURE: usize = 108;
const CANDIDATE: usize = 109;
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
                && ((id == SLOTS && note == LBN_SELCHANGE as usize)
                    || (matches!(id, REFRESH | LIST | SELECT | ANY | BONUS | OPTION)
                        && note == BN_CLICKED as usize))
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
        let mut m = lock(&link.mailbox);
        if m.prey.open {
            return;
        }
        m.prey.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("prey-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&rollback)
    }
}
fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Prey").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            760,
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
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, proc as *const () as isize);
        let slots = control(
            hwnd,
            "LISTBOX",
            "",
            SLOTS,
            [10, 10, 720, 230],
            LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let creature = control(
            hwnd,
            "EDIT",
            "oteryn:",
            CREATURE,
            [10, 250, 250, 26],
            WS_BORDER | WS_TABSTOP,
        );
        let candidate = control(
            hwnd,
            "EDIT",
            "1",
            CANDIDATE,
            [270, 250, 50, 26],
            ES_NUMBER as u32 | WS_BORDER,
        );
        let buttons = [
            ("Refresh", REFRESH),
            ("Reroll list", LIST),
            ("Select candidate", SELECT),
            ("Select any", ANY),
            ("Reroll bonus", BONUS),
            ("Set option", OPTION),
        ];
        for (i, (label, id)) in buttons.iter().enumerate() {
            control(
                hwnd,
                "BUTTON",
                label,
                *id,
                [
                    10 + (i as i32 % 3) * 180,
                    285 + (i as i32 / 3) * 34,
                    170,
                    28,
                ],
                WS_TABSTOP,
            );
        }
        let description = control(
            hwnd,
            "EDIT",
            "",
            110,
            [10, 360, 720, 150],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let mut held: Option<PreyPanel> = None;
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else { break };
            let mut mailbox = lock(&shared);
            if mailbox.prey_shutdown {
                break;
            }
            for action in std::mem::take(&mut actions.0) {
                let intent = match action {
                    SLOTS => {
                        if let Ok(row) = u8::try_from(SendMessageW(slots, LB_GETCURSEL, 0, 0)) {
                            mailbox.prey.slot = row + 1;
                        }
                        None
                    }
                    REFRESH => Some(Intent::Refresh),
                    LIST => Some(Intent::ListReroll),
                    SELECT => {
                        mailbox.prey.candidate = read_u8(candidate).unwrap_or(0);
                        Some(Intent::Select)
                    }
                    ANY => {
                        mailbox.prey.creature = read_text(creature);
                        Some(Intent::SelectAny)
                    }
                    BONUS => Some(Intent::BonusReroll),
                    OPTION => {
                        mailbox.prey.option = (mailbox.prey.option + 1) % 3;
                        Some(Intent::SetOption)
                    }
                    _ => None,
                };
                if let Some(intent) = intent {
                    super::super::input::enqueue(&mut mailbox, &commands, intent)
                }
            }
            if held.as_ref() != Some(&mailbox.prey) {
                SendMessageW(slots, LB_RESETCONTENT, 0, 0);
                if let Some(reply) = &mailbox.prey.reply {
                    for slot in &reply.slots {
                        SendMessageW(
                            slots,
                            LB_ADDSTRING,
                            0,
                            wide(&oteryn_client_ui::prey::slot_text(slot)).as_ptr() as isize,
                        );
                    }
                }
                SendMessageW(slots, LB_SETCURSEL, usize::from(mailbox.prey.slot - 1), 0);
                SetWindowTextW(description, wide(&panel_text(&mailbox.prey)).as_ptr());
                held = Some(mailbox.prey.clone());
            }
            drop(mailbox);
            std::thread::sleep(Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd)
        }
        close(&weak)
    }
}
fn close(weak: &WeakMailbox) {
    if let Some(m) = weak.upgrade() {
        lock(&m).prey.open = false
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    b: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            b[0],
            b[1],
            b[2],
            b[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
unsafe fn read_u8(hwnd: HWND) -> Option<u8> {
    read_text(hwnd).trim().parse().ok()
}
unsafe fn read_text(hwnd: HWND) -> String {
    unsafe {
        let length = GetWindowTextLengthW(hwnd);
        let mut value = vec![0; usize::try_from(length).unwrap_or(0) + 1];
        let read = GetWindowTextW(hwnd, value.as_mut_ptr(), length + 1);
        String::from_utf16_lossy(&value[..usize::try_from(read).unwrap_or(0)])
    }
}
