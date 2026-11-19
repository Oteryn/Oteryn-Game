use super::*;
#[cfg(bevy_shell)]
#[derive(Default)]
pub(crate) struct PreferenceSaveCompletion {
    pub(crate) before_restore: Option<ReferencePreferences>,
    pub(crate) error: Option<String>,
}

impl SettingsState {
    pub fn request_hud_toggle(&mut self, panel: crate::play::HudPanel) {
        self.hud_toggle = Some(panel);
    }
    pub fn take_hud_toggle(&mut self) -> Option<crate::play::HudPanel> {
        self.hud_toggle.take()
    }
    pub fn take_window_selection(&mut self) -> Option<(&'static str, Vec<NormalizedInputEvent>)> {
        let id = self.window_actions.pop_front()?;
        let raw = super::super::bindings::window_events(&self.preferences, id);
        let (events, _) = self.input(&raw);
        Some((id, events))
    }
    pub fn request_magical_archive(&mut self) {
        self.magical_archive_requested = true;
    }
    pub fn take_magical_archive_request(&mut self) -> bool {
        std::mem::take(&mut self.magical_archive_requested)
    }

    /// Global's ShowOptionsHotkeys: the options window shows `page`, open or not yet open.
    pub fn request_page(&mut self, page: SettingsPage) {
        self.page = page;
        self.page_request = Some(page);
    }
    pub fn take_page_request(&mut self) -> Option<SettingsPage> {
        self.page_request.take()
    }

    pub fn select_page(&mut self, index: usize) -> bool {
        let Some(page) = SettingsPage::ALL.get(index) else {
            return false;
        };
        self.page = *page;
        true
    }

    pub fn sync_local_preferences(&self, link: &crate::play::PlayLink) {
        if self.preferences.validate().is_err() {
            return;
        }
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        if mailbox.local_preferences.action_bar.three_rows != self.preferences.action_bar.three_rows
        {
            for (index, row) in mailbox.bar_actions.layout.rows.iter_mut().enumerate() {
                row.enabled = index == 0 || self.preferences.action_bar.three_rows;
            }
            mailbox.bar_actions.dirty = true;
            mailbox.bar_actions.edit = crate::play::feature_0017_attack::view::next_edit();
        }
        mailbox
            .skill_progress
            .order
            .clone_from(&self.preferences.skill_order);
        mailbox.condition_status.order = self
            .preferences
            .ordered_condition_keys()
            .iter()
            .filter_map(|key| {
                (1..=27).find(|icon| oteryn_client_ui::condition_status::icon_key(*icon) == key)
            })
            .collect();
        mailbox.condition_status.hidden = (1..=27)
            .filter(|icon| {
                self.preferences
                    .condition_status_hidden
                    .iter()
                    .any(|key| key == oteryn_client_ui::condition_status::icon_key(*icon))
            })
            .collect();
        if mailbox.local_preferences.console_font_px != self.preferences.console_font_px {
            mailbox.chat.console.display.font_px = self.preferences.console_font_px;
        }
        if mailbox.local_preferences != self.preferences {
            mailbox.local_preferences.clone_from(&self.preferences);
        }
    }

    pub fn refresh_custom_choices(&mut self, link: &crate::play::PlayLink) {
        if self.open
            && self.page == SettingsPage::CustomHotkeys
            && self
                .custom_choices_refreshed
                .is_none_or(|time| time.elapsed() >= std::time::Duration::from_millis(250))
        {
            self.custom_choices = link.custom_hotkey_choices();
            self.custom_choices_refreshed = Some(std::time::Instant::now());
        }
    }
    pub fn take_custom_actions(&mut self) -> Vec<super::super::CustomAction> {
        self.input.take_custom_actions()
    }

    pub fn queue_window(&mut self, id: &str) -> bool {
        let Some((id, _)) = WINDOW_ACTIONS.iter().find(|(key, _)| *key == id) else {
            return false;
        };
        if self.window_actions.len() >= 32 {
            return false;
        }
        self.window_actions.push_back(id);
        true
    }

    pub fn take_window_events(&mut self) -> Vec<NormalizedInputEvent> {
        let preferences = &self.preferences;
        self.window_actions
            .drain(..)
            .flat_map(|id| super::super::bindings::window_events(preferences, id))
            .collect()
    }

    pub fn export_to(&self, path: &Path) -> io::Result<()> {
        PreferenceStore::export_to(&self.preferences, path)
    }

    pub fn import_from(&mut self, path: &Path) -> io::Result<()> {
        self.replace_saved(PreferenceStore::read_transfer(path)?)
    }

    pub fn reset_defaults(&mut self) -> io::Result<()> {
        self.replace_saved(ReferencePreferences::default())
    }

    pub fn restore_backup(&mut self) -> io::Result<()> {
        #[cfg(bevy_shell)]
        if let Some(store) = &mut self.store
            && store.saving()
        {
            return store.restore_in_background(&self.preferences);
        }
        let path = self
            .store
            .as_ref()
            .ok_or_else(|| io::Error::other(tr!("Preference storage is unavailable")))?
            .backup_path();
        self.import_from(&path)
    }

    pub(super) fn replace_saved(
        &mut self,
        mut preferences: ReferencePreferences,
    ) -> io::Result<()> {
        preferences.miscellaneous = self.preferences.miscellaneous;
        let store = self
            .store
            .as_mut()
            .ok_or_else(|| io::Error::other(tr!("Preference storage is unavailable")))?;
        store.replace_with_backup(preferences.clone(), &self.preferences)?;
        self.accept_replaced(preferences);
        Ok(())
    }

    fn accept_replaced(&mut self, preferences: ReferencePreferences) {
        self.preferences = preferences;
        self.battle_edit = crate::play::feature_0017_attack::view::next_edit();
        self.skills_edit = self.battle_edit;
        self.item_hotkeys_edit = self.battle_edit;
        self.bar_actions_edit = self.battle_edit;
        self.minimap_edit = self.battle_edit;
        self.hud_edit = self.battle_edit;
        self.spellbook_edit = self.battle_edit;
        self.stash_sort_edit = self.battle_edit;
        self.archive_edit = self.battle_edit;
    }

    pub fn new(path: Option<PathBuf>) -> Self {
        let base_path = path.clone();
        let (store, feedback) = match path {
            Some(path) => match PreferenceStore::load(path.clone()) {
                Ok(store) => (Some(store), String::new()),
                Err(error) => (
                    Some(PreferenceStore::defaults(path)),
                    lfmt!("Could not load preferences: {error}", error = error),
                ),
            },
            None => (
                None,
                tr!("LOCALAPPDATA is unavailable; preferences cannot be saved").into(),
            ),
        };
        let preferences = store
            .as_ref()
            .map_or_else(ReferencePreferences::default, |s| s.preferences().clone());
        Self {
            minimap_transfer: None,
            help_report: None,
            screenshot_runtime: Default::default(),
            screenshot_requested: false,
            native_sign_out: false,
            web_link: None,
            #[cfg(bevy_shell)]
            skills_toggles: Vec::new(),
            window_actions: Default::default(),
            custom_choices: Vec::new(),
            custom_choices_refreshed: None,
            pending_chat_mode: None,
            chat_mode_seen: None,
            chat_context: None,
            help_report_pending: false,
            control_input: super::super::controls::ControlInput::default(),
            control_epoch: std::time::Instant::now(),
            base_path: base_path.clone(),
            character: None,
            skills_edit: 0,
            item_hotkeys_edit: 0,
            bar_actions_edit: 0,
            minimap_edit: 0,
            hud_edit: 0,
            spellbook_edit: 0,
            stash_sort_edit: 0,
            archive_edit: 0,
            battle_edit: 0,
            store,
            #[cfg(bevy_shell)]
            outgoing_stores: Vec::new(),
            preferences: {
                let mut preferences = preferences;
                preferences.reconcile_controls(None);
                preferences.miscellaneous = base_path
                    .as_ref()
                    .and_then(|path| {
                        super::super::MiscellaneousPreferences::read(
                            &path.with_extension("session-options.json"),
                        )
                        .ok()
                    })
                    .unwrap_or_default();
                preferences
            },
            feedback,
            open: false,
            shutdown: false,
            input: PreferenceInput::default(),
            page: SettingsPage::default(),
            hud_toggle: None,
            magical_archive_requested: false,
            page_request: None,
            last_key: Default::default(),
        }
    }

    pub fn request_open(&mut self) -> bool {
        if self.open || self.shutdown {
            return false;
        }
        self.open = true;
        true
    }

    pub fn control_events(
        &mut self,
        events: &[NormalizedInputEvent],
        chat_open: bool,
    ) -> Vec<NormalizedInputEvent> {
        self.control_events_at(events, chat_open, self.control_epoch.elapsed())
    }

    pub fn control_events_at(
        &mut self,
        events: &[NormalizedInputEvent],
        chat_open: bool,
        now: std::time::Duration,
    ) -> Vec<NormalizedInputEvent> {
        let mut controls = self.preferences.controls.clone();
        if self.preferences.swap_mouse_buttons {
            controls.mouse = super::super::MousePreset::LeftHanded;
        }
        if controls.rotation == super::super::RotationModifier::Control {
            controls.rotation = match self.preferences.rotation_modifier.as_str() {
                "ALT" => super::super::RotationModifier::Alt,
                "SHIFT" => super::super::RotationModifier::Shift,
                _ => super::super::RotationModifier::Control,
            };
        }
        controls.keyboard_delay_ms = controls
            .keyboard_delay_ms
            .max(self.preferences.keyboard_delay_ms);
        self.control_input.route(&controls, events, chat_open, now)
    }

    pub fn input(&mut self, events: &[NormalizedInputEvent]) -> (Vec<NormalizedInputEvent>, bool) {
        self.input_in_context_inner(events, self.preferences.hotkey_contexts.chat_on, false)
    }

    pub fn input_from_play(
        &mut self,
        events: &[NormalizedInputEvent],
        link: &crate::play::PlayLink,
    ) -> (Vec<NormalizedInputEvent>, bool) {
        self.input_in_context(events, link.preference_chat_open())
    }

    pub fn input_in_context(
        &mut self,
        events: &[NormalizedInputEvent],
        chat_on: bool,
    ) -> (Vec<NormalizedInputEvent>, bool) {
        self.input_in_context_inner(events, chat_on, true)
    }

    fn input_in_context_inner(
        &mut self,
        events: &[NormalizedInputEvent],
        chat_on: bool,
        controls_applied: bool,
    ) -> (Vec<NormalizedInputEvent>, bool) {
        let mut routed = self
            .input
            .route_chat(&self.preferences, events, chat_on)
            .unwrap_or_default();
        use oteryn_input_actions::{ButtonState, Modifier, Modifiers};
        let now = std::time::Instant::now();
        routed.retain_mut(|event| {
            if controls_applied
                && let NormalizedInputEvent::Key {
                    code, modifiers, ..
                } = event
                && (79..=82).contains(&code.get())
                && *modifiers == Modifiers::one(Modifier::Control)
            {
                *modifiers = Modifiers::one(match self.preferences.rotation_modifier.as_str() {
                    "ALT" => Modifier::Alt,
                    "SHIFT" => Modifier::Shift,
                    _ => Modifier::Control,
                });
            }
            match event {
                NormalizedInputEvent::MouseButton { button, .. }
                    if !controls_applied
                        && (self.preferences.swap_mouse_buttons
                            || self.preferences.controls.mouse
                                == super::super::MousePreset::LeftHanded) =>
                {
                    *button = super::super::controls::swap_button(*button);
                }
                NormalizedInputEvent::Key {
                    code,
                    state: ButtonState::Pressed,
                    repeat,
                    ..
                } => {
                    if *repeat
                        && self.last_key.get(code).is_some_and(|last| {
                            now.duration_since(*last).as_millis()
                                < u128::from(self.preferences.keyboard_delay_ms)
                        })
                    {
                        return false;
                    }
                    self.last_key.insert(*code, now);
                }
                NormalizedInputEvent::Key {
                    code,
                    state: ButtonState::Released,
                    ..
                } => {
                    self.last_key.remove(code);
                }
                NormalizedInputEvent::FocusChanged { focused: false }
                | NormalizedInputEvent::DeviceLost => self.last_key.clear(),
                _ => {}
            }
            true
        });
        if oteryn_client_ui::reference_preferences::open_key(&routed) {
            (Vec::new(), self.request_open())
        } else {
            (routed, false)
        }
    }

    pub fn apply(&mut self, mut preferences: ReferencePreferences) {
        if let Err(error) = preferences.validate() {
            self.feedback = error.to_string();
            return;
        }
        preferences.reconcile_controls(Some(&self.preferences));
        let result = self
            .store
            .as_mut()
            .ok_or_else(|| tr!("Preference storage is unavailable").to_owned())
            .and_then(|store| store.apply(preferences.clone()).map_err(|e| e.to_string()))
            .and_then(|()| self.save_miscellaneous(preferences.miscellaneous));
        // A disk error must not disable local controls for this process.
        if self.preferences.bar_actions != preferences.bar_actions {
            self.bar_actions_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if self.preferences.archive_bindings != preferences.archive_bindings {
            self.archive_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if self.preferences.stash_stock_order != preferences.stash_stock_order {
            self.stash_sort_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if self.preferences.spellbook_filter != preferences.spellbook_filter {
            self.spellbook_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if self.preferences.minimap != preferences.minimap {
            self.minimap_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if self.preferences.hud_layout != preferences.hud_layout
            || self.preferences.hud_bounds != preferences.hud_bounds
        {
            self.hud_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if self.preferences.skills_layout != preferences.skills_layout {
            self.skills_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if self.preferences.battle_lists != preferences.battle_lists
            || self.preferences.active_battle_list != preferences.active_battle_list
        {
            self.battle_edit = crate::play::feature_0017_attack::view::next_edit();
        }
        if !preferences.chat_context_hotkeys {
            self.pending_chat_mode = None;
            self.chat_context = None;
        }
        self.preferences = preferences;
        self.feedback = match result {
            #[cfg(bevy_shell)]
            Ok(()) if self.preference_save_pending() => "Saving preferences…".into(),
            Ok(()) => tr!("Applied and saved").into(),
            Err(error) => lfmt!(
                "Applied for this session; could not save: {error}",
                error = error
            ),
        };
    }

    #[cfg(bevy_shell)]
    pub(crate) fn apply_minimap_marks_in_background(&mut self, preferences: ReferencePreferences) {
        self.preferences.minimap = preferences.minimap;
        self.minimap_edit = crate::play::feature_0017_attack::view::next_edit();
        self.feedback = if let Some(store) = &mut self.store {
            store.apply_in_background(self.preferences.clone());
            String::new()
        } else {
            tr!("Preference storage is unavailable").into()
        };
    }

    #[cfg(bevy_shell)]
    pub(crate) fn preference_save_pending(&self) -> bool {
        self.store.as_ref().is_some_and(PreferenceStore::saving)
    }

    #[cfg(bevy_shell)]
    pub(crate) fn poll_background_save(&mut self) -> PreferenceSaveCompletion {
        let mut completion = PreferenceSaveCompletion::default();
        let mut failure = None;
        if let Some(results) = self.store.as_mut().and_then(PreferenceStore::poll_background_save) {
            if let Some(Ok(Some(restored))) = results.last() {
                completion.before_restore = Some(self.preferences.clone());
                self.accept_replaced(restored.clone());
            }
            failure = results.into_iter().find_map(Result::err);
            self.feedback = tr!("Applied and saved").into();
        }
        self.outgoing_stores.retain_mut(|store| {
            if let Some(results) = store.poll_background_save() {
                let error = results.into_iter().find_map(Result::err);
                failure = failure.take().or(error);
            }
            store.saving()
        });
        if let Some(error) = failure {
            let message = lfmt!(
                "Applied for this session; could not save: {error}",
                error = error
            );
            self.feedback.clone_from(&message);
            completion.error = Some(message);
        }
        completion
    }

    fn capture_hud_layout(&mut self, view: &PlayView) {
        if view.local.hud_edit > self.hud_edit {
            let edit = view.local.hud_edit;
            let mut preferences = self.preferences.clone();
            preferences.hud_layout = view.preferences().hud_layout.clone();
            preferences.hud_bounds = view.preferences().hud_bounds;
            self.apply(preferences);
            self.hud_edit = edit;
        }
    }

    pub fn restore(
        &mut self,
        view: &mut PlayView,
        viewport: [u32; 2],
    ) -> Result<(), PreferenceError> {
        self.capture_hud_layout(view);
        view.local.hud_edit = self.hud_edit;
        if view.local.minimap_edit > self.minimap_edit {
            let edit = view.local.minimap_edit;
            let mut preferences = self.preferences.clone();
            preferences.minimap = view.preferences().minimap.clone();
            self.apply(preferences);
            self.minimap_edit = edit;
        }
        view.local.minimap_edit = self.minimap_edit;
        view.set_world_viewport(viewport)?;
        if view.preferences() != &self.preferences {
            view.set_preferences(self.preferences.clone())?;
        }
        Ok(())
    }

    pub fn capture(&mut self, view: &mut PlayView) -> Result<(), PreferenceError> {
        self.capture_hud_layout(view);
        view.local.hud_edit = self.hud_edit;
        if view.preferences().audio != self.preferences.audio {
            let mut preferences = self.preferences.clone();
            preferences.audio = view.preferences().audio;
            self.apply(preferences);
        }
        // Merge any binding/zoom edit made while the shortcut was being handled.
        if view.preferences() != &self.preferences {
            view.set_preferences(self.preferences.clone())?;
        }
        Ok(())
    }
}
