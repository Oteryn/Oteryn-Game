//! Windows controls for the imbuement shrine; HWNDs stay on their creating thread.
#![allow(unsafe_code)]
use crate::combat_input::PlayCommand;
use crate::play::{PlayLink, mailbox::lock};
use oteryn_client_ui::imbue::{ImbuePanel, Intent, panel_text};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Weak};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const OFFERS: usize = 101;
const REFRESH: usize = 102;
const APPLY: usize = 103;
const CLEAR: usize = 104;

#[derive(Default)]
struct Actions(Vec<usize>);

unsafe extern "system" fn window_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    // SAFETY: the creating thread owns the live Actions userdata and all controls.
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            let notification = (w >> 16) & 0xffff;
            if !actions.is_null()
                && ((id == OFFERS && notification == LBN_SELCHANGE as usize)
                    || (matches!(id, REFRESH | APPLY | CLEAR)
                        && notification == BN_CLICKED as usize))
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
        if mailbox.imbue.open {
            return;
        }
        mailbox.imbue.open = true;
    }
    let weak = Arc::downgrade(&link.mailbox);
    let failed = weak.clone();
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name("imbuement-panel".into())
        .spawn(move || run(weak, commands))
        .is_err()
    {
        close(&failed);
    }
}

fn run(
    weak: Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>,
    commands: tokio::sync::mpsc::UnboundedSender<PlayCommand>,
) {
    // SAFETY: this thread creates, services, and destroys every HWND below.
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Imbuement Shrine").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            620,
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
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, window_proc as *const () as isize);
        let offers = control(
            hwnd,
            "LISTBOX",
            "",
            OFFERS,
            [10, 10, 580, 260],
            LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_TABSTOP,
        );
        let refresh = control(
            hwnd,
            "BUTTON",
            "Refresh",
            REFRESH,
            [10, 280, 140, 30],
            WS_TABSTOP,
        );
        let apply = control(
            hwnd,
            "BUTTON",
            "Apply selected",
            APPLY,
            [160, 280, 190, 30],
            WS_TABSTOP,
        );
        let clear = control(
            hwnd,
            "BUTTON",
            "Clear selected slot",
            CLEAR,
            [360, 280, 230, 30],
            WS_TABSTOP,
        );
        let status = control(
            hwnd,
            "EDIT",
            "",
            105,
            [10, 320, 580, 190],
            ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
        );
        if [offers, refresh, apply, clear, status]
            .iter()
            .any(|handle| handle.is_null())
        {
            DestroyWindow(hwnd);
            close(&weak);
            return;
        }
        let mut held: Option<ImbuePanel> = None;
        let mut message: MSG = std::mem::zeroed();
        loop {
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
            if mailbox.imbue_shutdown {
                break;
            }
            for action in std::mem::take(&mut actions.0) {
                let intent = match action {
                    OFFERS => {
                        if let Ok(row) = usize::try_from(SendMessageW(offers, LB_GETCURSEL, 0, 0)) {
                            mailbox.imbue.offer = row;
                        }
                        None
                    }
                    REFRESH => Some(Intent::Refresh),
                    APPLY => Some(Intent::Apply),
                    CLEAR => Some(Intent::Clear),
                    _ => None,
                };
                if let Some(intent) = intent {
                    super::super::input::enqueue(&mut mailbox, &commands, intent);
                }
            }
            if held.as_ref() != Some(&mailbox.imbue) {
                SendMessageW(offers, LB_RESETCONTENT, 0, 0);
                if let Some(window) = &mailbox.imbue.window {
                    for (index, offer) in window.offers.iter().enumerate() {
                        let label = format!(
                            "{index}: family {} tier {} — {} gold",
                            offer.family, offer.tier, offer.gold
                        );
                        SendMessageW(offers, LB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                    }
                }
                SendMessageW(offers, LB_SETCURSEL, mailbox.imbue.offer, 0);
                SetWindowTextW(status, wide(&panel_text(&mailbox.imbue)).as_ptr());
                held = Some(mailbox.imbue.clone());
            }
            drop(mailbox);
            std::thread::sleep(std::time::Duration::from_millis(30));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        close(&weak);
    }
}

fn close(weak: &Weak<std::sync::Mutex<crate::play::mailbox::Mailbox>>) {
    if let Some(mailbox) = weak.upgrade() {
        lock(&mailbox).imbue.open = false;
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
    // SAFETY: parent is live and the synchronous call copies both UTF-16 strings.
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
