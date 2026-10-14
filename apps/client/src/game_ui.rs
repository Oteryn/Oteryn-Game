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
    chat_history: ChatHistory,
    chat_ime_preedit: bool,
    recipient: String,
    channel: usize,
    chat_read_only: [bool; 6],
    chat_cleared_at: [Option<u64>; 6],
    chat_mode: ChatMode,
    chat_focus_request: Option<bool>,
    chat_select_all_request: bool,
    chat_copy_request: bool,
    chat_composer_id: Option<egui::Id>,
    chat_recipient_id: Option<egui::Id>,
    chat_output_id: Option<egui::Id>,
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

// A client memory bound, not an observed Tibia history capacity. No persistence
// or server-success claim: entries are texts accepted by the existing mailbox.
const MAX_SENT_CHAT_LINES: usize = 64;

#[derive(Default)]
struct ChatHistory {
    lines: std::collections::VecDeque<String>,
    position: Option<usize>,
    scratch: Option<String>,
}

impl ChatHistory {
    fn stop_browsing(&mut self) {
        self.position = None;
        self.scratch = None;
    }

    fn record(&mut self, text: String) {
        self.stop_browsing();
        if self.lines.len() == MAX_SENT_CHAT_LINES {
            self.lines.pop_front();
        }
        self.lines.push_back(text);
    }

    fn recall(&mut self, older: bool, draft: &str) -> Option<String> {
        let next = if older {
            match self.position {
                Some(0) => return None,
                Some(position) => position - 1,
                None => {
                    let newest = self.lines.len().checked_sub(1)?;
                    self.scratch = Some(draft.to_owned());
                    newest
                }
            }
        } else {
            let position = self.position?;
            if position + 1 == self.lines.len() {
                self.position = None;
                return self.scratch.take();
            }
            position + 1
        };
        self.position = Some(next);
        self.lines.get(next).cloned()
    }
}

fn chat_history_direction(event: &egui::Event) -> Option<bool> {
    match event {
        egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } if *modifiers == egui::Modifiers::SHIFT => match key {
            egui::Key::ArrowUp => Some(true),
            egui::Key::ArrowDown => Some(false),
            _ => None,
        },
        _ => None,
    }
}

fn chat_ime_active_after(event: &egui::Event, active: bool) -> bool {
    match event {
        egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) => !text.is_empty(),
        egui::Event::Ime(egui::ImeEvent::Commit(_)) => false,
        _ => active,
    }
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

    /// Copy acts on the composer only. Other chat editors and received output
    /// keep their ordinary egui clipboard handling, including the default chord.
    #[must_use]
    pub fn chat_copy_routes_keyboard(&self, ctx: &egui::Context) -> bool {
        !self
            .chat_read_only
            .get(self.channel)
            .copied()
            .unwrap_or(true)
            && self
                .chat_composer_id
                .is_some_and(|id| ctx.memory(|memory| memory.focused()) == Some(id))
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
            if self.chat_select_all_request {
                self.chat_select_all_request = false;
                if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(self.draft.chars().count()),
                        )));
                    state.store(ctx, id);
                }
            }
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

    /// The native shell appends this to upstream egui-winit's pending input at
    /// the claimed key's position. egui then copies its real current selection
    /// after earlier text/pointer events, rather than a previous-frame snapshot.
    pub fn take_chat_copy_event(&mut self, ctx: &egui::Context) -> Option<egui::Event> {
        (std::mem::take(&mut self.chat_copy_request) && self.chat_copy_routes_keyboard(ctx))
            .then_some(egui::Event::Copy)
    }

    fn set_chat_mode(&mut self, mode: ChatMode) {
        if mode == ChatMode::Off {
            self.chat_select_all_request = false;
            self.chat_copy_request = false;
        }
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
        self.chat_history.stop_browsing();
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

    fn chat_composer(
        &mut self,
        ui: &mut egui::Ui,
        enabled: bool,
        width: f32,
        hint: &str,
    ) -> egui::Response {
        let id = ui.make_persistent_id("chat-composer");
        let owns_input = enabled
            && ui.is_enabled()
            && !self.blocks_game_input()
            && self.chat_hotkey_context()
            && ui.memory(|memory| memory.has_focus(id))
            && ui.input(|input| input.focused);
        let initial_ime = owns_input && self.chat_ime_preedit;
        let (recall, next_ime) = ui.input(|input| {
            let mut composing = initial_ime;
            let mut recall = false;
            if owns_input {
                for event in &input.events {
                    composing = chat_ime_active_after(event, composing);
                    recall |= !composing && chat_history_direction(event).is_some();
                }
            }
            (recall, composing)
        });
        self.chat_ime_preedit = next_ime;
        if !recall {
            let response = ui.add_enabled(
                enabled,
                egui::TextEdit::singleline(&mut self.draft)
                    .id(id)
                    .desired_width(width)
                    .hint_text(hint),
            );
            if response.changed() {
                self.chat_history.stop_browsing();
            }
            return response;
        }

        // Upstream TextEdit owns ordinary edits, selection, IME and clipboard.
        // Process contiguous input segments around recall chords in order:
        // Text("x"), Up and Up, Text("x") must have different results. Hidden
        // intermediate paints use the same editor/rect; only the final text is
        // drawn. No vendor fork or substitute editing/clipboard implementation.
        let events = ui.input_mut(|input| std::mem::take(&mut input.events));
        let mut segments = Vec::new();
        let mut ordinary = Vec::new();
        let mut composing = initial_ime;
        for event in events {
            composing = chat_ime_active_after(&event, composing);
            if !composing && let Some(older) = chat_history_direction(&event) {
                segments.push((std::mem::take(&mut ordinary), Some(older)));
            } else {
                ordinary.push(event);
            }
        }
        segments.push((ordinary, None));
        let clip = ui.clip_rect();
        ui.set_clip_rect(egui::Rect::NOTHING);
        let mut rect = None;
        let mut remaining = Vec::new();
        let mut changed = false;
        for (events, direction) in segments {
            ui.input_mut(|input| input.events = events);
            let editor = egui::TextEdit::singleline(&mut self.draft)
                .id(id)
                .desired_width(width)
                .hint_text(hint);
            let response = match rect {
                None => ui.add(editor),
                Some(rect) => ui.place(rect, editor),
            };
            rect = Some(response.rect);
            if response.changed() {
                self.chat_history.stop_browsing();
                changed = true;
            }
            remaining.extend(ui.input_mut(|input| std::mem::take(&mut input.events)));
            // An earlier segment may have submitted with Enter or moved focus.
            // Never replace the line after its editor relinquished ownership.
            if let Some(older) = direction
                && ui.memory(|memory| memory.has_focus(id))
                && let Some(text) = self.chat_history.recall(older, &self.draft)
            {
                self.draft = text;
                changed = true;
                if let Some(mut state) = egui::TextEdit::load_state(ui.ctx(), id) {
                    state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::one(
                            egui::text::CCursor::new(self.draft.chars().count()),
                        )));
                    state.store(ui.ctx(), id);
                }
            }
        }
        ui.set_clip_rect(clip);
        let mut response = ui.place(
            rect.unwrap_or_else(|| ui.available_rect_before_wrap()),
            egui::TextEdit::singleline(&mut self.draft)
                .id(id)
                .desired_width(width)
                .hint_text(hint),
        );
        if changed {
            response.mark_changed();
        }
        ui.input_mut(|input| input.events = remaining);
        response
    }

    fn relinquish_hidden_chat_focus(&mut self, ctx: &egui::Context) {
        self.chat_ime_preedit = false;
        self.set_chat_mode(ChatMode::Off);
        self.chat_focus_request = None;
        if let Some(id) = self.chat_composer_id {
            ctx.memory_mut(|memory| memory.surrender_focus(id));
        }
        if let Some(id) = self.chat_recipient_id {
            ctx.memory_mut(|memory| memory.surrender_focus(id));
        }
        if let Some(id) = self.chat_output_id.take() {
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
        if self.route_chat_text_hotkey(action, chat_visible && rooms.is_some(), english) {
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

    fn route_chat_text_hotkey(&mut self, action: &str, available: bool, english: bool) -> bool {
        if action == "chat.text.copy" {
            // Hidden/unavailable/read-only consoles cannot claim stale draft
            // selections. Copy itself does not activate chat or change focus.
            self.chat_copy_request = available
                && self.chat_composer_id.is_some()
                && !self
                    .chat_read_only
                    .get(self.channel)
                    .copied()
                    .unwrap_or(true);
            return true;
        }
        if action != "chat.text.select_all" {
            return false;
        }
        if self.chat_composer_ready(available, english) && self.chat_composer_id.is_some() {
            self.chat_select_all_request = true;
            self.chat_focus_request = Some(true);
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
        let Some(intent) = chat_intent(self.channel, &self.recipient, text.clone()) else {
            self.notice = Some(tr("Podaj prawidłowego odbiorcę", "Enter a valid recipient").into());
            return false;
        };
        if !send(intent) {
            self.notice = Some(action_busy(english).into());
            return false;
        }
        self.chat_history.record(text);
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
            // Native file pickers belong to WindowsShell, not panel navigation.
            crate::settings_ui::SettingsRuntimeAction::ExportOptions
            | crate::settings_ui::SettingsRuntimeAction::ImportOptions => false,
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
                    ResolvedHotkey::Custom(action @ CustomHotkeyAction::Text { .. }) => action,
                    ResolvedHotkey::Custom(_) => continue,
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
                    CustomHotkeyAction::Spell { .. } | CustomHotkeyAction::Object { .. } => {}
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
        close_stale_item_menu(
            ctx,
            state.container.as_ref(),
            hud_enabled && settings.show_inventory,
            link.connection_generation(),
        );
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
        let pixels_per_point = ctx.pixels_per_point();
        let viewport = oteryn_client::layout::GameViewport::for_settings(
            rect.width() * pixels_per_point,
            rect.height() * pixels_per_point,
            pixels_per_point,
            settings,
        );
        if viewport.is_none() {
            // Oversized underlying HUD Areas must not occlude the recovery button.
            close_stale_item_menu(
                ctx,
                state.container.as_ref(),
                false,
                link.connection_generation(),
            );
            return show_window_recovery(ctx, en);
        }
        if let Some(viewport) = viewport {
            let scene = egui::Rect::from_min_size(
                egui::pos2(viewport.x, viewport.y) / pixels_per_point,
                egui::vec2(viewport.width, viewport.height) / pixels_per_point,
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
                                equipment(ui, state.inventory.as_ref(), state.vitals, en);
                                if let Some(inventory) = &state.inventory {
                                    ui.separator();
                                    panel_heading(ui, tr("Plecak", "Backpack"));
                                    if inventory.entries.is_empty() {
                                        ui.small(tr("Pusty", "Empty"));
                                    }
                                    item_grid(
                                        ui,
                                        "main-backpack",
                                        &inventory.entries,
                                        None,
                                        link,
                                        en,
                                        &mut self.notice,
                                    );
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
                                    item_grid(
                                        ui,
                                        "open-container",
                                        &container.entries,
                                        container.container_handle,
                                        link,
                                        en,
                                        &mut self.notice,
                                    );
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
                            let contents = chat_rows(
                                chat,
                                &state.chat_receipts,
                                self.channel,
                                en,
                                settings,
                                self.chat_cleared_at[self.channel],
                            )
                            .join("\n");
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
                                        egui::Button::new(tr("Wyczyść okno", "Clear Window")),
                                    )
                                    .clicked()
                                {
                                    self.chat_cleared_at[self.channel] = Some(
                                        state
                                            .chat_receipts
                                            .last()
                                            .map_or(0, |line| line.server_sequence),
                                    );
                                    ui.close();
                                }
                                if ui
                                    .add_enabled(
                                        !contents.is_empty(),
                                        egui::Button::new(tr("Zapisz okno", "Save Window")),
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
                                let contents = chat_rows(
                                    chat,
                                    &state.chat_receipts,
                                    self.channel,
                                    en,
                                    settings,
                                    self.chat_cleared_at[self.channel],
                                )
                                .join("\n");
                                let output =
                                    crate::chat_output::show(ui, self.channel, &contents, settings);
                                self.chat_output_id = Some(output.response.id);
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
                            let edit = self.chat_composer(
                                ui,
                                !read_only,
                                (ui.available_width() - 65.0).max(30.0),
                                tr("Napisz wiadomość", "Write a message"),
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

fn chat_rows(
    chat: &oteryn_session::ChatLog,
    receipts: &[oteryn_client::play::ChatReceipt],
    channel: usize,
    english: bool,
    settings: &ClientSettings,
    cleared_at: Option<u64>,
) -> Vec<String> {
    if receipts.is_empty() && cleared_at.is_none() {
        // A pre-existing projection has no receipt clock; never invent one.
        return chat
            .lines()
            .filter_map(|line| chat_line_text(line, channel, english))
            .collect();
    }
    receipts
        .iter()
        .filter(|receipt| cleared_at.is_none_or(|cutoff| receipt.server_sequence > cutoff))
        .filter_map(|receipt| {
            let text = chat_line_text(&receipt.line, channel, english)?;
            if settings.stored_bool("console.console.timestamps") == Some(true)
                && let Some([hour, minute, second]) = receipt.local_time
            {
                return Some(
                    if settings.stored_bool("console.console.seconds") == Some(true) {
                        format!("{hour:02}:{minute:02}:{second:02} {text}")
                    } else {
                        format!("{hour:02}:{minute:02} {text}")
                    },
                );
            }
            Some(text)
        })
        .collect()
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

fn item_grid(
    ui: &mut egui::Ui,
    id: &str,
    items: &[ItemEntry],
    container: Option<oteryn_session::ItemHandle>,
    link: &PlayLink,
    english: bool,
    notice: &mut Option<String>,
) {
    egui::Grid::new(id)
        .num_columns(4)
        .spacing([2.0, 2.0])
        .show(ui, |ui| {
            for (index, item) in items.iter().enumerate() {
                let identity = ui.make_persistent_id((
                    id,
                    link.connection_generation(),
                    container,
                    item.handle,
                ));
                ui.scope_builder(egui::UiBuilder::new().id(identity), |ui| {
                    let response =
                        item_button(ui, item).on_hover_text(item_description(item, english));
                    if let Some(container) = container {
                        item_menu(response, container, item, link, english, notice);
                    }
                });
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

fn item_menu_state_id() -> egui::Id {
    egui::Id::new("game-item-context-source")
}

fn close_stale_item_menu(
    ctx: &egui::Context,
    container: Option<&oteryn_session::OpenContainer>,
    enabled: bool,
    generation: Option<u64>,
) {
    let key = item_menu_state_id();
    if let Some((connection, source, item, popup)) = ctx.data(|data| {
        data.get_temp::<(
            Option<u64>,
            oteryn_session::ItemHandle,
            oteryn_session::ItemHandle,
            egui::Id,
        )>(key)
    }) && (!enabled
        || connection != generation
        || !ctx.input(|input| input.focused)
        || !container.is_some_and(|current| {
            current.container_handle == Some(source)
                && current.entries.iter().any(|entry| entry.handle == item)
        }))
    {
        egui::Popup::close_id(ctx, popup);
        ctx.data_mut(|data| {
            data.remove::<(
                Option<u64>,
                oteryn_session::ItemHandle,
                oteryn_session::ItemHandle,
                egui::Id,
            )>(key)
        });
    }
}

fn item_menu(
    response: egui::Response,
    container: oteryn_session::ItemHandle,
    item: &ItemEntry,
    link: &PlayLink,
    english: bool,
    notice: &mut Option<String>,
) {
    let popup = egui::Popup::default_response_id(&response);
    if !response.enabled() {
        egui::Popup::close_id(&response.ctx, popup);
        return;
    }
    egui::Popup::context_menu(&response)
        .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
        .show(|ui| {
            if ui
                .button(if english {
                    "Move to backpack"
                } else {
                    "Przenieś do plecaka"
                })
                .clicked()
            {
                if link.send_action(ActionBarCommand::MoveToBackpack(item.handle)) {
                    *notice = None;
                    ui.close();
                } else {
                    *notice = Some(action_busy(english).into());
                    ui.small(action_busy(english));
                }
            }
        });
    if egui::Popup::is_id_open(&response.ctx, popup) {
        response.ctx.data_mut(|data| {
            data.insert_temp(
                item_menu_state_id(),
                (link.connection_generation(), container, item.handle, popup),
            )
        });
    }
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
        // Main's capability-11 outcomes are not activated by this client
        // packet. Keep their fallback explicit and never describe them as Used.
        UseDisposition::Locked
        | UseDisposition::KeyMismatch
        | UseDisposition::Sealed
        | UseDisposition::PzBlocked
        | UseDisposition::NotMovable
        | UseDisposition::NoRoom
        | UseDisposition::NotPossible => {
            ("Ta czynność jest niedostępna", "This action is unavailable")
        }
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
        chat_rows, custom_text_action_available, cycle_chat_channel, export_chat_contents_to,
        item_button, panel_hotkey_destination, settings_with_panel_shortcuts, show_window_recovery,
    };
    use crate::preferences_browser::test_support::{button_position, click, frame};

    #[test]
    fn unactivated_world_interaction_outcomes_never_report_a_success() {
        use oteryn_session::UseDisposition;
        for outcome in [
            UseDisposition::Locked,
            UseDisposition::KeyMismatch,
            UseDisposition::Sealed,
            UseDisposition::PzBlocked,
            UseDisposition::NotMovable,
            UseDisposition::NoRoom,
            UseDisposition::NotPossible,
        ] {
            for english in [false, true] {
                assert_eq!(
                    super::use_feedback(outcome, english),
                    if english {
                        "This action is unavailable"
                    } else {
                        "Ta czynność jest niedostępna"
                    }
                );
                assert_ne!(
                    super::use_feedback(outcome, english),
                    super::use_feedback(UseDisposition::Committed, english)
                );
            }
        }
    }

    type MenuState = std::sync::Arc<std::sync::Mutex<oteryn_client::play::GameState>>;
    type MenuActions =
        std::sync::Arc<std::sync::Mutex<Vec<oteryn_client::action_bar::ActionBarCommand>>>;

    fn menu_lock<T>(mutex: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        mutex
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    struct ItemMenuFixture {
        link: oteryn_client::play::PlayLink,
        state: MenuState,
        english: bool,
        enabled: bool,
        notice: Option<String>,
    }

    impl ItemMenuFixture {
        fn show(&mut self, ctx: &egui::Context) {
            let state = menu_lock(&self.state).clone();
            super::close_stale_item_menu(
                ctx,
                state.container.as_ref(),
                self.enabled,
                self.link.connection_generation(),
            );
            egui::Window::new("Item command fixture")
                .fixed_pos(egui::Pos2::ZERO)
                .show(ctx, |ui| {
                    if !self.enabled {
                        ui.disable();
                    }
                    super::equipment(ui, state.inventory.as_ref(), None, self.english);
                    if let Some(inventory) = &state.inventory {
                        super::item_grid(
                            ui,
                            "main-backpack",
                            &inventory.entries,
                            None,
                            &self.link,
                            self.english,
                            &mut self.notice,
                        );
                    }
                    if let Some(container) = &state.container {
                        super::item_grid(
                            ui,
                            "open-container",
                            &container.entries,
                            container.container_handle,
                            &self.link,
                            self.english,
                            &mut self.notice,
                        );
                    }
                    if let Some(notice) = &self.notice {
                        ui.small(notice);
                    }
                });
        }
    }

    struct MenuStepper {
        state: MenuState,
        actions: MenuActions,
        generation: u64,
    }

    impl oteryn_client::play::Stepper for MenuStepper {
        fn connection_generation(&self) -> Option<u64> {
            Some(self.generation)
        }
        fn game_state(&self) -> oteryn_client::play::GameState {
            menu_lock(&self.state).clone()
        }
        async fn chat(
            &mut self,
            intent: &oteryn_session::ChatIntent,
        ) -> Result<Option<oteryn_session::ChatOutcome>, oteryn_session::SessionError> {
            menu_lock(&self.actions).push(oteryn_client::action_bar::ActionBarCommand::Chat(
                intent.clone(),
            ));
            Ok(None) // Routing fixture, not a successful server echo.
        }
        async fn action(
            &mut self,
            command: &oteryn_client::action_bar::ActionBarCommand,
        ) -> Result<Option<oteryn_client::action_bar::ActionBarOutcome>, oteryn_session::SessionError>
        {
            menu_lock(&self.actions).push(command.clone());
            Ok(Some(
                oteryn_client::action_bar::ActionBarOutcome::Unavailable,
            ))
        }
        async fn step(
            &mut self,
            _: oteryn_session::StepDirection,
        ) -> Result<oteryn_session::StepOutcome, oteryn_session::SessionError> {
            Err(oteryn_session::SessionError::SessionUnusable)
        }
        async fn serve(
            &mut self,
            _: std::time::Duration,
        ) -> Result<(), oteryn_session::SessionError> {
            std::future::pending().await
        }
        async fn serve_or_wake(
            &mut self,
            _: std::time::Duration,
            wake: &tokio::sync::Notify,
        ) -> Result<(), oteryn_session::SessionError> {
            wake.notified().await;
            Ok(())
        }
        fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
            Vec::new()
        }
        async fn combat(
            &mut self,
            _: oteryn_client::combat_input::CombatCommand,
        ) -> Result<oteryn_client::combat_input::CombatReport, oteryn_session::SessionError>
        {
            Err(oteryn_session::SessionError::SessionUnusable)
        }
    }

    fn menu_item(handle: u64, definition: u32) -> Result<oteryn_session::ItemEntry, &'static str> {
        Ok(oteryn_session::ItemEntry {
            handle: std::num::NonZeroU64::new(handle).ok_or("fixture handle")?,
            item_definition_ref: std::num::NonZeroU32::new(definition)
                .ok_or("fixture definition")?,
            count: std::num::NonZeroU32::new(if handle == 300 { 3 } else { 1 })
                .ok_or("fixture count")?,
            sub_type: 0,
        })
    }

    fn menu_fixture(
        english: bool,
        generation: u64,
    ) -> Result<
        (
            ItemMenuFixture,
            impl std::future::Future<Output = ()>,
            MenuActions,
        ),
        &'static str,
    > {
        let state = std::sync::Arc::new(std::sync::Mutex::new(oteryn_client::play::GameState {
            inventory: Some(oteryn_session::CharacterInventory {
                main_backpack: Some(menu_item(300, 300)?),
                entries: vec![menu_item(200, 200)?],
                equipment: Vec::new(),
            }),
            container: Some(oteryn_session::OpenContainer {
                container_handle: std::num::NonZeroU64::new(1000),
                entries: vec![menu_item(101, 101)?, menu_item(102, 102)?],
            }),
            ..Default::default()
        }));
        let actions = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let (link, commands, outbox) = oteryn_client::play::play_channel();
        let task = oteryn_client::play::run_session(
            MenuStepper {
                state: state.clone(),
                actions: actions.clone(),
                generation,
            },
            commands,
            outbox,
        );
        Ok((
            ItemMenuFixture {
                link,
                state,
                english,
                enabled: true,
                notice: None,
            },
            task,
            actions,
        ))
    }

    fn poll_menu_task(task: std::pin::Pin<&mut impl std::future::Future<Output = ()>>) {
        assert!(
            task.poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
                .is_pending()
        );
    }

    struct ChatConsumer {
        game: super::GameUi,
        view: oteryn_client::play::PlayView,
        settings: oteryn_client::settings::ClientSettings,
    }

    impl ChatConsumer {
        fn new(fixture: &ItemMenuFixture, english: bool) -> Result<Self, &'static str> {
            menu_lock(&fixture.state).chat = Some(oteryn_session::ChatLog::default());
            Ok(Self {
                game: super::GameUi {
                    chat_mode: ChatMode::On,
                    chat_focus_request: Some(true),
                    ..Default::default()
                },
                view: oteryn_client::play::PlayView::new(
                    oteryn_renderer::TileCoord::new(100, 100),
                    -7,
                    &[],
                )
                .map_err(|_| "chat fixture scene")?,
                settings: oteryn_client::settings::ClientSettings {
                    english,
                    show_inventory: false,
                    show_minimap: false,
                    ..Default::default()
                },
            })
        }

        fn show(&mut self, ctx: &egui::Context, fixture: &ItemMenuFixture) {
            self.game
                .show(ctx, &fixture.link, &self.view, &self.settings);
        }
    }

    fn recall_key(older: bool) -> egui::Event {
        egui::Event::Key {
            key: if older {
                egui::Key::ArrowUp
            } else {
                egui::Key::ArrowDown
            },
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::SHIFT,
        }
    }

    #[test]
    fn chat_history_actual_composer_recalls_unicode_and_restores_scratch_without_sending()
    -> Result<(), &'static str> {
        let size = egui::vec2(1200.0, 800.0);
        for english in [false, true] {
            let ctx = egui::Context::default();
            let (fixture, task, sent) = menu_fixture(english, 42)?;
            let mut chat = ChatConsumer::new(&fixture, english)?;
            let mut task = Box::pin(task);
            poll_menu_task(task.as_mut());
            for text in ["first 🦀", "Zażółć gęślą"] {
                chat.game.draft = text.into();
                let mut output = egui::FullOutput::default();
                for _ in 0..3 {
                    output = frame(&ctx, size, vec![], |ctx| chat.show(ctx, &fixture)).1;
                }
                let send = if english { "Send" } else { "Wyślij" };
                let position = button_position(&output, size, send).ok_or("actual Send")?;
                click(&ctx, size, position, |ctx| chat.show(ctx, &fixture));
                poll_menu_task(task.as_mut());
                assert!(chat.game.draft.is_empty());
            }
            assert_eq!(menu_lock(&sent).len(), 2);
            assert_eq!(
                fixture.link.game_state().chat.ok_or("received chat")?.len(),
                0
            );
            chat.game.draft = "  unsent 🦀  ".into();
            let id = chat.game.chat_composer_id.ok_or("production composer id")?;
            ctx.memory_mut(|memory| memory.request_focus(id));
            for (older, expected) in [
                (true, "Zażółć gęślą"),
                (true, "first 🦀"),
                (true, "first 🦀"),
                (false, "Zażółć gęślą"),
                (false, "  unsent 🦀  "),
                (false, "  unsent 🦀  "),
            ] {
                let output = frame(&ctx, size, vec![recall_key(older)], |ctx| {
                    chat.show(ctx, &fixture)
                })
                .1;
                assert_eq!(chat.game.draft, expected);
                assert!(output.shapes.iter().any(|shape| {
                    matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == expected)
                }), "final recalled text is painted");
                let state = egui::TextEdit::load_state(&ctx, id).ok_or("cursor state")?;
                let cursor = state.cursor.char_range().ok_or("Unicode cursor")?;
                assert!(cursor.is_empty());
                assert_eq!(
                    cursor.primary.index,
                    egui::text::CharIndex(expected.chars().count())
                );
                poll_menu_task(task.as_mut());
                assert_eq!(menu_lock(&sent).len(), 2); // Recall never sends.
            }
        }
        Ok(())
    }

    #[test]
    fn chat_history_actual_composer_preserves_mixed_event_order_and_edited_recalls()
    -> Result<(), &'static str> {
        let size = egui::vec2(1200.0, 800.0);
        for text_first in [false, true] {
            let ctx = egui::Context::default();
            let (fixture, task, sent) = menu_fixture(true, 42)?;
            let mut chat = ChatConsumer::new(&fixture, true)?;
            let mut task = Box::pin(task);
            poll_menu_task(task.as_mut());
            assert!(
                chat.game
                    .submit_chat_text("history 🦀".into(), true, true, |intent| {
                        fixture.link.send_chat(intent)
                    })
            );
            poll_menu_task(task.as_mut());
            chat.game.draft = "scratch".into();
            for _ in 0..3 {
                frame(&ctx, size, vec![], |ctx| chat.show(ctx, &fixture));
            }
            let id = chat.game.chat_composer_id.ok_or("composer")?;
            ctx.memory_mut(|memory| memory.request_focus(id));
            let mut events = vec![recall_key(true), egui::Event::Text("x".into())];
            if text_first {
                events.reverse();
            }
            frame(&ctx, size, events, |ctx| chat.show(ctx, &fixture));
            assert_eq!(
                chat.game.draft,
                if text_first {
                    "history 🦀"
                } else {
                    "history 🦀x"
                }
            );
            frame(&ctx, size, vec![recall_key(false)], |ctx| {
                chat.show(ctx, &fixture)
            });
            assert_eq!(
                chat.game.draft,
                if text_first {
                    "scratchx"
                } else {
                    "history 🦀x"
                }
            );
            assert_eq!(menu_lock(&sent).len(), 1);
        }
        Ok(())
    }

    #[test]
    fn chat_history_enter_and_recall_do_not_send_a_different_line() -> Result<(), &'static str> {
        let size = egui::vec2(1200.0, 800.0);
        for english in [false, true] {
            for enter_first in [false, true] {
                let ctx = egui::Context::default();
                let (fixture, task, sent) = menu_fixture(english, 42)?;
                let mut chat = ChatConsumer::new(&fixture, english)?;
                if let Some(profile) = chat.settings.hotkeys.active_mut() {
                    profile.general_chat_on.remove("chat.send");
                }
                let mut task = Box::pin(task);
                poll_menu_task(task.as_mut());
                assert!(chat.game.submit_chat_text(
                    "history 🦀".into(),
                    true,
                    english,
                    |intent| fixture.link.send_chat(intent)
                ));
                poll_menu_task(task.as_mut());
                chat.game.draft = "new Zażółć".into();
                for _ in 0..3 {
                    frame(&ctx, size, vec![], |ctx| chat.show(ctx, &fixture));
                }
                let id = chat.game.chat_composer_id.ok_or("composer")?;
                ctx.memory_mut(|memory| memory.request_focus(id));
                let enter = egui::Event::Key {
                    key: egui::Key::Enter,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::NONE,
                };
                let mut events = vec![recall_key(true), enter];
                if enter_first {
                    events.reverse();
                }
                frame(&ctx, size, events, |ctx| chat.show(ctx, &fixture));
                poll_menu_task(task.as_mut());
                let actions = menu_lock(&sent);
                assert_eq!(actions.len(), 2, "one submission, not a replay");
                assert!(matches!(actions.last(), Some(
                    oteryn_client::action_bar::ActionBarCommand::Chat(
                        oteryn_session::ChatIntent::Say { text, .. }
                    )
                ) if text == if enter_first { "new Zażółć" } else { "history 🦀" }));
                assert!(chat.game.draft.is_empty());
            }
        }
        Ok(())
    }

    #[test]
    fn chat_history_clipboard_events_keep_unicode_selection_and_event_order()
    -> Result<(), &'static str> {
        let size = egui::vec2(1200.0, 800.0);
        let scratch = "Zażółć 🦀";
        let pasted = "pasted 🦀\r\nź";
        for english in [false, true] {
            for operation in [
                egui::Event::Copy,
                egui::Event::Cut,
                egui::Event::Paste(pasted.into()),
            ] {
                for recall_first in [false, true] {
                    let ctx = egui::Context::default();
                    let (fixture, task, sent) = menu_fixture(english, 42)?;
                    let mut chat = ChatConsumer::new(&fixture, english)?;
                    let mut task = Box::pin(task);
                    poll_menu_task(task.as_mut());
                    assert!(chat.game.submit_chat_text(
                        "history 🦀".into(),
                        true,
                        english,
                        |intent| fixture.link.send_chat(intent)
                    ));
                    poll_menu_task(task.as_mut());
                    chat.game.draft = scratch.into();
                    for _ in 0..3 {
                        frame(&ctx, size, vec![], |ctx| chat.show(ctx, &fixture));
                    }
                    let id = chat.game.chat_composer_id.ok_or("composer")?;
                    select_copy_text(&ctx, id, 0, scratch.chars().count())?;
                    let mut events = vec![operation.clone(), recall_key(true)];
                    if recall_first {
                        events.reverse();
                    }
                    let output = frame(&ctx, size, events, |ctx| chat.show(ctx, &fixture)).1;
                    let copy = output
                        .platform_output
                        .commands
                        .iter()
                        .filter_map(|command| {
                            if let egui::OutputCommand::CopyText(text) = command {
                                Some(text.as_str())
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();
                    let expected_copy = if !recall_first
                        && matches!(operation, egui::Event::Copy | egui::Event::Cut)
                    {
                        vec![scratch]
                    } else {
                        vec![]
                    };
                    assert_eq!(
                        copy, expected_copy,
                        "{operation:?}, recall_first={recall_first}"
                    );
                    let edited_after_recall =
                        recall_first && matches!(operation, egui::Event::Paste(_));
                    let expected = if edited_after_recall {
                        "history 🦀pasted 🦀  ź"
                    } else {
                        "history 🦀"
                    };
                    assert_eq!(chat.game.draft, expected);
                    assert!(output.shapes.iter().any(|shape| {
                        matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text == expected)
                    }));
                    frame(&ctx, size, vec![recall_key(false)], |ctx| {
                        chat.show(ctx, &fixture)
                    });
                    assert_eq!(
                        chat.game.draft,
                        if edited_after_recall {
                            expected
                        } else if recall_first {
                            scratch
                        } else {
                            match operation {
                                egui::Event::Cut => "",
                                egui::Event::Paste(_) => "pasted 🦀  ź",
                                _ => scratch,
                            }
                        }
                    );
                    poll_menu_task(task.as_mut());
                    assert_eq!(menu_lock(&sent).len(), 1, "clipboard/history never sends");
                    assert_eq!(
                        fixture.link.game_state().chat.ok_or("received chat")?.len(),
                        0
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn chat_history_never_replaces_an_active_ime_preedit() -> Result<(), &'static str> {
        let size = egui::vec2(1200.0, 800.0);
        for english in [false, true] {
            for (same_frame, recall_with_commit) in
                [(false, false), (false, true), (true, false), (true, true)]
            {
                let ctx = egui::Context::default();
                let (fixture, task, sent) = menu_fixture(english, 42)?;
                let mut chat = ChatConsumer::new(&fixture, english)?;
                let mut task = Box::pin(task);
                poll_menu_task(task.as_mut());
                assert!(
                    chat.game
                        .submit_chat_text("history".into(), true, english, |intent| fixture
                            .link
                            .send_chat(intent))
                );
                poll_menu_task(task.as_mut());
                chat.game.draft = "before ".into();
                for _ in 0..3 {
                    frame(&ctx, size, vec![], |ctx| chat.show(ctx, &fixture));
                }
                let id = chat.game.chat_composer_id.ok_or("IME composer")?;
                select_copy_text(&ctx, id, 7, 7)?;
                let preedit = egui::Event::Ime(egui::ImeEvent::Preedit {
                    text: "漢字".into(),
                    active_range_chars: Some(0..1),
                });
                if same_frame {
                    frame(&ctx, size, vec![preedit, recall_key(true)], |ctx| {
                        chat.show(ctx, &fixture)
                    });
                } else {
                    frame(&ctx, size, vec![preedit], |ctx| chat.show(ctx, &fixture));
                    frame(&ctx, size, vec![recall_key(true)], |ctx| {
                        chat.show(ctx, &fixture)
                    });
                }
                assert_eq!(
                    chat.game.draft, "before 漢字",
                    "history must not overwrite composing text"
                );
                let mut commit = vec![egui::Event::Ime(egui::ImeEvent::Commit("日本語🦀".into()))];
                if recall_with_commit {
                    commit.push(recall_key(true));
                }
                frame(&ctx, size, commit, |ctx| chat.show(ctx, &fixture));
                if recall_with_commit {
                    assert_eq!(chat.game.draft, "history");
                    frame(&ctx, size, vec![recall_key(false)], |ctx| {
                        chat.show(ctx, &fixture)
                    });
                }
                assert!(chat.game.draft.contains("日本語🦀"));
                assert!(!chat.game.draft.contains("history"));
                assert!(chat.game.chat_history.position.is_none());
                // Commit ends IME ownership: a later recall is functional again.
                frame(&ctx, size, vec![recall_key(true)], |ctx| {
                    chat.show(ctx, &fixture)
                });
                assert_eq!(chat.game.draft, "history");
                poll_menu_task(task.as_mut());
                assert_eq!(menu_lock(&sent).len(), 1);
            }
        }
        Ok(())
    }

    #[test]
    fn chat_history_is_bounded_and_refused_or_invalid_submissions_are_not_recorded() {
        let mut game = super::GameUi::default();
        for i in 0..65 {
            assert!(game.submit_chat_text(format!("line{i}"), true, true, |_| true));
        }
        assert_eq!(game.chat_history.lines.len(), super::MAX_SENT_CHAT_LINES);
        assert_eq!(
            game.chat_history.lines.front().map(String::as_str),
            Some("line1")
        );
        let kept = game.chat_history.lines.clone();
        let mut invalid_sends = 0;
        for text in [
            " ".into(),
            "x".repeat(oteryn_session::MAX_CHAT_TEXT_BYTES + 1),
        ] {
            assert!(!game.submit_chat_text(text, true, true, |_| {
                invalid_sends += 1;
                true
            }));
        }
        assert!(!game.submit_chat_text("busy".into(), true, true, |_| false));
        game.channel = 5;
        assert!(!game.submit_chat_text("private".into(), true, true, |_| {
            invalid_sends += 1;
            true
        }));
        game.channel = 0;
        assert!(
            !game.submit_chat_text("unavailable".into(), false, true, |_| {
                invalid_sends += 1;
                true
            })
        );
        game.chat_read_only[0] = true;
        assert!(!game.submit_chat_text("readonly".into(), true, true, |_| {
            invalid_sends += 1;
            true
        }));
        assert_eq!(invalid_sends, 0);
        assert_eq!(game.chat_history.lines, kept);
    }

    #[test]
    fn chat_history_busy_or_closed_mailbox_keeps_draft_and_history_retryable()
    -> Result<(), &'static str> {
        let (fixture, task, sent) = menu_fixture(true, 42)?;
        let mut game = super::GameUi {
            draft: "first".into(),
            ..Default::default()
        };
        let mut task = Box::pin(task);
        poll_menu_task(task.as_mut());
        assert!(game.submit_current_chat(true, true, |intent| fixture.link.send_chat(intent)));
        game.draft = "retry 🦀".into();
        assert!(!game.submit_current_chat(true, true, |intent| fixture.link.send_chat(intent)));
        assert_eq!(game.draft, "retry 🦀");
        assert_eq!(game.chat_history.lines.len(), 1);
        poll_menu_task(task.as_mut());
        assert_eq!(menu_lock(&sent).len(), 1);
        assert!(game.submit_current_chat(true, true, |intent| fixture.link.send_chat(intent)));
        poll_menu_task(task.as_mut());
        assert_eq!(menu_lock(&sent).len(), 2);
        game.draft = "closed".into();
        drop(task);
        assert!(!game.submit_current_chat(true, true, |intent| fixture.link.send_chat(intent)));
        assert_eq!(game.draft, "closed");
        assert_eq!(game.chat_history.lines.len(), 2);
        assert_eq!(menu_lock(&sent).len(), 2);
        Ok(())
    }

    #[test]
    fn chat_history_actual_composer_does_not_claim_hidden_readonly_modal_or_other_focus()
    -> Result<(), &'static str> {
        let size = egui::vec2(1200.0, 800.0);
        for gate in [
            "hidden",
            "readonly",
            "modal",
            "recipient",
            "output",
            "off",
            "window",
            "plain",
            "ctrl",
        ] {
            let ctx = egui::Context::default();
            let (fixture, task, sent) = menu_fixture(true, 42)?;
            let mut chat = ChatConsumer::new(&fixture, true)?;
            let mut task = Box::pin(task);
            poll_menu_task(task.as_mut());
            assert!(
                chat.game
                    .submit_chat_text("history".into(), true, true, |intent| {
                        fixture.link.send_chat(intent)
                    })
            );
            poll_menu_task(task.as_mut());
            chat.game.draft = "scratch".into();
            chat.game.recipient = "Peer".into();
            if gate == "recipient" {
                chat.game.channel = 5;
            }
            for _ in 0..3 {
                frame(&ctx, size, vec![], |ctx| chat.show(ctx, &fixture));
            }
            let composer = chat.game.chat_composer_id.ok_or("composer")?;
            ctx.memory_mut(|memory| memory.request_focus(composer));
            match gate {
                "hidden" => chat.settings.show_chat = false,
                "readonly" => chat.game.chat_read_only[0] = true,
                "modal" => chat.game.panels.manage = true,
                "off" => chat.game.set_chat_mode(ChatMode::Off),
                "recipient" | "output" => {
                    let id = if gate == "recipient" {
                        chat.game.chat_recipient_id
                    } else {
                        chat.game.chat_output_id
                    };
                    ctx.memory_mut(|memory| memory.request_focus(id.unwrap_or(composer)));
                }
                _ => {}
            }
            let mut key = recall_key(true);
            if let egui::Event::Key { modifiers, .. } = &mut key {
                if gate == "plain" {
                    *modifiers = egui::Modifiers::NONE;
                }
                if gate == "ctrl" {
                    modifiers.ctrl = true;
                }
            }
            ctx.begin_pass(egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                focused: gate != "window",
                events: vec![key],
                ..Default::default()
            });
            chat.show(&ctx, &fixture);
            let _ = ctx.end_pass();
            assert_eq!(chat.game.draft, "scratch", "{gate}");
            assert!(chat.game.chat_history.position.is_none(), "{gate}");
            assert_eq!(chat.game.recipient, "Peer");
            assert_eq!(menu_lock(&sent).len(), 1, "{gate}");
        }
        Ok(())
    }

    fn open_item_menu(
        ctx: &egui::Context,
        fixture: &mut ItemMenuFixture,
        label: &str,
    ) -> Result<egui::FullOutput, &'static str> {
        let size = egui::vec2(900.0, 620.0);
        let mut output = egui::FullOutput::default();
        for _ in 0..3 {
            output = frame(ctx, size, vec![], |ctx| fixture.show(ctx)).1;
        }
        let pos = button_position(&output, size, label).ok_or("item geometry")?;
        for pressed in [true, false] {
            frame(
                ctx,
                size,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Secondary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                |ctx| fixture.show(ctx),
            );
        }
        Ok(frame(ctx, size, vec![], |ctx| fixture.show(ctx)).1)
    }

    #[test]
    fn item_menu_routes_only_received_container_sources_and_reports_refusals()
    -> Result<(), &'static str> {
        use oteryn_client::action_bar::ActionBarCommand;
        let size = egui::vec2(900.0, 620.0);
        for english in [false, true] {
            let ctx = egui::Context::default();
            let (mut fixture, task, actions) = menu_fixture(english, 42)?;
            let mut task = Box::pin(task);
            poll_menu_task(task.as_mut());
            let label = if english {
                "Move to backpack"
            } else {
                "Przenieś do plecaka"
            };
            let original = fixture.link.game_state();
            for item in ["#200", "3"] {
                // The paper doll's count is not a move source either.
                let output = open_item_menu(&ctx, &mut fixture, item)?;
                assert!(button_position(&output, size, label).is_none(), "{item}");
                poll_menu_task(task.as_mut());
                assert!(menu_lock(&actions).is_empty());
            }
            let output = open_item_menu(&ctx, &mut fixture, "#101")?;
            let pos = button_position(&output, size, label).ok_or("move menu")?;
            click(&ctx, size, pos, |ctx| fixture.show(ctx));
            assert!(fixture.notice.is_none());
            assert_eq!(fixture.link.game_state(), original); // No optimistic count mutation.
            poll_menu_task(task.as_mut());
            assert_eq!(
                *menu_lock(&actions),
                [ActionBarCommand::MoveToBackpack(
                    menu_item(101, 101)?.handle
                )]
            );
            assert_eq!(fixture.link.game_state(), original);
            let pending = ActionBarCommand::OpenItem(menu_item(999, 999)?.handle);
            assert!(fixture.link.send_action(pending.clone()));
            let output = open_item_menu(&ctx, &mut fixture, "#101")?;
            let pos = button_position(&output, size, label).ok_or("busy menu")?;
            click(&ctx, size, pos, |ctx| fixture.show(ctx));
            assert_eq!(fixture.notice.as_deref(), Some(super::action_busy(english)));
            let output = frame(&ctx, size, vec![], |ctx| fixture.show(ctx)).1;
            assert!(button_position(&output, size, super::action_busy(english)).is_some());
            assert!(button_position(&output, size, label).is_some()); // Refusal stays retryable.
            poll_menu_task(task.as_mut());
            assert_eq!(menu_lock(&actions).len(), 2);
            assert_eq!(menu_lock(&actions)[1], pending);
            click(&ctx, size, pos, |ctx| fixture.show(ctx));
            poll_menu_task(task.as_mut());
            assert_eq!(menu_lock(&actions).len(), 3);
            drop(task); // Real receiver closure, not a synthetic send closure.
            let output = open_item_menu(&ctx, &mut fixture, "#101")?;
            let pos = button_position(&output, size, label).ok_or("ended menu")?;
            click(&ctx, size, pos, |ctx| fixture.show(ctx));
            assert_eq!(fixture.notice.as_deref(), Some(super::action_busy(english)));
            assert_eq!(menu_lock(&actions).len(), 3);
        }
        Ok(())
    }

    #[test]
    fn item_menu_cannot_retarget_replaced_removed_or_unavailable_sources()
    -> Result<(), &'static str> {
        let size = egui::vec2(900.0, 620.0);
        for english in [false, true] {
            for change in [
                "replacement",
                "removal",
                "closed",
                "container",
                "modal",
                "focus",
                "connection",
            ] {
                let ctx = egui::Context::default();
                let (mut fixture, task, actions) = menu_fixture(english, 42)?;
                let mut task = Box::pin(task);
                poll_menu_task(task.as_mut());
                let label = if english {
                    "Move to backpack"
                } else {
                    "Przenieś do plecaka"
                };
                let output = open_item_menu(&ctx, &mut fixture, "#101")?;
                let old = button_position(&output, size, label).ok_or("stale menu")?;
                {
                    let mut state = menu_lock(&fixture.state);
                    let container = state.container.as_mut().ok_or("fixture container")?;
                    match change {
                        "replacement" => container.entries[0] = menu_item(103, 103)?,
                        "removal" => {
                            container.entries.remove(0);
                        }
                        "closed" => state.container = None,
                        "container" => container.container_handle = std::num::NonZeroU64::new(1001),
                        "modal" => fixture.enabled = false,
                        _ => {}
                    }
                }
                let mut new_task = None;
                if change == "connection" {
                    let (next, next_task, next_actions) = menu_fixture(english, 43)?;
                    fixture = next;
                    let mut next_task = Box::pin(next_task);
                    poll_menu_task(next_task.as_mut());
                    new_task = Some((next_task, next_actions));
                }
                let output = if change == "focus" {
                    ctx.begin_pass(egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                        focused: false,
                        ..Default::default()
                    });
                    fixture.show(&ctx);
                    ctx.end_pass()
                } else {
                    frame(&ctx, size, vec![], |ctx| fixture.show(ctx)).1
                };
                assert!(button_position(&output, size, label).is_none(), "{change}");
                click(&ctx, size, old, |ctx| fixture.show(ctx));
                if change != "connection" {
                    poll_menu_task(task.as_mut());
                }
                if let Some((next_task, next_actions)) = &mut new_task {
                    poll_menu_task(next_task.as_mut());
                    assert!(menu_lock(next_actions).is_empty(), "new connection");
                }
                assert!(menu_lock(&actions).is_empty(), "{change}");
            }
        }
        Ok(())
    }

    #[test]
    fn item_menu_keeps_the_original_handle_after_received_reordering() -> Result<(), &'static str> {
        let size = egui::vec2(900.0, 620.0);
        let ctx = egui::Context::default();
        let (mut fixture, task, actions) = menu_fixture(true, 42)?;
        let mut task = Box::pin(task);
        poll_menu_task(task.as_mut());
        open_item_menu(&ctx, &mut fixture, "#101")?;
        menu_lock(&fixture.state)
            .container
            .as_mut()
            .ok_or("fixture container")?
            .entries
            .reverse();
        let output = frame(&ctx, size, vec![], |ctx| fixture.show(ctx)).1;
        let pos = button_position(&output, size, "Move to backpack").ok_or("reordered menu")?;
        click(&ctx, size, pos, |ctx| fixture.show(ctx));
        poll_menu_task(task.as_mut());
        assert_eq!(
            *menu_lock(&actions),
            [oteryn_client::action_bar::ActionBarCommand::MoveToBackpack(
                menu_item(101, 101)?.handle
            )]
        );
        Ok(())
    }

    #[test]
    fn routed_settings_transactions_reach_consumers_and_preserve_failed_saves()
    -> Result<(), Box<dyn std::error::Error>> {
        use crate::settings_ui::SettingsPanel;
        use oteryn_client::{settings::ClientSettings, settings_catalog::FutureValue};
        const FIXTURE: &str = "OTERYN_SETTINGS_TRANSACTION_FIXTURE";
        let Some(fixture) = std::env::var_os(FIXTURE) else {
            let directory = tempfile::tempdir()?;
            let output = std::process::Command::new(std::env::current_exe()?)
                .args(["--exact", "game_ui::tests::routed_settings_transactions_reach_consumers_and_preserve_failed_saves", "--nocapture"])
                .env(FIXTURE, directory.path()).env("LOCALAPPDATA", directory.path()).output()?;
            assert!(
                output.status.success(),
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return Ok(());
        };
        let path = std::path::PathBuf::from(fixture).join("Oteryn/settings.json");
        assert_eq!(ClientSettings::path(), Some(path.clone()));
        fn click_control(
            ctx: &egui::Context,
            panel: &mut SettingsPanel,
            page: &str,
            label: &str,
        ) -> Result<(), Box<dyn std::error::Error>> {
            let size = egui::vec2(900.0, 620.0);
            assert!(panel.open_complete_section(page));
            let mut output = egui::FullOutput::default();
            for _ in 0..4 {
                output = frame(ctx, size, vec![], |ctx| panel.show(ctx, None, None)).1;
            }
            if button_position(&output, size, label).is_none() {
                let (body, _) = crate::preferences_browser::test_support::scroll_geometry(
                    ctx,
                    "all-preferences-body-geometry",
                )
                .ok_or("settings body")?;
                output = crate::preferences_browser::test_support::reveal(
                    ctx,
                    size,
                    label,
                    body.center(),
                    |ctx| panel.show(ctx, None, None),
                );
            }
            let position = button_position(&output, size, label).ok_or_else(|| {
                std::io::Error::other(format!("routed control not visible: {page}/{label}"))
            })?;
            click(ctx, size, position, |ctx| panel.show(ctx, None, None));
            Ok(())
        }
        fn assert_consumers(
            settings: &ClientSettings,
            changed: bool,
            ctx: &egui::Context,
        ) -> Result<(), Box<dyn std::error::Error>> {
            assert_eq!(settings.redraw_fps(true), if changed { 0 } else { 144 });
            assert_eq!(settings.last_frame_limit, 144);
            assert_eq!(settings.target_frame_enabled(), !changed);
            assert_eq!(settings.target_highlight_enabled(), !changed);
            let delay = settings.movement_repeat_delay().ok_or("repeat delay")?;
            assert_eq!(delay.as_millis(), if changed { 38 } else { 250 });
            let mut repeat = oteryn_client::input::MovementRepeat::default();
            let now = std::time::Instant::now();
            repeat.pressed(
                oteryn_input_actions::KeyCode::new(26)?,
                oteryn_client::input::StepDir::North,
                now,
            );
            assert!(
                repeat
                    .poll(now + delay - std::time::Duration::from_millis(1), delay)
                    .is_none()
            );
            assert_eq!(
                repeat.poll(now + delay, delay),
                Some(oteryn_client::input::StepDir::North)
            );
            let output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 620.0),
                    )),
                    focused: true,
                    events: vec![egui::Event::PointerMoved(egui::pos2(20.0, 20.0))],
                    ..Default::default()
                },
                |ui| {
                    crate::mouse_cursor::show(ui.ctx(), settings);
                    crate::actor_hud::show(
                        ui.ctx(),
                        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(480.0, 352.0)),
                        [200.0, 160.0],
                        settings,
                        Some(oteryn_session::ActorVitals {
                            health: 75,
                            max_health: 100,
                            harmony: 7,
                            ..Default::default()
                        }),
                        None,
                    );
                },
            );
            assert_eq!(
                output.platform_output.cursor_icon,
                if changed {
                    egui::CursorIcon::None
                } else {
                    egui::CursorIcon::Default
                }
            );
            let arrow = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Mesh(mesh) if mesh.vertices.len() == 7 => Some(mesh.calc_bounds()),
                _ => None,
            });
            if changed {
                assert_eq!(
                    arrow.ok_or("actual cursor mesh")?.size(),
                    egui::vec2(38.0, 54.0)
                );
            } else {
                assert!(arrow.is_none());
            }
            let harmony = output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::Shape::Text(text) if text.galley.job.text == "7" => {
                    Some(text.visual_bounding_rect().center().x)
                }
                _ => None,
            });
            if changed {
                let scale = 480.0
                    / (oteryn_client::scene::SCENE_COLUMNS * oteryn_client::scene::SCENE_TILE_PX)
                        as f32;
                let center = (200.0 + oteryn_client::scene::SCENE_TILE_PX as f32 / 2.0) * scale;
                assert_eq!(settings.stored_choice("hud.harmony_position"), Some("mana"));
                assert!(harmony.ok_or("actual Harmony text")? > center);
            } else {
                assert!(harmony.is_none());
            }
            assert_eq!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Rect(rect) if rect.fill == egui::Color32::from_rgb(55,183,61))), changed);
            let chat = oteryn_session::ChatLog::default();
            let receipts = [oteryn_client::play::ChatReceipt {
                server_sequence: 1,
                local_time: Some([9, 7, 3]),
                line: oteryn_session::ChatLine::Private {
                    speaker_name: "Peer".into(),
                    text: "received".into(),
                },
            }];
            assert_eq!(
                chat_rows(&chat, &receipts, 5, settings.english, settings, None),
                [if changed {
                    "09:07:03 Peer: received"
                } else {
                    "Peer: received"
                }]
            );
            assert_eq!(settings.action_bar.row_is_visible(0), !changed);
            let before = ClientSettings {
                action_bar: oteryn_client::action_bar::ActionBarPreferences::default(),
                ..settings.clone()
            };
            let original =
                oteryn_client::layout::GameViewport::for_settings(1125.0, 775.0, 1.25, &before)
                    .ok_or("original viewport")?;
            let actual =
                oteryn_client::layout::GameViewport::for_settings(1125.0, 775.0, 1.25, settings)
                    .ok_or("actual viewport")?;
            if changed {
                assert!(actual.height > original.height);
            } else {
                assert_eq!(actual, original);
            }
            Ok(())
        }
        for english in [false, true] {
            let ctx = egui::Context::default();
            let consumer_context = egui::Context::default();
            let mut current = ClientSettings {
                english,
                fps: 144,
                last_frame_limit: 144,
                ..Default::default()
            };
            for (key, value) in [
                (
                    "controls.controls.default_key_delay",
                    FutureValue::Bool(true),
                ),
                ("controls.controls.key_delay_ms", FutureValue::Int(37)),
                ("interface.interface.system_cursor", FutureValue::Bool(true)),
                (
                    "game_window.window.target_marking",
                    FutureValue::Choice("frame_and_highlight".into()),
                ),
            ] {
                current.future_preferences.insert(key.into(), value);
            }
            current.save(&path)?;
            let original_file = std::fs::read(&path)?;
            let mut panel = SettingsPanel::for_settings_audit(current.clone());
            assert_consumers(&current, false, &consumer_context)?;
            for (page, pl, en) in [
                (
                    "interface",
                    "Pokaż animowany kursor",
                    "Show Animated Mouse Cursor",
                ),
                ("hud", "Pokaż zdrowie", "Show Health"),
                (
                    "console",
                    "Sekundy w znacznikach czasu",
                    "Show Seconds in Timestamps",
                ),
            ] {
                click_control(&ctx, &mut panel, page, if english { en } else { pl })?;
                assert_eq!(panel.audit_draft(), &current);
                assert_eq!(panel.current, current);
                assert_eq!(std::fs::read(&path)?, original_file);
            }
            assert!(panel.open_complete_section("controls"));
            for _ in 0..4 {
                frame(&ctx, egui::vec2(900.0, 620.0), vec![], |ctx| {
                    panel.show(ctx, None, None)
                });
            }
            let arrow = ctx
                .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("slider-arrow", true))))
                .ok_or("disabled delay arrow")?;
            click(&ctx, egui::vec2(900.0, 620.0), arrow.center(), |ctx| {
                panel.show(ctx, None, None)
            });
            assert_eq!(panel.audit_draft(), &current);
            let stage = |ctx: &egui::Context,
                         panel: &mut SettingsPanel|
             -> Result<(), Box<dyn std::error::Error>> {
                for (page, pl, en) in [
                    ("graphics", "Bez limitu FPS", "No Frame Rate Limit"),
                    (
                        "interface",
                        "Używaj kursora systemowego",
                        "Use Native Mouse Cursor",
                    ),
                    ("interface", "Pokaż duży kursor", "Show Big Mouse Cursor"),
                    (
                        "controls",
                        "Używaj domyślnego opóźnienia klawiatury (250 ms)",
                        "Use Default Keyboard Delay (250 ms)",
                    ),
                    (
                        "hud",
                        "Pokaż HUD własnej postaci",
                        "Show HUD for Own Character",
                    ),
                    ("hud", "Pokaż paski", "Show Bars"),
                    ("hud", "Pokaż zdrowie", "Show Health"),
                    ("hud", "Pokaż harmonię", "Show Harmony"),
                    ("hud", "obok łuku many", "next to Mana Arc"),
                    ("console", "Znaczniki czasu", "Show Timestamps"),
                    (
                        "console",
                        "Sekundy w znacznikach czasu",
                        "Show Seconds in Timestamps",
                    ),
                    ("action_bars", "Wszystkie", "All"),
                ] {
                    click_control(ctx, panel, page, if english { en } else { pl })?;
                }
                assert!(panel.open_complete_section("controls"));
                for _ in 0..4 {
                    frame(ctx, egui::vec2(900.0, 620.0), vec![], |ctx| {
                        panel.show(ctx, None, None)
                    });
                }
                let arrow = ctx
                    .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("slider-arrow", true))))
                    .ok_or("delay increase arrow")?;
                click(ctx, egui::vec2(900.0, 620.0), arrow.center(), |ctx| {
                    panel.show(ctx, None, None)
                });
                click_control(
                    ctx,
                    panel,
                    "game_window",
                    if english {
                        "Frame & Highlight"
                    } else {
                        "Ramka i wyróżnienie"
                    },
                )?;
                click_control(
                    ctx,
                    panel,
                    "game_window",
                    if english { "None" } else { "Brak" },
                )?;
                Ok(())
            };
            stage(&ctx, &mut panel)?;
            assert_eq!(panel.current, current);
            assert!(!panel.applied);
            assert_eq!(std::fs::read(&path)?, original_file);
            assert_consumers(panel.audit_draft(), true, &consumer_context)?;
            let staged = panel.audit_draft().clone();
            click_control(
                &ctx,
                &mut panel,
                "graphics",
                if english { "Cancel" } else { "Anuluj" },
            )?;
            assert_eq!(panel.audit_draft(), &current);
            assert!(!panel.open && !panel.applied);
            assert_consumers(&panel.current, false, &consumer_context)?;
            stage(&ctx, &mut panel)?;
            assert_eq!(panel.audit_draft(), &staged);
            click_control(
                &ctx,
                &mut panel,
                "graphics",
                if english { "Apply" } else { "Zastosuj" },
            )?;
            assert!(panel.open && panel.applied);
            assert_consumers(&panel.current, true, &consumer_context)?;
            assert_consumers(&ClientSettings::load(&path)?, true, &consumer_context)?;
            let committed = panel.current.clone();
            let committed_file = std::fs::read(&path)?;
            panel.applied = false;
            click_control(
                &ctx,
                &mut panel,
                "graphics",
                if english {
                    "No Frame Rate Limit"
                } else {
                    "Bez limitu FPS"
                },
            )?;
            let rejected = panel.audit_draft().clone();
            let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
            std::fs::create_dir(&temporary)?;
            click_control(
                &ctx,
                &mut panel,
                "graphics",
                if english { "Apply" } else { "Zastosuj" },
            )?;
            assert_eq!(panel.current, committed);
            assert_eq!(panel.audit_draft(), &rejected);
            assert!(panel.open && !panel.applied);
            let error_frame = frame(&ctx, egui::vec2(900.0, 620.0), vec![], |ctx| {
                panel.show(ctx, None, None)
            })
            .1;
            assert!(error_frame.shapes.iter().any(|shape| matches!(&shape.shape, egui::Shape::Text(text) if text.galley.job.text.starts_with(if english { "Not saved." } else { "Nie zapisano." }))));
            assert_eq!(std::fs::read(&path)?, committed_file);
            assert_consumers(&panel.current, true, &consumer_context)?;
            std::fs::remove_dir(&temporary)?;
            click_control(&ctx, &mut panel, "graphics", "Ok")?;
            assert!(!panel.open && panel.applied);
            assert_eq!(panel.current.redraw_fps(true), 144);
            assert_eq!(ClientSettings::load(&path)?.redraw_fps(true), 144);
        }
        Ok(())
    }

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
    fn output_selection_cannot_route_gameplay_and_hidden_chat_releases_its_focus()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(480.0, 120.0);
        let mut ui = super::GameUi {
            draft: "unsent 🦀".into(),
            ..Default::default()
        };
        let (output, _) = frame(&ctx, size, Vec::new(), |ctx| {
            egui::Area::new(egui::Id::new("chat-output-focus-test"))
                .show(ctx, |frame_ui| {
                    frame_ui.set_width(440.0);
                    crate::chat_output::show(
                        frame_ui,
                        0,
                        "received",
                        &oteryn_client::settings::ClientSettings::default(),
                    )
                })
                .inner
        });
        let id = output.response.id;
        ui.chat_output_id = Some(id);
        ctx.memory_mut(|memory| memory.request_focus(id));
        assert!(!ui.chat_routes_keyboard(&ctx));
        assert!(ctx.egui_wants_keyboard_input());
        assert_eq!(ui.chat_mode, ChatMode::Off);
        ui.relinquish_hidden_chat_focus(&ctx);
        assert_eq!(ctx.memory(|memory| memory.focused()), None);
        assert_eq!(ui.chat_output_id, None);
        assert_eq!(ui.draft, "unsent 🦀");
        Ok(())
    }

    #[test]
    fn select_all_hotkey_selects_real_unicode_composer_without_changing_text()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(320.0, 80.0);
        let mut ui = super::GameUi {
            draft: "Zażółć 🦀".into(),
            recipient: "recipient".into(),
            ..Default::default()
        };
        let draw = |ctx: &egui::Context, ui: &mut super::GameUi| {
            egui::Window::new("select-all-chat-test")
                .show(ctx, |frame_ui| {
                    let recipient = frame_ui.text_edit_singleline(&mut ui.recipient);
                    ui.observe_chat_text_focus(&recipient);
                    let edit = frame_ui.text_edit_singleline(&mut ui.draft);
                    ui.apply_chat_composer_focus(&edit, true);
                    edit.id
                })
                .and_then(|window| window.inner)
        };
        let (initial, _) = frame(&ctx, size, Vec::new(), |ctx| draw(ctx, &mut ui));
        let composer = initial.ok_or("composer")?;
        let recipient = ui.chat_recipient_id.ok_or("recipient")?;
        ctx.memory_mut(|memory| memory.request_focus(recipient));
        assert!(ui.route_chat_text_hotkey("chat.text.select_all", true, true));
        ui.apply_chat_focus_transition(&ctx);
        let state = egui::TextEdit::load_state(&ctx, composer).ok_or("selection state")?;
        assert_eq!(
            state.cursor.char_range().ok_or("selection")?,
            egui::text::CCursorRange::two(egui::text::CCursor::new(0), egui::text::CCursor::new(8),)
        );
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(composer));
        assert_eq!(ui.draft, "Zażółć 🦀");
        assert!(!ui.chat_select_all_request);
        let _ = frame(
            &ctx,
            size,
            vec![egui::Event::Text("replacement".into())],
            |ctx| draw(ctx, &mut ui),
        );
        assert_eq!(ui.draft, "replacement");
        assert_eq!(ui.recipient, "recipient");
        assert!(!ui.route_chat_text_hotkey("chat.text.unknown", true, true));
        for available in [false, true] {
            ui.chat_read_only[ui.channel] = available;
            assert!(ui.route_chat_text_hotkey("chat.text.select_all", available, true));
            assert!(!ui.chat_select_all_request);
        }
        Ok(())
    }

    fn draw_copy_composer(ctx: &egui::Context, ui: &mut super::GameUi) -> Option<egui::Id> {
        egui::Window::new("copy-chat-test")
            .show(ctx, |frame_ui| {
                let recipient = frame_ui.text_edit_singleline(&mut ui.recipient);
                ui.observe_chat_text_focus(&recipient);
                let edit = frame_ui.text_edit_singleline(&mut ui.draft);
                ui.apply_chat_composer_focus(&edit, false);
                edit.id
            })
            .and_then(|window| window.inner)
    }

    fn chat_copy_fixture() -> Result<(egui::Context, super::GameUi, egui::Id), &'static str> {
        let ctx = egui::Context::default();
        let mut ui = super::GameUi {
            draft: "prefix Zażółć 🦀 suffix".into(),
            recipient: "recipient".into(),
            chat_mode: ChatMode::On,
            ..Default::default()
        };
        let (id, _) = frame(&ctx, egui::vec2(400.0, 100.0), Vec::new(), |ctx| {
            draw_copy_composer(ctx, &mut ui)
        });
        Ok((ctx, ui, id.ok_or("composer")?))
    }

    fn select_copy_text(
        ctx: &egui::Context,
        id: egui::Id,
        start: usize,
        end: usize,
    ) -> Result<(), &'static str> {
        let mut state = egui::TextEdit::load_state(ctx, id).ok_or("selection state")?;
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(start),
                egui::text::CCursor::new(end),
            )));
        state.store(ctx, id);
        ctx.memory_mut(|memory| memory.request_focus(id));
        Ok(())
    }

    #[test]
    fn chat_copy_hotkey_emits_real_unicode_selection_without_editing_or_sending()
    -> Result<(), &'static str> {
        let (ctx, mut ui, composer) = chat_copy_fixture()?;
        // Reversed selection and a four-byte scalar must use character offsets,
        // not byte slicing or the entire draft/received channel contents.
        select_copy_text(&ctx, composer, 15, 7)?;
        let before = ui.draft.clone();
        assert!(ui.route_chat_text_hotkey("chat.text.copy", true, true));
        let events = ui.take_chat_copy_event(&ctx).into_iter().collect();
        assert!(!ui.chat_copy_request);
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(composer));
        let (_, output) = frame(&ctx, egui::vec2(400.0, 100.0), events, |ctx| {
            draw_copy_composer(ctx, &mut ui)
        });
        let copies: Vec<_> = output
            .platform_output
            .commands
            .iter()
            .filter_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(copies, vec!["Zażółć 🦀"]);
        assert_eq!(ui.draft, before);
        assert_eq!(ui.recipient, "recipient");
        assert_eq!(ui.chat_mode, ChatMode::On);
        assert_eq!(ui.chat_focus_request, None);
        assert_eq!(ui.notice, None);
        let state = egui::TextEdit::load_state(&ctx, composer).ok_or("selection state")?;
        assert_eq!(
            state
                .cursor
                .char_range()
                .ok_or("selection")?
                .sorted_cursors(),
            [egui::text::CCursor::new(7), egui::text::CCursor::new(15)]
        );
        let (_, repeated) = frame(&ctx, egui::vec2(400.0, 100.0), Vec::new(), |ctx| {
            draw_copy_composer(ctx, &mut ui)
        });
        assert!(
            repeated
                .platform_output
                .commands
                .iter()
                .all(|command| { !matches!(command, egui::OutputCommand::CopyText(_)) })
        );
        Ok(())
    }

    #[test]
    fn chat_copy_hotkey_observes_earlier_queued_text_and_selection_in_event_order()
    -> Result<(), &'static str> {
        let (ctx, mut ui, composer) = chat_copy_fixture()?;
        select_copy_text(&ctx, composer, 7, 15)?;
        assert!(ui.route_chat_text_hotkey("chat.text.copy", true, true));
        let copy = ui.take_chat_copy_event(&ctx).ok_or("native Copy event")?;
        let (_, output) = frame(
            &ctx,
            egui::vec2(400.0, 100.0),
            vec![
                egui::Event::Text("updated 🦀".into()),
                egui::Event::Key {
                    key: egui::Key::Home,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers {
                        shift: true,
                        ..Default::default()
                    },
                },
                copy,
                egui::Event::Text("after".into()),
            ],
            |ctx| draw_copy_composer(ctx, &mut ui),
        );
        let copies: Vec<_> = output
            .platform_output
            .commands
            .iter()
            .filter_map(|command| match command {
                egui::OutputCommand::CopyText(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(copies, vec!["prefix updated 🦀"]);
        assert_eq!(ui.draft, "after suffix");
        assert_eq!(ui.recipient, "recipient");
        assert_eq!(ui.chat_mode, ChatMode::On);
        Ok(())
    }

    #[test]
    fn chat_copy_hotkey_rejects_empty_stale_read_only_and_unavailable_selections()
    -> Result<(), &'static str> {
        let (ctx, mut ui, composer) = chat_copy_fixture()?;
        for (available, read_only, start, end) in [
            (false, false, 7, 15),
            (true, true, 7, 15),
            (true, false, 7, 7),
        ] {
            select_copy_text(&ctx, composer, start, end)?;
            ui.chat_read_only[ui.channel] = read_only;
            assert!(ui.route_chat_text_hotkey("chat.text.copy", available, true));
            let events = ui.take_chat_copy_event(&ctx).into_iter().collect();
            let (_, output) = frame(&ctx, egui::vec2(400.0, 100.0), events, |ctx| {
                draw_copy_composer(ctx, &mut ui)
            });
            assert!(
                output
                    .platform_output
                    .commands
                    .iter()
                    .all(|command| { !matches!(command, egui::OutputCommand::CopyText(_)) })
            );
            assert!(!ui.chat_copy_request);
            assert_eq!(ui.draft, "prefix Zażółć 🦀 suffix");
            assert_eq!(ui.recipient, "recipient");
            assert_eq!(ui.notice, None);
        }
        // Recheck ownership at the native flush even if it changed after claim.
        ui.chat_read_only[ui.channel] = false;
        select_copy_text(&ctx, composer, 7, 15)?;
        assert!(ui.route_chat_text_hotkey("chat.text.copy", true, true));
        ui.chat_read_only[ui.channel] = true;
        assert_eq!(ui.take_chat_copy_event(&ctx), None);
        ui.chat_read_only[ui.channel] = false;
        select_copy_text(&ctx, composer, 7, 15)?;
        ui.draft.clear();
        assert!(ui.route_chat_text_hotkey("chat.text.copy", true, true));
        let events = ui.take_chat_copy_event(&ctx).into_iter().collect();
        let (_, output) = frame(&ctx, egui::vec2(400.0, 100.0), events, |ctx| {
            draw_copy_composer(ctx, &mut ui)
        });
        assert!(
            output
                .platform_output
                .commands
                .iter()
                .all(|command| { !matches!(command, egui::OutputCommand::CopyText(_)) })
        );
        Ok(())
    }

    #[test]
    fn chat_copy_hotkey_preserves_other_focus_and_cancels_when_chat_is_hidden()
    -> Result<(), &'static str> {
        let (ctx, mut ui, composer) = chat_copy_fixture()?;
        let recipient = ui.chat_recipient_id.ok_or("recipient")?;
        let (received, _) = frame(&ctx, egui::vec2(400.0, 100.0), Vec::new(), |ctx| {
            egui::Area::new("received-copy-test".into())
                .show(ctx, |frame_ui| {
                    crate::chat_output::show(
                        frame_ui,
                        0,
                        "received history",
                        &oteryn_client::settings::ClientSettings::default(),
                    )
                })
                .inner
                .response
                .id
        });
        for focus in [
            Some(recipient),
            Some(received),
            Some(egui::Id::new("modal-editor")),
            None,
        ] {
            select_copy_text(&ctx, composer, 7, 15)?;
            ctx.memory_mut(|memory| {
                memory.surrender_focus(composer);
                if let Some(id) = focus {
                    memory.request_focus(id);
                }
            });
            assert!(!ui.chat_copy_routes_keyboard(&ctx));
            assert!(ui.route_chat_text_hotkey("chat.text.copy", true, true));
            assert_eq!(ui.take_chat_copy_event(&ctx), None);
            assert_eq!(ctx.memory(|memory| memory.focused()), focus);
            let (_, output) = frame(&ctx, egui::vec2(400.0, 100.0), Vec::new(), |ctx| {
                draw_copy_composer(ctx, &mut ui)
            });
            assert!(
                output
                    .platform_output
                    .commands
                    .iter()
                    .all(|command| { !matches!(command, egui::OutputCommand::CopyText(_)) })
            );
        }
        select_copy_text(&ctx, composer, 7, 15)?;
        assert!(ui.route_chat_text_hotkey("chat.text.copy", true, true));
        assert!(ui.chat_copy_request);
        ui.relinquish_hidden_chat_focus(&ctx);
        assert!(!ui.chat_copy_request);
        assert_eq!(ui.take_chat_copy_event(&ctx), None);
        assert_eq!(ctx.memory(|memory| memory.focused()), None);
        let (_, output) = frame(&ctx, egui::vec2(400.0, 100.0), Vec::new(), |ctx| {
            draw_copy_composer(ctx, &mut ui)
        });
        assert!(
            output
                .platform_output
                .commands
                .iter()
                .all(|command| { !matches!(command, egui::OutputCommand::CopyText(_)) })
        );
        assert_eq!(ui.chat_mode, ChatMode::Off);
        assert_eq!(ui.draft, "prefix Zażółć 🦀 suffix");
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
    fn unavailable_legacy_actions_do_not_claim_input_or_enqueue_commands() {
        let settings = oteryn_client::settings::ClientSettings::default();
        let (link, mut commands, _outbox) = oteryn_client::play::play_channel();
        for chat_on in [false, true] {
            let mut ui = super::GameUi {
                draft: "ordinary input".into(),
                ..Default::default()
            };
            assert!(!ui.route_actions(
                &[],
                &[
                    oteryn_client::hotkeys::ResolvedHotkey::Custom(
                        oteryn_client::hotkeys::CustomHotkeyAction::Spell {
                            spell_key: "legacy".into(),
                            parameter: String::new(),
                        }
                    ),
                    oteryn_client::hotkeys::ResolvedHotkey::Custom(
                        oteryn_client::hotkeys::CustomHotkeyAction::Object {
                            item_definition_ref: 3003,
                            use_mode: oteryn_client::hotkeys::ObjectUseMode::Use,
                        }
                    ),
                ],
                &link,
                &settings,
                chat_on,
                false
            ));
            assert_eq!(ui.draft, "ordinary input");
            assert!(ui.notice.is_none());
            assert!(commands.try_recv().is_err());
            assert!(link.action_result().is_none());
            assert!(link.chat_result().is_none());
        }
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
    fn chat_clock_consumes_receipt_time_and_clear_preserves_new_identical_messages() {
        use oteryn_client::{
            play::ChatReceipt, settings::ClientSettings, settings_catalog::FutureValue,
        };
        let chat = oteryn_session::ChatLog::default();
        let line = oteryn_session::ChatLine::Private {
            speaker_name: "Peer".into(),
            text: "same".into(),
        };
        let receipts = vec![
            ChatReceipt {
                server_sequence: 41,
                line: line.clone(),
                local_time: Some([9, 7, 3]),
            },
            ChatReceipt {
                server_sequence: 42,
                line,
                local_time: Some([9, 7, 4]),
            },
        ];
        let mut settings = ClientSettings::default();
        assert_eq!(
            chat_rows(&chat, &receipts, 5, true, &settings, None),
            ["Peer: same", "Peer: same"]
        );
        settings
            .future_preferences
            .insert("console.console.timestamps".into(), FutureValue::Bool(true));
        assert_eq!(
            chat_rows(&chat, &receipts, 5, true, &settings, None),
            ["09:07 Peer: same", "09:07 Peer: same"]
        );
        settings
            .future_preferences
            .insert("console.console.seconds".into(), FutureValue::Bool(true));
        assert_eq!(
            chat_rows(&chat, &receipts, 5, true, &settings, Some(41)),
            ["09:07:04 Peer: same"]
        );
        assert!(chat_rows(&chat, &receipts, 5, true, &settings, Some(42)).is_empty());
        // Another tab has an independent cutoff; clearing cannot mutate receipts.
        assert_eq!(
            chat_rows(&chat, &receipts, 5, true, &settings, None).len(),
            2
        );
        assert!(chat_rows(&chat, &receipts, 0, true, &settings, None).is_empty());
        let unknown = [ChatReceipt {
            server_sequence: 43,
            line: receipts[0].line.clone(),
            local_time: None,
        }];
        assert_eq!(
            chat_rows(&chat, &unknown, 5, true, &settings, Some(42)),
            ["Peer: same"]
        );
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
