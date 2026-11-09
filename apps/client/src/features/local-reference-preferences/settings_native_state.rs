use super::*;
impl Settings {
    pub(in super::super) fn new() -> Self {
        Self {
            action_bar: None,
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
    pub(in super::super) fn attach(&mut self, window: &Window) {
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
                AppendMenuW(menu, MF_STRING, 25001, wide(tr!("Settings...")).as_ptr());
                let windows = CreatePopupMenu();
                if !windows.is_null() {
                    for (index, (_, label)) in
                        oteryn_client::play::WINDOW_ACTIONS.iter().enumerate()
                    {
                        AppendMenuW(
                            windows,
                            MF_STRING,
                            25100 + index,
                            wide(oteryn_client_ui::localization::tr(label)).as_ptr(),
                        );
                    }
                    AppendMenuW(
                        menu,
                        MF_POPUP,
                        windows as usize,
                        wide(tr!("Game windows")).as_ptr(),
                    );
                }
                if SetMenu(self.parent, menu) == 0 {
                    DestroyMenu(menu);
                }
                DrawMenuBar(self.parent);
            }
            let hud_menu = CreatePopupMenu();
            let menu = GetMenu(self.parent);
            if !hud_menu.is_null() && !menu.is_null() {
                for (index, panel) in oteryn_client::play::HudPanel::ALL.into_iter().enumerate() {
                    AppendMenuW(
                        hud_menu,
                        MF_STRING,
                        25440 + index,
                        wide(panel.title()).as_ptr(),
                    );
                }
                AppendMenuW(menu, MF_POPUP, hud_menu as usize, wide("HUD").as_ptr());
                DrawMenuBar(self.parent);
            }
            self.collapse = control(
                self.parent,
                "BUTTON",
                tr!("▼ Game"),
                25200,
                [8, 64, 150, 26],
                0,
            );
            self.action_bar = action_bar::Buttons::new(self.parent);
            self.shortcuts = oteryn_client::play::WINDOW_ACTIONS
                .iter()
                .enumerate()
                .map(|(index, (_, label))| {
                    let label = oteryn_client_ui::localization::tr(label);
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
    pub(in super::super) fn take_hud_toggle(&self) -> Option<oteryn_client::play::HudPanel> {
        lock(&self.shared).take_hud_toggle()
    }
    pub(in super::super) fn sync_hud_menu(&self, layout: &oteryn_client::play::HudLayout) {
        unsafe {
            let menu = GetSubMenu(GetMenu(self.parent), 2);
            if !menu.is_null() {
                for (index, panel) in oteryn_client::play::HudPanel::ALL.into_iter().enumerate() {
                    CheckMenuItem(
                        menu,
                        (25440 + index) as u32,
                        MF_BYCOMMAND
                            | if layout.pane(panel).visible {
                                MF_CHECKED
                            } else {
                                MF_UNCHECKED
                            },
                    );
                }
            }
        }
    }
    pub(in super::super) fn take_magical_archive_request(&self) -> bool {
        lock(&self.shared).take_magical_archive_request()
    }
    pub(in super::super) fn take_window_events(
        &self,
        playing: bool,
        open_states: &[bool],
    ) -> Option<(
        &'static str,
        Vec<oteryn_input_actions::NormalizedInputEvent>,
    )> {
        let mut state = lock(&self.shared);
        unsafe {
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(self.parent, &mut rect);
            if let Some(bar) = &self.action_bar {
                bar.show(
                    &state.preferences,
                    [rect.right.max(0) as u32, rect.bottom.max(0) as u32],
                    playing,
                );
            }
            let percentage = state.preferences.interface_scale;
            let row = oteryn_client::play::scale_ui_rect([0, 64, 150, 26], percentage);
            let x = (rect.right - row[2] - 10).max(0);
            MoveWindow(self.collapse, x, row[1], row[2], row[3], 1);
            ShowWindow(self.collapse, if playing { SW_SHOW } else { SW_HIDE });
            SetWindowTextW(
                self.collapse,
                wide(if state.preferences.shortcuts_collapsed {
                    tr!("▶ Game")
                } else {
                    tr!("▼ Game")
                })
                .as_ptr(),
            );
            for (index, handle) in self.shortcuts.iter().enumerate() {
                let id = oteryn_client::play::WINDOW_ACTIONS[index].0;
                let open = open_states.get(index).copied().unwrap_or(false);
                SendMessageW(*handle, BM_SETSTATE, usize::from(open), 0);
                let windows = GetSubMenu(GetMenu(self.parent), 1);
                if !windows.is_null() {
                    CheckMenuItem(
                        windows,
                        (25100 + index) as u32,
                        MF_BYCOMMAND | if open { MF_CHECKED } else { MF_UNCHECKED },
                    );
                }
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
        let events = state.take_window_selection();
        drop(state);
        self.ui_scaler.borrow_mut().refresh(self.parent, percentage);
        events
    }
    pub(in super::super) fn control_events(
        &self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        link: Option<&oteryn_client::play::PlayLink>,
    ) -> Vec<oteryn_input_actions::NormalizedInputEvent> {
        lock(&self.shared)
            .control_events(events, link.is_some_and(|link| link.preference_chat_open()))
    }
    pub(in super::super) fn input(
        &mut self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        link: Option<&oteryn_client::play::PlayLink>,
    ) -> (
        Vec<oteryn_input_actions::NormalizedInputEvent>,
        Vec<oteryn_client::play::CustomAction>,
    ) {
        let mut state = lock(&self.shared);
        state.open_help_report(link);
        let (events, requested) = match link {
            Some(link) => state.input_from_play(events, link),
            None => state.input_in_context(events, false),
        };
        let actions = state.take_custom_actions();
        drop(state);
        if requested {
            spawn_window(Arc::clone(&self.shared));
        }
        (events, actions)
    }
    pub(in super::super) fn restore(
        &self,
        view: &mut PlayView,
        link: &oteryn_client::play::PlayLink,
    ) {
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
        state.sync_bar_actions(link);
        state.sync_spellbook_filter(link);
        state.sync_archive_bindings(link);
        state.refresh_custom_choices(link);
        state.sync_chat_context(link);
        state.sync_battle_lists(link);
        state.sync_local_preferences(link);
        state.open_typed_help_report(link);
        oteryn_client_ui::localization::set_default(state.preferences.language());
        match state.apply_minimap_transfer(view) {
            Ok(true) => state.feedback = tr!("Minimap transfer saved").into(),
            Err(error) => state.feedback = lfmt!("Minimap transfer failed: {error}", error = error),
            Ok(false) => {}
        }
        state.sync_presentation(link);
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
    pub(in super::super) fn request_screenshot(&self) {
        lock(&self.shared).request_screenshot();
    }
    pub(in super::super) fn reset_screenshots(&self) {
        lock(&self.shared).reset_screenshot_session();
    }
    pub(in super::super) fn screenshot_preferences(
        &self,
    ) -> oteryn_client::play::ScreenshotPreferences {
        lock(&self.shared).preferences.screenshots.effective()
    }
    pub(in super::super) fn set_screenshot_preferences(
        &self,
        preferences: oteryn_client::play::ScreenshotPreferences,
    ) {
        let mut state = lock(&self.shared);
        let mut prefs = state.preferences.clone();
        let mut preferences = preferences;
        preferences = preferences.reconciled(&prefs.screenshots);
        prefs.screenshots = preferences;
        state.apply(prefs);
    }
    pub(in super::super) fn graphics_settings(
        &self,
    ) -> (oteryn_client::play::GraphicsPreferences, u16) {
        let state = lock(&self.shared);
        (state.preferences.graphics, state.preferences.ui_scale)
    }
    pub(in super::super) fn capture(&self, view: &mut PlayView) {
        let mut state = lock(&self.shared);
        if let Err(error) = state.capture(view) {
            state.feedback = error.to_string();
        }
    }
}

impl Settings {
    pub(in super::super) fn take_native_sign_out(&self) -> bool {
        lock(&self.shared).take_native_sign_out()
    }
    pub(in super::super) fn confirm_exit(&self) -> bool {
        !lock(&self.shared).preferences.miscellaneous.confirm_exit
            || oteryn_client::device_login::confirm_action("Close Oteryn?")
    }
    pub(in super::super) fn confirm_logout(&self) -> bool {
        !lock(&self.shared).preferences.miscellaneous.confirm_logout
            || oteryn_client::device_login::confirm_action("Log out of the game?")
    }
}
