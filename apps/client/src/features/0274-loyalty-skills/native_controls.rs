//! Construction of the panel's native controls on its owning thread.
use super::*;
pub(super) struct PanelControls {
    pub(super) header: HWND,
    pub(super) level: HWND,
    pub(super) labels: Vec<HWND>,
    pub(super) bars: Vec<HWND>,
    pub(super) family: HWND,
    pub(super) visible: HWND,
    pub(super) numbers: HWND,
    pub(super) progress: HWND,
    pub(super) refresh: HWND,
}
impl PanelControls {
    pub(super) unsafe fn new(hwnd: HWND, weak: &Weak<Mutex<Mailbox>>) -> Option<Self> {
        unsafe {
            let header = control(hwnd, "STATIC", "", 0, [16, 10, 870, 70], 0);
            let level = control(hwnd, "msctls_progress32", "", 0, [16, 82, 870, 16], 0);
            SendMessageW(level, PBM_SETRANGE32, 0, 10_000);
            let labels: Vec<_> = (0..8)
                .map(|i| control(hwnd, "STATIC", "", 0, [16, 110 + i * 48, 870, 24], 0))
                .collect();
            let bars: Vec<_> = (0..8)
                .map(|i| {
                    let bar = control(
                        hwnd,
                        "msctls_progress32",
                        "",
                        0,
                        [16, 136 + i * 48, 870, 14],
                        0,
                    );
                    SendMessageW(bar, PBM_SETRANGE32, 0, 10_000);
                    bar
                })
                .collect();
            let caption = control(
                hwnd,
                "STATIC",
                "Skill / visibility / order / level values",
                0,
                [16, 510, 870, 24],
                0,
            );
            let family = control(
                hwnd,
                "COMBOBOX",
                "",
                200,
                [16, 540, 180, 220],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            for name in FAMILIES {
                SendMessageW(family, CB_ADDSTRING, 0, wide(name).as_ptr() as isize);
            }
            let visible = control(
                hwnd,
                "BUTTON",
                "Show skill",
                201,
                [210, 540, 140, 28],
                BS_AUTOCHECKBOX as u32,
            );
            let up = control(hwnd, "BUTTON", "Move up", 202, [360, 540, 110, 28], 0);
            let down = control(hwnd, "BUTTON", "Move down", 203, [480, 540, 110, 28], 0);
            let numbers = control(
                hwnd,
                "COMBOBOX",
                "",
                204,
                [610, 540, 270, 180],
                CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
            );
            for label in [
                "All levels",
                "Trained levels",
                "Loyalty levels",
                "Current levels",
            ] {
                SendMessageW(numbers, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
            }
            let progress = control(
                hwnd,
                "BUTTON",
                "Show progress bars and counts",
                205,
                [16, 590, 330, 28],
                BS_AUTOCHECKBOX as u32,
            );
            let refresh = control(hwnd, "BUTTON", tr!("Refresh"), 1, [740, 590, 140, 30], 0);
            let help = control(
                hwnd,
                "STATIC",
                "Select a skill to show/hide it or change its order. Layout follows character settings.\r\nCurrent levels remain unavailable when the server cannot qualify them.",
                0,
                [16, 640, 870, 60],
                0,
            );
            if labels
                .iter()
                .chain(bars.iter())
                .chain(
                    [
                        header, level, caption, family, visible, up, down, numbers, progress,
                        refresh, help,
                    ]
                    .iter(),
                )
                .any(|h| h.is_null())
            {
                DestroyWindow(hwnd);
                close(&weak);
                return None;
            }

            Some(Self {
                header,
                level,
                labels,
                bars,
                family,
                visible,
                numbers,
                progress,
                refresh,
            })
        }
    }
}
