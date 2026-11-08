//! Routes settings navigation without retaining the settings mutex across a child dialog.
use super::Shared;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::Foundation::HWND;
pub(super) fn open(action: usize, shared: Arc<Mutex<Shared>>, parent: HWND) {
    match action {
        120 => super::profile_window::open(shared, parent),
        121 => super::help_window::open(shared, parent),
        122 => super::controls_window::open(shared, parent),
        123 => super::shortcuts_window::open(shared, parent),
        124 => super::minimap_window::open(shared, parent),
        125 => super::frames_window::open(shared, parent),
        126 => super::basic_window::open(shared, parent),
        _ => {}
    }
}
