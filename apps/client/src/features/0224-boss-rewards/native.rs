//! Windows personal boss reward panel; HWND access remains on its creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::boss_rewards::{BossRewardsPanel, Intent, panel_text};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
const DEATH: usize = 101;
const ORDINAL: usize = 102;
const BROWSE: usize = 103;
const TAKE: usize = 104;
const GROUP: usize = 105;
const ALL: usize = 106;
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
                && matches!(id, BROWSE | TAKE | GROUP | ALL)
                && note == BN_CLICKED as usize
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
    if link.uses_shared_frontend() {
        return;
    }
    {
        let mut m = lock(&link.mailbox);
        if m.boss_rewards.open {
            return;
        }
        m.boss_rewards.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("boss-rewards-panel".into())
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
            wide(tr!("Boss rewards")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            650,
            480,
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
        control(
            hwnd,
            "STATIC",
            tr!("Death id (32 hex digits)"),
            110,
            [10, 10, 190, 24],
            0,
        );
        let death = control(
            hwnd,
            "EDIT",
            "",
            DEATH,
            [205, 10, 400, 24],
            WS_BORDER | WS_TABSTOP,
        );
        control(
            hwnd,
            "STATIC",
            tr!("Reward ordinal"),
            111,
            [10, 45, 120, 24],
            0,
        );
        let ordinal = control(
            hwnd,
            "EDIT",
            "1",
            ORDINAL,
            [135, 45, 80, 24],
            ES_NUMBER as u32 | WS_BORDER | WS_TABSTOP,
        );
        for (i, (label, id)) in [
            (tr!("Browse"), BROWSE),
            (tr!("Take item"), TAKE),
            (tr!("Collect group"), GROUP),
            (tr!("Collect all"), ALL),
        ]
        .iter()
        .enumerate()
        {
            control(
                hwnd,
                "BUTTON",
                label,
                *id,
                [10 + i as i32 * 145, 80, 135, 30],
                WS_TABSTOP,
            );
        }
        let description = control(
            hwnd,
            "EDIT",
            "",
            112,
            [10, 125, 595, 295],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        let mut held: Option<BossRewardsPanel> = None;
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
            if mailbox.boss_rewards_shutdown {
                break;
            }
            for action in std::mem::take(&mut actions.0) {
                if action != ALL {
                    let text = read_text(death);
                    mailbox.boss_rewards.death = parse_death(&text).unwrap_or([0; 16]);
                }
                mailbox.boss_rewards.selected_ordinal =
                    read_text(ordinal).trim().parse().unwrap_or(0);
                let intent = match action {
                    BROWSE => Intent::Browse,
                    TAKE => Intent::Take,
                    GROUP => Intent::CollectGroup,
                    ALL => Intent::CollectAll,
                    _ => continue,
                };
                super::super::input::enqueue(&mut mailbox, &commands, intent);
            }
            if held.as_ref() != Some(&mailbox.boss_rewards) {
                SetWindowTextW(
                    description,
                    wide(&panel_text(&mailbox.boss_rewards)).as_ptr(),
                );
                held = Some(mailbox.boss_rewards.clone());
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
fn parse_death(text: &str) -> Option<[u8; 16]> {
    let text = text.trim();
    if text.len() != 32 {
        return None;
    }
    let mut out = [0; 16];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}
fn close(weak: &WeakMailbox) {
    if let Some(m) = weak.upgrade() {
        lock(&m).boss_rewards.open = false;
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
unsafe fn read_text(hwnd: HWND) -> String {
    unsafe {
        let length = GetWindowTextLengthW(hwnd);
        let mut value = vec![0; usize::try_from(length).unwrap_or(0) + 1];
        let read = GetWindowTextW(hwnd, value.as_mut_ptr(), length + 1);
        String::from_utf16_lossy(&value[..usize::try_from(read).unwrap_or(0)])
    }
}
