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
    pub(super) fn input(
        &mut self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
    ) -> Vec<oteryn_input_actions::NormalizedInputEvent> {
        let (events, requested) = lock(&self.shared).input(events);
        if requested {
            spawn_window(Arc::clone(&self.shared));
        }
        events
    }
    pub(super) fn restore(&self, view: &mut PlayView) {
        let mut viewport = [960, 540];
        unsafe {
            let mut rect: RECT = std::mem::zeroed();
            if !self.parent.is_null() && GetClientRect(self.parent, &mut rect) != 0 {
                viewport = [rect.right.max(0) as u32, rect.bottom.max(0) as u32];
            }
        }
        let mut state = lock(&self.shared);
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
                && matches!(id, 103 | 106 | 110..=113 | 223)
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
fn run(shared: Arc<Mutex<Shared>>) {
    unsafe {
        let hwnd = CreateWindowExW(
            0,
            wide("STATIC").as_ptr(),
            wide("Oteryn Settings").as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            760,
            540,
            null_mut(),
            null_mut(),
            null_mut(),
            null(),
        );
        if hwnd.is_null() {
            lock(&shared).open = false;
            return;
        }
        let mut actions = Box::<Actions>::default();
        SetWindowLongPtrW(
            hwnd,
            GWLP_USERDATA,
            (&mut *actions as *mut Actions) as isize,
        );
        SetWindowLongPtrW(hwnd, GWLP_WNDPROC, panel_proc as *const () as isize);
        let pages = control(
            hwnd,
            "COMBOBOX",
            "",
            220,
            [10, 10, 720, 240],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        for page in SettingsPage::ALL {
            SendMessageW(pages, CB_ADDSTRING, 0, wide(page.label()).as_ptr() as isize);
        }
        SendMessageW(pages, CB_SETCURSEL, 0, 0);
        let help = control(
            hwnd,
            "STATIC",
            "Key bindings: select an action, enter e.g. ALT+H, then Rebind.\nMouse buttons follow the selected preset. Conflicts include all registered actions.",
            0,
            [10, 50, 720, 45],
            0,
        );
        let list = control(
            hwnd,
            "COMBOBOX",
            "",
            101,
            [10, 100, 440, 280],
            CBS_DROPDOWNLIST as u32 | WS_VSCROLL,
        );
        let chord = control(
            hwnd,
            "EDIT",
            "",
            102,
            [460, 100, 140, 26],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        let rebind = control(hwnd, "BUTTON", "Rebind", 103, [610, 100, 110, 26], 0);
        let mouse_preset = control(
            hwnd,
            "BUTTON",
            "Swap primary / secondary mouse buttons",
            221,
            [10, 150, 360, 26],
            BS_AUTOCHECKBOX as u32,
        );
        let delay_label = control(
            hwnd,
            "STATIC",
            "Keyboard repeat delay ms (0–1000)",
            0,
            [10, 185, 360, 26],
            0,
        );
        let delay = control(
            hwnd,
            "EDIT",
            "",
            222,
            [380, 185, 100, 26],
            WS_BORDER | ES_NUMBER as u32,
        );
        let controls_apply = control(
            hwnd,
            "BUTTON",
            "Apply controls",
            223,
            [500, 185, 180, 26],
            0,
        );
        let rotation_label = control(
            hwnd,
            "STATIC",
            "Turn modifier (CTRL / ALT / SHIFT)",
            0,
            [10, 225, 360, 26],
            0,
        );
        let rotation = control(
            hwnd,
            "EDIT",
            "",
            224,
            [380, 225, 100, 26],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        let label = control(
            hwnd,
            "STATIC",
            "World zoom % (80–180, fitted to window)",
            0,
            [10, 110, 700, 25],
            0,
        );
        let scale = control(
            hwnd,
            "EDIT",
            "",
            104,
            [10, 145, 100, 26],
            WS_BORDER | ES_NUMBER as u32,
        );
        let volume = control(
            hwnd,
            "EDIT",
            "",
            105,
            [130, 145, 100, 26],
            WS_BORDER | ES_NUMBER as u32,
        );
        let mute = control(
            hwnd,
            "BUTTON",
            "Mute",
            107,
            [250, 145, 100, 26],
            BS_AUTOCHECKBOX as u32,
        );
        let apply = control(hwnd, "BUTTON", "Apply", 106, [380, 145, 110, 26], 0);
        let transfer_label = control(
            hwnd,
            "STATIC",
            "Settings transfer file (JSON). Import/defaults save a recoverable backup.",
            0,
            [10, 195, 720, 25],
            0,
        );
        let transfer_path = control(
            hwnd,
            "EDIT",
            "",
            114,
            [10, 225, 710, 26],
            WS_BORDER | ES_AUTOHSCROLL as u32,
        );
        SendMessageW(transfer_path, EM_SETLIMITTEXT, 240, 0);
        let import = control(hwnd, "BUTTON", "Import...", 110, [10, 265, 150, 30], 0);
        let export = control(
            hwnd,
            "BUTTON",
            "Export new file",
            111,
            [175, 265, 160, 30],
            0,
        );
        let backup = control(
            hwnd,
            "BUTTON",
            "Restore backup...",
            112,
            [350, 265, 170, 30],
            0,
        );
        let defaults = control(
            hwnd,
            "BUTTON",
            "Restore defaults...",
            113,
            [535, 265, 185, 30],
            0,
        );
        let hud_checks = [
            "HP / MP bars",
            "Harmony",
            "Serene",
            "Light effects",
            "Combat effects",
            "Creature health / icons",
        ]
        .into_iter()
        .enumerate()
        .map(|(index, label)| {
            control(
                hwnd,
                "BUTTON",
                label,
                120 + index,
                [
                    10 + (index % 3) as i32 * 230,
                    310 + (index / 3) as i32 * 30,
                    220,
                    26,
                ],
                BS_AUTOCHECKBOX as u32,
            )
        })
        .collect::<Vec<_>>();
        let feedback = control(hwnd, "STATIC", "", 0, [10, 380, 710, 90], 0);
        if [
            rotation_label,
            rotation,
            mouse_preset,
            delay_label,
            delay,
            controls_apply,
            pages,
            help,
            list,
            chord,
            rebind,
            label,
            scale,
            volume,
            mute,
            apply,
            feedback,
            transfer_label,
            transfer_path,
            import,
            export,
            backup,
            defaults,
        ]
        .iter()
        .chain(hud_checks.iter())
        .any(|h| h.is_null())
        {
            DestroyWindow(hwnd);
            lock(&shared).open = false;
            return;
        }
        let bindings = preference_bindings().unwrap_or_default();
        let mut held = None;
        let mut selected = -1;
        let mut message: MSG = std::mem::zeroed();
        loop {
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                if IsDialogMessageW(hwnd, &message) == 0 {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
            if IsWindow(hwnd) == 0 {
                break;
            }
            let mut state = lock(&shared);
            if state.shutdown {
                break;
            }
            let page_index = SendMessageW(pages, CB_GETCURSEL, 0, 0);
            if let Some(page) = usize::try_from(page_index)
                .ok()
                .and_then(|i| SettingsPage::ALL.get(i))
            {
                state.page = *page;
            }
            for (handles, page) in [
                (
                    &[
                        help,
                        list,
                        chord,
                        rebind,
                        mouse_preset,
                        delay_label,
                        delay,
                        controls_apply,
                        rotation_label,
                        rotation,
                    ][..],
                    SettingsPage::Gameplay,
                ),
                (&[label, scale][..], SettingsPage::Interface),
                (&[volume, mute][..], SettingsPage::Sound),
                (
                    &[
                        transfer_label,
                        transfer_path,
                        import,
                        export,
                        backup,
                        defaults,
                    ][..],
                    SettingsPage::Help,
                ),
            ] {
                for handle in handles {
                    ShowWindow(*handle, if state.page == page { SW_SHOW } else { SW_HIDE });
                }
            }
            for (index, handle) in hud_checks.iter().enumerate() {
                let page = if index < 3 {
                    SettingsPage::Interface
                } else {
                    SettingsPage::Graphics
                };
                ShowWindow(*handle, if state.page == page { SW_SHOW } else { SW_HIDE });
            }
            ShowWindow(
                apply,
                if matches!(
                    state.page,
                    SettingsPage::Interface | SettingsPage::Sound | SettingsPage::Graphics
                ) {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
            let index = SendMessageW(list, CB_GETCURSEL, 0, 0);
            if index != selected && index >= 0 {
                selected = index;
                if let Some(binding) = bindings.get(index as usize) {
                    SetWindowTextW(
                        chord,
                        wide(state.preferences.binding_chord(binding)).as_ptr(),
                    );
                }
            }
            for action in std::mem::take(&mut actions.0) {
                if (110..=113).contains(&action) {
                    drop(state);
                    let confirmed = action == 111
                        || MessageBoxW(
                            hwnd,
                            wide("Replace your current settings? A backup will be saved first.")
                                .as_ptr(),
                            wide("Oteryn Settings").as_ptr(),
                            MB_YESNO | MB_ICONQUESTION,
                        ) == IDYES;
                    state = lock(&shared);
                    if !confirmed || state.shutdown {
                        continue;
                    }
                    let path = std::path::PathBuf::from(text(transfer_path));
                    let result = match action {
                        110 => state.import_from(&path),
                        111 => state.export_to(&path),
                        112 => state.restore_backup(),
                        _ => state.reset_defaults(),
                    };
                    state.feedback = match result {
                        Ok(()) if action == 111 => "Export saved. Existing files are never overwritten.".into(),
                        Ok(()) => "Settings replaced and saved. The previous settings are available through Restore backup.".into(),
                        Err(error) => format!("Settings transfer failed: {error}"),
                    };
                    continue;
                }
                let mut preferences = state.preferences.clone();
                if action == 223 {
                    match text(delay).parse::<u16>() {
                        Ok(value) => {
                            preferences.rotation_modifier =
                                text(rotation).trim().to_ascii_uppercase();
                            preferences.keyboard_delay_ms = value;
                            preferences.swap_mouse_buttons =
                                SendMessageW(mouse_preset, BM_GETCHECK, 0, 0) == 1;
                            state.apply(preferences);
                        }
                        Err(_) => {
                            state.feedback = "Enter a keyboard delay from 0 to 1000 ms".into()
                        }
                    }
                } else if action == 103 {
                    if let Some(binding) = bindings.get(selected.max(0) as usize) {
                        match preferences.rebind(binding.id, &text(chord)) {
                            Ok(()) => state.apply(preferences),
                            Err(error) => {
                                state.feedback = format!("{error}. Choose a different chord.")
                            }
                        }
                    }
                } else {
                    match (text(scale).parse::<u16>(), text(volume).parse::<u8>()) {
                        (Ok(scale), Ok(volume)) => {
                            preferences.ui_scale = scale;
                            preferences.audio.master_volume = volume;
                            preferences.audio.muted = SendMessageW(mute, BM_GETCHECK, 0, 0) == 1;
                            preferences.hud_bars =
                                SendMessageW(hud_checks[0], BM_GETCHECK, 0, 0) == 1;
                            preferences.show_harmony =
                                SendMessageW(hud_checks[1], BM_GETCHECK, 0, 0) == 1;
                            preferences.show_serene =
                                SendMessageW(hud_checks[2], BM_GETCHECK, 0, 0) == 1;
                            preferences.show_lights =
                                SendMessageW(hud_checks[3], BM_GETCHECK, 0, 0) == 1;
                            preferences.show_effects =
                                SendMessageW(hud_checks[4], BM_GETCHECK, 0, 0) == 1;
                            preferences.creature_bars =
                                SendMessageW(hud_checks[5], BM_GETCHECK, 0, 0) == 1;
                            state.apply(preferences);
                        }
                        _ => state.feedback = "Enter valid zoom and volume percentages".into(),
                    }
                }
            }
            if held.as_ref() != Some(&state.preferences) {
                let selection = selected.max(0) as usize;
                SendMessageW(list, CB_RESETCONTENT, 0, 0);
                for binding in &bindings {
                    let label = format!(
                        "{}: {}",
                        binding.id,
                        state.preferences.binding_chord(binding)
                    );
                    SendMessageW(list, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
                }
                SendMessageW(list, CB_SETCURSEL, selection, 0);
                SetWindowTextW(
                    scale,
                    wide(&state.preferences.ui_scale.to_string()).as_ptr(),
                );
                SetWindowTextW(
                    volume,
                    wide(&state.preferences.audio.master_volume.to_string()).as_ptr(),
                );
                SendMessageW(
                    mute,
                    BM_SETCHECK,
                    usize::from(state.preferences.audio.muted),
                    0,
                );
                for (control, checked) in hud_checks.iter().zip([
                    state.preferences.hud_bars,
                    state.preferences.show_harmony,
                    state.preferences.show_serene,
                    state.preferences.show_lights,
                    state.preferences.show_effects,
                    state.preferences.creature_bars,
                ]) {
                    SendMessageW(*control, BM_SETCHECK, usize::from(checked), 0);
                }
                SendMessageW(
                    mouse_preset,
                    BM_SETCHECK,
                    usize::from(state.preferences.swap_mouse_buttons),
                    0,
                );
                SetWindowTextW(
                    delay,
                    wide(&state.preferences.keyboard_delay_ms.to_string()).as_ptr(),
                );
                SetWindowTextW(
                    rotation,
                    wide(&state.preferences.rotation_modifier).as_ptr(),
                );
                selected = -1;
                held = Some(state.preferences.clone());
            }
            let page_help = match state.page {
                SettingsPage::Graphics => {
                    "Renderer and window options use the supported native renderer. Additional controls are not available yet."
                }
                SettingsPage::Sound => {
                    "Master volume % (0–100). Changes apply to playback immediately."
                }
                _ => "",
            };
            SetWindowTextW(
                feedback,
                wide(&format!("{page_help}\r\n{}", state.feedback)).as_ptr(),
            );
            drop(state);
            std::thread::sleep(Duration::from_millis(20));
        }
        if IsWindow(hwnd) != 0 {
            DestroyWindow(hwnd);
        }
        lock(&shared).open = false;
    }
}
