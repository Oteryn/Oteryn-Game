//! Independent owner/other-actor HUD visibility, consuming only confirmed entity data.
use super::*;
use oteryn_client::play::ReferencePreferences;
pub(crate) struct HudControls {
    label: HWND,
    checks: [HWND; 10],
}
impl HudControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let label = control(
                hwnd,
                "STATIC",
                "HUD visibility (resources and marks use confirmed server data)",
                0,
                [10, 110, 700, 26],
                0,
            );
            let checks = [
                tr!("Own name"),
                tr!("Other player names"),
                "Creature names",
                "Own HP bar",
                "Own mana bar",
                "Other HP bars",
                "Own marks",
                "Other marks",
                "NPC icons",
                "NPC names",
            ]
            .map(|text| {
                control(
                    hwnd,
                    "BUTTON",
                    text,
                    0,
                    [0, 0, 220, 26],
                    BS_AUTOCHECKBOX as u32,
                )
            });
            for (index, handle) in checks.iter().enumerate() {
                MoveWindow(
                    *handle,
                    10 + (index % 3) as i32 * 230,
                    185 + (index / 3) as i32 * 35,
                    220,
                    26,
                    1,
                );
            }
            (![label]
                .iter()
                .chain(checks.iter())
                .any(|handle| handle.is_null()))
            .then_some(Self { label, checks })
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in [self.label].into_iter().chain(self.checks) {
                ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) {
        unsafe {
            let values = self
                .checks
                .map(|handle| SendMessageW(handle, BM_GETCHECK, 0, 0) == 1);
            preferences.hud_names.copy_from_slice(&values[..3]);
            preferences.hud_health = values[3];
            preferences.hud_mana = values[4];
            preferences.hud_bars = values[3] || values[4];
            preferences.creature_bars = values[5];
            preferences.hud_marks_own = values[6];
            preferences.hud_marks_other = values[7];
            preferences.hud_npc_icons = values[8];
            preferences.hud_npc_names = Some(values[9]);
        }
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            let values = [
                preferences.hud_names[0],
                preferences.hud_names[1],
                preferences.hud_names[2],
                preferences.hud_bars && preferences.hud_health,
                preferences.hud_bars && preferences.hud_mana,
                preferences.creature_bars,
                preferences.hud_marks_own,
                preferences.hud_marks_other,
                preferences.hud_npc_icons,
                preferences
                    .hud_npc_names
                    .unwrap_or(preferences.hud_names[2]),
            ];
            for (handle, value) in self.checks.iter().zip(values) {
                SendMessageW(*handle, BM_SETCHECK, usize::from(value), 0);
            }
        }
    }
}
