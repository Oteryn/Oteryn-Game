//! Appearance controls and a live standing-outfit preview from pinned local artwork.
#![allow(
    unsafe_code,
    reason = "Win32 controls and their userdata are owned by this thread"
)]
use crate::combat_input::PlayCommand;
use crate::play::{
    PlayLink,
    mailbox::{Mailbox, lock},
};
use std::ptr::{null, null_mut};
use std::sync::{Arc, Mutex, Weak};
use tokio::sync::mpsc::UnboundedSender;
use windows_sys::Win32::{
    Foundation::*,
    UI::{
        Controls::EM_SETLIMITTEXT, Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*,
    },
};

type Snapshot = (bool, bool, Vec<String>, String);
pub(in crate::play) struct Form {
    pub title: &'static str,
    pub labels: &'static [&'static str],
    pub buttons: &'static [&'static str],
    pub snapshot: fn(&Mailbox) -> Snapshot,
    pub set_open: fn(&mut Mailbox, bool),
    pub submit: fn(&mut Mailbox, &UnboundedSender<PlayCommand>, usize, Vec<String>),
}
type WeakMailbox = Weak<Mutex<Mailbox>>;
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && (w >> 16) & 0xffff == BN_CLICKED as usize {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            if !actions.is_null() && (*actions).0.len() < 32 {
                (*actions).0.push(w & 0xffff);
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
pub(in crate::play) fn open(link: &PlayLink, form: Form) {
    {
        let mut mailbox = lock(&link.mailbox);
        if (form.snapshot)(&mailbox).0 {
            return;
        }
        (form.set_open)(&mut mailbox, true);
    }
    let weak = Arc::downgrade(&link.mailbox);
    let rollback = weak.clone();
    let set_open = form.set_open;
    let commands = link.commands.clone();
    if std::thread::Builder::new()
        .name(format!("{}-panel", form.title))
        .spawn(move || run(weak, commands, form))
        .is_err()
        && let Some(shared) = rollback.upgrade()
    {
        set_open(&mut lock(&shared), false);
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | style,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            parent,
            id as _,
            null_mut(),
            null(),
        )
    }
}
unsafe fn text(hwnd: HWND) -> String {
    unsafe {
        let count = GetWindowTextLengthW(hwnd).clamp(0, 8192) as usize;
        let mut buffer = vec![0u16; count + 1];
        let length = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32).max(0) as usize;
        String::from_utf16_lossy(&buffer[..length])
    }
}
pub(in crate::play) fn run(weak: WeakMailbox, commands: UnboundedSender<PlayCommand>, form: Form) {
    unsafe {
        let mut bounds =
            crate::window_layout::restore(form.title, [CW_USEDEFAULT, CW_USEDEFAULT, 900, 760]);
        bounds[2] = bounds[2].max(900);
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide(form.title).as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE | WS_THICKFRAME,
            bounds[0],
            bounds[1],
            bounds[2],
            bounds[3],
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if !hwnd.is_null() {
            let mut actions = Box::<Actions>::default();
            SetWindowLongPtrW(
                hwnd,
                GWLP_USERDATA,
                actions.as_mut() as *mut Actions as isize,
            );
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
            let mut edits = Vec::new();
            let mut y = 10;
            for (index, label) in form.labels.iter().enumerate() {
                control(hwnd, "STATIC", label, 0, [12, y, 640, 20], 0);
                y += 22;
                let height = 26;
                let edit = control(
                    hwnd,
                    "EDIT",
                    "",
                    index + 10,
                    [12, y, 640, height],
                    WS_BORDER | WS_TABSTOP | ES_AUTOHSCROLL as u32,
                );
                SendMessageW(edit, EM_SETLIMITTEXT, 256, 0);
                edits.push(edit);
                y += height + 8;
            }
            let mut buttons = Vec::new();
            for (index, label) in form.buttons.iter().enumerate() {
                buttons.push(control(
                    hwnd,
                    "BUTTON",
                    label,
                    index + 100,
                    [
                        12 + (index % 4) as i32 * 160,
                        y + (index / 4) as i32 * 34,
                        150,
                        28,
                    ],
                    WS_TABSTOP,
                ));
            }
            y += form.buttons.len().div_ceil(4) as i32 * 34 + 8;
            let summary = control(
                hwnd,
                "EDIT",
                "",
                0,
                [12, y, 640, 700 - y],
                WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
            );
            let mut artwork = std::env::var_os(crate::map::ASSET_DIR_ENV)
                .and_then(|root| super::preview::Assets::load(std::path::Path::new(&root)).ok());
            control(hwnd, "STATIC", "Outfit preview", 0, [680, 10, 200, 25], 0);
            let artwork_status = control(hwnd, "STATIC", "", 0, [680, 240, 200, 80], 0);
            let mut last_fields: Vec<String> = Vec::new();
            let mut last_summary = String::new();
            let mut message: MSG = std::mem::zeroed();
            loop {
                let mut rect: RECT = std::mem::zeroed();
                if GetWindowRect(hwnd, &mut rect) != 0 && IsIconic(hwnd) == 0 {
                    bounds = [
                        rect.left,
                        rect.top,
                        rect.right - rect.left,
                        rect.bottom - rect.top,
                    ];
                }
                while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                    if IsDialogMessageW(hwnd, &message) == 0 {
                        TranslateMessage(&message);
                        DispatchMessageW(&message);
                    }
                }
                if IsWindow(hwnd) == 0 || commands.is_closed() {
                    break;
                }
                let Some(shared) = weak.upgrade() else {
                    break;
                };
                // Read HWNDs outside the mailbox lock; the session never waits for Win32.
                let values: Vec<String> = edits.iter().map(|edit| text(*edit)).collect();
                let (open, pending, fields, body, preview) = {
                    let mut mailbox = lock(&shared);
                    for action in actions.0.drain(..) {
                        if let Some(index) = action.checked_sub(100) {
                            (form.submit)(&mut mailbox, &commands, index, values.clone());
                        }
                    }
                    let (open, pending, fields, body) = (form.snapshot)(&mailbox);
                    (
                        open,
                        pending,
                        fields,
                        body,
                        mailbox.properties.appearance.preview(),
                    )
                };
                if !open {
                    break;
                }
                for (index, (edit, field)) in edits.iter().zip(&fields).enumerate() {
                    if last_fields.get(index) != Some(field) {
                        SetWindowTextW(*edit, wide(field).as_ptr());
                    }
                    EnableWindow(*edit, i32::from(!pending));
                }
                last_fields = fields;
                for button in &buttons {
                    EnableWindow(*button, i32::from(!pending));
                }
                if body != last_summary {
                    SetWindowTextW(summary, wide(&body).as_ptr());
                    last_summary = body;
                }
                match (artwork.as_mut(), preview.as_ref()) {
                    (Some(assets), Some(preview)) => match assets.draw(preview) {
                        Ok(pixels) => {
                            paint(hwnd, &pixels);
                            SetWindowTextW(artwork_status, wide("").as_ptr());
                        }
                        Err(message) => {
                            paint(hwnd, &vec![32; 96 * 96 * 4]);
                            SetWindowTextW(artwork_status, wide(&message).as_ptr());
                        }
                    },
                    _ => {
                        paint(hwnd, &vec![32; 96 * 96 * 4]);
                        SetWindowTextW(artwork_status, wide("Preview unavailable: load local outfit artwork and select a valid outfit.").as_ptr());
                    }
                };
                MsgWaitForMultipleObjects(0, null(), 0, 50, QS_ALLINPUT);
            }
            crate::window_layout::remember(form.title, bounds);
            if IsWindow(hwnd) != 0 {
                DestroyWindow(hwnd);
            }
        }
    }
    if let Some(shared) = weak.upgrade() {
        (form.set_open)(&mut lock(&shared), false);
    }
}

fn paint(hwnd: HWND, pixels: &[u8]) {
    use windows_sys::Win32::Graphics::Gdi::*;
    unsafe {
        let mut info: BITMAPINFO = std::mem::zeroed();
        info.bmiHeader.biSize = std::mem::size_of::<BITMAPINFOHEADER>() as u32;
        info.bmiHeader.biWidth = 96;
        info.bmiHeader.biHeight = -96;
        info.bmiHeader.biPlanes = 1;
        info.bmiHeader.biBitCount = 32;
        info.bmiHeader.biCompression = BI_RGB;
        let dc = GetDC(hwnd);
        if !dc.is_null() {
            StretchDIBits(
                dc,
                680,
                40,
                192,
                192,
                0,
                0,
                96,
                96,
                pixels.as_ptr().cast(),
                &info,
                DIB_RGB_COLORS,
                SRCCOPY,
            );
            ReleaseDC(hwnd, dc);
        }
    }
}
