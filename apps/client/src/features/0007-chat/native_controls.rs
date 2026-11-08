//! Construction of the panel's native controls on its owning thread.
use super::*;
pub(super) struct PanelControls {
    pub(super) rooms: HWND,
    pub(super) message: HWND,
    pub(super) recipient: HWND,
    pub(super) transcript: HWND,
    pub(super) tabs: HWND,
    pub(super) log_path: HWND,
}
impl PanelControls {
    pub(super) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let rooms = control(
                hwnd,
                "LISTBOX",
                "",
                MODES,
                [10, 10, 230, 180],
                LBS_NOTIFY as u32 | WS_BORDER,
            );
            for mode in ["Say", "Whisper", "Yell", "Private", "Guild"] {
                SendMessageW(rooms, LB_ADDSTRING, 0, wide(mode).as_ptr() as isize);
            }
            SendMessageW(rooms, LB_SETCURSEL, 0, 0);
            let message = control(
                hwnd,
                "EDIT",
                "",
                MESSAGE,
                [250, 10, 430, 28],
                WS_BORDER | WS_TABSTOP,
            );
            let recipient = control(
                hwnd,
                "EDIT",
                "",
                RECIPIENT,
                [250, 48, 270, 28],
                WS_BORDER | WS_TABSTOP,
            );
            control(
                hwnd,
                "STATIC",
                "Private recipient",
                RECIPIENT_LABEL,
                [10, 200, 230, 28],
                0,
            );
            control(hwnd, "BUTTON", "Send", SEND, [540, 48, 100, 28], WS_TABSTOP);
            let transcript = control(
                hwnd,
                "EDIT",
                "",
                106,
                [250, 86, 430, 380],
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_BORDER | WS_VSCROLL,
            );
            let tabs = control(
                hwnd,
                "LISTBOX",
                "",
                TAB,
                [10, 245, 230, 75],
                LBS_NOTIFY as u32 | WS_BORDER,
            );
            for tab in ["All", "Local", "Private", "Guild"] {
                SendMessageW(tabs, LB_ADDSTRING, 0, wide(tab).as_ptr() as isize);
            }
            SendMessageW(tabs, LB_SETCURSEL, 0, 0);
            control(
                hwnd,
                "BUTTON",
                "Toggle messages",
                FILTER,
                [10, 325, 230, 28],
                WS_TABSTOP,
            );
            control(
                hwnd,
                "BUTTON",
                "Chat on/off/temporary",
                CHAT_MODE,
                [10, 360, 230, 28],
                WS_TABSTOP,
            );
            for (id, title, y) in [
                (IGNORE, "Ignore recipient", 400),
                (ALLOW, "Whitelist recipient", 435),
                (BLOCK, "Blacklist recipient", 470),
                (CLEAR_LISTS, "Remove recipient from lists", 505),
            ] {
                control(hwnd, "BUTTON", title, id, [10, y, 230, 28], WS_TABSTOP);
            }
            let log_path = control(
                hwnd,
                "EDIT",
                "chat.log",
                LOG_PATH,
                [250, 480, 300, 28],
                WS_BORDER | WS_TABSTOP,
            );
            control(
                hwnd,
                "BUTTON",
                "Save log",
                SAVE,
                [250, 515, 140, 28],
                WS_TABSTOP,
            );
            control(
                hwnd,
                "BUTTON",
                "Append log",
                APPEND,
                [400, 515, 140, 28],
                WS_TABSTOP,
            );

            Some(Self {
                rooms,
                message,
                recipient,
                transcript,
                tabs,
                log_path,
            })
        }
    }
}
