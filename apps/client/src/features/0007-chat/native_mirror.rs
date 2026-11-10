//! A second transcript shares the source history and has no composer or send controls.
use super::*;
use oteryn_client_ui::chat::{ChatPanel, ConsoleTab, mirror_text};
use windows_sys::Win32::{
    Foundation::{POINT, RECT},
    Graphics::Gdi::ScreenToClient,
    UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture},
};

pub(super) struct Mirror {
    parent: HWND,
    transcript: HWND,
    dragged: Option<ConsoleTab>,
    held: String,
}
impl Mirror {
    pub(super) unsafe fn new(parent: HWND) -> Self {
        unsafe {
            control(
                parent,
                "STATIC",
                tr!("Read-only mirror: drag a channel tab here"),
                0,
                [1100, 48, 370, 28],
                0,
            );
            let transcript = control(
                parent,
                "EDIT",
                "",
                0,
                [1100, 86, 370, 380],
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
            );
            ShowWindow(transcript, SW_HIDE);
            Self {
                parent,
                transcript,
                dragged: None,
                held: String::new(),
            }
        }
    }
    pub(super) unsafe fn input(&mut self, msg: &MSG, tabs: HWND, weak: &WeakMailbox) {
        unsafe {
            if msg.hwnd == tabs && msg.message == WM_LBUTTONDOWN {
                let hit = SendMessageW(tabs, LB_ITEMFROMPOINT, 0, msg.lParam) as usize;
                self.dragged = if hit >> 16 == 0 {
                    match hit & 0xffff {
                        0 => Some(ConsoleTab::All),
                        1 => Some(ConsoleTab::Local),
                        2 => Some(ConsoleTab::Private),
                        3 => Some(ConsoleTab::Guild),
                        _ => None,
                    }
                } else {
                    None
                };
                if self.dragged.is_some() {
                    SetCapture(tabs);
                }
            } else if msg.message == WM_LBUTTONUP
                && let Some(tab) = self.dragged.take()
            {
                ReleaseCapture();
                let mut point: POINT = msg.pt;
                let mut bounds: RECT = std::mem::zeroed();
                if ScreenToClient(self.parent, &mut point) != 0
                    && GetClientRect(self.parent, &mut bounds) != 0
                    && point.y >= 0
                    && point.y < bounds.bottom
                    && let Some(shared) = weak.upgrade()
                {
                    console::drop_tab(&mut lock(&shared), tab, point.x, bounds.right);
                }
            } else if GetCapture() != tabs {
                self.dragged = None;
            }
        }
    }
    pub(super) unsafe fn render(&mut self, panel: &ChatPanel, original: HWND) {
        unsafe {
            let Some(text) = mirror_text(panel) else {
                ShowWindow(self.transcript, SW_HIDE);
                return;
            };
            SendMessageW(
                self.transcript,
                WM_SETFONT,
                SendMessageW(original, WM_GETFONT, 0, 0) as usize,
                1,
            );
            SetPropW(
                self.transcript,
                wide(crate::play::FIXED_PIXEL_FONT_PROPERTY).as_ptr(),
                std::ptr::without_provenance_mut(1),
            );
            if text != self.held {
                SetWindowTextW(self.transcript, wide(&text).as_ptr());
                self.held = text;
            }
            ShowWindow(self.transcript, SW_SHOWNA);
        }
    }
}
