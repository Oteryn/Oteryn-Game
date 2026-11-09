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
            "Profile names use letters, digits, spaces, _ or -. Up to 16 saved profiles. Loading a profile selects manual hotkeys. Use New / edit to assign object, text and read-only ability cooldown actions."
        }
        SettingsPage::BarHotkeys => {
            "Configure all three action bar rows (12 slots each). Empty chords clear slots 5–36. The first four slots retain their legacy defaults until rebound."
        }
        SettingsPage::Graphics => {
            "Renderer and window options use the supported native renderer. Additional controls are not available yet."
        }
        SettingsPage::Sound => "Master volume % (0–100). Changes apply to playback immediately.",
        _ => "",
    }
}
