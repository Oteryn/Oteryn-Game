//! Native controls remain on the creating form thread. Win32 FFI is confined here.
#![allow(unsafe_code)]
use oteryn_foundation::CancellationToken;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*};

fn wide(value: &str) -> Vec<u16> {
    value
        .replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}
unsafe extern "system" fn procedure(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_CLOSE {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 99);
            return 0;
        }
        if msg == WM_COMMAND
            && (1..=99).contains(&(w & 0xffff))
            && (w >> 16) & 0xffff == BN_CLICKED as usize
        {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (w & 0xffff) as isize);
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
pub(crate) struct Window {
    hwnd: HWND,
    cancel: CancellationToken,
}
#[derive(Clone, Copy)]
pub(crate) struct Control {
    handle: HWND,
    combo: bool,
}
impl Window {
    pub(crate) fn new(
        title: &str,
        width: i32,
        height: i32,
        cancel: CancellationToken,
    ) -> Option<Self> {
        unsafe {
            let hwnd = CreateWindowExW(
                WS_EX_CONTROLPARENT,
                wide("STATIC").as_ptr(),
                wide(title).as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                width,
                height,
                null_mut(),
                null_mut(),
                null_mut(),
                null(),
            );
            if hwnd.is_null() {
                return None;
            }
            SetWindowLongPtrW(hwnd, GWLP_WNDPROC, procedure as *const () as isize);
            Some(Self { hwnd, cancel })
        }
    }
    fn control(
        &self,
        class: &str,
        text: &str,
        id: usize,
        bounds: [i32; 4],
        style: u32,
        combo: bool,
    ) -> Option<Control> {
        unsafe {
            let handle = CreateWindowExW(
                0,
                wide(class).as_ptr(),
                wide(text).as_ptr(),
                WS_CHILD | WS_VISIBLE | style,
                bounds[0],
                bounds[1],
                bounds[2],
                bounds[3],
                self.hwnd,
                id as _,
                null_mut(),
                null(),
            );
            (!handle.is_null()).then_some(Control { handle, combo })
        }
    }
    pub(crate) fn label(&self, text: &str, bounds: [i32; 4]) -> Option<Control> {
        self.control("STATIC", text, 0, bounds, 0, false)
    }
    pub(crate) fn output(&self, text: &str, bounds: [i32; 4]) -> Option<Control> {
        self.control(
            "EDIT",
            text,
            0,
            bounds,
            WS_TABSTOP | WS_BORDER | WS_VSCROLL | ES_MULTILINE as u32 | ES_READONLY as u32,
            false,
        )
    }
    pub(crate) fn button(&self, text: &str, id: usize, bounds: [i32; 4]) -> Option<Control> {
        self.control("BUTTON", text, id, bounds, WS_TABSTOP, false)
    }
    pub(crate) fn edit(&self, text: &str, id: usize, bounds: [i32; 4]) -> Option<Control> {
        self.control(
            "EDIT",
            text,
            id,
            bounds,
            WS_TABSTOP | WS_BORDER | ES_AUTOHSCROLL as u32,
            false,
        )
    }
    pub(crate) fn list(&self, id: usize, bounds: [i32; 4]) -> Option<Control> {
        self.control(
            "LISTBOX",
            "",
            id,
            bounds,
            WS_TABSTOP | WS_BORDER | WS_VSCROLL | WS_HSCROLL | LBS_NOTIFY as u32,
            false,
        )
    }
    pub(crate) fn combo(&self, id: usize, bounds: [i32; 4]) -> Option<Control> {
        self.control(
            "COMBOBOX",
            "",
            id,
            bounds,
            WS_TABSTOP | CBS_DROPDOWNLIST as u32,
            true,
        )
    }
    pub(crate) fn event(&self) -> usize {
        unsafe {
            if self.cancel.is_cancelled() {
                return 99;
            }
            let mut msg: MSG = std::mem::zeroed();
            while PeekMessageW(&mut msg, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(self.hwnd, &msg) == 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
            let result = GetWindowLongPtrW(self.hwnd, GWLP_USERDATA);
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            std::thread::sleep(std::time::Duration::from_millis(30));
            result as usize
        }
    }
    pub(crate) fn confirm(&self, text: &str) -> bool {
        unsafe {
            MessageBoxW(
                self.hwnd,
                wide(text).as_ptr(),
                wide(tr!("Confirm Oteryn account action")).as_ptr(),
                MB_YESNO | MB_DEFBUTTON2,
            ) == IDYES
        }
    }
    pub(crate) fn explain(&self, text: &str) {
        unsafe {
            MessageBoxW(
                self.hwnd,
                wide(text).as_ptr(),
                wide(tr!("Oteryn account services")).as_ptr(),
                MB_OK,
            );
        }
    }
}
impl Drop for Window {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}
impl Control {
    pub(crate) fn add(self, text: &str) {
        unsafe {
            if !self.combo {
                SendMessageW(self.handle, LB_SETHORIZONTALEXTENT, 4096, 0);
            }
            SendMessageW(
                self.handle,
                if self.combo {
                    CB_ADDSTRING
                } else {
                    LB_ADDSTRING
                },
                0,
                wide(text).as_ptr() as LPARAM,
            );
        }
    }
    pub(crate) fn select(self, index: usize) {
        unsafe {
            SendMessageW(
                self.handle,
                if self.combo {
                    CB_SETCURSEL
                } else {
                    LB_SETCURSEL
                },
                index,
                0,
            );
        }
    }
    pub(crate) fn selected(self) -> Option<usize> {
        unsafe {
            usize::try_from(SendMessageW(
                self.handle,
                if self.combo {
                    CB_GETCURSEL
                } else {
                    LB_GETCURSEL
                },
                0,
                0,
            ))
            .ok()
        }
    }
    pub(crate) fn text(self) -> String {
        unsafe {
            let len = GetWindowTextLengthW(self.handle).clamp(0, 4096);
            let mut text = vec![0; len as usize + 1];
            let n = GetWindowTextW(self.handle, text.as_mut_ptr(), len + 1).max(0) as usize;
            String::from_utf16_lossy(&text[..n])
        }
    }
    pub(crate) fn set(self, text: &str) {
        unsafe {
            SetWindowTextW(self.handle, wide(text).as_ptr());
        }
    }
    pub(crate) fn enable(self, enabled: bool) {
        unsafe {
            EnableWindow(self.handle, i32::from(enabled));
        }
    }
}

/// Keeps a waiting notice responsive while the async caller performs bounded HTTP work.
pub(crate) struct Progress {
    done: std::sync::mpsc::Sender<()>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Progress {
    pub(crate) fn open(cancel: CancellationToken) -> Option<Self> {
        let (done, incoming) = std::sync::mpsc::channel();
        let thread = std::thread::Builder::new().name("account-service-wait".into()).spawn(move || {
            let Some(window) = Window::new(tr!("Oteryn — Account service"), 540, 170, cancel) else { return; };
            let _label = window.label(tr!("Contacting Oteryn…\nClosing this notice does not cancel a submitted action.\nCheck its receipt before submitting another action."), [15, 15, 500, 100]);
            loop {
                match incoming.try_recv() {
                    Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                    Err(std::sync::mpsc::TryRecvError::Empty) => {}
                }
                if window.event() == 99 { break; }
            }
        }).ok()?;
        Some(Self {
            done,
            thread: Some(thread),
        })
    }
}
impl Drop for Progress {
    fn drop(&mut self) {
        let _ = self.done.send(());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
