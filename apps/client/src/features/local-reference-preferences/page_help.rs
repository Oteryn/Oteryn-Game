use super::*;
pub(super) fn page_help(page: SettingsPage) -> &'static str {
    match page {
        SettingsPage::CustomHotkeys => {
            "Profile names use letters, digits, spaces, _ or -. Up to 16 saved profiles. Object, text and passive action assignment is not available yet."
        }
        SettingsPage::BarHotkeys => {
            "Configure each supported action bar slot above. This client currently exposes one row of four spell slots."
        }
        SettingsPage::Graphics => {
            "Renderer and window options use the supported native renderer. Additional controls are not available yet."
        }
        SettingsPage::Sound => "Master volume % (0–100). Changes apply to playback immediately.",
        _ => "",
    }
}
