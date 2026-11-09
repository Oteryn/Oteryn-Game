//! Named modifiers share the Character list and use the existing page controls.
#![allow(unsafe_code)]
use super::controls::{control, wide};
use oteryn_client_ui::cyclopedia_character::{
    AugmentRow, CyclopediaCharacterPanel, augment_heading, augment_rows,
};
use std::ptr::null_mut;
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};
pub(super) fn controls(hwnd: HWND) {
    control(
        hwnd,
        "BUTTON",
        "Spell modifiers",
        180,
        [545, 155, 165, 28],
        WS_TABSTOP,
    );
    for (i, label) in [
        "All sources",
        "Equipment",
        "Proficiency",
        "Wheel",
        "Sort spell / source",
    ]
    .into_iter()
    .enumerate()
    {
        control(
            hwnd,
            "BUTTON",
            label,
            181 + i,
            [10 + i as i32 * 140, 615, 130, 28],
            WS_TABSTOP,
        );
    }
    hide(hwnd, false);
}
pub(super) fn hide(hwnd: HWND, offence: bool) {
    unsafe {
        ShowWindow(
            GetDlgItem(hwnd, 180),
            if offence { SW_SHOW } else { SW_HIDE },
        );
        for id in 181..=185 {
            ShowWindow(GetDlgItem(hwnd, id), SW_HIDE);
        }
        for id in 145..=155 {
            ShowWindow(GetDlgItem(hwnd, id), SW_SHOW);
        }
    }
}
pub(super) unsafe fn paint(
    p: &CyclopediaCharacterPanel,
    hwnd: HWND,
    handles: [HWND; 6],
    cards: &[HWND],
    held: &mut Vec<AugmentRow>,
) {
    let [output, stamina, training, heading, detail, list] = handles;
    unsafe {
        for id in (145..=155).chain(114..=116) {
            ShowWindow(GetDlgItem(hwnd, id), SW_HIDE);
        }
        for id in 180..=185 {
            ShowWindow(GetDlgItem(hwnd, id), SW_SHOW);
        }
        for handle in cards.iter().copied().chain([output, stamina, training]) {
            ShowWindow(handle, SW_HIDE);
        }
        for handle in [heading, detail, list] {
            ShowWindow(handle, SW_SHOW);
        }
        SetWindowPos(
            heading,
            null_mut(),
            10,
            355,
            690,
            22,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        SetWindowPos(
            list,
            null_mut(),
            10,
            380,
            690,
            155,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        SetWindowPos(
            detail,
            null_mut(),
            10,
            545,
            690,
            50,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        SetWindowTextW(heading, wide(&augment_heading(p)).as_ptr());
        let rows = augment_rows(p);
        if *held != rows || SendMessageW(list, LB_GETCOUNT, 0, 0) != rows.len() as isize {
            SendMessageW(list, LB_RESETCONTENT, 0, 0);
            for row in &rows {
                SendMessageW(list, LB_ADDSTRING, 0, wide(&row.label).as_ptr() as isize);
            }
            *held = rows;
        }
        let selected = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0)).ok();
        let text = selected.and_then(|i| held.get(i)).map_or(
            "Select a modifier for its source, units and cast conditions",
            |row| row.detail.as_str(),
        );
        SetWindowTextW(detail, wide(text).as_ptr());
    }
}
