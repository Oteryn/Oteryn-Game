//! Native settings controls and menu, with preferences shared only on the shell thread boundary.
// SAFETY: HWNDs and subclass data are created, used and destroyed on their owning thread.
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

type Shared = SettingsState;
#[path = "basic_native.rs"]
mod basic_window;
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
#[path = "settings_window.rs"]
mod settings_window;
#[path = "shortcuts_native.rs"]
mod shortcuts_window;
#[path = "subpages.rs"]
mod subpages;
use settings_window::run;
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
    // Win32 borrows the Arc handle address; Box keeps it stable when Settings moves.
    #[allow(clippy::redundant_allocation)]
    shared: Box<Arc<Mutex<Shared>>>,
    parent: HWND,
}
impl Settings {
    pub(super) fn new() -> Self {
        Self {
            shared: Box::new(Arc::new(Mutex::new(SettingsState::new(
                PreferenceStore::path(),
            )))),
            parent: null_mut(),
        }
    }
    pub(super) fn attach(&mut self, window: &Window) {
        let Ok(handle) = window.window_handle() else {
            return;
        };
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };
        self.parent = handle.hwnd.get() as HWND;
        unsafe {
            let menu = CreateMenu();
            if !menu.is_null() {
                AppendMenuW(menu, MF_STRING, 25001, wide("Settings...").as_ptr());
                let windows = CreatePopupMenu();
                if !windows.is_null() {
                    for (index, (_, label)) in
                        oteryn_client::play::WINDOW_ACTIONS.iter().enumerate()
                    {
                        AppendMenuW(windows, MF_STRING, 25100 + index, wide(label).as_ptr());
                    }
                    AppendMenuW(
                        menu,
                        MF_POPUP,
                        windows as usize,
                        wide("Game windows").as_ptr(),
                    );
                }
                if SetMenu(self.parent, menu) == 0 {
                    DestroyMenu(menu);
                }
                DrawMenuBar(self.parent);
            }
            SetWindowSubclass(
                self.parent,
                Some(menu_proc),
                25001,
                (&*self.shared as *const Arc<Mutex<Shared>>) as usize,
            );
        }
    }
    pub(super) fn take_window_events(&self) -> Vec<oteryn_input_actions::NormalizedInputEvent> {
        lock(&self.shared).take_window_events()
    }
    pub(super) fn control_events(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        link: Option<&oteryn_client::play::PlayLink>,
    ) -> Vec<oteryn_input_actions::NormalizedInputEvent> {
        lock(&self.shared)
            .control_events(events, link.is_some_and(|link| link.preference_chat_open()))
    }
    pub(super) fn input(
        &mut self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        link: Option<&oteryn_client::play::PlayLink>,
    ) -> Vec<oteryn_input_actions::NormalizedInputEvent> {
        let mut state = lock(&self.shared);
        state.open_help_report(link);
        let (events, requested) = match link {
            Some(link) => state.input_from_play(events, link),
            None => state.input_in_context(events, false),
        };
        drop(state);
        if requested {
            spawn_window(Arc::clone(&self.shared));
        }
        events
    }
    pub(super) fn restore(&self, view: &mut PlayView, link: &oteryn_client::play::PlayLink) {
        let mut viewport = [960, 540];
        unsafe {
            let mut rect: RECT = std::mem::zeroed();
            if !self.parent.is_null() && GetClientRect(self.parent, &mut rect) != 0 {
                viewport = [rect.right.max(0) as u32, rect.bottom.max(0) as u32];
            }
        }
        let mut state = lock(&self.shared);
        state.select_character(view.preference_character());
        state.open_help_report(Some(link));
        state.sync_battle_lists(link);
        state.sync_skills_layout(link);
        state.sync_item_hotkeys(link);
        state.sync_spellbook_filter(link);
        state.sync_archive_bindings(link);
        if let Err(error) = state.restore(view, viewport) {
            state.feedback = error.to_string();
        }
    }
    pub(super) fn capture(&self, view: &mut PlayView) {
        let mut state = lock(&self.shared);
        if let Err(error) = state.capture(view) {
            state.feedback = error.to_string();
        }
    }
}
impl Drop for Settings {
    fn drop(&mut self) {
        lock(&self.shared).shutdown = true;
        unsafe {
            if !self.parent.is_null() && IsWindow(self.parent) != 0 {
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
        if msg == WM_COMMAND {
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
        .spawn(move || run(shared))
        .is_err()
    {
        let mut state = lock(&fallback);
        state.open = false;
        state.feedback = "Could not create settings window".into();
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
                && matches!(id, 103 | 106 | 110..=113 | 120..=126)
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
