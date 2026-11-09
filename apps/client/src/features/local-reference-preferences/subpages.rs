//! Routes settings navigation without retaining the settings mutex across a child dialog.
use super::Shared;
use std::sync::{Arc, Mutex};
use windows_sys::Win32::Foundation::HWND;
pub(super) fn open(action: usize, shared: Arc<Mutex<Shared>>, parent: HWND) {
    match action {
        160 => super::profile_window::open(shared, parent),
        161 => super::help_window::open(shared, parent),
        162 => super::controls_window::open(shared, parent),
        163 => super::shortcuts_window::open(shared, parent),
        164 => super::minimap_window::open(shared, parent),
        165 => super::frames_window::open(shared, parent),
        166 => super::basic_window::open(shared, parent),
        _ => {}
    }
}
