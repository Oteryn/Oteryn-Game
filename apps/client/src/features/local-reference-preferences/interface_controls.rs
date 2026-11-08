//! Native controls for the supported interface presentation preferences.
use super::*;
use oteryn_client::play::ReferencePreferences;

pub(crate) struct InterfaceControls {
    label: HWND,
    scale: HWND,
    cursor: HWND,
    names: [HWND; 3],
}

impl InterfaceControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let names = ["Own name", "Other player names", "Creature names"].map(|label| {
                control(
                    hwnd,
                    "BUTTON",
                    label,
                    0,
                    [0, 0, 220, 26],
                    BS_AUTOCHECKBOX as u32,
                )
            });
            for (index, handle) in names.iter().enumerate() {
                MoveWindow(*handle, 10 + index as i32 * 230, 265, 220, 26, 1);
            }
            if names.iter().any(|handle| handle.is_null()) {
                return None;
            }
            let label = control(
                hwnd,
                "STATIC",
                "HUD scale % (80–200)",
                0,
                [10, 185, 300, 26],
                0,
            );
            let scale = control(
                hwnd,
                "EDIT",
                "100",
                235,
                [380, 185, 100, 26],
                WS_BORDER | ES_NUMBER as u32,
            );
            let cursor = control(
                hwnd,
                "BUTTON",
                "Crosshair cursor",
                236,
                [10, 225, 300, 26],
                BS_AUTOCHECKBOX as u32,
            );
            (![label, scale, cursor].iter().any(|handle| handle.is_null())).then_some(Self {
                label,
                scale,
                cursor,
                names,
            })
        }
    }

    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in [self.label, self.scale, self.cursor]
                .into_iter()
                .chain(self.names)
            {
                ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }

    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) -> Result<(), String> {
        unsafe {
            preferences.hud_names = self
                .names
                .map(|handle| SendMessageW(handle, BM_GETCHECK, 0, 0) == 1);
            preferences.interface_scale = text(self.scale)
                .parse()
                .map_err(|_| "Enter a HUD scale from 80 to 200%".to_owned())?;
            preferences.cursor_crosshair = SendMessageW(self.cursor, BM_GETCHECK, 0, 0) == 1;
            Ok(())
        }
    }

    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            for (handle, enabled) in self.names.iter().zip(preferences.hud_names) {
                SendMessageW(*handle, BM_SETCHECK, usize::from(enabled), 0);
            }
            SetWindowTextW(
                self.scale,
                wide(&preferences.interface_scale.to_string()).as_ptr(),
            );
            SendMessageW(
                self.cursor,
                BM_SETCHECK,
                usize::from(preferences.cursor_crosshair),
                0,
            );
        }
    }
}
