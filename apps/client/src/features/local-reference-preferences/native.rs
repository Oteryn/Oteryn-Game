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
    ui_scaler: std::cell::RefCell<native_ui_scale::UiScaler>,
    tooltip: std::cell::RefCell<native_tooltips::NativeTooltip>,
    // Win32 borrows the Arc handle address; Box keeps it stable when Settings moves.
    #[allow(clippy::redundant_allocation)]
    shared: Box<Arc<Mutex<Shared>>>,
    parent: HWND,
    shortcuts: Vec<HWND>,
    collapse: HWND,
}
impl Settings {
    pub(super) fn new() -> Self {
        Self {
            ui_scaler: std::cell::RefCell::new(native_ui_scale::UiScaler::default()),
            tooltip: std::cell::RefCell::new(native_tooltips::NativeTooltip::default()),
            shared: Box::new(Arc::new(Mutex::new(SettingsState::new(
                PreferenceStore::path(),
            )))),
            parent: null_mut(),
            shortcuts: Vec::new(),
            collapse: null_mut(),
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
            self.collapse = control(self.parent, "BUTTON", "▼ Game", 25200, [8, 64, 150, 26], 0);
            self.shortcuts = oteryn_client::play::WINDOW_ACTIONS
                .iter()
                .enumerate()
                .map(|(index, (_, label))| {
                    control(
                        self.parent,
                        "BUTTON",
                        &format!("◆ {label}"),
                        25100 + index,
                        [8, 94 + index as i32 * 28, 150, 26],
                        0,
                    )
                })
                .collect();
            SetWindowSubclass(
                self.parent,
                Some(menu_proc),
                25001,
                (&*self.shared as *const Arc<Mutex<Shared>>) as usize,
            );
        }
    }
    pub(super) fn take_window_events(
        &self,
        playing: bool,
    ) -> Vec<oteryn_input_actions::NormalizedInputEvent> {
        let mut state = lock(&self.shared);
        unsafe {
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(self.parent, &mut rect);
            let percentage = state.preferences.interface_scale;
            let row = oteryn_client::play::scale_ui_rect([0, 64, 150, 26], percentage);
            let x = (rect.right - row[2] - 10).max(0);
            MoveWindow(self.collapse, x, row[1], row[2], row[3], 1);
            ShowWindow(self.collapse, if playing { SW_SHOW } else { SW_HIDE });
            SetWindowTextW(
                self.collapse,
                wide(if state.preferences.shortcuts_collapsed {
                    "▶ Game"
                } else {
                    "▼ Game"
                })
                .as_ptr(),
            );
            for (index, handle) in self.shortcuts.iter().enumerate() {
                let id = oteryn_client::play::WINDOW_ACTIONS[index].0;
                let position = state
                    .preferences
                    .shortcut_order
                    .iter()
                    .position(|key| key == id);
                if let Some(position) = position {
                    let row = oteryn_client::play::scale_ui_rect(
                        [0, 94 + position as i32 * 28, 150, 26],
                        percentage,
                    );
                    MoveWindow(*handle, x, row[1], row[2], row[3], 1);
                }
                ShowWindow(
                    *handle,
                    if playing && position.is_some() && !state.preferences.shortcuts_collapsed {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
            }
        }
        let percentage = state.preferences.interface_scale;
        let events = state.take_window_events();
        drop(state);
        self.ui_scaler.borrow_mut().refresh(self.parent, percentage);
        events
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
        state.sync_skills_layout(link);
        state.sync_item_hotkeys(link);
        state.sync_spellbook_filter(link);
        state.sync_archive_bindings(link);
        state.sync_chat_context(link);
        state.sync_battle_lists(link);
        state.sync_local_preferences(link);
        if let Err(error) = state.restore(view, viewport) {
            state.feedback = error.to_string();
        }
        let enabled = state.preferences.tooltips_enabled;
        let delay = state.preferences.tooltip_delay_ms;
        let percentage = state.preferences.interface_scale;
        drop(state);
        let font = self.ui_scaler.borrow_mut().font(percentage);
        self.tooltip
            .borrow_mut()
            .refresh(self.parent, view, enabled, delay, font);
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
        if msg == WM_COMMAND && w & 0xffff == 25200 {
            let shared = &*(data as *const Arc<Mutex<Shared>>);
            let mut state = lock(shared);
            let mut preferences = state.preferences.clone();
            preferences.shortcuts_collapsed = !preferences.shortcuts_collapsed;
            state.apply(preferences);
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
        .spawn(move || dialog::run(shared))
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
                && matches!(id, 103 | 106 | 110..=113 | 160..=166 | 223 | 225..=227 | 231..=233 | 241..=244)
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
