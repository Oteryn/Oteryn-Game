//! HWNDs stay on the form thread; the shell exchanges configuration and public results only.
// Win32 requires FFI for native edit controls and their window procedure.
#![allow(unsafe_code)]
use super::{FIELDS, LoginForm, LoginSettings, settings_path};
use crate::{GameplayEntryError, NativeLoginConfig};
use std::ptr::{null, null_mut};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::thread::JoinHandle;
use std::time::Duration;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

// IDOK lets IsDialogMessage route Enter to the default button.
const SUBMIT: usize = 1;
enum Update {
    Show(String),
    Finish(Result<(), GameplayEntryError>),
}
pub struct LoginWindow {
    updates: Option<Sender<Update>>,
    requests: Receiver<NativeLoginConfig>,
    thread: Option<JoinHandle<()>>,
}
impl LoginWindow {
    pub fn open() -> Result<Self, String> {
        let (updates, incoming) = mpsc::channel();
        let (requests, outgoing) = mpsc::channel();
        let (ready, initialized) = mpsc::sync_channel(1);
        let thread = std::thread::Builder::new()
            .name("native-login".into())
            .spawn(move || {
                let result = run(incoming, requests, &ready);
                if let Err(error) = result {
                    let _ = ready.send(Err(error));
                }
            })
            .map_err(|_| "Cannot start the login window.".to_owned())?;
        initialized
            .recv()
            .map_err(|_| "Login window stopped.".to_owned())??;
        Ok(Self {
            updates: Some(updates),
            requests: outgoing,
            thread: Some(thread),
        })
    }
    pub fn poll(&self) -> Option<NativeLoginConfig> {
        self.requests.try_recv().ok()
    }
    pub fn show(&self, message: String) {
        self.send(Update::Show(message));
    }
    pub fn finish(&self, result: Result<(), GameplayEntryError>) {
        self.send(Update::Finish(result));
    }
    fn send(&self, update: Update) {
        if let Some(sender) = &self.updates {
            let _ = sender.send(update);
        }
    }
}
impl Drop for LoginWindow {
    fn drop(&mut self) {
        self.updates.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
unsafe extern "system" fn procedure(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if message == WM_COMMAND
            && (1..=5).contains(&(w & 0xffff))
            && (w >> 16) & 0xffff == BN_CLICKED as usize
        {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (w & 0xffff) as isize);
            return 0;
        }
        if message == WM_CLOSE {
            ShowWindow(hwnd, SW_HIDE);
            return 0;
        }
        DefWindowProcW(hwnd, message, w, l)
    }
}
fn run(
    updates: Receiver<Update>,
    requests: Sender<NativeLoginConfig>,
    ready: &mpsc::SyncSender<Result<(), String>>,
) -> Result<(), String> {
    crate::crash_report::review_pending();
    let path = settings_path();
    let mut bulletin = crate::bulletin::Browser::default();
    let bulletin_warning = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(std::path::Path::to_owned))
        .and_then(|p| match crate::bulletin::Browser::load(&p) {
            Ok(loaded) => {
                bulletin = loaded;
                None
            }
            Err(error) => Some(error),
        });
    let loaded = path
        .as_ref()
        .map_err(Clone::clone)
        .and_then(|path| LoginSettings::load(path));
    let warning = loaded.as_ref().err().cloned();
    let mut form = LoginForm::new(
        loaded
            .unwrap_or_default()
            .with_env(|name| std::env::var(name).ok()),
    );
    if let Some(warning) = warning {
        form.feedback(warning);
    }
    unsafe {
        let hwnd = CreateWindowExW(
            WS_EX_CONTROLPARENT,
            wide("STATIC").as_ptr(),
            wide("Oteryn — Connection and sign-in").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            680,
            760,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            return Err("Cannot create the login window.".into());
        }
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
        let mut controls = Vec::new();
        for (index, (label, _)) in FIELDS.iter().enumerate() {
            let y = 15 + index as i32 * 48;
            controls.push(control(hwnd, "STATIC", label, 0, [15, y, 630, 20], 0));
            controls.push(control(
                hwnd,
                "EDIT",
                form.settings.values()[index],
                100 + index,
                [15, y + 20, 630, 25],
                WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
            ));
        }
        let button = control(
            hwnd,
            "BUTTON",
            "Save and sign in",
            SUBMIT,
            [15, 315, 200, 32],
            WS_TABSTOP | BS_DEFPUSHBUTTON as u32,
        );
        for (id, label, x) in [
            (2, "News", 15),
            (3, "Previous month", 155),
            (4, "Calendar", 315),
            (5, "Next month", 475),
        ] {
            controls.push(control(
                hwnd,
                "BUTTON",
                label,
                id,
                [x, 355, 140, 30],
                WS_TABSTOP,
            ));
        }
        if let Some(error) = bulletin_warning {
            form.feedback(error);
        }
        let status = control(
            hwnd,
            "EDIT",
            form.message(),
            201,
            [15, 400, 630, 300],
            WS_BORDER
                | WS_VSCROLL
                | ES_MULTILINE as u32
                | ES_READONLY as u32
                | ES_AUTOVSCROLL as u32,
        );
        if controls
            .iter()
            .chain([&button, &status])
            .any(|control| control.is_null())
        {
            DestroyWindow(hwnd);
            return Err("Cannot create login fields.".into());
        }
        SetFocus(controls[1]);
        let _ = ready.send(Ok(()));
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            match updates.try_recv() {
                Ok(Update::Show(message)) => {
                    form.feedback(message);
                    ShowWindow(hwnd, SW_SHOW);
                    SetForegroundWindow(hwnd);
                }
                Ok(Update::Finish(result)) => {
                    form.finish(result);
                    ShowWindow(hwnd, if result.is_ok() { SW_HIDE } else { SW_SHOW });
                }
                Err(TryRecvError::Disconnected) => break,
                Err(TryRecvError::Empty) => {}
            }
            let action = GetWindowLongPtrW(hwnd, GWLP_USERDATA);
            if action >= 2 {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                if action == 3 {
                    bulletin.previous_month();
                }
                if action == 5 {
                    bulletin.next_month();
                }
                form.feedback(if action == 2 {
                    bulletin.news()
                } else {
                    bulletin.month()
                });
            }
            if action == SUBMIT as isize {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                if !form.pending() {
                    form.settings = LoginSettings::from_values(std::array::from_fn(|index| {
                        read_text(controls[index * 2 + 1])
                    }));
                    let saved = path
                        .as_ref()
                        .map_err(Clone::clone)
                        .and_then(|path| form.settings.save(path));
                    match saved.and_then(|()| form.submit()) {
                        Ok(config) => {
                            if requests.send(config).is_err() {
                                break;
                            }
                        }
                        Err(error) => form.feedback(error),
                    }
                }
            }
            for edit in controls
                .iter()
                .take(FIELDS.len() * 2)
                .skip(1)
                .step_by(2)
                .chain(std::iter::once(&button))
            {
                EnableWindow(*edit, i32::from(!form.pending()));
            }
            SetWindowTextW(button, wide(form.submit_caption()).as_ptr());
            SetWindowTextW(status, wide(form.message()).as_ptr());
            std::thread::sleep(Duration::from_millis(30));
        }
        DestroyWindow(hwnd);
    }
    Ok(())
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
        let length = GetWindowTextLengthW(hwnd).clamp(0, 4096);
        let mut value = vec![0; length as usize + 1];
        let read = GetWindowTextW(hwnd, value.as_mut_ptr(), length + 1).max(0) as usize;
        String::from_utf16_lossy(&value[..read])
    }
}
