//! Battle-source mixer controls use the source tags carried by presentation sound events.
use super::*;
use oteryn_client::play::ReferencePreferences;
pub(crate) struct BattleSoundControls {
    device_label: HWND,
    device: HWND,
    labels: [HWND; 4],
    ui_checks: [HWND; 4],
    edits: [HWND; 4],
}
impl BattleSoundControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let device_label = control(hwnd, "STATIC", "Output device", 0, [10, 100, 220, 26], 0);
            let device = control(
                hwnd,
                "COMBOBOX",
                "",
                0,
                [240, 100, 470, 200],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            for name in std::iter::once("System default".to_owned())
                .chain(oteryn_client::play::audio_device_names())
            {
                SendMessageW(device, CB_ADDSTRING, 0, wide(&name).as_ptr() as isize);
            }
            SendMessageW(device, CB_SETCURSEL, 0, 0);
            let ui_checks = [
                "UI interactions",
                "Party messages",
                "VIP changes",
                "Chat channels",
            ]
            .map(|label| {
                control(
                    hwnd,
                    "BUTTON",
                    label,
                    0,
                    [0, 330, 170, 26],
                    BS_AUTOCHECKBOX as u32,
                )
            });
            for (index, handle) in ui_checks.iter().enumerate() {
                MoveWindow(*handle, 10 + index as i32 * 175, 330, 170, 26, 1);
            }
            let labels = [
                "Own battle volume %",
                "Other players volume %",
                "Creatures volume %",
                "Ambient volume %",
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
                MoveWindow(labels[i], 10, 185 + i as i32 * 35, 340, 26, 1);
                MoveWindow(edits[i], 380, 185 + i as i32 * 35, 100, 26, 1);
            }
            (!labels
                .iter()
                .chain(&edits)
                .chain(&ui_checks)
                .chain(&[device_label, device])
                .any(|handle| handle.is_null()))
            .then_some(Self {
                device_label,
                device,
                labels,
                edits,
                ui_checks,
            })
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in self
                .labels
                .iter()
                .chain(&self.edits)
                .chain(&self.ui_checks)
                .chain(&[self.device_label, self.device])
            {
                ShowWindow(*handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) -> Result<(), String> {
        unsafe {
            preferences.audio_device = if SendMessageW(self.device, CB_GETCURSEL, 0, 0) == 0 {
                String::new()
            } else {
                text(self.device)
            };
            for (index, handle) in self.edits[..3].iter().enumerate() {
                preferences.audio.battle_volumes[index] = text(*handle)
                    .parse()
                    .map_err(|_| "Enter source volumes from 0 to 100")?;
            }
        }
        preferences.audio.ambient_volume = unsafe { text(self.edits[3]) }
            .parse()
            .map_err(|_| "Enter ambient volume from 0 to 100")?;
        preferences.audio.ui_sounds = self
            .ui_checks
            .map(|handle| unsafe { SendMessageW(handle, BM_GETCHECK, 0, 0) == 1 });
        Ok(())
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            let name = if preferences.audio_device.is_empty() {
                "System default"
            } else {
                &preferences.audio_device
            };
            let mut index = SendMessageW(
                self.device,
                CB_FINDSTRINGEXACT,
                usize::MAX,
                wide(name).as_ptr() as isize,
            );
            if index == CB_ERR as isize {
                index = SendMessageW(self.device, CB_ADDSTRING, 0, wide(name).as_ptr() as isize);
            }
            SendMessageW(self.device, CB_SETCURSEL, index as usize, 0);
            for (handle, enabled) in self.ui_checks.iter().zip(preferences.audio.ui_sounds) {
                SendMessageW(*handle, BM_SETCHECK, usize::from(enabled), 0);
            }
            for (handle, value) in self.edits.iter().zip([
                preferences.audio.battle_volumes[0],
                preferences.audio.battle_volumes[1],
                preferences.audio.battle_volumes[2],
                preferences.audio.ambient_volume,
            ]) {
                SetWindowTextW(*handle, wide(&value.to_string()).as_ptr());
            }
        }
    }
}
