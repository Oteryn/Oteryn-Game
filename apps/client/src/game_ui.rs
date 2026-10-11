//! In-world UI consumes session projections; it never invents character state.
use egui::{Color32, RichText};
use oteryn_client::action_bar::{ACTION_BAR_ROWS, ActionBar, ActionBarCommand, ActionBarOutcome};
use oteryn_client::layout::{HUD_CHAT_HEIGHT, HUD_MARGIN, HUD_SIDEBAR_WIDTH, HUD_TOP_HEIGHT};
use oteryn_client::minimap::{LoadedMinimap, MINIMAP_SIDE, MinimapZoom};
use oteryn_client::{
    hotkeys::{CustomHotkeyAction, ResolvedHotkey},
    play::{PlayLink, PlayView},
    settings::ClientSettings,
};
use oteryn_session::{
    ChatDisposition, ChatIntent, ChatLine, ChatRoom, ChatSpeechMode, EntityDetail, EntityKind,
    EquipmentSlot, ItemEntry, ItemMoveOutcome, UseDisposition,
};

fn tile_overlay_rect(
    scene: egui::Rect,
    view: &oteryn_renderer::TileView,
    tile: oteryn_renderer::TileCoord,
) -> Option<egui::Rect> {
    if !view.contains(tile) {
        return None;
    }
    let scale = scene.width()
        / (oteryn_client::scene::SCENE_COLUMNS * oteryn_client::scene::SCENE_TILE_PX) as f32;
    let [x, y] = view.tile_to_screen(tile);
    Some(egui::Rect::from_min_size(
        scene.min + egui::vec2(x, y) * scale,
        egui::Vec2::splat(oteryn_client::scene::SCENE_TILE_PX as f32 * scale),
    ))
}

fn scene_setting_overlays(
    ctx: &egui::Context,
    scene: egui::Rect,
    view: &PlayView,
    settings: &ClientSettings,
) {
    if let Some(alpha) = ambient_overlay_alpha(settings) {
        ctx.layer_painter(egui::LayerId::new(
            egui::Order::Background,
            "scene-ambient-light".into(),
        ))
        .rect_filled(scene, 0.0, Color32::from_black_alpha(alpha));
    }
    let painter = ctx
        .layer_painter(egui::LayerId::new(
            egui::Order::Background,
            "scene-setting-overlays".into(),
        ))
        .with_clip_rect(scene);
    if settings.target_highlight_enabled()
        && let Some(target) = view.scene().target()
        && let Some(rect) = tile_overlay_rect(scene, view.scene().view(), target.tile)
    {
        painter.rect_filled(rect, 0.0, Color32::from_rgba_unmultiplied(235, 195, 64, 58));
    }
    if settings.stored_bool("interface.interface.highlight_mouse") == Some(true)
        && let Some(pointer) = ctx.pointer_hover_pos()
        && scene.contains(pointer)
    {
        let scale = scene.width()
            / (oteryn_client::scene::SCENE_COLUMNS * oteryn_client::scene::SCENE_TILE_PX) as f32;
        let local = (pointer - scene.min) / scale;
        if let Some(tile) = view.scene().view().screen_to_tile(local.x, local.y)
            && let Some(rect) = tile_overlay_rect(scene, view.scene().view(), tile)
        {
            painter.rect_stroke(
                rect.shrink(1.0),
                0.0,
                egui::Stroke::new(1.5, Color32::from_rgb(225, 207, 143)),
                egui::StrokeKind::Inside,
            );
        }
    }
}

fn ambient_overlay_alpha(settings: &ClientSettings) -> Option<u8> {
    (settings.stored_bool("effects.effects.lighting") == Some(true))
        .then(|| settings.stored_int("effects.effects.ambient_light"))
        .flatten()
        .map(|ambient| (((100 - ambient.clamp(0, 100)) as f32 / 100.0) * 190.0).round() as u8)
}

fn settings_with_panel_shortcuts(
    pending: Option<ClientSettings>,
    current: &ClientSettings,
    shortcuts: Vec<String>,
) -> ClientSettings {
    let mut next = pending.unwrap_or_else(|| current.clone());
    next.panel_shortcuts = shortcuts;
    next
}

#[derive(Default)]
pub struct GameUi {
    character_name: Option<String>,
    draft: String,
    recipient: String,
    channel: usize,
    chat_read_only: [bool; 6],
    chat_mode: ChatMode,
    chat_focus_request: Option<bool>,
    chat_composer_id: Option<egui::Id>,
    chat_recipient_id: Option<egui::Id>,
    notice: Option<String>,
    action_bar: Option<ActionBar>,
    minimap: Option<(LoadedMinimap, egui::TextureHandle)>,
    minimap_zoom: usize,
    minimap_pan: [i32; 2],
    minimap_retry_at: f64,
    panels: crate::client_panels::ClientPanels,
    dialogs: crate::reference_dialogs::ReferenceDialogs,
    preferences_changes: Option<ClientSettings>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum ChatMode {
    #[default]
    Off,
    On,
    Temporary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PanelHotkeyDestination {
    Toggle(&'static str),
    Open(&'static str),
    OpenTab(&'static str, &'static str),
}

fn panel_hotkey_destination(action: &str) -> Option<PanelHotkeyDestination> {
    Some(match action {
        "window.vip" => PanelHotkeyDestination::Toggle("vip"),
        "window.battle" => PanelHotkeyDestination::Toggle("battle"),
        "window.skills" => PanelHotkeyDestination::Toggle("skills"),
        "window.spell_list" => PanelHotkeyDestination::Toggle("spells"),
        "window.analytics_selector" => PanelHotkeyDestination::Toggle("analytics"),
        "window.bestiary_tracker" => PanelHotkeyDestination::Toggle("bestiary-tracker"),
        "window.bosstiary_tracker" => PanelHotkeyDestination::Toggle("bosstiary-tracker"),
        "window.imbuement_tracker" => PanelHotkeyDestination::Toggle("imbuement-tracker"),
        "window.kill_tracker" => PanelHotkeyDestination::Toggle("kill-tracker"),
        "window.party" => PanelHotkeyDestination::Toggle("party"),
        "window.player_guide" => PanelHotkeyDestination::Toggle("player-guide"),
        "window.quest_tracker" => PanelHotkeyDestination::Toggle("quest-tracker"),
        "dialog.compendium" => PanelHotkeyDestination::Open("compendium"),
        "dialog.quest_log" => PanelHotkeyDestination::Open("quest-log"),
        "dialog.prey" => PanelHotkeyDestination::Open("prey"),
        "dialog.cyclopedia.bestiary" => PanelHotkeyDestination::OpenTab("cyclopedia", "bestiary"),
        "dialog.cyclopedia.character" => PanelHotkeyDestination::OpenTab("cyclopedia", "character"),
        "dialog.cyclopedia.charms" => PanelHotkeyDestination::OpenTab("cyclopedia", "charms"),
        "dialog.cyclopedia.items" => PanelHotkeyDestination::OpenTab("cyclopedia", "items"),
        "dialog.cyclopedia.magical_archive" => {
            PanelHotkeyDestination::OpenTab("cyclopedia", "archive")
        }
        "dialog.cyclopedia.map" => PanelHotkeyDestination::OpenTab("cyclopedia", "map"),
        "dialog.social.assemble_team" => PanelHotkeyDestination::OpenTab("social", "assemble"),
        "dialog.social.badges" => PanelHotkeyDestination::OpenTab("social", "badges"),
        "dialog.social.friend_list" => PanelHotkeyDestination::OpenTab("social", "friends"),
        "dialog.social.friends_config" => {
            PanelHotkeyDestination::OpenTab("social", "friends-config")
        }
        "dialog.social.friend_invitations" => {
            PanelHotkeyDestination::OpenTab("social", "invitations")
        }
        "dialog.social.friend_search" => PanelHotkeyDestination::OpenTab("social", "search"),
        "dialog.social.join_team" => PanelHotkeyDestination::OpenTab("social", "finder"),
        "dialog.exaltation_forge" => PanelHotkeyDestination::Open("forge"),
        "dialog.reward_wall" => PanelHotkeyDestination::Open("reward-wall"),
        "dialog.task_board" => PanelHotkeyDestination::Open("task-board"),
        "dialog.weapon_proficiency" => PanelHotkeyDestination::Open("weapon-proficiency"),
        "dialog.wheel_of_destiny" => PanelHotkeyDestination::Open("wheel"),
        _ => return None,
    })
}

const fn custom_text_action_available(
    send_automatically: bool,
    chat_visible: bool,
    transport_available: bool,
) -> bool {
    transport_available && (send_automatically || chat_visible)
}

impl GameUi {
    pub fn set_character_name(&mut self, name: String) {
        self.character_name = Some(name);
    }

    pub fn blocks_game_input(&self) -> bool {
        self.panels.manage || self.dialogs.any_open()
    }

    /// The hotkey context is owned by the in-game chat state, rather than by
    /// whatever widget happened to report keyboard focus for the last frame.
    #[must_use]
    pub fn chat_hotkey_context(&self) -> bool {
        self.chat_mode != ChatMode::Off
    }

    /// Chat-mode bindings own native keyboard routing only while the chat
    /// composer (or its private-recipient field) owns focus. An unrelated
    /// text editor must remain an egui/modal input even when Chat On remains
    /// selected in the console.
    #[must_use]
    pub fn chat_routes_keyboard(&self, ctx: &egui::Context) -> bool {
        if !self.chat_hotkey_context() {
            return false;
        }
        ctx.memory(|memory| {
            memory.focused().is_none_or(|focused| {
                self.chat_composer_id == Some(focused) || self.chat_recipient_id == Some(focused)
            })
        })
    }

    fn chat_mode_label(&self, english: bool) -> &'static str {
        match (self.chat_mode, english) {
            (ChatMode::Off, true) => "Chat Off",
            (ChatMode::Off, false) => "Czat wyłączony",
            (ChatMode::On, true) => "Chat On",
            (ChatMode::On, false) => "Czat włączony",
            (ChatMode::Temporary, true) => "Chat On*",
            (ChatMode::Temporary, false) => "Czat włączony*",
        }
    }

    /// Apply a chat focus transition after the native router has claimed the
    /// activating key, but before a later native input event can be queued.
    pub fn apply_chat_focus_transition(&mut self, ctx: &egui::Context) {
        let Some(focus) = self.chat_focus_request else {
            return;
        };
        if focus {
            let Some(id) = self.chat_composer_id else {
                return;
            };
            ctx.memory_mut(|memory| memory.request_focus(id));
        } else {
            ctx.memory_mut(|memory| {
                if let Some(id) = self.chat_composer_id {
                    memory.surrender_focus(id);
                }
                if let Some(id) = self.chat_recipient_id {
                    memory.surrender_focus(id);
                }
            });
        }
        self.chat_focus_request = None;
    }

    fn set_chat_mode(&mut self, mode: ChatMode) {
        self.chat_mode = mode;
        self.chat_focus_request = Some(mode != ChatMode::Off);
    }

    fn route_chat_mode_hotkey(
        &mut self,
        action: &str,
        composer_available: bool,
        english: bool,
    ) -> bool {
        let mode = match action {
            "chat.mode.off" => {
                self.set_chat_mode(ChatMode::Off);
                return true;
            }
            "chat.mode.on" => ChatMode::On,
            "chat.mode.temporary" => ChatMode::Temporary,
            _ => return false,
        };
        if !self.chat_composer_ready(composer_available, english) {
            return true;
        }
        self.set_chat_mode(mode);
        true
    }

    fn chat_composer_ready(&mut self, available: bool, english: bool) -> bool {
        if !available {
            if self.chat_hotkey_context() {
                self.set_chat_mode(ChatMode::Off);
            }
            self.notice = Some(chat_feedback(ChatDisposition::ChatUnavailable, english).into());
            return false;
        }
        if self
            .chat_read_only
            .get(self.channel)
            .copied()
            .unwrap_or(true)
        {
            if self.chat_hotkey_context() {
                self.set_chat_mode(ChatMode::Off);
            }
            self.notice = Some(
                if english {
                    "This tab is read-only."
                } else {
                    "Ta karta jest tylko do odczytu."
                }
                .into(),
            );
            return false;
        }
        true
    }

    fn place_chat_text(&mut self, text: &str, available: bool, english: bool) -> bool {
        if !self.chat_composer_ready(available, english) {
            return false;
        }
        self.draft = text.to_owned();
        if self.chat_mode == ChatMode::Off {
            self.set_chat_mode(ChatMode::Temporary);
        } else {
            self.chat_focus_request = Some(true);
        }
        true
    }

    fn apply_chat_composer_focus(&mut self, edit: &egui::Response, keep_focused: bool) {
        self.chat_composer_id = Some(edit.id);
        match self.chat_focus_request.take() {
            Some(true) => edit.request_focus(),
            Some(false) => edit.surrender_focus(),
            None => {}
        }
        if edit.gained_focus() && self.chat_mode == ChatMode::Off {
            self.chat_mode = ChatMode::On;
        }
        let unrelated_widget_has_focus = edit.ctx.memory(|memory| {
            memory.focused().is_some_and(|focused| {
                self.chat_composer_id != Some(focused) && self.chat_recipient_id != Some(focused)
            })
        });
        if keep_focused
            && self.chat_hotkey_context()
            && !edit.has_focus()
            && !unrelated_widget_has_focus
        {
            edit.request_focus();
        }
    }

    fn observe_chat_text_focus(&mut self, edit: &egui::Response) {
        self.chat_recipient_id = Some(edit.id);
        if edit.gained_focus() && self.chat_mode == ChatMode::Off {
            self.chat_mode = ChatMode::On;
        }
    }

    fn relinquish_hidden_chat_focus(&mut self, ctx: &egui::Context) {
        self.set_chat_mode(ChatMode::Off);
        self.chat_focus_request = None;
        if let Some(id) = self.chat_composer_id {
            ctx.memory_mut(|memory| memory.surrender_focus(id));
        }
        if let Some(id) = self.chat_recipient_id {
            ctx.memory_mut(|memory| memory.surrender_focus(id));
        }
    }

    pub fn preferences_save_failed(&mut self, english: bool) {
        self.notice = Some(
            if english {
                "Preferences were not saved. Check access to the preferences directory."
            } else {
                "Nie zapisano ustawień. Sprawdź dostęp do folderu ustawień."
            }
            .into(),
        );
    }

    pub fn screenshot_finished(&mut self, saved: bool, english: bool) {
        self.notice = Some(
            match (saved, english) {
                (true, true) => "Screenshot saved.",
                (true, false) => "Zapisano zrzut ekranu.",
                (false, true) => "Screenshot could not be saved.",
                (false, false) => "Nie udało się zapisać zrzutu ekranu.",
            }
            .into(),
        );
    }
    pub fn take_preferences(&mut self) -> Option<ClientSettings> {
        self.preferences_changes.take()
    }

    pub fn route_chat_hotkey(
        &mut self,
        action: &str,
        link: &PlayLink,
        english: bool,
        chat_visible: bool,
    ) -> bool {
        let rooms = link.game_state().chat.map(|chat| chat.rooms());
        if self.route_chat_mode_hotkey(action, chat_visible && rooms.is_some(), english) {
            return true;
        }
        match action {
            "chat.next" | "chat.previous" => {
                if let Some(rooms) = rooms {
                    self.channel = cycle_chat_channel(self.channel, rooms, action == "chat.next");
                }
            }
            "chat.local" => self.channel = 0,
            "chat.help" => {
                self.channel = 3;
                match rooms {
                    Some(rooms) if !rooms.contains(ChatRoom::Help) => {
                        self.notice = if link.send_chat(ChatIntent::OpenRoom(ChatRoom::Help)) {
                            None
                        } else {
                            Some(action_busy(english).into())
                        };
                    }
                    Some(_) => self.notice = None,
                    None => {
                        self.notice =
                            Some(chat_feedback(ChatDisposition::ChatUnavailable, english).into());
                    }
                }
            }
            "chat.close" => {
                let room = self
                    .channel
                    .checked_sub(1)
                    .and_then(|index| ChatRoom::ALL.get(index).copied());
                if let Some(room) = room
                    && rooms.is_some_and(|rooms| rooms.contains(room))
                {
                    if link.send_chat(ChatIntent::CloseRoom(room)) {
                        self.channel = 0;
                        self.notice = None;
                    } else {
                        self.notice = Some(action_busy(english).into());
                    }
                }
            }
            "chat.list" => self.dialogs.open_chat_channels(),
            "chat.send" => {
                let available = chat_visible && rooms.is_some();
                self.submit_current_chat(available, english, |intent| link.send_chat(intent));
                if self.chat_hotkey_context() {
                    self.chat_focus_request = Some(true);
                }
            }
            _ => return false,
        }
        true
    }

    fn submit_current_chat(
        &mut self,
        available: bool,
        english: bool,
        send: impl FnOnce(ChatIntent) -> bool,
    ) -> bool {
        let text = self.draft.trim().to_owned();
        if !self.submit_chat_text(text, available, english, send) {
            return false;
        }
        self.draft.clear();
        self.complete_chat_send();
        true
    }

    fn submit_chat_text(
        &mut self,
        text: String,
        available: bool,
        english: bool,
        send: impl FnOnce(ChatIntent) -> bool,
    ) -> bool {
        let tr = |pl, en| if english { en } else { pl };
        if !self.chat_composer_ready(available, english) {
            return false;
        }
        let text = text.trim().to_owned();
        if text.is_empty() {
            return false;
        }
        if text.len() > oteryn_session::MAX_CHAT_TEXT_BYTES {
            self.notice = Some(tr("Wiadomość jest zbyt długa", "Message is too long").into());
            return false;
        }
        let Some(intent) = chat_intent(self.channel, &self.recipient, text) else {
            self.notice = Some(tr("Podaj prawidłowego odbiorcę", "Enter a valid recipient").into());
            return false;
        };
        if !send(intent) {
            self.notice = Some(action_busy(english).into());
            return false;
        }
        self.notice = None;
        true
    }

    fn complete_chat_send(&mut self) {
        match self.chat_mode {
            ChatMode::Temporary => self.set_chat_mode(ChatMode::Off),
            ChatMode::On => self.chat_focus_request = Some(true),
            ChatMode::Off => {}
        }
    }

    pub fn route_panel_hotkey(&mut self, action: &str) -> bool {
        match panel_hotkey_destination(action) {
            Some(PanelHotkeyDestination::Toggle(panel)) => self.panels.toggle(panel),
            Some(PanelHotkeyDestination::Open(panel)) => self.panels.open(panel),
            Some(PanelHotkeyDestination::OpenTab(panel, tab)) => self.panels.open_tab(panel, tab),
            None => false,
        }
    }

    pub(crate) fn route_settings_action(
        &mut self,
        action: crate::settings_ui::SettingsRuntimeAction,
    ) -> bool {
        match action {
            crate::settings_ui::SettingsRuntimeAction::OpenCompendium => {
                self.panels.open("compendium")
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn is_panel_open(&self, id: &str) -> bool {
        self.panels.is_open(id)
    }

    pub fn route_minimap_hotkey(&mut self, action: &str) -> bool {
        match action {
            "minimap.zoom_out" => self.minimap_zoom = (self.minimap_zoom + 1).min(2),
            "minimap.zoom_in" => self.minimap_zoom = self.minimap_zoom.saturating_sub(1),
            "minimap.north" => self.minimap_pan[1] = (self.minimap_pan[1] - 1).max(-16),
            "minimap.east" => self.minimap_pan[0] = (self.minimap_pan[0] + 1).min(16),
            "minimap.south" => self.minimap_pan[1] = (self.minimap_pan[1] + 1).min(16),
            "minimap.west" => self.minimap_pan[0] = (self.minimap_pan[0] - 1).max(-16),
            "minimap.center" => self.minimap_pan = [0, 0],
            _ => return false,
        }
        true
    }
    pub fn clear_action_row(&mut self, row: usize, settings: &ClientSettings) {
        let cleared = self
            .bar(settings)
            .is_some_and(|bar| bar.row_has_assignments(row) && bar.clear_row(row).is_ok());
        if !cleared {
            self.notice = Some(
                if settings.english {
                    "This action row is locked or unavailable."
                } else {
                    "Ten pasek akcji jest zablokowany lub niedostępny."
                }
                .into(),
            );
        }
    }

    #[must_use]
    pub fn action_bar_rows_with_assignments(&self) -> [bool; ACTION_BAR_ROWS] {
        std::array::from_fn(|row| {
            self.action_bar
                .as_ref()
                .is_some_and(|bar| bar.row_has_assignments(row))
        })
    }
    fn bar(&mut self, settings: &ClientSettings) -> Option<&mut ActionBar> {
        let fallback_movement_keys = settings
            .hotkeys
            .active_movement_fallback_keys(&settings.movement_keys);
        if self.action_bar.is_none() {
            self.action_bar =
                ActionBar::new(settings.action_bar.clone(), &fallback_movement_keys).ok();
        }
        let bar = self.action_bar.as_mut()?;
        if bar.preferences() != &settings.action_bar {
            let _ = bar.reconfigure(settings.action_bar.clone(), &fallback_movement_keys);
        }
        Some(bar)
    }

    pub fn route_actions(
        &mut self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        hotkeys: &[ResolvedHotkey],
        link: &PlayLink,
        settings: &ClientSettings,
        chat_on: bool,
        modal: bool,
    ) -> bool {
        let mut matched = false;
        if !modal {
            for hotkey in hotkeys {
                let action = match hotkey {
                    ResolvedHotkey::General(action) if action.starts_with("chat.") => {
                        matched |= self.route_chat_hotkey(
                            action,
                            link,
                            settings.english,
                            settings.show_chat,
                        );
                        continue;
                    }
                    ResolvedHotkey::General(action)
                        if action.starts_with("window.") || action.starts_with("dialog.") =>
                    {
                        matched |= self.route_panel_hotkey(action);
                        continue;
                    }
                    ResolvedHotkey::General(action) if action.starts_with("minimap.") => {
                        matched |= self.route_minimap_hotkey(action);
                        continue;
                    }
                    ResolvedHotkey::General(_) => continue,
                    ResolvedHotkey::Custom(action) => action,
                };
                matched = true;
                match action {
                    CustomHotkeyAction::Text {
                        text,
                        send_automatically,
                    } => {
                        let available = custom_text_action_available(
                            *send_automatically,
                            settings.show_chat,
                            link.game_state().chat.is_some(),
                        );
                        if !*send_automatically {
                            let _ = self.place_chat_text(text, available, settings.english);
                        } else {
                            let accepted = self.submit_chat_text(
                                text.clone(),
                                available,
                                settings.english,
                                |intent| link.send_chat(intent),
                            );
                            if accepted {
                                self.complete_chat_send();
                            }
                            if self.chat_hotkey_context() {
                                self.chat_focus_request = Some(true);
                            }
                        }
                    }
                    CustomHotkeyAction::Spell { .. } | CustomHotkeyAction::Object { .. } => {
                        self.notice = Some(
                            if settings.english {
                                "This custom action has no authoritative session assignment."
                            } else {
                                "Ta własna czynność nie ma autorytatywnego przypisania sesji."
                            }
                            .into(),
                        );
                    }
                }
            }
        }
        if let Some(bar) = self.bar(settings) {
            if bar.set_input_context(chat_on, modal).is_err() {
                return matched;
            }
            let (commands, action_bar_matched) = bar.route_with_match(events);
            matched |= action_bar_matched;
            for command in commands {
                if !link.send_action(command) {
                    self.notice = Some(action_busy(settings.english).into());
                }
            }
        }
        matched
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        link: &PlayLink,
        view: &PlayView,
        settings: &ClientSettings,
    ) -> bool {
        let state = link.game_state();
        let rect = ctx.content_rect();
        let en = settings.english;
        let tr = |pl, english| if en { english } else { pl };
        if !settings.show_chat || state.chat.is_none() {
            self.relinquish_hidden_chat_focus(ctx);
        }
        let mut open_settings = false;
        let hud_enabled = !self.blocks_game_input();
        self.panels.initialize(&settings.panel_shortcuts);
        let now = ctx.input(|input| input.time);
        if settings.show_minimap && self.minimap.is_none() && now >= self.minimap_retry_at {
            self.minimap_retry_at = now + 1.0;
            if let Some(map) = view.minimap_image() {
                let pixels = map
                    .pixels()
                    .iter()
                    .map(|p| Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                    .collect();
                let texture = ctx.load_texture(
                    "loaded-minimap",
                    egui::ColorImage::new([usize::from(MINIMAP_SIDE); 2], pixels),
                    egui::TextureOptions::NEAREST,
                );
                self.minimap = Some((map, texture));
            }
        }
        let chat_height = if settings.show_chat {
            HUD_CHAT_HEIGHT
        } else {
            HUD_MARGIN
        };
        let body = egui::Rect::from_min_max(
            egui::pos2(rect.left() + HUD_MARGIN, rect.top() + HUD_TOP_HEIGHT),
            egui::pos2(
                (rect.right() - HUD_SIDEBAR_WIDTH).max(rect.left() + HUD_MARGIN + 1.0),
                (rect.bottom() - chat_height).max(rect.top() + HUD_TOP_HEIGHT + 1.0),
            ),
        );
        let usable_scene = oteryn_client::layout::GameViewport::fit_with_action_rows(
            rect.width(),
            rect.height(),
            1.0,
            settings.show_chat,
            settings.action_bar.visible_rows(),
        )
        .is_some();
        if !usable_scene {
            // Oversized underlying HUD Areas must not occlude the recovery button.
            return show_window_recovery(ctx, en);
        }
        if let Some(viewport) = oteryn_client::layout::GameViewport::fit_with_action_rows(
            rect.width(),
            rect.height(),
            1.0,
            settings.show_chat,
            settings.action_bar.visible_rows(),
        ) {
            let scene = egui::Rect::from_min_size(
                egui::pos2(viewport.x, viewport.y),
                egui::vec2(viewport.width, viewport.height),
            );
            scene_setting_overlays(ctx, scene, view, settings);
            crate::actor_hud::show(
                ctx,
                scene,
                view.scene().view().tile_to_screen(view.own()),
                settings,
                state.vitals,
                self.character_name.as_deref(),
            );
            crate::actor_hud::show_other_health(
                ctx,
                scene,
                view.scene().view(),
                view.own(),
                view.floor(),
                settings,
                &state.entities,
            );
        }
        let chat_context = self.chat_hotkey_context();
        let (commands, lock_change) = self.bar(settings).map_or_else(
            || (Vec::new(), None),
            |bar| {
                // Keep both activation and the visible shortcut labels in sync
                // even when the console toggle changed without a native key event.
                let _ = bar.set_input_context(chat_context, !hud_enabled);
                // Available spells need an authoritative catalogue projection.
                let commands =
                    crate::action_bar_ui::show(ctx, bar, &state, &[], settings, body, hud_enabled);
                (commands, bar.take_assignments_lock_change())
            },
        );
        if let Some(locked) = lock_change {
            let mut preferences = self
                .preferences_changes
                .take()
                .unwrap_or_else(|| settings.clone());
            preferences.action_bar.set_assignments_locked(locked);
            self.preferences_changes = Some(preferences);
        }
        for command in commands {
            if !link.send_action(command) {
                self.notice = Some(action_busy(en).into());
            }
        }
        egui::Area::new("game-header".into())
            .enabled(hud_enabled)
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                ui.set_width((rect.width() - HUD_MARGIN * 2.0).max(1.0));
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 1.0);
                ui.horizontal(|ui| {
                    let width = ((ui.available_width() - 2.0) * 0.5).max(1.0);
                    for (label, values, color) in [
                        (
                            "HP",
                            state.vitals.map(|v| (v.health, v.max_health)),
                            Color32::from_rgb(56, 170, 53),
                        ),
                        (
                            "MP",
                            state.vitals.map(|v| (v.mana, v.max_mana)),
                            Color32::from_rgb(54, 112, 189),
                        ),
                    ] {
                        if let Some((value, maximum)) = values {
                            ui.add(
                                egui::ProgressBar::new(ratio(value, maximum))
                                    .desired_width(width)
                                    .desired_height(11.0)
                                    .fill(color)
                                    .text(format!("{value}/{maximum}")),
                            );
                        } else {
                            let (track, _) = ui
                                .allocate_exact_size(egui::vec2(width, 11.0), egui::Sense::hover());
                            crate::client_chrome::surface(ui, track, true);
                            ui.painter().text(
                                track.center(),
                                egui::Align2::CENTER_CENTER,
                                format!("{label} —"),
                                egui::FontId::proportional(9.0),
                                ui.visuals().text_color(),
                            );
                        }
                    }
                });
                ui.horizontal(|ui| {
                    if let Some(vitals) = state.vitals {
                        ui.small(format!("Soul {} · Harmony {}", vitals.soul, vitals.harmony));
                    }
                    if settings.stored_bool("graphics.graphics.fps_indicator") == Some(true) {
                        let fps = ctx.input(|input| {
                            if input.stable_dt > 0.0 {
                                1.0 / input.stable_dt
                            } else {
                                0.0
                            }
                        });
                        ui.small(format!("{fps:.0} FPS"));
                    }
                    if ui.small_button(tr("Ustawienia", "Settings")).clicked() {
                        open_settings = true;
                    }
                    if ui
                        .small_button("+")
                        .on_hover_text(tr("Panele i skróty", "Panels and shortcuts"))
                        .clicked()
                    {
                        self.panels.manage = true;
                    }
                    if ui.small_button(tr("Więcej", "More")).clicked() {
                        self.dialogs.manage = true;
                    }
                });
            });
        egui::Area::new("game-sidebar".into())
            .enabled(hud_enabled)
            .fixed_pos(egui::pos2(
                rect.right() - HUD_SIDEBAR_WIDTH,
                rect.top() + HUD_TOP_HEIGHT,
            ))
            .default_size([
                HUD_SIDEBAR_WIDTH - HUD_MARGIN,
                (rect.height() - HUD_TOP_HEIGHT - HUD_MARGIN).max(10.0),
            ])
            .show(ctx, |ui| {
                ui.set_height((rect.height() - HUD_TOP_HEIGHT - HUD_MARGIN).max(10.0));
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                    ui.set_width((HUD_SIDEBAR_WIDTH - 12.0).max(1.0));
                    egui::ScrollArea::vertical()
                        .scroll_bar_visibility(
                            egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                        )
                        .max_height((rect.height() - HUD_TOP_HEIGHT - 12.0).max(10.0))
                        .show(ui, |ui| {
                            if settings.show_minimap {
                                ui.horizontal(|ui| {
                                    ui.small(tr("Minimapa", "Minimap"));
                                    ui.menu_button("▾", |ui| {
                                        for (index, label) in [
                                            tr("Blisko", "Nearby"),
                                            tr("Okolica", "Region"),
                                            tr("Wczytany obszar", "Loaded area"),
                                        ]
                                        .into_iter()
                                        .enumerate()
                                        {
                                            if ui
                                                .selectable_value(
                                                    &mut self.minimap_zoom,
                                                    index,
                                                    label,
                                                )
                                                .clicked()
                                            {
                                                ui.close();
                                            }
                                        }
                                    });
                                    if ui
                                        .small_button("−")
                                        .on_hover_text(tr("Oddal", "Zoom out"))
                                        .clicked()
                                    {
                                        self.minimap_zoom = (self.minimap_zoom + 1).min(2);
                                    }
                                    if ui
                                        .small_button("+")
                                        .on_hover_text(tr("Przybliż", "Zoom in"))
                                        .clicked()
                                    {
                                        self.minimap_zoom = self.minimap_zoom.saturating_sub(1);
                                    }
                                });
                                let zoom = [
                                    MinimapZoom::Nearby,
                                    MinimapZoom::Region,
                                    MinimapZoom::LoadedArea,
                                ][self.minimap_zoom];
                                if let Some((map, texture)) = &self.minimap
                                    && let Some(crop) = map.shifted_viewport(
                                        view.minimap_location(),
                                        oteryn_client::world::START_FLOOR,
                                        zoom,
                                        self.minimap_pan,
                                    )
                                {
                                    let side = f32::from(MINIMAP_SIDE);
                                    let uv = egui::Rect::from_min_max(
                                        egui::pos2(
                                            f32::from(crop.origin[0]) / side,
                                            f32::from(crop.origin[1]) / side,
                                        ),
                                        egui::pos2(
                                            f32::from(crop.origin[0] + crop.size[0]) / side,
                                            f32::from(crop.origin[1] + crop.size[1]) / side,
                                        ),
                                    );
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(112.0, 112.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().image(texture.id(), rect, uv, Color32::WHITE);
                                    let marker = rect.min
                                        + egui::vec2(
                                            (f32::from(crop.player[0]) + 0.5)
                                                / f32::from(crop.size[0])
                                                * rect.width(),
                                            (f32::from(crop.player[1]) + 0.5)
                                                / f32::from(crop.size[1])
                                                * rect.height(),
                                        );
                                    ui.painter().circle_filled(
                                        marker,
                                        3.0,
                                        Color32::from_rgb(245, 90, 85),
                                    );
                                } else {
                                    ui.small(tr(
                                        "Mapa tej okolicy jest niedostępna",
                                        "Map of this area is unavailable",
                                    ));
                                }
                                ui.separator();
                            }
                            self.panels.shortcuts(ui, en);
                            ui.separator();
                            if settings.show_inventory {
                                equipment(ui, state.inventory.as_ref(), state.vitals, link, en);
                                if let Some(inventory) = &state.inventory {
                                    ui.separator();
                                    panel_heading(ui, tr("Plecak", "Backpack"));
                                    if inventory.entries.is_empty() {
                                        ui.small(tr("Pusty", "Empty"));
                                    }
                                    item_grid(ui, "main-backpack", &inventory.entries, link, en);
                                } else {
                                    ui.small(tr(
                                        "Brak danych ekwipunku",
                                        "Inventory data unavailable",
                                    ));
                                }
                                if let Some(container) = &state.container
                                    && container.container_handle.is_some()
                                {
                                    ui.separator();
                                    panel_heading(ui, tr("Otwarty kontener", "Open container"));
                                    item_grid(ui, "open-container", &container.entries, link, en);
                                }
                            }
                            if settings.show_battle {
                                ui.separator();
                                panel_heading(ui, tr("Lista walki", "Battle list"));
                                for entity in &state.entities {
                                    if let EntityDetail::Actor { health_percent, .. } =
                                        entity.detail
                                    {
                                        ui.small(actor_kind(entity.kind, en)).on_hover_text(
                                            format!("{}, {}", entity.position.x, entity.position.y),
                                        );
                                        ui.add(
                                            egui::ProgressBar::new(
                                                f32::from(health_percent) / 100.0,
                                            )
                                            .desired_width(ui.available_width())
                                            .desired_height(4.0)
                                            .fill(Color32::from_rgb(55, 172, 47)),
                                        );
                                    }
                                }
                            }
                        });
                });
            });
        if settings.show_chat {
            egui::Area::new("game-console".into())
                .enabled(hud_enabled)
                .fixed_pos(egui::pos2(rect.left(), rect.bottom() - HUD_CHAT_HEIGHT))
                .show(ctx, |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.set_width(
                            (rect.width() - HUD_SIDEBAR_WIDTH - HUD_MARGIN * 2.0).max(100.0),
                        );
                        ui.set_max_height(HUD_CHAT_HEIGHT - 4.0);
                        ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                        ui.set_clip_rect(ui.clip_rect().intersect(egui::Rect::from_min_max(
                            egui::pos2(rect.left(), rect.bottom() - HUD_CHAT_HEIGHT),
                            egui::pos2(rect.right() - HUD_SIDEBAR_WIDTH, rect.bottom()),
                        )));
                        let Some(chat) = &state.chat else {
                            ui.horizontal_wrapped(|ui| {
                                chat_channel_tabs(ui, &mut self.channel, en, false);
                            });
                            ui.separator();
                            ui.allocate_ui(
                                egui::vec2(
                                    ui.available_width(),
                                    (ui.available_height() - ui.spacing().interact_size.y - 12.0)
                                        .max(1.0),
                                ),
                                |ui| {
                                    ui.weak(tr(
                                        "Kanały czatu są niedostępne w tej sesji.",
                                        "Chat channels are unavailable in this session.",
                                    ));
                                },
                            );
                            ui.horizontal(|ui| {
                                ui.add_enabled(
                                    false,
                                    egui::Button::new(tr("Czat wyłączony", "Chat Off")),
                                );
                                ui.add_enabled(
                                    false,
                                    egui::TextEdit::singleline(&mut self.draft)
                                        .hint_text(tr("Wiadomość", "Message"))
                                        .desired_width((ui.available_width() - 80.0).max(50.0)),
                                );
                                ui.add_enabled(false, egui::Button::new(tr("Wyślij", "Send")));
                            });
                            return;
                        };
                        ui.horizontal_wrapped(|ui| {
                            chat_channel_tabs(ui, &mut self.channel, en, true);
                            let contents = chat_contents(chat, self.channel, en);
                            ui.menu_button("▾", |ui| {
                                ui.checkbox(
                                    &mut self.chat_read_only[self.channel],
                                    tr("Tylko do odczytu", "Read-only"),
                                );
                                if ui
                                    .add_enabled(
                                        !contents.is_empty(),
                                        egui::Button::new(tr("Kopiuj treść", "Copy contents")),
                                    )
                                    .clicked()
                                {
                                    ui.ctx().copy_text(contents.clone());
                                    ui.close();
                                }
                                if ui
                                    .add_enabled(
                                        !contents.is_empty(),
                                        egui::Button::new(tr("Zapisz treść", "Save contents")),
                                    )
                                    .clicked()
                                {
                                    self.notice =
                                        Some(match export_chat_contents(self.channel, &contents) {
                                            Ok(path) if en => {
                                                format!("Chat exported to {}", path.display())
                                            }
                                            Ok(path) => {
                                                format!("Wyeksportowano czat do {}", path.display())
                                            }
                                            Err(_) if en => "Chat was not exported.".into(),
                                            Err(_) => "Nie wyeksportowano czatu.".into(),
                                        });
                                    ui.close();
                                }
                            });
                        });
                        let room = self
                            .channel
                            .checked_sub(1)
                            .and_then(|index| ChatRoom::ALL.get(index).copied());
                        if let Some(room) = room
                            && !chat.rooms().contains(room)
                            && ui.button(tr("Otwórz kanał", "Open channel")).clicked()
                        {
                            let _ = link.send_chat(ChatIntent::OpenRoom(room));
                        }
                        egui::ScrollArea::vertical()
                            .max_height((ui.available_height() - 36.0).max(12.0))
                            .stick_to_bottom(true)
                            .show(ui, |ui| {
                                for line in chat.lines() {
                                    if let Some(text) = chat_line_text(line, self.channel, en) {
                                        if let Some(size) = settings
                                            .stored_int("console.console.font_size")
                                            .map(|size| size.clamp(10, 32) as f32)
                                        {
                                            ui.label(RichText::new(text).size(size));
                                        } else {
                                            ui.label(text);
                                        }
                                    }
                                }
                            });
                        ui.horizontal(|ui| {
                            let read_only = self.chat_read_only[self.channel];
                            if read_only && self.chat_hotkey_context() {
                                self.set_chat_mode(ChatMode::Off);
                            }
                            let mode_label = self.chat_mode_label(en);
                            if ui
                                .add_enabled(!read_only, egui::Button::new(mode_label))
                                .clicked()
                            {
                                let action = if self.chat_hotkey_context() {
                                    "chat.mode.off"
                                } else {
                                    "chat.mode.on"
                                };
                                let _ = self.route_chat_mode_hotkey(action, true, en);
                            }
                            let mut alternate_chat_input_focused = false;
                            if self.channel == 5 {
                                let recipient = ui.add_enabled(
                                    !read_only,
                                    egui::TextEdit::singleline(&mut self.recipient)
                                        .desired_width(100.0)
                                        .hint_text(tr("Odbiorca", "Recipient")),
                                );
                                self.observe_chat_text_focus(&recipient);
                                alternate_chat_input_focused = recipient.has_focus();
                            }
                            let edit = ui.add_enabled(
                                !read_only,
                                egui::TextEdit::singleline(&mut self.draft)
                                    .desired_width((ui.available_width() - 65.0).max(30.0))
                                    .hint_text(tr("Napisz wiadomość", "Write a message")),
                            );
                            let legacy_enter = settings.hotkeys.active().is_none_or(|profile| {
                                !profile.general_chat_on.contains_key("chat.send")
                            }) && edit.lost_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::Enter));
                            let submit = ui
                                .add_enabled(!read_only, egui::Button::new(tr("Wyślij", "Send")))
                                .clicked()
                                || !read_only && legacy_enter;
                            if submit {
                                let accepted = self
                                    .submit_current_chat(true, en, |intent| link.send_chat(intent));
                                if !accepted && self.chat_hotkey_context() {
                                    self.chat_focus_request = Some(true);
                                }
                            }
                            self.apply_chat_composer_focus(
                                &edit,
                                hud_enabled && !alternate_chat_input_focused,
                            );
                        });
                        if self.chat_read_only[self.channel] {
                            ui.small(tr(
                                "Ta karta jest tylko do odczytu.",
                                "This tab is read-only.",
                            ));
                        }
                        if let Some(result) = link.chat_result()
                            && result.disposition != oteryn_session::ChatDisposition::Ok
                        {
                            ui.small(format!(
                                "{} · {} s",
                                chat_feedback(result.disposition, en),
                                result.wait_seconds
                            ));
                        }
                    });
                });
        }
        egui::Area::new("game-feedback".into())
            .anchor(egui::Align2::LEFT_TOP, [HUD_MARGIN, HUD_TOP_HEIGHT])
            .show(ctx, |ui| {
                if let Some(notice) = &self.notice {
                    ui.small(notice);
                }
                if let Some(result) = link.action_result() {
                    let feedback = match result {
                        ActionBarOutcome::Cast(result) => {
                            oteryn_client::spell::feedback_text(result.disposition).to_owned()
                        }
                        ActionBarOutcome::Used(result) => {
                            use_feedback(result.disposition, en).to_owned()
                        }
                        ActionBarOutcome::Moved(result) => {
                            move_feedback(result.outcome, en).to_owned()
                        }
                        ActionBarOutcome::Chat(result) => {
                            chat_feedback(result.disposition, en).to_owned()
                        }
                        ActionBarOutcome::Unavailable => tr(
                            "Ta akcja jest niedostępna w tej sesji",
                            "This action is unavailable in this session",
                        )
                        .into(),
                    };
                    ui.small(feedback);
                }
            });
        self.panels.show(ctx, &state, en);
        self.dialogs.set_session_views(&state);
        self.dialogs.show(ctx, en);
        if let Some(intent) = self.dialogs.take_chat_intent() {
            let _ = link.send_chat(intent);
        }
        if let Some(shortcuts) = self.panels.take_shortcuts() {
            self.preferences_changes = Some(settings_with_panel_shortcuts(
                self.preferences_changes.take(),
                settings,
                shortcuts,
            ));
        }
        open_settings
    }
}

fn chat_intent(channel: usize, recipient: &str, text: String) -> Option<ChatIntent> {
    if text.is_empty() || text.len() > oteryn_session::MAX_CHAT_TEXT_BYTES {
        return None;
    }
    if channel == 5 {
        let recipient = recipient.trim();
        if recipient.is_empty() || recipient.len() > oteryn_session::MAX_CHAT_NAME_BYTES {
            return None;
        }
        return Some(ChatIntent::Private {
            recipient_name: recipient.into(),
            text,
        });
    }
    let room = channel
        .checked_sub(1)
        .and_then(|index| ChatRoom::ALL.get(index).copied());
    Some(if let Some(room) = room {
        ChatIntent::Room { room, text }
    } else {
        ChatIntent::Say {
            mode: ChatSpeechMode::Say,
            text,
        }
    })
}

fn cycle_chat_channel(current: usize, rooms: oteryn_session::ChatRoomSet, forward: bool) -> usize {
    let mut available = vec![0];
    available.extend(
        ChatRoom::ALL
            .iter()
            .enumerate()
            .filter_map(|(index, room)| rooms.contains(*room).then_some(index + 1)),
    );
    available.push(5);
    let current = available
        .iter()
        .position(|channel| *channel == current)
        .unwrap_or(0);
    let next = if forward {
        (current + 1) % available.len()
    } else {
        current.checked_sub(1).unwrap_or(available.len() - 1)
    };
    available[next]
}

fn chat_channel_tabs(ui: &mut egui::Ui, channel: &mut usize, english: bool, available: bool) {
    let labels = if english {
        [
            "Local",
            "World",
            "English",
            "Help",
            "Advertising",
            "Private",
        ]
    } else {
        [
            "Lokalny",
            "World",
            "English",
            "Help",
            "Advertising",
            "Prywatny",
        ]
    };
    ui.add_enabled_ui(available, |ui| {
        for (index, label) in labels.into_iter().enumerate() {
            ui.selectable_value(channel, index, label);
        }
    });
}

fn chat_contents(chat: &oteryn_session::ChatLog, channel: usize, english: bool) -> String {
    chat.lines()
        .filter_map(|line| chat_line_text(line, channel, english))
        .collect::<Vec<_>>()
        .join("\n")
}

fn chat_line_text(line: &ChatLine, channel: usize, english: bool) -> Option<String> {
    let room = channel
        .checked_sub(1)
        .and_then(|index| ChatRoom::ALL.get(index).copied());
    match line {
        ChatLine::Local {
            speaker_name, text, ..
        } if channel == 0 => Some(format!("{speaker_name}: {text}")),
        ChatLine::Private { speaker_name, text } if channel == 5 => {
            Some(format!("{speaker_name}: {text}"))
        }
        ChatLine::Room {
            room: line_room,
            speaker_name,
            text,
        } if room == Some(*line_room) => Some(format!("{speaker_name}: {text}")),
        ChatLine::Dropped => Some(
            if english {
                "[Older messages were dropped]"
            } else {
                "[Pominięto starsze wiadomości]"
            }
            .into(),
        ),
        _ => None,
    }
}

pub(crate) fn export_chat_contents(
    channel: usize,
    contents: &str,
) -> std::io::Result<std::path::PathBuf> {
    let root = oteryn_client::settings::ClientSettings::path()
        .and_then(|path| path.parent().map(|parent| parent.join("chat")))
        .ok_or_else(|| std::io::Error::other("Preferences directory unavailable"))?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(std::io::Error::other)?
        .as_secs();
    export_chat_contents_to(&root, channel, contents, timestamp)
}

fn export_chat_contents_to(
    root: &std::path::Path,
    channel: usize,
    contents: &str,
    timestamp: u64,
) -> std::io::Result<std::path::PathBuf> {
    const MAX_EXPORT_BYTES: usize = 256 * 1024;
    const CHANNEL_NAMES: [&str; 6] = [
        "local",
        "world",
        "english",
        "help",
        "advertising",
        "private",
    ];
    let name = CHANNEL_NAMES
        .get(channel)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "Unknown chat tab"))?;
    if contents.is_empty() || contents.len() > MAX_EXPORT_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Chat export is empty or too large",
        ));
    }
    std::fs::create_dir_all(root)?;
    for suffix in 0..100_u8 {
        let suffix = if suffix == 0 {
            String::new()
        } else {
            format!("-{suffix}")
        };
        let path = root.join(format!("{name}-{timestamp}{suffix}.txt"));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&path) {
            Ok(mut file) => {
                use std::io::Write;
                if let Err(error) = file
                    .write_all(contents.as_bytes())
                    .and_then(|()| file.sync_all())
                {
                    drop(file);
                    let _cleanup = std::fs::remove_file(&path);
                    return Err(error);
                }
                return Ok(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "Chat export name space exhausted",
    ))
}

fn show_window_recovery(ctx: &egui::Context, english: bool) -> bool {
    let rect = ctx.content_rect();
    let tr = |pl, en| if english { en } else { pl };
    let mut open_settings = false;
    egui::Window::new(tr("Za małe okno", "Small window"))
        .id("game-window-size-help".into())
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .constrain_to(rect)
        .max_width((rect.width() - 24.0).max(1.0))
        .show(ctx, |ui| {
            open_settings = ui
                .button(tr("Ustawienia", "Settings"))
                .on_hover_text(tr(
                    "Powiększ okno albo zmniejsz skalę interfejsu lub liczbę pasków akcji.",
                    "Enlarge the window, or reduce interface scale or the number of action rows.",
                ))
                .clicked();
        });
    open_settings
}

fn ratio(value: u32, maximum: u32) -> f32 {
    if maximum == 0 {
        0.0
    } else {
        (value as f32 / maximum as f32).clamp(0.0, 1.0)
    }
}

fn action_busy(english: bool) -> &'static str {
    if english {
        "Wait for the previous action to finish"
    } else {
        "Poczekaj na zakończenie poprzedniej czynności"
    }
}

fn panel_heading(ui: &mut egui::Ui, label: &str) {
    egui::Frame::NONE
        .fill(ui.visuals().widgets.inactive.bg_fill)
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(5, 3))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).size(11.0).strong());
        });
}

fn equipment(
    ui: &mut egui::Ui,
    inventory: Option<&oteryn_session::CharacterInventory>,
    vitals: Option<oteryn_session::ActorVitals>,
    link: &PlayLink,
    english: bool,
) {
    let slots = [
        Some(EquipmentSlot::Necklace),
        Some(EquipmentSlot::Head),
        None,
        Some(EquipmentSlot::RightHand),
        Some(EquipmentSlot::Armor),
        Some(EquipmentSlot::LeftHand),
        Some(EquipmentSlot::Ring),
        Some(EquipmentSlot::Legs),
        Some(EquipmentSlot::Ammo),
        None,
        Some(EquipmentSlot::Feet),
        None,
    ];
    egui::Grid::new("equipment-paper-doll")
        .num_columns(3)
        .spacing([2.0, 2.0])
        .show(ui, |ui| {
            for (index, slot) in slots.into_iter().enumerate() {
                if slot.is_none() && index != 2 {
                    ui.allocate_ui_with_layout(
                        egui::vec2(30.0, 30.0),
                        egui::Layout::top_down(egui::Align::Center),
                        |ui| {
                            ui.small(if index == 9 { "Soul" } else { "Cap" });
                            ui.small(if index == 9 {
                                vitals.map_or_else(|| "—".into(), |v| v.soul.to_string())
                            } else {
                                "—".into()
                            });
                        },
                    );
                } else {
                    let (id, label) = slot.map_or(
                        ("backpack", if english { "Backpack" } else { "Plecak" }),
                        |slot| slot_label(slot, english),
                    );
                    let entry = inventory.and_then(|inventory| {
                        slot.map_or(inventory.main_backpack.as_ref(), |slot| {
                            inventory
                                .equipment
                                .iter()
                                .find(|entry| entry.slot == slot)
                                .map(|entry| &entry.item)
                        })
                    });
                    let response = crate::panel_icons::equipment(ui, id, label);
                    if let Some(item) = entry {
                        ui.painter()
                            .with_clip_rect(response.rect.shrink(1.0).intersect(ui.clip_rect()))
                            .text(
                                response.rect.right_bottom() - egui::vec2(3.0, 3.0),
                                egui::Align2::RIGHT_BOTTOM,
                                item.count.to_string(),
                                egui::FontId::proportional(9.0),
                                ui.visuals().text_color(),
                            );
                        response
                            .clone()
                            .on_hover_text(item_description(item, english));
                        item_menu(response, item, link, english);
                    } else {
                        response.on_hover_text(if english {
                            if inventory.is_some() {
                                "Empty slot."
                            } else {
                                "Item data is unavailable."
                            }
                        } else {
                            if inventory.is_some() {
                                "Puste miejsce."
                            } else {
                                "Dane przedmiotu są niedostępne."
                            }
                        });
                    }
                }
                if index % 3 == 2 {
                    ui.end_row();
                }
            }
        });
}

fn slot_label(slot: EquipmentSlot, english: bool) -> (&'static str, &'static str) {
    let (id, pl, en) = match slot {
        EquipmentSlot::Head => ("head", "Głowa", "Head"),
        EquipmentSlot::Necklace => ("necklace", "Naszyjnik", "Necklace"),
        EquipmentSlot::Armor => ("armor", "Pancerz", "Armor"),
        EquipmentSlot::RightHand => ("right-hand", "Prawa ręka", "Right hand"),
        EquipmentSlot::LeftHand => ("left-hand", "Lewa ręka", "Left hand"),
        EquipmentSlot::Legs => ("legs", "Nogi", "Legs"),
        EquipmentSlot::Feet => ("feet", "Stopy", "Feet"),
        EquipmentSlot::Ring => ("ring", "Pierścień", "Ring"),
        EquipmentSlot::Ammo => ("ammo", "Amunicja", "Ammo"),
    };
    (id, if english { en } else { pl })
}

fn item_grid(ui: &mut egui::Ui, id: &str, items: &[ItemEntry], link: &PlayLink, english: bool) {
    egui::Grid::new(id)
        .num_columns(4)
        .spacing([2.0, 2.0])
        .show(ui, |ui| {
            for (index, item) in items.iter().enumerate() {
                let response = item_button(ui, item).on_hover_text(item_description(item, english));
                item_menu(response, item, link, english);
                if index % 4 == 3 {
                    ui.end_row();
                }
            }
        });
}

fn item_button(ui: &mut egui::Ui, item: &ItemEntry) -> egui::Response {
    let response = ui.add_sized([30.0, 30.0], egui::Button::new(""));
    for (row, text) in [
        format!("#{}", item.item_definition_ref),
        format!("×{}", item.count),
    ]
    .into_iter()
    .enumerate()
    {
        let rect = egui::Rect::from_min_size(
            response.rect.min + egui::vec2(2.0, 2.0 + row as f32 * 13.0),
            egui::vec2(26.0, 12.0),
        );
        let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
        let mut job = egui::text::LayoutJob::simple_singleline(
            text,
            egui::FontId::proportional(9.0),
            ui.visuals().text_color(),
        );
        job.wrap.max_width = rect.width();
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        let galley = painter.layout_job(job);
        painter.galley(
            rect.center() - galley.size() * 0.5,
            galley,
            ui.visuals().text_color(),
        );
    }
    response
}

fn item_description(item: &ItemEntry, english: bool) -> String {
    if english {
        format!(
            "Item #{} · quantity {}",
            item.item_definition_ref, item.count
        )
    } else {
        format!(
            "Przedmiot #{} · ilość {}",
            item.item_definition_ref, item.count
        )
    }
}

fn item_menu(response: egui::Response, item: &ItemEntry, link: &PlayLink, english: bool) {
    if !response.enabled() {
        egui::Popup::close_id(&response.ctx, egui::Popup::default_response_id(&response));
        return;
    }
    response.context_menu(|ui| {
        if ui
            .button(if english {
                "Move to backpack"
            } else {
                "Przenieś do plecaka"
            })
            .clicked()
        {
            let _ = link.send_action(ActionBarCommand::MoveToBackpack(item.handle));
            ui.close();
        }
    });
}

fn actor_kind(kind: EntityKind, english: bool) -> &'static str {
    let (pl, en) = match kind {
        EntityKind::Player => ("Gracz", "Player"),
        EntityKind::Creature => ("Stworzenie", "Creature"),
        EntityKind::Npc => ("NPC", "NPC"),
        EntityKind::Corpse => ("Zwłoki", "Corpse"),
        EntityKind::GroundItem => ("Przedmiot", "Item"),
    };
    if english { en } else { pl }
}

fn use_feedback(value: UseDisposition, english: bool) -> &'static str {
    let (pl, en) = match value {
        UseDisposition::Committed => ("Wykonano czynność", "Action completed"),
        UseDisposition::NothingToUse => ("Nie ma czego użyć", "Nothing to use"),
        UseDisposition::Occupied => ("Miejsce jest zajęte", "Location is occupied"),
        UseDisposition::StaleState => ("Stan przedmiotu zmienił się", "Item state changed"),
        UseDisposition::TooFar => ("Za daleko", "Too far away"),
        UseDisposition::Rejected => (
            "Nie można wykonać czynności",
            "Action could not be completed",
        ),
        UseDisposition::RequirementNotMet => ("Nie spełniasz wymagań", "Requirements are not met"),
        UseDisposition::Exhausted => (
            "Poczekaj przed kolejną czynnością",
            "Wait before the next action",
        ),
        UseDisposition::Full => ("Brak miejsca", "No space available"),
        UseDisposition::NoTarget => ("Wybierz cel", "Select a target"),
    };
    if english { en } else { pl }
}

fn move_feedback(value: ItemMoveOutcome, english: bool) -> &'static str {
    let (pl, en) = match value {
        ItemMoveOutcome::Moved => ("Przeniesiono przedmiot", "Item moved"),
        ItemMoveOutcome::Stale => ("Przedmiot zmienił się", "Item changed"),
        ItemMoveOutcome::TooFar => ("Za daleko", "Too far away"),
        ItemMoveOutcome::NoBackpack => ("Brak plecaka", "No backpack"),
        ItemMoveOutcome::NoRoom => ("Brak miejsca", "No space available"),
        ItemMoveOutcome::NotOwner => ("Przedmiot nie należy do ciebie", "You do not own this item"),
        ItemMoveOutcome::NotPickupable => (
            "Nie można podnieść przedmiotu",
            "This item cannot be picked up",
        ),
        ItemMoveOutcome::NotSupported => {
            ("Ta czynność jest niedostępna", "This action is unavailable")
        }
        ItemMoveOutcome::Rejected => ("Nie można przenieść przedmiotu", "Item could not be moved"),
        ItemMoveOutcome::SlotMismatch => (
            "Przedmiot nie pasuje do slotu",
            "Item does not fit this slot",
        ),
        ItemMoveOutcome::RequirementNotMet => ("Nie spełniasz wymagań", "Requirements are not met"),
        ItemMoveOutcome::Blocked => (
            "Miejsce docelowe jest niedostępne",
            "Destination is unavailable",
        ),
    };
    if english { en } else { pl }
}

fn chat_feedback(value: ChatDisposition, english: bool) -> &'static str {
    let (pl, en) = match value {
        ChatDisposition::Ok => ("Wysłano", "Sent"),
        ChatDisposition::Muted => ("Czat jest wyciszony", "Chat is muted"),
        ChatDisposition::Exhausted => (
            "Poczekaj przed kolejną wiadomością",
            "Wait before the next message",
        ),
        ChatDisposition::LevelTooLow => ("Wymagany wyższy poziom", "A higher level is required"),
        ChatDisposition::NotOnline => ("Odbiorca jest offline", "Recipient is offline"),
        ChatDisposition::NoVocation => ("Wymagana profesja", "A vocation is required"),
        ChatDisposition::RoomNotOpen => ("Otwórz kanał", "Open the channel"),
        ChatDisposition::ChatUnavailable => ("Czat jest niedostępny", "Chat is unavailable"),
        ChatDisposition::Rejected => ("Wiadomość odrzucona", "Message was rejected"),
    };
    if english { en } else { pl }
}

#[cfg(test)]
mod tests {
    use super::{
        ChatMode, PanelHotkeyDestination, ambient_overlay_alpha, chat_channel_tabs, chat_line_text,
        custom_text_action_available, cycle_chat_channel, export_chat_contents_to, item_button,
        panel_hotkey_destination, settings_with_panel_shortcuts, show_window_recovery,
    };
    use crate::preferences_browser::test_support::{button_position, click, frame};

    #[test]
    fn chat_channel_cycle_uses_only_local_open_rooms_and_private() {
        let mut rooms = oteryn_session::ChatRoomSet::default();
        rooms.insert(oteryn_session::ChatRoom::World);
        rooms.insert(oteryn_session::ChatRoom::Help);
        assert_eq!(cycle_chat_channel(0, rooms, true), 1);
        assert_eq!(cycle_chat_channel(1, rooms, true), 3);
        assert_eq!(cycle_chat_channel(3, rooms, true), 5);
        assert_eq!(cycle_chat_channel(5, rooms, true), 0);
        assert_eq!(cycle_chat_channel(0, rooms, false), 5);
        assert_eq!(cycle_chat_channel(5, rooms, false), 3);
        assert_eq!(cycle_chat_channel(2, rooms, true), 1);
    }

    #[test]
    fn unavailable_chat_tabs_keep_the_visible_selection_but_cannot_change_it()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(420.0, 90.0);
        let mut channel = 0;
        let mut output = egui::FullOutput::default();
        for _ in 0..2 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                egui::Window::new("unavailable-chat-tabs").show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| chat_channel_tabs(ui, &mut channel, true, false));
                });
            })
            .1;
        }
        let world = button_position(&output, size, "World").ok_or("World chat tab")?;
        click(&ctx, size, world, |ctx| {
            egui::Window::new("unavailable-chat-tabs").show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| chat_channel_tabs(ui, &mut channel, true, false));
            });
        });
        assert_eq!(
            channel, 0,
            "an unavailable session must not accept tab input"
        );

        for _ in 0..2 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                egui::Window::new("unavailable-chat-tabs").show(ctx, |ui| {
                    ui.horizontal_wrapped(|ui| chat_channel_tabs(ui, &mut channel, true, true));
                });
            })
            .1;
        }
        let world = button_position(&output, size, "World").ok_or("enabled World chat tab")?;
        click(&ctx, size, world, |ctx| {
            egui::Window::new("unavailable-chat-tabs").show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| chat_channel_tabs(ui, &mut channel, true, true));
            });
        });
        assert_eq!(
            channel, 1,
            "the same tab must remain interactive with chat state"
        );
        Ok(())
    }

    #[test]
    fn screenshot_feedback_reports_success_and_failure_in_both_languages() {
        let mut ui = super::GameUi::default();
        for (saved, english, expected) in [
            (true, true, "Screenshot saved."),
            (true, false, "Zapisano zrzut ekranu."),
            (false, true, "Screenshot could not be saved."),
            (false, false, "Nie udało się zapisać zrzutu ekranu."),
        ] {
            ui.screenshot_finished(saved, english);
            assert_eq!(ui.notice.as_deref(), Some(expected));
        }
    }

    #[test]
    fn minimap_zoom_hotkeys_share_the_visible_control_bounds() {
        let mut ui = super::GameUi::default();
        assert!(ui.route_minimap_hotkey("minimap.zoom_out"));
        assert_eq!(ui.minimap_zoom, 1);
        assert!(ui.route_minimap_hotkey("minimap.zoom_out"));
        assert!(ui.route_minimap_hotkey("minimap.zoom_out"));
        assert_eq!(ui.minimap_zoom, 2);
        assert!(ui.route_minimap_hotkey("minimap.zoom_in"));
        assert_eq!(ui.minimap_zoom, 1);
        for _ in 0..40 {
            assert!(ui.route_minimap_hotkey("minimap.north"));
            assert!(ui.route_minimap_hotkey("minimap.east"));
        }
        assert_eq!(ui.minimap_pan, [16, -16]);
        assert!(ui.route_minimap_hotkey("minimap.south"));
        assert!(ui.route_minimap_hotkey("minimap.west"));
        assert_eq!(ui.minimap_pan, [15, -15]);
        assert!(ui.route_minimap_hotkey("minimap.center"));
        assert_eq!(ui.minimap_pan, [0, 0]);
        assert!(ui.route_minimap_hotkey("minimap.center"));
        assert!(!ui.route_minimap_hotkey("minimap.floor_up"));
    }

    #[test]
    fn panel_hotkeys_preserve_show_hide_and_open_page_semantics() {
        assert_eq!(
            panel_hotkey_destination("window.spell_list"),
            Some(PanelHotkeyDestination::Toggle("spells"))
        );
        assert_eq!(
            panel_hotkey_destination("dialog.quest_log"),
            Some(PanelHotkeyDestination::Open("quest-log"))
        );
        assert_eq!(
            panel_hotkey_destination("dialog.compendium"),
            Some(PanelHotkeyDestination::Open("compendium"))
        );
        for (action, tab) in [
            ("dialog.cyclopedia.map", "map"),
            ("dialog.cyclopedia.items", "items"),
            ("dialog.cyclopedia.character", "character"),
            ("dialog.cyclopedia.bestiary", "bestiary"),
            ("dialog.cyclopedia.charms", "charms"),
            ("dialog.cyclopedia.magical_archive", "archive"),
        ] {
            assert_eq!(
                panel_hotkey_destination(action),
                Some(PanelHotkeyDestination::OpenTab("cyclopedia", tab))
            );
        }
        for (action, panel) in [
            ("dialog.exaltation_forge", "forge"),
            ("dialog.reward_wall", "reward-wall"),
            ("dialog.task_board", "task-board"),
            ("dialog.weapon_proficiency", "weapon-proficiency"),
            ("dialog.wheel_of_destiny", "wheel"),
        ] {
            assert_eq!(
                panel_hotkey_destination(action),
                Some(PanelHotkeyDestination::Open(panel))
            );
        }
        for (action, tab) in [
            ("dialog.social.assemble_team", "assemble"),
            ("dialog.social.badges", "badges"),
            ("dialog.social.friend_list", "friends"),
            ("dialog.social.friends_config", "friends-config"),
            ("dialog.social.friend_invitations", "invitations"),
            ("dialog.social.friend_search", "search"),
            ("dialog.social.join_team", "finder"),
        ] {
            assert_eq!(
                panel_hotkey_destination(action),
                Some(PanelHotkeyDestination::OpenTab("social", tab))
            );
        }
        for (action, panel) in [
            ("window.analytics_selector", "analytics"),
            ("window.bestiary_tracker", "bestiary-tracker"),
            ("window.bosstiary_tracker", "bosstiary-tracker"),
            ("window.imbuement_tracker", "imbuement-tracker"),
            ("window.kill_tracker", "kill-tracker"),
            ("window.party", "party"),
            ("window.player_guide", "player-guide"),
            ("window.quest_tracker", "quest-tracker"),
        ] {
            assert_eq!(
                panel_hotkey_destination(action),
                Some(PanelHotkeyDestination::Toggle(panel))
            );
        }
        for unsupported in [
            "dialog.lenshelp",
            "dialog.manage_containers",
            "window.battle.secondary",
            "window.boss_cooldowns",
            "window.damage_input_analyser",
            "window.drop_tracker",
            "window.hunting_analyser",
            "window.impact_analyser",
            "window.loot_analyser",
            "window.party_hunt_analyser",
            "window.supply_analyser",
            "window.xp_analyser",
        ] {
            assert_eq!(panel_hotkey_destination(unsupported), None, "{unsupported}");
        }
    }

    #[test]
    fn panel_hotkeys_apply_real_toggle_open_and_social_tab_state() {
        let mut ui = super::GameUi::default();
        for (action, panel) in [
            ("window.analytics_selector", "analytics"),
            ("window.bestiary_tracker", "bestiary-tracker"),
            ("window.bosstiary_tracker", "bosstiary-tracker"),
            ("window.imbuement_tracker", "imbuement-tracker"),
            ("window.kill_tracker", "kill-tracker"),
            ("window.party", "party"),
            ("window.player_guide", "player-guide"),
            ("window.quest_tracker", "quest-tracker"),
        ] {
            assert!(ui.route_panel_hotkey(action));
            assert!(ui.panels.is_open(panel), "{action}");
            assert!(ui.route_panel_hotkey(action));
            assert!(!ui.panels.is_open(panel), "{action}");
        }

        assert!(ui.route_panel_hotkey("dialog.compendium"));
        assert!(ui.panels.is_open("compendium"));
        assert!(ui.route_panel_hotkey("dialog.compendium"));
        assert!(ui.panels.is_open("compendium"));

        for (action, tab) in [
            ("dialog.social.assemble_team", "assemble"),
            ("dialog.social.badges", "badges"),
            ("dialog.social.friend_list", "friends"),
            ("dialog.social.friends_config", "friends-config"),
            ("dialog.social.friend_invitations", "invitations"),
            ("dialog.social.friend_search", "search"),
            ("dialog.social.join_team", "finder"),
        ] {
            assert!(ui.route_panel_hotkey(action));
            assert!(ui.panels.is_open("social"), "{action}");
            assert_eq!(ui.panels.selected_tab("social"), Some(tab), "{action}");
            assert!(ui.route_panel_hotkey(action));
            assert!(ui.panels.is_open("social"), "{action}");
            assert_eq!(ui.panels.selected_tab("social"), Some(tab), "{action}");
        }
    }

    #[test]
    fn settings_compendium_action_opens_the_existing_panel_idempotently() {
        let mut ui = super::GameUi::default();
        for _ in 0..2 {
            assert!(
                ui.route_settings_action(crate::settings_ui::SettingsRuntimeAction::OpenCompendium)
            );
            assert!(ui.panels.is_open("compendium"));
        }
    }

    #[test]
    fn chat_submit_hotkey_uses_the_composer_validation_and_clears_only_on_accept() {
        use std::cell::RefCell;

        let sent = RefCell::new(Vec::new());
        let mut ui = super::GameUi {
            draft: "  hello  ".into(),
            ..Default::default()
        };
        assert!(ui.submit_current_chat(true, true, |intent| {
            sent.borrow_mut().push(intent);
            true
        }));
        assert_eq!(
            sent.into_inner(),
            vec![oteryn_session::ChatIntent::Say {
                mode: oteryn_session::ChatSpeechMode::Say,
                text: "hello".into(),
            }]
        );
        assert!(ui.draft.is_empty());

        ui.channel = 5;
        ui.recipient = " Alice ".into();
        ui.draft = "private".into();
        let mut private = None;
        assert!(ui.submit_current_chat(true, true, |intent| {
            private = Some(intent);
            true
        }));
        assert_eq!(
            private,
            Some(oteryn_session::ChatIntent::Private {
                recipient_name: "Alice".into(),
                text: "private".into(),
            })
        );

        ui.channel = 0;
        ui.draft = "retry".into();
        assert!(!ui.submit_current_chat(true, true, |_| false));
        assert_eq!(ui.draft, "retry");
        assert_eq!(
            ui.notice.as_deref(),
            Some("Wait for the previous action to finish")
        );
    }

    #[test]
    fn chat_submit_hotkey_rejects_unavailable_read_only_and_invalid_drafts_without_sending() {
        let mut ui = super::GameUi {
            draft: "hello".into(),
            ..Default::default()
        };
        let mut calls = 0;
        assert!(!ui.submit_current_chat(false, true, |_| {
            calls += 1;
            true
        }));
        ui.chat_read_only[0] = true;
        assert!(!ui.submit_current_chat(true, true, |_| {
            calls += 1;
            true
        }));
        ui.chat_read_only[0] = false;
        ui.draft = " ".into();
        assert!(!ui.submit_current_chat(true, true, |_| {
            calls += 1;
            true
        }));
        ui.channel = 5;
        ui.draft = "private".into();
        ui.recipient.clear();
        assert!(!ui.submit_current_chat(true, true, |_| {
            calls += 1;
            true
        }));
        ui.channel = 0;
        ui.draft = "x".repeat(oteryn_session::MAX_CHAT_TEXT_BYTES + 1);
        assert!(!ui.submit_current_chat(true, true, |_| {
            calls += 1;
            true
        }));
        assert_eq!(calls, 0);
        assert!(!ui.draft.is_empty());
    }

    #[test]
    fn automatic_custom_text_needs_chat_transport_but_not_a_visible_composer() {
        assert!(custom_text_action_available(true, false, true));
        assert!(custom_text_action_available(true, true, true));
        assert!(!custom_text_action_available(true, true, false));
        assert!(!custom_text_action_available(false, false, true));
        assert!(custom_text_action_available(false, true, true));
    }

    #[test]
    fn exact_enter_binding_switches_resolver_context_and_temporary_send_returns_off()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_client::hotkeys::{HotkeyPreferences, ResolvedHotkey};
        use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};

        let preferences = HotkeyPreferences::default();
        for key in [40, 88] {
            let enter = NormalizedInputEvent::Key {
                code: KeyCode::new(key)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::NONE,
                repeat: false,
            };
            let mut ui = super::GameUi::default();
            let off_actions =
                preferences.resolve(std::slice::from_ref(&enter), ui.chat_hotkey_context());
            assert_eq!(
                off_actions,
                vec![ResolvedHotkey::General("chat.mode.temporary".into())]
            );
            let ResolvedHotkey::General(action) = &off_actions[0] else {
                return Err("expected general hotkey".into());
            };
            assert!(ui.route_chat_mode_hotkey(action, true, true));
            assert_eq!(ui.chat_mode, ChatMode::Temporary);
            assert!(ui.chat_hotkey_context());
            assert_eq!(
                preferences.resolve(std::slice::from_ref(&enter), ui.chat_hotkey_context()),
                vec![ResolvedHotkey::General("chat.send".into())]
            );

            ui.draft = "hello".into();
            assert!(ui.submit_current_chat(true, true, |_| true));
            assert_eq!(ui.chat_mode, ChatMode::Off);
            assert!(!ui.chat_hotkey_context());
        }
        Ok(())
    }

    #[test]
    fn configured_on_and_off_actions_resolve_into_the_game_ui_state()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_client::{
            hotkeys::{HotkeyPair, HotkeyPreferences, ResolvedHotkey},
            settings_catalog::ShortcutBinding,
        };
        use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};

        let pair = |key| HotkeyPair {
            first: Some(ShortcutBinding {
                key,
                shift: false,
                ctrl: false,
                alt: false,
                meta: false,
            }),
            second: None,
        };
        let mut preferences = HotkeyPreferences::default();
        let profile = preferences.active_mut().ok_or("default hotkey profile")?;
        profile.action_bar.shortcuts[10] = None;
        profile.action_bar.chat_on_shortcuts[11] = None;
        profile
            .general_chat_off
            .insert("chat.mode.on".into(), pair(68));
        profile
            .general_chat_on
            .insert("chat.mode.off".into(), pair(69));
        preferences.validate(&[82, 79, 81, 80])?;

        let pressed = |key| -> Result<NormalizedInputEvent, Box<dyn std::error::Error>> {
            Ok(NormalizedInputEvent::Key {
                code: KeyCode::new(key)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::NONE,
                repeat: false,
            })
        };
        let mut ui = super::GameUi::default();
        let actions = preferences.resolve(&[pressed(68)?], ui.chat_hotkey_context());
        assert_eq!(
            actions,
            vec![ResolvedHotkey::General("chat.mode.on".into())]
        );
        let ResolvedHotkey::General(action) = &actions[0] else {
            return Err("expected general hotkey".into());
        };
        assert!(ui.route_chat_mode_hotkey(action, true, true));
        assert_eq!(ui.chat_mode, ChatMode::On);

        let actions = preferences.resolve(&[pressed(69)?], ui.chat_hotkey_context());
        assert_eq!(
            actions,
            vec![ResolvedHotkey::General("chat.mode.off".into())]
        );
        let ResolvedHotkey::General(action) = &actions[0] else {
            return Err("expected general hotkey".into());
        };
        assert!(ui.route_chat_mode_hotkey(action, true, true));
        assert_eq!(ui.chat_mode, ChatMode::Off);
        Ok(())
    }

    #[test]
    fn temporary_activation_focuses_without_replaying_the_claimed_enter() -> Result<(), &'static str>
    {
        let ctx = egui::Context::default();
        let size = egui::vec2(320.0, 80.0);
        let mut ui = super::GameUi {
            draft: "existing draft".into(),
            ..Default::default()
        };
        let draw_composer = |ctx: &egui::Context, ui: &mut super::GameUi| {
            let window = egui::Window::new("temporary-chat-focus-test").show(ctx, |frame_ui| {
                let edit = frame_ui.text_edit_singleline(&mut ui.draft);
                let legacy_submit = edit.lost_focus()
                    && frame_ui.input(|input| input.key_pressed(egui::Key::Enter));
                let id = edit.id;
                ui.apply_chat_composer_focus(&edit, true);
                (id, legacy_submit)
            })?;
            window.inner
        };

        let (initial, _) = frame(&ctx, size, Vec::new(), |ctx| draw_composer(ctx, &mut ui));
        let (composer, _) = initial.ok_or("focus test window")?;
        assert_eq!(ctx.memory(|memory| memory.focused()), None);
        assert!(ui.route_chat_mode_hotkey("chat.mode.temporary", true, true));
        ui.apply_chat_focus_transition(&ctx);
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(composer));

        // The native shell suppresses the claimed activation key before egui;
        // this next render must not replay that Enter into the focused editor.
        let (result, _) = frame(&ctx, size, Vec::new(), |ctx| draw_composer(ctx, &mut ui));
        let (rendered_composer, legacy_submit) = result.ok_or("focus test window")?;
        assert!(!legacy_submit);
        assert_eq!(ui.draft, "existing draft");
        assert_eq!(ui.chat_mode, ChatMode::Temporary);
        assert_eq!(rendered_composer, composer);
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(composer));
        Ok(())
    }

    #[test]
    fn explicit_chat_on_persists_after_send_and_off_relinquishes_the_composer()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(320.0, 80.0);
        let mut ui = super::GameUi::default();

        assert!(ui.route_chat_mode_hotkey("chat.mode.on", true, true));
        let (composer, _) = frame(&ctx, size, Vec::new(), |ctx| {
            let window = egui::Window::new("chat-focus-test").show(ctx, |frame_ui| {
                let edit = frame_ui.text_edit_singleline(&mut ui.draft);
                let id = edit.id;
                ui.apply_chat_composer_focus(&edit, true);
                id
            })?;
            window.inner
        });
        let composer = composer.ok_or("focus test window")?;
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(composer));

        ui.draft = "persistent".into();
        assert!(ui.submit_current_chat(true, true, |_| true));
        assert_eq!(ui.chat_mode, ChatMode::On);
        assert!(ui.chat_hotkey_context());

        assert!(ui.route_chat_mode_hotkey("chat.mode.off", true, true));
        ui.apply_chat_focus_transition(&ctx);
        assert_eq!(ctx.memory(|memory| memory.focused()), None);
        let _ = frame(&ctx, size, Vec::new(), |ctx| {
            let _ = egui::Window::new("chat-focus-test").show(ctx, |frame_ui| {
                let edit = frame_ui.text_edit_singleline(&mut ui.draft);
                ui.apply_chat_composer_focus(&edit, true);
            });
        });
        assert_eq!(ctx.memory(|memory| memory.focused()), None);
        assert_eq!(ui.chat_mode, ChatMode::Off);
        Ok(())
    }

    #[test]
    fn chat_on_does_not_reclaim_focus_from_an_unrelated_text_editor() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(360.0, 100.0);
        let mut ui = super::GameUi::default();
        let mut unrelated_text = String::new();

        assert!(ui.route_chat_mode_hotkey("chat.mode.on", true, true));
        let ids = frame(&ctx, size, Vec::new(), |ctx| {
            let window = egui::Window::new("chat-unrelated-focus-test").show(ctx, |frame_ui| {
                let unrelated = frame_ui.text_edit_singleline(&mut unrelated_text);
                let composer = frame_ui.text_edit_singleline(&mut ui.draft);
                ui.apply_chat_composer_focus(&composer, true);
                (unrelated.id, composer.id)
            })?;
            window.inner
        })
        .0
        .ok_or("focus test window")?;
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(ids.1));
        assert!(ui.chat_routes_keyboard(&ctx));

        ctx.memory_mut(|memory| memory.request_focus(ids.0));
        let _ = frame(&ctx, size, Vec::new(), |ctx| {
            let _ = egui::Window::new("chat-unrelated-focus-test").show(ctx, |frame_ui| {
                let unrelated = frame_ui.text_edit_singleline(&mut unrelated_text);
                let composer = frame_ui.text_edit_singleline(&mut ui.draft);
                assert_eq!(unrelated.id, ids.0);
                assert_eq!(composer.id, ids.1);
                ui.apply_chat_composer_focus(&composer, true);
            });
        });
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(ids.0));
        assert!(ui.chat_hotkey_context());
        assert!(!ui.chat_routes_keyboard(&ctx));
        Ok(())
    }

    #[test]
    fn read_only_or_unavailable_composer_never_enters_chat_mode() {
        let mut ui = super::GameUi::default();
        assert!(ui.route_chat_mode_hotkey("chat.mode.temporary", false, true));
        assert_eq!(ui.chat_mode, ChatMode::Off);
        ui.chat_read_only[0] = true;
        assert!(ui.route_chat_mode_hotkey("chat.mode.on", true, true));
        assert_eq!(ui.chat_mode, ChatMode::Off);
        assert_eq!(ui.notice.as_deref(), Some("This tab is read-only."));
    }

    #[test]
    fn console_mode_indicator_exposes_off_on_and_temporary_states() {
        let mut ui = super::GameUi::default();
        assert_eq!(ui.chat_mode_label(true), "Chat Off");
        assert_eq!(ui.chat_mode_label(false), "Czat wyłączony");
        assert!(ui.route_chat_mode_hotkey("chat.mode.on", true, true));
        assert_eq!(ui.chat_mode_label(true), "Chat On");
        assert!(ui.route_chat_mode_hotkey("chat.mode.temporary", true, true));
        assert_eq!(ui.chat_mode_label(true), "Chat On*");
        assert!(ui.route_chat_mode_hotkey("chat.mode.off", true, true));
        assert_eq!(ui.chat_mode_label(true), "Chat Off");
    }

    #[test]
    fn custom_text_uses_the_composer_contract_and_preserves_an_unrelated_draft_on_auto_send() {
        let mut ui = super::GameUi {
            draft: "unchanged".into(),
            ..Default::default()
        };
        assert!(!ui.place_chat_text("preset", false, true));
        assert_eq!(ui.draft, "unchanged");
        assert_eq!(ui.chat_mode, ChatMode::Off);

        ui.chat_read_only[0] = true;
        assert!(!ui.place_chat_text("preset", true, true));
        assert_eq!(ui.draft, "unchanged");
        ui.chat_read_only[0] = false;
        assert!(ui.place_chat_text("preset", true, true));
        assert_eq!(ui.draft, "preset");
        assert_eq!(ui.chat_mode, ChatMode::Temporary);

        ui.draft = "manual draft".into();
        let mut sent = None;
        assert!(
            ui.submit_chat_text("automatic".into(), true, true, |intent| {
                sent = Some(intent);
                true
            })
        );
        ui.complete_chat_send();
        assert_eq!(
            sent,
            Some(oteryn_session::ChatIntent::Say {
                mode: oteryn_session::ChatSpeechMode::Say,
                text: "automatic".into(),
            })
        );
        assert_eq!(ui.draft, "manual draft");
        assert_eq!(ui.chat_mode, ChatMode::Off);
    }

    #[test]
    fn ambient_overlay_requires_the_light_master_and_retains_its_bounded_alpha() {
        let mut settings = oteryn_client::settings::ClientSettings::default();
        settings.future_preferences.insert(
            "effects.effects.ambient_light".into(),
            oteryn_client::settings_catalog::FutureValue::Int(25),
        );
        assert_eq!(ambient_overlay_alpha(&settings), None);
        settings.future_preferences.insert(
            "effects.effects.lighting".into(),
            oteryn_client::settings_catalog::FutureValue::Bool(false),
        );
        assert_eq!(ambient_overlay_alpha(&settings), None);
        settings.future_preferences.insert(
            "effects.effects.lighting".into(),
            oteryn_client::settings_catalog::FutureValue::Bool(true),
        );
        assert_eq!(ambient_overlay_alpha(&settings), Some(143));
        settings.future_preferences.insert(
            "effects.effects.ambient_light".into(),
            oteryn_client::settings_catalog::FutureValue::Int(100),
        );
        assert_eq!(ambient_overlay_alpha(&settings), Some(0));
    }

    #[test]
    fn panel_shortcut_save_preserves_a_pending_action_bar_lock_change() {
        let current = oteryn_client::settings::ClientSettings::default();
        let mut pending = current.clone();
        pending.action_bar.set_assignments_locked(true);

        let merged = settings_with_panel_shortcuts(
            Some(pending),
            &current,
            vec!["skills".into(), "battle".into()],
        );

        assert!(merged.action_bar.assignments_locked());
        assert_eq!(merged.panel_shortcuts, ["skills", "battle"]);
    }

    #[test]
    fn action_bar_clear_availability_tracks_assignments_per_row()
    -> Result<(), Box<dyn std::error::Error>> {
        use oteryn_client::action_bar::{
            ACTION_BAR_ROWS, ACTION_BAR_SLOTS, ActionBarCommand, SlotAssignment,
        };

        let settings = oteryn_client::settings::ClientSettings::default();
        let mut ui = super::GameUi::default();
        assert_eq!(
            ui.action_bar_rows_with_assignments(),
            [false; ACTION_BAR_ROWS]
        );

        let assignment = SlotAssignment {
            label: "Help".into(),
            command: ActionBarCommand::Chat(oteryn_session::ChatIntent::OpenRoom(
                oteryn_session::ChatRoom::Help,
            )),
        };
        let bar = ui
            .bar(&settings)
            .ok_or_else(|| std::io::Error::other("action bar unavailable"))?;
        bar.assign(2 * ACTION_BAR_SLOTS + 4, Some(assignment.clone()))?;
        bar.assign(7 * ACTION_BAR_SLOTS, Some(assignment))?;

        let mut expected = [false; ACTION_BAR_ROWS];
        expected[2] = true;
        expected[7] = true;
        assert_eq!(ui.action_bar_rows_with_assignments(), expected);

        ui.clear_action_row(2, &settings);
        expected[2] = false;
        assert_eq!(ui.action_bar_rows_with_assignments(), expected);
        Ok(())
    }

    #[test]
    fn chat_export_partitions_tabs_and_never_overwrites_an_existing_snapshot()
    -> Result<(), Box<dyn std::error::Error>> {
        let private = oteryn_session::ChatLine::Private {
            speaker_name: "Alice".into(),
            text: "hello".into(),
        };
        let help = oteryn_session::ChatLine::Room {
            room: oteryn_session::ChatRoom::Help,
            speaker_name: "Guide".into(),
            text: "welcome".into(),
        };
        assert_eq!(
            chat_line_text(&private, 5, true).as_deref(),
            Some("Alice: hello")
        );
        assert!(chat_line_text(&private, 3, true).is_none());
        assert_eq!(
            chat_line_text(&help, 3, true).as_deref(),
            Some("Guide: welcome")
        );
        assert!(chat_line_text(&help, 2, true).is_none());

        let root = std::env::temp_dir().join(format!(
            "oteryn-chat-export-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        let first = export_chat_contents_to(&root, 5, "Alice: hello", 42)?;
        let second = export_chat_contents_to(&root, 5, "Alice: hello again", 42)?;
        assert_eq!(
            first.file_name().and_then(|name| name.to_str()),
            Some("private-42.txt")
        );
        assert_eq!(
            second.file_name().and_then(|name| name.to_str()),
            Some("private-42-1.txt")
        );
        assert_eq!(std::fs::read_to_string(first)?, "Alice: hello");
        assert_eq!(std::fs::read_to_string(second)?, "Alice: hello again");
        assert!(export_chat_contents_to(&root, 6, "text", 42).is_err());
        assert!(export_chat_contents_to(&root, 0, "", 42).is_err());
        std::fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn long_authoritative_item_labels_stay_inside_compact_tiles() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(156.0, 100.0);
        for value in [1, u32::MAX] {
            let item = oteryn_session::ItemEntry {
                handle: std::num::NonZeroU64::new(1).ok_or("handle")?,
                item_definition_ref: std::num::NonZeroU32::new(value).ok_or("definition")?,
                count: std::num::NonZeroU32::new(value).ok_or("count")?,
                sub_type: 0,
            };
            let mut rect = egui::Rect::NOTHING;
            let mut output = egui::FullOutput::default();
            for _ in 0..2 {
                output = frame(&ctx, size, Vec::new(), |ctx| {
                    egui::Area::new("item-tile-test".into()).show(ctx, |ui| {
                        rect = item_button(ui, &item).rect;
                    });
                })
                .1;
            }
            assert_eq!(rect.size(), egui::vec2(30.0, 30.0));
            let mut painted_rows = 0;
            for shape in output.shapes {
                if let egui::epaint::Shape::Text(text) = shape.shape
                    && !text.galley.job.text.is_empty()
                {
                    assert!(rect.contains_rect(shape.clip_rect));
                    painted_rows += 1;
                }
            }
            assert_eq!(painted_rows, 2);
        }
        Ok(())
    }

    #[test]
    fn a_scene_too_small_for_action_rows_keeps_settings_clickable() -> Result<(), &'static str> {
        for english in [false, true] {
            let ctx = egui::Context::default();
            for size in [egui::vec2(900.0, 620.0), egui::vec2(300.0, 200.0) / 1.8] {
                let mut output = egui::FullOutput::default();
                for _ in 0..4 {
                    output = frame(&ctx, size, Vec::new(), |ctx| {
                        show_window_recovery(ctx, english)
                    })
                    .1;
                }
                let label = if english { "Settings" } else { "Ustawienia" };
                let position =
                    button_position(&output, size, label).ok_or("recovery control clipped")?;
                assert!(click(&ctx, size, position, |ctx| show_window_recovery(
                    ctx, english
                )));
            }
        }
        Ok(())
    }
}
