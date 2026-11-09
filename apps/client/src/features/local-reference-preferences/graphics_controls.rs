//! Controls expose only renderer modes implemented by this native build.
use super::*;
use oteryn_client::play::ReferencePreferences;
pub(crate) struct GraphicsControls {
    labels: [HWND; 3],
    checks: [HWND; 3],
}
impl GraphicsControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = [
                control(
                    hwnd,
                    "STATIC",
                    &format!(
                        "Graphics engine: {}",
                        oteryn_renderer::GRAPHICS_ENGINE.label()
                    ),
                    0,
                    [10, 50, 710, 26],
                    0,
                ),
                control(
                    hwnd,
                    "STATIC",
                    &format!(
                        "Antialiasing: {} (only supported mode)",
                        oteryn_renderer::ANTIALIASING.label()
                    ),
                    0,
                    [10, 80, 710, 26],
                    0,
                ),
                control(
                    hwnd,
                    "STATIC",
                    "VSync uses the driver's supported presentation modes. Integral scaling keeps whole sprite pixels; small windows may crop the map.",
                    0,
                    [10, 230, 300, 60],
                    0,
                ),
            ];
            let checks = [
                ("Borderless fullscreen", 110),
                ("VSync", 150),
                ("Integral world scaling", 190),
            ]
            .map(|(label, y)| {
                control(
                    hwnd,
                    "BUTTON",
                    label,
                    0,
                    [10, y, 300, 28],
                    BS_AUTOCHECKBOX as u32,
                )
            });
            if labels.iter().chain(checks.iter()).any(|h| h.is_null()) {
                return None;
            }
            Some(Self { labels, checks })
        }
    }
    pub(crate) unsafe fn show(&self, show: bool) {
        unsafe {
            for h in self.labels.iter().chain(self.checks.iter()) {
                ShowWindow(*h, if show { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn read(&self, p: &mut ReferencePreferences) {
        unsafe {
            let values = self.checks.map(|h| SendMessageW(h, BM_GETCHECK, 0, 0) == 1);
            p.graphics.fullscreen = values[0];
            p.graphics.vsync = values[1];
            p.graphics.integer_scaling = values[2];
        }
    }
    pub(crate) unsafe fn refresh(&self, p: &ReferencePreferences) {
        unsafe {
            for (h, value) in self.checks.iter().zip([
                p.graphics.fullscreen,
                p.graphics.vsync,
                p.graphics.integer_scaling,
            ]) {
                SendMessageW(*h, BM_SETCHECK, usize::from(value), 0);
            }
        }
    }
}
