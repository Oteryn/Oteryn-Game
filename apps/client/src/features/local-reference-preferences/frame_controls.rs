//! Graphics / Effects controls use the existing local opacity preferences.
use super::*;
use oteryn_client::play::ReferencePreferences;
use windows_sys::Win32::UI::Controls::*;

/// Trackbar position query (`WM_USER`); windows-sys does not export `TBM_GETPOS`.
const TBM_GETPOS: u32 = 0x0400;

pub(crate) struct FrameControls {
    labels: [HWND; 7],
    edits: [HWND; 4],
    sliders: [HWND; 3],
    tooltip: HWND,
    // Drop destroys the borrowing tooltip before these strings are released.
    _tooltip_text: [Vec<u16>; 3],
}
impl FrameControls {
    pub(crate) unsafe fn new(hwnd: HWND) -> Option<Self> {
        unsafe {
            let common = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_BAR_CLASSES | ICC_WIN95_CLASSES,
            };
            if InitCommonControlsEx(&common) == 0 {
                return None;
            }
            let specs = ReferencePreferences::EFFECT_OPACITY_SLIDERS;
            let labels = [
                "Foreground FPS (1–500)",
                "Background FPS (1–500)",
                "Minimum ambient light % (0–100)",
                specs[0].0,
                specs[1].0,
                specs[2].0,
                "Boss-area effect opacity %",
            ]
            .map(|label| control(hwnd, "STATIC", label, 0, [0, 0, 340, 26], 0));
            let edits = [60, 15, 0, 100].map(|fps| {
                control(
                    hwnd,
                    "EDIT",
                    &fps.to_string(),
                    0,
                    [0, 0, 100, 26],
                    WS_BORDER | ES_NUMBER as u32,
                )
            });
            let sliders = std::array::from_fn(|i| {
                let slider = control(
                    hwnd,
                    "msctls_trackbar32",
                    "",
                    0,
                    [370, 205 + i as i32 * 35, 330, 30],
                    TBS_NOTICKS | TBS_TOOLTIPS,
                );
                SendMessageW(slider, TBM_SETRANGEMIN, 0, 0);
                SendMessageW(slider, TBM_SETRANGEMAX, 1, 100);
                SendMessageW(slider, TBM_SETLINESIZE, 0, 1);
                SendMessageW(slider, TBM_SETPAGESIZE, 0, 1);
                slider
            });
            for (i, label) in labels.iter().enumerate() {
                MoveWindow(*label, 10, 100 + i as i32 * 35, 340, 26, 1);
            }
            for (handle, row) in edits.iter().zip([0, 1, 2, 6]) {
                MoveWindow(*handle, 380, 100 + row * 35, 100, 26, 1);
            }
            let tooltips = CreateWindowExW(
                WS_EX_TOPMOST,
                TOOLTIPS_CLASSW,
                null(),
                WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                hwnd,
                null_mut(),
                null_mut(),
                null(),
            );
            if tooltips.is_null()
                || labels
                    .iter()
                    .chain(&edits)
                    .chain(&sliders)
                    .any(|handle| handle.is_null())
            {
                return None;
            }
            let mut tooltip_text = specs.map(|(_, description)| wide(description));
            for (slider, description) in sliders.iter().zip(&mut tooltip_text) {
                let mut info: TTTOOLINFOW = std::mem::zeroed();
                info.cbSize = std::mem::size_of::<TTTOOLINFOW>() as u32;
                info.hwnd = hwnd;
                info.uId = *slider as usize;
                info.uFlags = TTF_IDISHWND | TTF_SUBCLASS;
                info.lpszText = description.as_mut_ptr();
                if SendMessageW(tooltips, TTM_ADDTOOLW, 0, &info as *const _ as isize) == 0 {
                    DestroyWindow(tooltips);
                    return None;
                }
            }
            SendMessageW(tooltips, TTM_SETMAXTIPWIDTH, 0, 360);
            Some(Self {
                labels,
                edits,
                sliders,
                tooltip: tooltips,
                _tooltip_text: tooltip_text,
            })
        }
    }
    pub(crate) unsafe fn show(&self, visible: bool) {
        unsafe {
            for handle in self.labels.iter().chain(&self.edits).chain(&self.sliders) {
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
            for (index, slider) in self.sliders.iter().enumerate() {
                preferences
                    .set_effect_opacity_slider(
                        index,
                        SendMessageW(*slider, TBM_GETPOS, 0, 0) as i32,
                    )
                    .map_err(|error| error.to_string())?;
            }
            preferences.boss_area_opacity = text(self.edits[3])
                .parse()
                .map_err(|_| "Enter boss-area opacity from 0 to 100")?;
            preferences.frames.foreground = preferences.foreground_fps;
            preferences.frames.background = preferences.background_fps;
            Ok(())
        }
    }
    pub(crate) unsafe fn refresh(&self, preferences: &ReferencePreferences) {
        unsafe {
            for (slider, value) in self.sliders.iter().zip(preferences.effect_opacity) {
                SendMessageW(*slider, TBM_SETPOS, 1, isize::from(value));
            }
            for (handle, value) in self.edits.iter().zip([
                preferences.foreground_fps,
                preferences.background_fps,
                u16::from(preferences.ambient_brightness),
                u16::from(preferences.boss_area_opacity),
            ]) {
                SetWindowTextW(*handle, wide(&value.to_string()).as_ptr());
            }
        }
    }
}
impl Drop for FrameControls {
    fn drop(&mut self) {
        unsafe {
            if IsWindow(self.tooltip) != 0 {
                DestroyWindow(self.tooltip);
            }
        }
    }
}
