//! Screenshot destination, scope and manual request on the settings thread.
use super::*;
use oteryn_client::play::ReferencePreferences;

pub(crate) struct ScreenshotControls([HWND; 10]);
impl ScreenshotControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let handles = [
                control(
                    hwnd,
                    "STATIC",
                    tr!("Screenshot folder (BMP images)"),
                    0,
                    [10, 60, 710, 26],
                    0,
                ),
                control(
                    hwnd,
                    "EDIT",
                    "",
                    252,
                    [10, 95, 710, 26],
                    WS_BORDER | ES_AUTOHSCROLL as u32,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    tr!("Game only (off: entire game window)"),
                    253,
                    [10, 135, 710, 26],
                    BS_AUTOCHECKBOX as u32,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    tr!("Save screenshot settings"),
                    250,
                    [10, 180, 300, 30],
                    0,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    tr!("Capture now"),
                    251,
                    [330, 180, 300, 30],
                    0,
                ),
                control(
                    hwnd,
                    "STATIC",
                    tr!("Pre-event frames (0–10 seconds, max 64 MiB)"),
                    0,
                    [10, 225, 530, 26],
                    0,
                ),
                control(
                    hwnd,
                    "EDIT",
                    "0",
                    254,
                    [560, 225, 70, 26],
                    WS_BORDER | ES_NUMBER as u32,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    tr!("Capture on death"),
                    255,
                    [10, 265, 700, 26],
                    BS_AUTOCHECKBOX as u32,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    tr!("Capture on level advance"),
                    256,
                    [10, 300, 700, 26],
                    BS_AUTOCHECKBOX as u32,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    tr!("Capture on trained skill advance"),
                    257,
                    [10, 335, 700, 26],
                    BS_AUTOCHECKBOX as u32,
                ),
            ];
            SendMessageW(handles[1], EM_SETLIMITTEXT, 240, 0);
            (!handles.iter().any(|h| h.is_null())).then_some(Self(handles))
        }
    }
    pub(crate) unsafe fn show(&self, show: bool) {
        for handle in self.0 {
            unsafe {
                ShowWindow(handle, if show { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            SetWindowTextW(
                self.0[6],
                wide(&preferences.screenshots.backlog_frames.to_string()).as_ptr(),
            );
            for (handle, enabled) in self.0[7..].iter().zip([
                preferences.screenshots.on_death,
                preferences.screenshots.on_level,
                preferences.screenshots.on_skill,
            ]) {
                SendMessageW(*handle, BM_SETCHECK, usize::from(enabled), 0);
            }
            SetWindowTextW(self.0[1], wide(&preferences.screenshots.folder).as_ptr());
            SendMessageW(
                self.0[2],
                BM_SETCHECK,
                usize::from(preferences.screenshots.game_only),
                0,
            );
        }
    }
    pub(crate) unsafe fn action(&self, action: usize, state: &mut SettingsState) -> bool {
        if !matches!(action, 250 | 251) {
            return false;
        }
        unsafe {
            let mut preferences = state.preferences.clone();
            let Ok(frames) = text(self.0[6]).parse::<u8>() else {
                state.feedback = tr!("Enter 0 to 10 backlog frames").into();
                return true;
            };
            if frames > 10 {
                state.feedback = tr!("Enter 0 to 10 backlog frames").into();
                return true;
            }
            preferences.screenshots.backlog_frames = frames;
            preferences.screenshots.on_death = SendMessageW(self.0[7], BM_GETCHECK, 0, 0) == 1;
            preferences.screenshots.on_level = SendMessageW(self.0[8], BM_GETCHECK, 0, 0) == 1;
            preferences.screenshots.on_skill = SendMessageW(self.0[9], BM_GETCHECK, 0, 0) == 1;
            preferences.screenshots.folder = text(self.0[1]);
            preferences.screenshots.game_only = SendMessageW(self.0[2], BM_GETCHECK, 0, 0) == 1;
            state.apply(preferences);
            if action == 251 {
                state.request_screenshot();
            }
        }
        true
    }
}
