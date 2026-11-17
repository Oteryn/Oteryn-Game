//! Controls expose only renderer modes implemented by this native build.
use super::*;
use oteryn_client::play::ReferencePreferences;
pub(crate) struct GraphicsControls {
    labels: [HWND; 3],
    checks: [HWND; 3],
    choices: [HWND; 2],
}
impl GraphicsControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = [
                control(
                    hwnd,
                    "STATIC",
                    "Engine (restart client)",
                    0,
                    [10, 50, 160, 26],
                    0,
                ),
                control(hwnd, "STATIC", "Antialiasing", 0, [10, 80, 160, 26], 0),
                control(
                    hwnd,
                    "STATIC",
                    "VSync uses the driver's supported presentation modes. Integral scaling keeps whole sprite pixels; small windows may crop the map.",
                    0,
                    [10, 230, 300, 60],
                    0,
                ),
            ];
            let choices = [50, 80].map(|y| {
                control(
                    hwnd,
                    "COMBOBOX",
                    "",
                    0,
                    [175, y, 260, 150],
                    CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
                )
            });
            for engine in oteryn_renderer::GraphicsEngine::ALL {
                SendMessageW(
                    choices[0],
                    CB_ADDSTRING,
                    0,
                    wide(engine.label()).as_ptr() as isize,
                );
            }
            for mode in oteryn_renderer::Antialiasing::ALL {
                SendMessageW(
                    choices[1],
                    CB_ADDSTRING,
                    0,
                    wide(mode.label()).as_ptr() as isize,
                );
            }
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
            if labels
                .iter()
                .chain(checks.iter())
                .chain(choices.iter())
                .any(|h| h.is_null())
            {
                return None;
            }
            Some(Self {
                labels,
                checks,
                choices,
            })
        }
    }
    pub(crate) unsafe fn show(&self, show: bool) {
        unsafe {
            for h in self
                .labels
                .iter()
                .chain(self.checks.iter())
                .chain(self.choices.iter())
            {
                ShowWindow(*h, if show { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn read(&self, p: &mut ReferencePreferences) {
        unsafe {
            if let Some(engine) = oteryn_renderer::GraphicsEngine::ALL.get(SendMessageW(
                self.choices[0],
                CB_GETCURSEL,
                0,
                0,
            ) as usize)
            {
                p.graphics.engine = *engine;
            }
            if let Some(mode) = oteryn_renderer::Antialiasing::ALL.get(SendMessageW(
                self.choices[1],
                CB_GETCURSEL,
                0,
                0,
            ) as usize)
            {
                p.graphics.antialiasing = *mode;
            }
            let values = self.checks.map(|h| SendMessageW(h, BM_GETCHECK, 0, 0) == 1);
            p.graphics.fullscreen = values[0];
            p.graphics.vsync = values[1];
            p.graphics.integer_scaling = values[2];
        }
    }
    pub(crate) unsafe fn refresh(&self, p: &ReferencePreferences) {
        unsafe {
            for (handle, index) in self.choices.iter().zip([
                oteryn_renderer::GraphicsEngine::ALL
                    .iter()
                    .position(|engine| *engine == p.graphics.engine)
                    .unwrap_or(0),
                oteryn_renderer::Antialiasing::ALL
                    .iter()
                    .position(|mode| *mode == p.graphics.antialiasing)
                    .unwrap_or(0),
            ]) {
                SendMessageW(*handle, CB_SETCURSEL, index, 0);
            }
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
