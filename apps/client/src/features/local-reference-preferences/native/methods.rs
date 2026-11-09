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
                        windows,
                        MF_STRING,
                        25300,
                        wide("Cyclopedia: Magical Archive").as_ptr(),
                    );
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
            self.collapse = control(self.parent, "BUTTON", "▼ Game", 25200, [8, 64, 150, 26], 0);
            self.shortcuts = oteryn_client::play::WINDOW_ACTIONS
                .iter()
                .enumerate()
                .map(|(index, (id, label))| {
                    control(
                        self.parent,
                        "BUTTON",
                        &format!("{} {label}", oteryn_client::play::window_icon(id)),
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
    pub(super) fn take_hud_toggle(&self) -> Option<oteryn_client::play::HudPanel> {
        lock(&self.shared).take_hud_toggle()
    }
    pub(super) fn sync_hud_menu(&self, layout: &oteryn_client::play::HudLayout) {
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
    pub(super) fn take_magical_archive_request(&self) -> bool {
        lock(&self.shared).take_magical_archive_request()
    }
    pub(super) fn take_window_events(
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
