//! Battle-source mixer controls use the source tags carried by presentation sound events.
use super::*;
use oteryn_client::play::ReferencePreferences;
pub(crate) struct BattleSoundControls {
    labels: [HWND; 3],
    edits: [HWND; 3],
}
impl BattleSoundControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = [
                "Own battle volume %",
                "Other players volume %",
                "Creatures volume %",
            ]
            .map(|label| control(hwnd, "STATIC", label, 0, [0, 0, 340, 26], 0));
            let edits = [0; 3].map(|_| {
                control(
                    hwnd,
                    "EDIT",
                    "100",
                    0,
                    [0, 0, 100, 26],
                    WS_BORDER | ES_NUMBER as u32,
                )
            });
            for i in 0..3 {
                MoveWindow(labels[i], 10, 185 + i as i32 * 35, 340, 26, 1);
                MoveWindow(edits[i], 380, 185 + i as i32 * 35, 100, 26, 1);
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
            for (index, handle) in self.edits.iter().enumerate() {
                preferences.audio.battle_volumes[index] = text(*handle)
                    .parse()
                    .map_err(|_| "Enter source volumes from 0 to 100")?;
            }
        }
        Ok(())
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            for (handle, value) in self.edits.iter().zip(preferences.audio.battle_volumes) {
                SetWindowTextW(*handle, wide(&value.to_string()).as_ptr());
            }
        }
    }
}
