//! Popup lifetime and HWND access stay on the shell thread.
#![allow(
    unsafe_code,
    reason = "Win32 popup is created and destroyed on the UI thread"
)]
use crate::play::{PlayLink, PlayView};
use oteryn_client_ui::world_interaction::menu::Menu;
use std::ptr::null;
use windows_sys::Win32::{Foundation::POINT, UI::WindowsAndMessaging::*};
pub fn open_menu(view: &PlayView, link: &PlayLink, menu: &Menu) {
    // SAFETY: called from the native shell input phase; all handles remain on this thread.
    unsafe {
        let popup = CreatePopupMenu();
        if popup.is_null() {
            return;
        }
        for (index, (label, _)) in menu.entries.iter().enumerate() {
            let wide: Vec<u16> = label.encode_utf16().chain(Some(0)).collect();
            AppendMenuW(popup, MF_STRING, index + 1, wide.as_ptr());
        }
        let mut point = POINT { x: 0, y: 0 };
        let owner = GetForegroundWindow();
        if !owner.is_null() && GetCursorPos(&mut point) != 0 {
            let selected = TrackPopupMenu(
                popup,
                TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                owner,
                null(),
            );
            if let Ok(index) = usize::try_from(selected)
                && let Some((_, action)) = index
                    .checked_sub(1)
                    .and_then(|index| menu.entries.get(index))
            {
                view.world_context_choose(link, menu, *action);
            }
        }
        DestroyMenu(popup);
    }
}
