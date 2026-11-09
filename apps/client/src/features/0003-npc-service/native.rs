//! Native dialogue controls stay on their creating thread.
#![allow(unsafe_code)]
use crate::{
    combat_input::PlayCommand,
    play::{
        PlayLink,
        mailbox::{Mailbox, lock},
    },
};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, Weak},
    time::Duration,
};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
    UI::WindowsAndMessaging::*,
};
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: userdata belongs to this window thread until destruction.
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && w >> 16 == BN_CLICKED as usize && (*actions).0.len() < 16 {
                (*actions).0.push(w & 0xffff);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(in crate::play) fn context_menu(link: &PlayLink, npc: oteryn_session::EntityRef) {
    // SAFETY: menu and cursor operations run on the shell UI thread.
    unsafe {
        let menu = CreatePopupMenu();
        if menu.is_null() {
            return;
        }
        AppendMenuW(menu, MF_STRING, 1, wide(tr!("Talk")).as_ptr());
        AppendMenuW(menu, MF_STRING, 2, wide(tr!("Trade")).as_ptr());
        let mut point = POINT { x: 0, y: 0 };
        GetCursorPos(&mut point);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            GetForegroundWindow(),
            null(),
        );
        DestroyMenu(menu);
        match chosen {
            1 => link.talk_to_npc(npc, "hi"),
            2 => link.talk_to_npc(npc, "trade"),
            _ => {}
        }
    }
}
pub(in crate::play) fn open_window(link: &PlayLink) {
    {
        let mut mailbox = lock(&link.mailbox);
        if !mailbox.dialogue.visible() || mailbox.dialogue_window || mailbox.dialogue_shutdown {
            return;
        }
        mailbox.dialogue_window = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let fallback = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("npc-dialogue".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&fallback);
    }
}
fn close(weak: &Weak<Mutex<Mailbox>>) {
    if let Some(shared) = weak.upgrade() {
        lock(&shared).dialogue_window = false;
    }
}
fn run(weak: Weak<Mutex<Mailbox>>, commands: UnboundedSender<PlayCommand>) {
    // SAFETY: all HWNDs and their userdata are created, used and destroyed on this thread.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(tr!("NPC dialogue — Ctrl+Shift+X")).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
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
        let reply = control(
            hwnd,
            "EDIT",
            "",
            0,
            (10, 10, 680, 300),
            WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
        );
        let input = control(
            hwnd,
            "EDIT",
            "",
            101,
            (10, 320, 550, 30),
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(input, EM_SETLIMITTEXT, 255, 0);
        let send = control(hwnd, "BUTTON", tr!("Send"), 102, (570, 320, 110, 30), 0);
        control(hwnd, "BUTTON", tr!("Trade"), 105, (400, 510, 110, 30), 0);
        let previous = control(
            hwnd,
            "BUTTON",
            tr!("Previous keywords"),
            103,
            (10, 510, 180, 30),
            0,
        );
        let next = control(
            hwnd,
            "BUTTON",
            tr!("Next keywords"),
            104,
            (200, 510, 180, 30),
            0,
        );
        if [reply, input, send, previous, next]
            .iter()
            .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut page = 0usize;
        let mut redraw = true;
        let mut buttons = Vec::new();
        let mut held = None;
        let mut words: Vec<String> = Vec::new();
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let Some(shared) = weak.upgrade() else {
                break;
            };
            let mut mailbox = lock(&shared);
            if mailbox.dialogue_shutdown || !mailbox.dialogue.visible() {
                break;
            }
            let panel = &mut mailbox.dialogue;
            for id in std::mem::take(&mut actions.0) {
                if id == 103 || id == 104 {
                    page = if id == 103 {
                        page.saturating_sub(1)
                    } else {
                        (page + 1).min(words.len().saturating_sub(1) / 16)
                    };
                    redraw = true;
                    continue;
                }
                let text = if id == 105 {
                    "trade".into()
                } else if id == 102 {
                    let mut buffer = [0u16; 256];
                    let len = GetWindowTextW(input, buffer.as_mut_ptr(), 256);
                    String::from_utf16_lossy(&buffer[..len.max(0) as usize])
                } else if let Some(word) = id.checked_sub(200).and_then(|i| words.get(i)) {
                    word.clone()
                } else {
                    continue;
                };
                if let Some(npc) = panel.target
                    && let Some(intent) = oteryn_client_ui::npc_dialogue::request(panel, npc, &text)
                {
                    if commands.send(PlayCommand::NpcTalk(intent)).is_err() {
                        panel.open = false;
                    }
                    SetWindowTextW(input, wide("").as_ptr());
                }
            }
            if redraw || held.as_ref() != Some(panel) {
                SetWindowTextW(
                    reply,
                    wide(&oteryn_client_ui::npc_dialogue::panel_text(panel).replace('\n', "\r\n"))
                        .as_ptr(),
                );
                let new_words: Vec<String> =
                    panel.keywords().into_iter().map(str::to_owned).collect();
                if new_words != words {
                    page = 0;
                    redraw = true;
                }
                if redraw {
                    for button in buttons.drain(..) {
                        DestroyWindow(button);
                    }
                    words = new_words;
                    for (i, word) in words.iter().enumerate().skip(page * 16).take(16) {
                        let button = control(
                            hwnd,
                            "BUTTON",
                            word,
                            200 + i,
                            (
                                10 + (i % 4) as i32 * 170,
                                365 + ((i % 16) / 4) as i32 * 32,
                                165,
                                30,
                            ),
                            0,
                        );
                        buttons.push(button);
                    }
                }
                for h in buttons.iter().copied().chain([send, input]) {
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                        h,
                        i32::from(!panel.pending && panel.selected && panel.target.is_some()),
                    );
                }
                for (button, enabled) in
                    [(previous, page > 0), (next, (page + 1) * 16 < words.len())]
                {
                    windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow(
                        button,
                        i32::from(enabled),
                    );
                }
                redraw = false;
                held = Some(panel.clone());
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
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    bounds: (i32, i32, i32, i32),
    style: u32,
) -> HWND {
    // SAFETY: parent and temporary UTF-16 strings remain valid for this call.
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | style,
            bounds.0,
            bounds.1,
            bounds.2,
            bounds.3,
            parent,
            id as *mut _,
            null_mut(),
            null(),
        )
    }
}
