//! Routes settings navigation without retaining the settings mutex across a child dialog.
use super::Shared;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::Foundation::HWND;
pub(super) fn open(action: usize, shared: Arc<Mutex<Shared>>, parent: HWND) {
    match action {
        26000 => super::profile_window::open(shared, parent),
        26001 => super::help_window::open(shared, parent),
        26002 => super::controls_window::open(shared, parent),
        26003 => super::shortcuts_window::open(shared, parent),
        26004 => super::minimap_window::open(shared, parent),
        26005 => super::frames_window::open(shared, parent),
        26006 => super::basic_window::open(shared, parent),
        _ => {}
    }
}

/// Child buttons share the settings dialog thread and use a separate control-id range.
pub(super) unsafe fn create(parent: HWND) -> Vec<HWND> {
    [
        "Profiles",
        "Help",
        "Controls",
        "Shortcuts",
        "Minimap transfer",
        "Frame pacing",
        "Basic options",
    ]
    .into_iter()
    .enumerate()
    .map(|(index, label)| unsafe {
        super::control(
            parent,
            "BUTTON",
            label,
            26000 + index,
            [
                10 + (index % 4) as i32 * 180,
                480 + (index / 4) as i32 * 30,
                175,
                28,
            ],
            0,
        )
    })
    .collect()
}
