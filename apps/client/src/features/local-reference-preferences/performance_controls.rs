//! Graphics-page controls; validation uses the same persisted model as the frame scheduler.
use super::*;
use oteryn_client::play::{PerformanceSettings, ReferencePreferences};
pub(crate) struct PerformanceControls {
    handles: [HWND; 6],
}
impl PerformanceControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let handles = [
                control(
                    hwnd,
                    "STATIC",
                    "Focused FPS (0 unlimited, 1–360)",
                    0,
                    [10, 110, 340, 26],
                    0,
                ),
                control(
                    hwnd,
                    "EDIT",
                    "60",
                    260,
                    [360, 110, 100, 26],
                    WS_BORDER | ES_NUMBER as u32,
                ),
                control(
                    hwnd,
                    "STATIC",
                    "Background / covered FPS (1–120)",
                    0,
                    [10, 190, 340, 26],
                    0,
                ),
                control(
                    hwnd,
                    "EDIT",
                    "15",
                    261,
                    [360, 190, 100, 26],
                    WS_BORDER | ES_NUMBER as u32,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    "Show measured FPS",
                    262,
                    [10, 240, 240, 26],
                    BS_AUTOCHECKBOX as u32,
                ),
                control(
                    hwnd,
                    "BUTTON",
                    "Show command latency",
                    263,
                    [270, 240, 300, 26],
                    BS_AUTOCHECKBOX as u32,
                ),
            ];
            (!handles.iter().any(|handle| handle.is_null())).then_some(Self { handles })
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in self.handles {
                ShowWindow(handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) -> Result<(), String> {
        unsafe {
            let candidate = PerformanceSettings {
                foreground_fps: text(self.handles[1])
                    .parse()
                    .map_err(|_| "Enter focused FPS from 0 to 360")?,
                background_fps: text(self.handles[3])
                    .parse()
                    .map_err(|_| "Enter background FPS from 1 to 120")?,
                show_fps: SendMessageW(self.handles[4], BM_GETCHECK, 0, 0) == 1,
                show_latency: SendMessageW(self.handles[5], BM_GETCHECK, 0, 0) == 1,
            };
            if !candidate.valid() {
                return Err("Focused FPS must be 0–360; background FPS must be 1–120".into());
            }
            preferences.performance = candidate;
            Ok(())
        }
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            let settings = preferences.performance;
            SetWindowTextW(
                self.handles[1],
                wide(&settings.foreground_fps.to_string()).as_ptr(),
            );
            SetWindowTextW(
                self.handles[3],
                wide(&settings.background_fps.to_string()).as_ptr(),
            );
            SendMessageW(
                self.handles[4],
                BM_SETCHECK,
                usize::from(settings.show_fps),
                0,
            );
            SendMessageW(
                self.handles[5],
                BM_SETCHECK,
                usize::from(settings.show_latency),
                0,
            );
        }
    }
}
