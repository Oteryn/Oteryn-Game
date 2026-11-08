//! Tracking tooltip lifetime and HWND access stay on the shell's creating thread.
use super::*;
use oteryn_client::play::HoverTooltip;
use std::time::Instant;
use windows_sys::Win32::{Foundation::POINT, Graphics::Gdi::ScreenToClient, UI::Controls::*};

#[derive(Default)]
pub(super) struct NativeTooltip {
    hwnd: HWND,
    parent: HWND,
    text: Vec<u16>,
    hover: HoverTooltip,
}
impl NativeTooltip {
    unsafe fn info(&mut self) -> TOOLINFOW {
        unsafe {
            let mut info: TOOLINFOW = std::mem::zeroed();
            info.cbSize = std::mem::size_of::<TOOLINFOW>() as u32;
            info.hwnd = self.parent;
            info.uId = 1;
            info.uFlags = TTF_TRACK | TTF_ABSOLUTE;
            info.lpszText = self.text.as_mut_ptr();
            info
        }
    }
    pub(super) fn refresh(&mut self, parent: HWND, view: &PlayView, enabled: bool, delay_ms: u16) {
        unsafe {
            let mut cursor: POINT = std::mem::zeroed();
            let valid =
                enabled && GetForegroundWindow() == parent && GetCursorPos(&mut cursor) != 0;
            let mut local = cursor;
            let target = if valid && ScreenToClient(parent, &mut local) != 0 {
                view.tooltip_target([local.x as f32, local.y as f32])
            } else {
                None
            };
            let label = self
                .hover
                .update(
                    target,
                    Instant::now(),
                    Duration::from_millis(u64::from(delay_ms)),
                )
                .map(str::to_owned);
            let Some(label) = label else {
                if !self.hwnd.is_null() {
                    let info = self.info();
                    SendMessageW(self.hwnd, TTM_TRACKACTIVATE, 0, &info as *const _ as isize);
                }
                return;
            };
            self.parent = parent;
            let old = std::mem::replace(&mut self.text, wide(&label));
            if self.hwnd.is_null() {
                let common = INITCOMMONCONTROLSEX {
                    dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                    dwICC: ICC_WIN95_CLASSES,
                };
                InitCommonControlsEx(&common);
                self.hwnd = CreateWindowExW(
                    WS_EX_TOPMOST,
                    TOOLTIPS_CLASSW,
                    null(),
                    WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    CW_USEDEFAULT,
                    parent,
                    null_mut(),
                    null_mut(),
                    null(),
                );
                if self.hwnd.is_null() {
                    return;
                }
                let info = self.info();
                SendMessageW(self.hwnd, TTM_ADDTOOLW, 0, &info as *const _ as isize);
                SendMessageW(self.hwnd, TTM_SETMAXTIPWIDTH, 0, 400);
            } else {
                let info = self.info();
                SendMessageW(self.hwnd, TTM_UPDATETIPTEXTW, 0, &info as *const _ as isize);
            }
            drop(old);
            let position =
                u32::from((cursor.x + 16) as u16) | (u32::from((cursor.y + 24) as u16) << 16);
            SendMessageW(self.hwnd, TTM_TRACKPOSITION, 0, position as isize);
            let info = self.info();
            SendMessageW(self.hwnd, TTM_TRACKACTIVATE, 1, &info as *const _ as isize);
        }
    }
}
impl Drop for NativeTooltip {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.is_null() {
                DestroyWindow(self.hwnd);
            }
        }
    }
}
