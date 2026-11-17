//! Standard Win32 tooltips follow each slot button's mouse events.
#![allow(unsafe_code)] // Owned HWND and text buffers live on the shell thread.
use windows_sys::Win32::{
    Foundation::HWND,
    UI::{Controls::*, WindowsAndMessaging::*},
};
pub(super) struct Tips {
    hwnd: HWND,
    text: Vec<Vec<u16>>,
    added: Vec<bool>,
}
impl Tips {
    pub(super) fn new(parent: HWND, count: usize) -> Self {
        let hwnd = unsafe {
            let common = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_WIN95_CLASSES,
            };
            InitCommonControlsEx(&common);
            CreateWindowExW(
                WS_EX_TOPMOST,
                TOOLTIPS_CLASSW,
                std::ptr::null(),
                WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
                0,
                0,
                0,
                0,
                parent,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null(),
            )
        };
        Self {
            hwnd,
            text: vec![vec![0]; count],
            added: vec![false; count],
        }
    }
    pub(super) fn update(&mut self, parent: HWND, buttons: &[HWND], labels: &[String]) {
        if self.hwnd.is_null() {
            return;
        }
        for (slot, button) in buttons.iter().enumerate() {
            let text = labels.get(slot).map_or("", String::as_str);
            let previous = std::mem::replace(
                &mut self.text[slot],
                text.encode_utf16().chain(Some(0)).collect(),
            );
            unsafe {
                let mut info: TTTOOLINFOW = std::mem::zeroed();
                info.cbSize = std::mem::size_of::<TTTOOLINFOW>() as u32;
                info.uFlags = TTF_IDISHWND | TTF_SUBCLASS;
                info.hwnd = parent;
                info.uId = *button as usize;
                info.lpszText = self.text[slot].as_mut_ptr();
                SendMessageW(
                    self.hwnd,
                    if self.added[slot] {
                        TTM_UPDATETIPTEXTW
                    } else {
                        TTM_ADDTOOLW
                    },
                    0,
                    &info as *const _ as isize,
                );
                self.added[slot] = true;
            }
            drop(previous);
        }
    }
}
impl Drop for Tips {
    fn drop(&mut self) {
        if !self.hwnd.is_null() {
            unsafe {
                DestroyWindow(self.hwnd);
            }
        }
    }
}
