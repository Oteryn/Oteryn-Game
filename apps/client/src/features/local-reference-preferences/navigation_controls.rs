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
            "Screenshots...",
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
            "Fullscreen, VSync and integral world scaling apply immediately. Engine and antialiasing show the supported modes."
        }
        SettingsPage::Sound => {
            tr!("Master volume % (0–100). Changes apply to playback immediately.")
        }
        _ => "",
    }
}

pub(super) unsafe fn refresh_bindings(
    list: HWND,
    bindings: &[oteryn_client::play::PreferenceBinding],
    selected: isize,
    state: &SettingsState,
) {
    unsafe {
        let selection = selected.max(0) as usize;
        SendMessageW(list, CB_RESETCONTENT, 0, 0);
        for binding in bindings {
            let label = format!(
                "{}: {}",
                binding.id,
                state.preferences.binding_chord(binding)
            );
            SendMessageW(list, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
        }
        SendMessageW(list, CB_SETCURSEL, selection, 0);
    }
}
