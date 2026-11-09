//! Native settings controls and menu, with preferences shared only on the shell thread boundary.
// SAFETY: Owned HWND lifetimes stay on their creating thread; cross-panel geometry and font changes are posted to their owners.
#![allow(unsafe_code)]
use oteryn_client::play::{
    PlayView, PreferenceStore, SettingsPage, SettingsState, preference_bindings,
};
use std::{
    ptr::{null, null_mut},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    UI::{
        Controls::EM_SETLIMITTEXT,
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};
use winit::{
    raw_window_handle::{HasWindowHandle, RawWindowHandle},
    window::Window,
};

#[path = "native_tooltips.rs"]
mod native_tooltips;
#[path = "native_ui_scale.rs"]
mod native_ui_scale;

type Shared = SettingsState;
#[path = "action_bar_controls.rs"]
mod action_bar;
#[path = "basic_native.rs"]
mod basic_window;
#[path = "capture_native.rs"]
mod capture_window;
#[path = "controls_native.rs"]
mod controls_window;
#[path = "frames_native.rs"]
mod frames_window;
#[path = "help_native.rs"]
mod help_window;
#[path = "minimap_native.rs"]
mod minimap_window;
#[path = "profiles_native.rs"]
mod profile_window;
#[path = "screenshot_native.rs"]
mod screenshot_native;

#[path = "shortcuts_native.rs"]
mod shortcuts_window;
#[path = "subpages.rs"]
mod subpages;
fn lock(shared: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    shared
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
fn wide(text: &str) -> Vec<u16> {
    text.replace('\0', "\u{fffd}")
        .encode_utf16()
        .chain(Some(0))
        .collect()
}

pub(super) struct Settings {
    action_bar: Option<action_bar::Buttons>,
    ui_scaler: std::cell::RefCell<native_ui_scale::UiScaler>,
    tooltip: std::cell::RefCell<native_tooltips::NativeTooltip>,
    // Win32 borrows the Arc handle address; Box keeps it stable when Settings moves.
    #[allow(clippy::redundant_allocation)]
    shared: Box<Arc<Mutex<Shared>>>,
    parent: HWND,
    shortcuts: Vec<HWND>,
    collapse: HWND,
    performance_label: HWND,
}
#[path = "settings_native_state.rs"]
mod settings_native_state;

impl Drop for Settings {
    fn drop(&mut self) {
        lock(&self.shared).shutdown = true;
        unsafe {
            if !self.parent.is_null() && IsWindow(self.parent) != 0 {
                for handle in &self.shortcuts {
                    DestroyWindow(*handle);
                }
                DestroyWindow(self.collapse);
                RemoveWindowSubclass(self.parent, Some(menu_proc), 25001);
                let menu = GetMenu(self.parent);
                SetMenu(self.parent, null_mut());
                if !menu.is_null() {
                    DestroyMenu(menu);
                }
            }
        }
    }
}
unsafe extern "system" fn menu_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _id: usize,
    data: usize,
) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND && w & 0xffff == 25001 {
            let shared = &*(data as *const Arc<Mutex<Shared>>);
            open(Arc::clone(shared));
            return 0;
        }
        if msg == WM_COMMAND && (25440..25444).contains(&(w & 0xffff)) {
            let shared = &*(data as *const Arc<Mutex<Shared>>);
            if let Some(panel) = oteryn_client::play::HudPanel::ALL.get((w & 0xffff) - 25440) {
                lock(shared).request_hud_toggle(*panel);
            }
            return 0;
        }
        if msg == WM_COMMAND && w & 0xffff == 25300 {
            let shared = &*(data as *const Arc<Mutex<Shared>>);
            lock(shared).request_magical_archive();
            return 0;
        }
        if msg == WM_COMMAND && w & 0xffff == 25200 {
            let shared = &*(data as *const Arc<Mutex<Shared>>);
            let mut state = lock(shared);
            let mut preferences = state.preferences.clone();
            preferences.shortcuts_collapsed = !preferences.shortcuts_collapsed;
            state.apply(preferences);
            return 0;
        }
        if msg == WM_COMMAND {
            let slot = (w & 0xffff).wrapping_sub(25300);
            if slot < 4 {
                let shared = &*(data as *const Arc<Mutex<Shared>>);
                lock(shared).queue_bar_slot(slot);
                windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(hwnd);
                return 0;
            }
            let index = (w & 0xffff).wrapping_sub(25100);
            if let Some((id, _)) = oteryn_client::play::WINDOW_ACTIONS.get(index) {
                let shared = &*(data as *const Arc<Mutex<Shared>>);
                lock(shared).queue_window(id);
                return 0;
            }
        }
        DefSubclassProc(hwnd, msg, w, l)
    }
}
fn open(shared: Arc<Mutex<Shared>>) {
    if lock(&shared).request_open() {
        spawn_window(shared);
    }
}
fn spawn_window(shared: Arc<Mutex<Shared>>) {
    let fallback = Arc::clone(&shared);
    if std::thread::Builder::new()
        .name("local-settings".into())
        .spawn(move || dialog::run(shared))
        .is_err()
    {
        let mut state = lock(&fallback);
        state.open = false;
        state.feedback = tr!("Could not create settings window").into();
    }
}
#[derive(Default)]
struct Actions(Vec<usize>);
unsafe extern "system" fn panel_proc(hwnd: HWND, msg: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_COMMAND {
            let actions = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Actions;
            let id = w & 0xffff;
            if !actions.is_null()
                && (*actions).0.len() < 16
                && matches!(id, 103 | 106 | 110..=113 | 160..=167 | 223 | 225..=227 | 231..=234 | 241..=244 | 250..=251 | 260..=263 | 270..=271 | 280..=283 | 360..=363)
                && w >> 16 == BN_CLICKED as usize
            {
                (*actions).0.push(id);
            }
            return 0;
        }
        DefWindowProcW(hwnd, msg, w, l)
    }
}
unsafe fn control(
    parent: HWND,
    class: &str,
    text: &str,
    id: usize,
    rect: [i32; 4],
    style: u32,
) -> HWND {
    unsafe {
        CreateWindowExW(
            0,
            wide(class).as_ptr(),
            wide(text).as_ptr(),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | style,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            parent,
            id as *mut _,
            null_mut(),
            null(),
        )
    }
}
unsafe fn text(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    unsafe {
        let length = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
        String::from_utf16_lossy(&buffer[..length.max(0) as usize])
    }
}

#[path = "dialog.rs"]
mod dialog;

#[path = "native_helpers.rs"]
mod helpers;
use helpers::*;

#[path = "transfer.rs"]
mod transfer;

#[path = "navigation_controls.rs"]
mod navigation_controls;

#[path = "localization_window.rs"]
mod localization_window;

#[path = "performance_native.rs"]
mod performance_native;
