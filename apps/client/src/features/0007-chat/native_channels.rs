//! The channel list dialog (Ctrl+O). The state and what a confirmation opens live in
//! `oteryn_client_ui::chat`; this window only shows the rows and the name box.
#![allow(unsafe_code)]
use super::*;
const LIST: usize = 130;
const NAME: usize = 131;
const OPEN: usize = 132;
const CANCEL: usize = 133;
const NAME_LABEL: usize = 134;
pub(super) fn accepts(id: usize, note: usize) -> bool {
    match id {
        LIST => note == LBN_DBLCLK as usize,
        OPEN | CANCEL => note == BN_CLICKED as usize,
        _ => false,
    }
}
#[derive(Default)]
pub(super) struct Controls {
    window: HWND,
    created: bool,
    rows: Vec<String>,
}
impl Controls {
    pub(super) unsafe fn action(
        &mut self,
        id: usize,
        mailbox: &mut crate::play::mailbox::Mailbox,
        commands: &UnboundedSender<PlayCommand>,
    ) -> bool {
        unsafe {
            match id {
                LIST | OPEN => {
                    self.read(mailbox);
                    console::dialog_confirm(mailbox, commands);
                }
                CANCEL => mailbox.chat.console.dialog.hide(),
                _ => return false,
            }
            self.sync(mailbox);
            true
        }
    }
    /// Keep the window in step with the dialog state: open it, list the rows, read the choice.
    pub(super) unsafe fn tick(
        &mut self,
        mailbox: &mut crate::play::mailbox::Mailbox,
        parent: HWND,
    ) {
        unsafe {
            if self.created && IsWindow(self.window) == 0 {
                // The player closed the window: the dialog is done.
                self.created = false;
                mailbox.chat.console.dialog.hide();
            }
            if mailbox.chat.console.dialog.open && !self.created {
                self.create(parent);
            }
            if self.created {
                self.read(mailbox);
            }
            self.sync(mailbox);
        }
    }
    unsafe fn create(&mut self, parent: HWND) {
        unsafe {
            self.window = CreateWindowExW(
                0,
                wide("STATIC").as_ptr(),
                wide(tr!("Channels")).as_ptr(),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                360,
                400,
                parent,
                null_mut(),
                null_mut(),
                null(),
            );
            if self.window.is_null() {
                return;
            }
            SetWindowLongPtrW(
                self.window,
                GWLP_USERDATA,
                GetWindowLongPtrW(parent, GWLP_USERDATA),
            );
            SetWindowLongPtrW(self.window, GWLP_WNDPROC, proc as *const () as isize);
            control(
                self.window,
                "LISTBOX",
                "",
                LIST,
                [10, 10, 330, 230],
                LBS_NOTIFY as u32 | WS_BORDER | WS_VSCROLL | WS_TABSTOP,
            );
            control(
                self.window,
                "STATIC",
                tr!("Open a private conversation with:"),
                NAME_LABEL,
                [10, 250, 330, 20],
                0,
            );
            control(
                self.window,
                "EDIT",
                "",
                NAME,
                [10, 272, 330, 26],
                WS_BORDER | WS_TABSTOP,
            );
            control(
                self.window,
                "BUTTON",
                tr!("Open"),
                OPEN,
                [10, 310, 150, 28],
                WS_TABSTOP,
            );
            control(
                self.window,
                "BUTTON",
                tr!("Cancel"),
                CANCEL,
                [190, 310, 150, 28],
                WS_TABSTOP,
            );
            self.created = true;
            self.rows.clear();
            ShowWindow(self.window, SW_SHOWNORMAL);
            SetForegroundWindow(self.window);
        }
    }
    /// The selected row and the typed name become the dialog state.
    unsafe fn read(&self, mailbox: &mut crate::play::mailbox::Mailbox) {
        unsafe {
            let rows = self.rows.len();
            let selected = SendMessageW(GetDlgItem(self.window, LIST as i32), LB_GETCURSEL, 0, 0);
            if let Ok(index) = usize::try_from(selected) {
                mailbox.chat.console.dialog.select(index, rows);
            }
            let name = read_text(GetDlgItem(self.window, NAME as i32));
            mailbox.chat.console.dialog.set_name(&name);
        }
    }
    unsafe fn sync(&mut self, mailbox: &crate::play::mailbox::Mailbox) {
        unsafe {
            if !self.created {
                return;
            }
            if !mailbox.chat.console.dialog.open {
                if IsWindow(self.window) != 0 {
                    DestroyWindow(self.window);
                }
                self.created = false;
                return;
            }
            let entries = oteryn_client_ui::chat::dialog_entries(
                mailbox.chat_private.list.as_ref(),
                mailbox.chat_public.rooms,
            );
            let rows = oteryn_client_ui::chat::dialog_rows(&entries, &mailbox.chat.console);
            let list = GetDlgItem(self.window, LIST as i32);
            if rows != self.rows {
                SendMessageW(list, LB_RESETCONTENT, 0, 0);
                for row in &rows {
                    SendMessageW(list, LB_ADDSTRING, 0, wide(row).as_ptr() as isize);
                }
                SendMessageW(
                    list,
                    LB_SETCURSEL,
                    mailbox.chat.console.dialog.selected,
                    0,
                );
                self.rows = rows;
            }
        }
    }
}
impl Drop for Controls {
    fn drop(&mut self) {
        unsafe {
            if self.created && IsWindow(self.window) != 0 {
                DestroyWindow(self.window);
            }
        }
    }
}
