//! Source-specific attack, healing and support notification filters.
use super::*;
use oteryn_client::play::ReferencePreferences;
pub(crate) struct BattleFilterControls {
    labels: [HWND; 3],
    checks: [[HWND; 3]; 3],
}
impl BattleFilterControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = ["Own sounds", "Other players", "Creatures"]
                .map(|label| control(hwnd, "STATIC", label, 0, [0, 0, 220, 26], 0));
            let checks = [0; 3].map(|_| {
                ["Attack sounds", "Healing sounds", "Support sounds"].map(|label| {
                    control(
                        hwnd,
                        "BUTTON",
                        label,
                        0,
                        [0, 0, 220, 26],
                        BS_AUTOCHECKBOX as u32,
                    )
                })
            });
            for source in 0..3 {
                MoveWindow(labels[source], 10 + source as i32 * 230, 80, 220, 26, 1);
                for (class, check) in checks[source].iter().enumerate() {
                    MoveWindow(
                        *check,
                        10 + source as i32 * 230,
                        120 + class as i32 * 40,
                        220,
                        26,
                        1,
                    );
                }
            }
            (!labels
                .iter()
                .chain(checks.iter().flatten())
                .any(|handle| handle.is_null()))
            .then_some(Self { labels, checks })
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in self.labels.iter().chain(self.checks.iter().flatten()) {
                ShowWindow(*handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) -> Result<(), String> {
        preferences.audio.battle_filters = self.checks.map(|checks| {
            checks.map(|handle| unsafe { SendMessageW(handle, BM_GETCHECK, 0, 0) == 1 })
        });
        Ok(())
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            for (handle, enabled) in self
                .checks
                .iter()
                .flatten()
                .zip(preferences.audio.battle_filters.iter().flatten())
            {
                SendMessageW(*handle, BM_SETCHECK, usize::from(*enabled), 0);
            }
        }
    }
}
