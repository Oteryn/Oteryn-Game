//! Help stays on the settings thread; report opening is deferred to the game thread.
#![allow(unsafe_code, reason = "Win32 controls are owned by this dialog thread")]
use super::{Shared, control, lock, wide};
use oteryn_client::play::{HelpTopic, help_text};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};

unsafe extern "system" fn panel_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == 602 && w >> 16 == BN_CLICKED as usize {
            let pending = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut bool;
            if !pending.is_null() {
                *pending = true;
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}

pub(super) fn open(shared: Arc<Mutex<Shared>>, parent: HWND) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Oteryn Help").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            800,
            570,
            parent,
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return;
        }
        EnableWindow(parent, 0);
        let mut pending = Box::new(false);
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (&mut *pending as *mut bool) as isize);
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, panel_proc as *const () as isize);
        let topics = control(
            hwnd,
            "COMBOBOX",
            "",
            601,
            [16, 16, 480, 160],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let reports = control(hwnd, "BUTTON", "Open Reports", 602, [520, 16, 240, 28], 0);
        let body = control(
            hwnd,
            "EDIT",
            "",
            603,
            [16, 62, 744, 450],
            WS_BORDER
                | WS_VSCROLL
                | ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32,
        );
        if [topics, reports, body].iter().any(|h| h.is_null()) {
            DestroyWindow(hwnd);
            EnableWindow(parent, 1);
            return;
        }
        for label in [
            "Frequently asked questions",
            "Playing manual",
            "Client help and troubleshooting",
        ] {
            SendMessageW(topics, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(topics, CB_SETCURSEL, 0, 0);
        let mut selected = -1;
        let mut message: MSG = std::mem::zeroed();
        while IsWindow(hwnd) != 0 && !lock(&shared).shutdown {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if std::mem::take(&mut *pending) {
                lock(&shared).request_help_report();
            }
            let index = SendMessageW(topics, CB_GETCURSEL, 0, 0);
            if index != selected {
                selected = index;
                let topic = match index {
                    1 => HelpTopic::Manual,
                    2 => HelpTopic::Client,
                    _ => HelpTopic::Questions,
                };
                SetWindowTextW(body, wide(help_text(topic)).as_ptr());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        EnableWindow(parent, 1);
    }
}
