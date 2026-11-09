//! Local display controls; HFONT is owned until the transcript releases it.
#![allow(unsafe_code)]
use super::*;
use windows_sys::Win32::Graphics::Gdi::{CreateFontW, DeleteObject, HFONT};
const FONT: usize = 117;
const TIME: usize = 118;
const SECONDS: usize = 119;
const LEVELS: usize = 120;
const OPTIONS: usize = 121;
pub(super) fn accepts(id: usize, note: usize) -> bool {
    (FONT..=OPTIONS).contains(&id) && note == BN_CLICKED as usize
}
pub(super) struct Controls {
    hwnd: HWND,
    options: HWND,
    font: HFONT,
    size: u16,
    queried: Option<std::time::Instant>,
}
impl Controls {
    pub(super) unsafe fn new(hwnd: HWND) -> Self {
        unsafe {
            control(
                hwnd,
                "BUTTON",
                "Console display...",
                OPTIONS,
                [250, 555, 430, 28],
                WS_TABSTOP,
            );
            Self {
                hwnd,
                options: null_mut(),
                font: null_mut(),
                size: 0,
                queried: None,
            }
        }
    }
    pub(super) unsafe fn action(
        &mut self,
        id: usize,
        mailbox: &mut crate::play::mailbox::Mailbox,
        commands: &UnboundedSender<PlayCommand>,
        transcript: HWND,
    ) -> bool {
        unsafe {
            if id == OPTIONS {
                if IsWindow(self.options) == 0 {
                    self.options = CreateWindowExW(
                        0,
                        wide("STATIC").as_ptr(),
                        wide("Oteryn console display").as_ptr(),
                        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                        CW_USEDEFAULT,
                        CW_USEDEFAULT,
                        410,
                        200,
                        self.hwnd,
                        null_mut(),
                        null_mut(),
                        null(),
                    );
                    if self.options.is_null() {
                        return true;
                    }
                    SetWindowLongPtrW(
                        self.options,
                        GWLP_USERDATA,
                        GetWindowLongPtrW(self.hwnd, GWLP_USERDATA),
                    );
                    SetWindowLongPtrW(self.options, GWLP_WNDPROC, proc as *const () as isize);
                    for (id, text, y) in [
                        (FONT, "Console font size", 10),
                        (TIME, "Receipt time (UTC)", 40),
                        (SECONDS, "Include seconds", 70),
                        (LEVELS, "Show current sender levels", 100),
                    ] {
                        control(
                            self.options,
                            "BUTTON",
                            text,
                            id,
                            [10, y, 370, 28],
                            WS_TABSTOP
                                | if id == FONT {
                                    0
                                } else {
                                    BS_AUTOCHECKBOX as u32
                                },
                        );
                    }
                }
                ShowWindow(self.options, SW_SHOWNORMAL);
                SetForegroundWindow(self.options);
                self.tick(mailbox, commands, transcript);
                return true;
            }
            if !(FONT..=LEVELS).contains(&id) {
                return false;
            }
            let display = &mut mailbox.chat.console.display;
            if id == FONT {
                display.font_px = match display.font_px {
                    12 => 14,
                    14 => 16,
                    16 => 18,
                    18 => 20,
                    20 => 24,
                    _ => 12,
                };
            } else {
                let checked =
                    SendMessageW(GetDlgItem(self.options, id as i32), BM_GETCHECK, 0, 0) == 1;
                match id {
                    TIME => display.time = checked,
                    SECONDS => display.seconds = checked,
                    _ => display.levels = checked,
                }
            }
            self.queried = None;
            self.tick(mailbox, commands, transcript);
            true
        }
    }
    pub(super) unsafe fn tick(
        &mut self,
        mailbox: &mut crate::play::mailbox::Mailbox,
        commands: &UnboundedSender<PlayCommand>,
        transcript: HWND,
    ) {
        unsafe {
            let display = mailbox.chat.console.display;
            for (id, checked) in [
                (TIME, display.time),
                (SECONDS, display.seconds),
                (LEVELS, display.levels),
            ] {
                SendMessageW(
                    GetDlgItem(self.options, id as i32),
                    BM_SETCHECK,
                    usize::from(checked),
                    0,
                );
            }
            if self.size != display.font_px {
                let font = CreateFontW(
                    -(i32::from(display.font_px)),
                    0,
                    0,
                    0,
                    400,
                    0,
                    0,
                    0,
                    1,
                    0,
                    0,
                    0,
                    0,
                    wide("Consolas").as_ptr(),
                );
                if !font.is_null() {
                    SendMessageW(transcript, WM_SETFONT, font as usize, 1);
                    if !self.font.is_null() {
                        DeleteObject(self.font);
                    }
                    self.font = font;
                    self.size = display.font_px;
                }
            }
            if IsWindow(self.options) != 0 {
                SetWindowTextW(
                    GetDlgItem(self.options, FONT as i32),
                    wide(&format!("Console font: {} px (cycle)", display.font_px)).as_ptr(),
                );
            }
            if display.levels
                && self
                    .queried
                    .is_none_or(|last| last.elapsed().as_secs() >= 2)
            {
                super::super::console::refresh(mailbox, commands);
                self.queried = Some(std::time::Instant::now());
            }
        }
    }
}
impl Drop for Controls {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.options) != 0 {
                DestroyWindow(self.options);
            }
            if !self.font.is_null() {
                DeleteObject(self.font);
            }
        }
    }
}
