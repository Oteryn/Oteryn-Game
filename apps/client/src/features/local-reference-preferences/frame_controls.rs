//! Native foreground/background frame limit controls.
use super::*;
use oteryn_client::play::ReferencePreferences;

pub(crate) struct FrameControls {
    labels: [HWND; 6],
    edits: [HWND; 6],
}
impl FrameControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let labels = [
                "Foreground FPS (1–500)",
                "Background FPS (1–500)",
                "Minimum ambient light % (0–100)",
                "Own effect opacity %",
                "Other players opacity %",
                "Creature effect opacity %",
            ]
            .map(|label| control(hwnd, "STATIC", label, 0, [0, 0, 340, 26], 0));
            let edits = [60, 15, 0, 100, 100, 100].map(|fps| {
                control(
                    hwnd,
                    "EDIT",
                    &fps.to_string(),
                    0,
                    [0, 0, 100, 26],
                    WS_BORDER | ES_NUMBER as u32,
                )
            });
            for i in 0..3 {
                MoveWindow(labels[i], 10, 185 + i as i32 * 40, 340, 26, 1);
                MoveWindow(edits[i], 380, 185 + i as i32 * 40, 100, 26, 1);
            }
            for i in 3..6 {
                MoveWindow(labels[i], 10 + (i - 3) as i32 * 230, 50, 220, 26, 1);
                MoveWindow(edits[i], 10 + (i - 3) as i32 * 230, 80, 100, 26, 1);
            }
            (!labels.iter().chain(&edits).any(|handle| handle.is_null()))
                .then_some(Self { labels, edits })
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in self.labels.iter().chain(&self.edits) {
                ShowWindow(*handle, if visible { SW_SHOW } else { SW_HIDE });
            }
        }
    }
    pub(crate) unsafe fn read(&self, preferences: &mut ReferencePreferences) -> Result<(), String> {
        unsafe {
            preferences.foreground_fps = text(self.edits[0])
                .parse()
                .map_err(|_| "Enter foreground FPS from 1 to 500")?;
            preferences.background_fps = text(self.edits[1])
                .parse()
                .map_err(|_| "Enter background FPS from 1 to 500")?;
            preferences.ambient_brightness = text(self.edits[2])
                .parse()
                .map_err(|_| "Enter ambient brightness from 0 to 100")?;
            for (value, handle) in preferences.effect_opacity.iter_mut().zip(&self.edits[3..]) {
                *value = text(*handle)
                    .parse()
                    .map_err(|_| "Enter effect opacity from 0 to 100")?;
            }
            Ok(())
        }
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            for (handle, value) in self.edits.iter().zip([
                preferences.foreground_fps,
                preferences.background_fps,
                u16::from(preferences.ambient_brightness),
                u16::from(preferences.effect_opacity[0]),
                u16::from(preferences.effect_opacity[1]),
                u16::from(preferences.effect_opacity[2]),
            ]) {
                SetWindowTextW(*handle, wide(&value.to_string()).as_ptr());
            }
        }
    }
}
