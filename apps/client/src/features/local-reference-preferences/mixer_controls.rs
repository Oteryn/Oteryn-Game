//! Independent music, item, event and UI mixer controls.
use super::*;
use oteryn_client::play::ReferencePreferences;
pub(crate) struct MixerControls {
    labels: [HWND; 4],
    edits: [HWND; 4],
}
impl MixerControls {
    pub(crate) unsafe fn position_apply(&self, handle: HWND, page: SettingsPage) {
        let active = matches!(
            page,
            SettingsPage::Mixer | SettingsPage::Battle | SettingsPage::Effects
        );
        let [x, y] = if active { [550, 280] } else { [380, 145] };
        unsafe {
            MoveWindow(handle, x, y, 110, 26, 1);
        }
    }

    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = [
                "Music volume % (0–100)",
                "Item volume % (0–100)",
                "Event volume % (0–100)",
                "UI volume % (0–100)",
            ]
            .map(|label| control(hwnd, "STATIC", label, 0, [0, 0, 340, 26], 0));
            let edits = [0; 4].map(|_| {
                control(
                    hwnd,
                    "EDIT",
                    "100",
                    0,
                    [0, 0, 100, 26],
                    WS_BORDER | ES_NUMBER as u32,
                )
            });
            for i in 0..4 {
                MoveWindow(labels[i], 10, 100 + i as i32 * 45, 340, 26, 1);
                MoveWindow(edits[i], 380, 100 + i as i32 * 45, 100, 26, 1);
            }
            (!labels.iter().chain(&edits).any(|handle| handle.is_null()))
                .then_some(Self { labels, edits })
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in self.labels.iter().chain(&self.edits) {
                ShowWindow(*handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) -> Result<(), String> {
        unsafe {
            for (volume, handle) in preferences
                .audio
                .channel_volumes
                .iter_mut()
                .zip(&self.edits)
            {
                *volume = text(*handle)
                    .parse()
                    .map_err(|_| "Enter mixer volumes from 0 to 100")?;
            }
        }
        Ok(())
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            for (handle, volume) in self.edits.iter().zip(preferences.audio.channel_volumes) {
                SetWindowTextW(*handle, wide(&volume.to_string()).as_ptr());
            }
        }
    }
}
