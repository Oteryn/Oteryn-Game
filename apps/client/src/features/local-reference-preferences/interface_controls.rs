//! Native controls for the supported interface presentation preferences.
use super::*;
use oteryn_client::play::ReferencePreferences;

pub(crate) struct InterfaceControls {
    label: HWND,
    scale: HWND,
    cursor: HWND,
}

impl InterfaceControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
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
            })
        }
    }

    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in [self.label, self.scale, self.cursor] {
                ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }

    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) -> Result<(), String> {
        unsafe {
            preferences.interface_scale = text(self.scale)
                .parse()
                .map_err(|_| "Enter a HUD scale from 80 to 200%".to_owned())?;
            preferences.cursor_crosshair = SendMessageW(self.cursor, BM_GETCHECK, 0, 0) == 1;
            Ok(())
        }
    }

    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
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
