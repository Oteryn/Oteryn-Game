//! Profile controls share the existing Character window and server-confirmed target.
#![allow(unsafe_code)]
use super::controls::wide;
use oteryn_client_ui::cyclopedia_character::{
    CyclopediaCharacterPanel, ProfileRow, profile_heading, profile_rows,
};
use windows_sys::Win32::{Foundation::HWND, UI::WindowsAndMessaging::*};
pub(super) unsafe fn paint(
    p: &CyclopediaCharacterPanel,
    handles: [HWND; 4],
    cards: &[HWND],
    bars: [HWND; 2],
    held: &mut Vec<ProfileRow>,
) {
    use oteryn_client_runtime::cyclopedia_character::CharacterTab;
    if p.tab != CharacterTab::Inspection {
        held.clear();
        return;
    }
    let [list, heading, detail, output] = handles;
    unsafe {
        for handle in cards.iter().copied().chain(bars).chain([output]) {
            ShowWindow(handle, SW_HIDE);
        }
        for handle in [list, heading, detail] {
            ShowWindow(handle, SW_SHOW);
        }
        SetWindowTextW(heading, wide(&profile_heading(p)).as_ptr());
        let rows = profile_rows(p);
        // The main window clears the shared list on a tab change. Repaint even an empty profile.
        let selected = usize::try_from(SendMessageW(list, LB_GETCURSEL, 0, 0)).ok();
        if *held != rows || SendMessageW(list, LB_GETCOUNT, 0, 0) != rows.len() as isize {
            SendMessageW(list, LB_RESETCONTENT, 0, 0);
            for row in &rows {
                SendMessageW(list, LB_ADDSTRING, 0, wide(&row.label).as_ptr() as isize);
            }
            if let Some(i) = selected.filter(|i| *i < rows.len()) {
                SendMessageW(list, LB_SETCURSEL, i, 0);
            }
            *held = rows;
        }
        let text = selected.and_then(|i| held.get(i)).map_or(
            if p.profile.is_none() {
                "No confirmed profile. Another character must grant profile access."
            } else if p.badges_open {
                "Select a badge to inspect its requirement"
            } else {
                "Select an equipped item; a confirmed empty list means no equipment"
            },
            |row| row.detail.as_str(),
        );
        SetWindowTextW(detail, wide(text).as_ptr());
    }
}

/// Existing social badge controls stay in the shared Character window.
pub(super) fn badge_controls(hwnd: HWND) -> HWND {
    use super::controls::control;
    for (id, label, x) in [
        (145, "All badges", 10),
        (146, "Earned badges", 185),
        (147, "Locked badges", 360),
        (148, "Hidden locally", 535),
    ] {
        control(hwnd, "BUTTON", label, id, [x, 615, 165, 28], WS_TABSTOP);
    }
    control(hwnd, "STATIC", "Badge ID", 151, [10, 655, 90, 25], 0);
    let badge_id = control(
        hwnd,
        "EDIT",
        "",
        149,
        [105, 655, 90, 25],
        WS_BORDER | WS_TABSTOP,
    );
    control(
        hwnd,
        "BUTTON",
        "Hide / show locally",
        150,
        [210, 655, 200, 25],
        WS_TABSTOP,
    );

    badge_id
}
pub(super) fn toggle_badge(p: &mut CyclopediaCharacterPanel, edit: HWND) {
    use super::controls::read_text;
    if let Ok(id) = read_text(edit).parse::<u8>()
        && p.badges.as_ref().is_some_and(|ids| ids.contains(&id))
        && !p.hidden_badges.remove(&id)
    {
        p.hidden_badges.insert(id);
    }
}
