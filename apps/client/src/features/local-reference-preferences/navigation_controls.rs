use super::*;

pub(super) unsafe fn create(hwnd: HWND) -> Vec<HWND> {
    unsafe {
        [
            "Hotkey profiles...",
            "Help and reports...",
            "Mouse and movement...",
            "Shortcut buttons...",
            "Minimap...",
            "Frames...",
            "General options...",
        ]
        .iter()
        .enumerate()
        .map(|(i, label)| {
            control(
                hwnd,
                "BUTTON",
                label,
                160 + i,
                [
                    10 + (i % 3) as i32 * 240,
                    440 + (i / 3) as i32 * 35,
                    230,
                    28,
                ],
                0,
            )
        })
        .collect()
    }
}

pub(super) fn page_help(page: SettingsPage) -> &'static str {
    match page {
        SettingsPage::CustomHotkeys => {
            "Profile names use letters, digits, spaces, _ or -. Up to 16 saved profiles. Loading a profile selects manual hotkeys. Object, text and passive action assignment is not available yet."
        }
        SettingsPage::BarHotkeys => {
            "Configure each supported action bar slot above. This client currently exposes one row of four spell slots."
        }
        SettingsPage::Graphics => {
            "Fullscreen, VSync and integral world scaling apply immediately. Engine and antialiasing show the supported modes."
        }
        SettingsPage::Sound => "Master volume % (0–100). Changes apply to playback immediately.",
        _ => "",
    }
}
