//! Transcript typography controls, stored with the local client preferences.
use super::*;
use oteryn_client::play::ReferencePreferences;
pub(crate) struct ConsoleControls {
    label: HWND,
    font: HWND,
}
impl ConsoleControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let label = control(
                hwnd,
                "STATIC",
                "Console font size in pixels (10–32)",
                0,
                [10, 110, 700, 26],
                0,
            );
            let font = control(
                hwnd,
                "EDIT",
                "14",
                0,
                [10, 145, 100, 26],
                WS_BORDER | ES_NUMBER as u32,
            );
            (![label, font].iter().any(|handle| handle.is_null())).then_some(Self { label, font })
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in [self.label, self.font] {
                ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) -> Result<(), String> {
        unsafe {
            preferences.console_font_px = text(self.font)
                .parse()
                .map_err(|_| "Invalid console font size".to_owned())?;
            Ok(())
        }
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            SetWindowTextW(
                self.font,
                wide(&preferences.console_font_px.to_string()).as_ptr(),
            );
        }
    }
}
