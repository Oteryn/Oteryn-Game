//! Controls for the existing confirmed-health/mana arc renderer.
use super::*;
use oteryn_client::play::ReferencePreferences;

pub(crate) struct ArcControls {
    label: HWND,
    distance: HWND,
    size: HWND,
    enabled: HWND,
    opacity: HWND,
}
impl ArcControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let opacity = control(
                hwnd,
                "EDIT",
                "100",
                248,
                [250, 145, 100, 26],
                WS_BORDER | ES_NUMBER as u32,
            );
            let label = control(
                hwnd,
                "STATIC",
                "Arc distance (0–80) / size (1–8) / opacity % (0–100)",
                0,
                [10, 110, 700, 26],
                0,
            );
            let distance = control(
                hwnd,
                "EDIT",
                "8",
                245,
                [10, 145, 100, 26],
                WS_BORDER | ES_NUMBER as u32,
            );
            let size = control(
                hwnd,
                "EDIT",
                "3",
                246,
                [130, 145, 100, 26],
                WS_BORDER | ES_NUMBER as u32,
            );
            let enabled = control(
                hwnd,
                "BUTTON",
                "Show HP / mana arcs",
                247,
                [10, 185, 300, 26],
                BS_AUTOCHECKBOX as u32,
            );
            (![label, distance, size, enabled, opacity]
                .iter()
                .any(|handle| handle.is_null()))
            .then_some(Self {
                label,
                distance,
                size,
                enabled,
                opacity,
            })
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in [
                self.label,
                self.distance,
                self.size,
                self.enabled,
                self.opacity,
            ] {
                ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) -> Result<(), String> {
        unsafe {
            preferences.arc_opacity = text(self.opacity)
                .parse()
                .map_err(|_| "Invalid arc opacity".to_owned())?;
            preferences.arc_distance = text(self.distance)
                .parse()
                .map_err(|_| "Invalid arc distance".to_owned())?;
            preferences.arc_size = text(self.size)
                .parse()
                .map_err(|_| "Invalid arc size".to_owned())?;
            preferences.health_circle = SendMessageW(self.enabled, BM_GETCHECK, 0, 0) == 1;
            Ok(())
        }
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            SetWindowTextW(
                self.opacity,
                wide(&preferences.arc_opacity.to_string()).as_ptr(),
            );
            SetWindowTextW(
                self.distance,
                wide(&preferences.arc_distance.to_string()).as_ptr(),
            );
            SetWindowTextW(self.size, wide(&preferences.arc_size.to_string()).as_ptr());
            SendMessageW(
                self.enabled,
                BM_SETCHECK,
                usize::from(preferences.health_circle),
                0,
            );
        }
    }
}
