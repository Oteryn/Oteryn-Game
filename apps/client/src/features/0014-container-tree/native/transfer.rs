//! Direct target-view entry complements selection-based destination controls.
use super::*;
pub(super) unsafe fn create(parent: HWND) -> HWND {
    unsafe {
        let target = CreateWindowExW(
            0,
            wide("EDIT").as_ptr(),
            wide("").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP,
            10,
            410,
            235,
            28,
            parent,
            null_mut(),
            null_mut(),
            null(),
        );
        CreateWindowExW(
            0,
            wide("BUTTON").as_ptr(),
            wide("Move all to view ID").as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP,
            255,
            410,
            235,
            28,
            parent,
            108 as _,
            null_mut(),
            null(),
        );
        target
    }
}
pub(super) unsafe fn submit(
    action: u16,
    target: HWND,
    mailbox: &mut Mailbox,
    commands: &UnboundedSender<PlayCommand>,
) -> bool {
    if action != 108 {
        return false;
    }
    unsafe {
        let mut text = [0u16; 20];
        let length = GetWindowTextW(target, text.as_mut_ptr(), 20).max(0) as usize;
        if let Ok(id) = String::from_utf16_lossy(&text[..length])
            .trim()
            .parse::<u8>()
        {
            super::super::super::input::bulk::begin(mailbox, commands, id);
        }
    }
    true
}
