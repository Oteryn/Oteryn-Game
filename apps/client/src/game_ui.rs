//! In-world UI consumes session projections; it never invents character state.
use egui::{Color32, RichText};
use oteryn_client::action_bar::{ActionBar, ActionBarCommand, ActionBarOutcome};
use oteryn_client::layout::{HUD_CHAT_HEIGHT, HUD_MARGIN, HUD_SIDEBAR_WIDTH, HUD_TOP_HEIGHT};
use oteryn_client::minimap::{LoadedMinimap, MINIMAP_SIDE, MinimapZoom};
use oteryn_client::{
    play::{PlayLink, PlayView},
    settings::ClientSettings,
};
use oteryn_session::{
    ChatDisposition, ChatIntent, ChatLine, ChatRoom, ChatSpeechMode, EntityDetail, EntityKind,
    EquipmentSlot, ItemEntry, ItemMoveOutcome, UseDisposition,
};

#[derive(Default)]
pub struct GameUi {
    draft: String,
    recipient: String,
    channel: usize,
    notice: Option<String>,
    action_bar: Option<ActionBar>,
    minimap: Option<(LoadedMinimap, egui::TextureHandle)>,
    minimap_zoom: usize,
    minimap_retry_at: f64,
    panels: crate::client_panels::ClientPanels,
    dialogs: crate::reference_dialogs::ReferenceDialogs,
    preferences_changes: Option<ClientSettings>,
}

impl GameUi {
    pub fn blocks_game_input(&self) -> bool {
        self.panels.manage || self.dialogs.any_open()
    }

    pub fn preferences_save_failed(&mut self, english: bool) {
        self.notice = Some(
            if english {
                "Panel shortcuts were not saved. Check access to the preferences directory."
            } else {
                "Nie zapisano skrótów paneli. Sprawdź dostęp do folderu ustawień."
            }
            .into(),
        );
    }
    pub fn take_preferences(&mut self) -> Option<ClientSettings> {
        self.preferences_changes.take()
    }
    pub fn clear_action_row(&mut self, row: usize, settings: &ClientSettings) {
        if self
            .bar(settings)
            .is_none_or(|bar| bar.clear_row(row).is_err())
        {
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
    fn bar(&mut self, settings: &ClientSettings) -> Option<&mut ActionBar> {
        if self.action_bar.is_none() {
            self.action_bar =
                ActionBar::new(settings.action_bar.clone(), &settings.movement_keys).ok();
        }
        let bar = self.action_bar.as_mut()?;
        if bar.preferences() != &settings.action_bar {
            let _ = bar.reconfigure(settings.action_bar.clone(), &settings.movement_keys);
        }
        Some(bar)
    }

    pub fn route_actions(
        &mut self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        link: &PlayLink,
        settings: &ClientSettings,
        typing: bool,
        modal: bool,
    ) {
        if let Some(bar) = self.bar(settings) {
            if bar.set_input_context(typing, modal).is_err() {
                return;
            }
            for command in bar.route(events) {
                if !link.send_action(command) {
                    self.notice = Some(action_busy(settings.english).into());
                }
            }
        }
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
            crate::actor_hud::show(
                ctx,
                scene,
                view.scene().view().tile_to_screen(view.own()),
                settings,
                state.vitals,
            );
        }
        let commands = self.bar(settings).map_or_else(Vec::new, |bar| {
            // Available spells need an authoritative catalogue projection.
            crate::action_bar_ui::show(ctx, bar, &state, &[], en, body, hud_enabled)
        });
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
                    if ui
                        .small_button(tr("Ustawienia · F10", "Settings · F10"))
                        .clicked()
                    {
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
                                    && let Some(crop) = map.viewport(
                                        view.minimap_location(),
                                        oteryn_client::world::START_FLOOR,
                                        zoom,
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
                                for (index, label) in [
                                    tr("Lokalny", "Local"),
                                    "World",
                                    "English",
                                    "Help",
                                    "Advertising",
                                    tr("Prywatny", "Private"),
                                ]
                                .iter()
                                .enumerate()
                                {
                                    ui.selectable_value(&mut self.channel, index, *label);
                                }
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
                                    egui::TextEdit::singleline(&mut self.draft)
                                        .hint_text(tr("Wiadomość", "Message"))
                                        .desired_width((ui.available_width() - 80.0).max(50.0)),
                                );
                                ui.add_enabled(false, egui::Button::new(tr("Wyślij", "Send")));
                            });
                            return;
                        };
                        ui.horizontal(|ui| {
                            for (index, label) in [
                                tr("Lokalny", "Local"),
                                "World",
                                "English",
                                "Help",
                                "Advertising",
                                tr("Prywatny", "Private"),
                            ]
                            .iter()
                            .enumerate()
                            {
                                ui.selectable_value(&mut self.channel, index, *label);
                            }
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
                                    let text = match line {
                                        ChatLine::Local {
                                            speaker_name, text, ..
                                        } if self.channel == 0 => {
                                            Some(format!("{speaker_name}: {text}"))
                                        }
                                        ChatLine::Private { speaker_name, text }
                                            if self.channel == 5 =>
                                        {
                                            Some(format!("{speaker_name}: {text}"))
                                        }
                                        ChatLine::Room {
                                            room: line_room,
                                            speaker_name,
                                            text,
                                        } if room == Some(*line_room) => {
                                            Some(format!("{speaker_name}: {text}"))
                                        }
                                        ChatLine::Dropped => Some(
                                            tr(
                                                "Pominięto starsze wiadomości",
                                                "Older messages were dropped",
                                            )
                                            .into(),
                                        ),
                                        _ => None,
                                    };
                                    if let Some(text) = text {
                                        ui.label(text);
                                    }
                                }
                            });
                        ui.horizontal(|ui| {
                            if self.channel == 5 {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.recipient)
                                        .desired_width(100.0)
                                        .hint_text(tr("Odbiorca", "Recipient")),
                                );
                            }
                            let edit = ui.add(
                                egui::TextEdit::singleline(&mut self.draft)
                                    .desired_width((ui.available_width() - 65.0).max(30.0))
                                    .hint_text(tr("Napisz wiadomość", "Write a message")),
                            );
                            let submit = ui.button(tr("Wyślij", "Send")).clicked()
                                || edit.lost_focus()
                                    && ui.input(|input| input.key_pressed(egui::Key::Enter));
                            if submit && !self.draft.trim().is_empty() {
                                let text = self.draft.trim().to_owned();
                                if text.len() > oteryn_session::MAX_CHAT_TEXT_BYTES {
                                    self.notice = Some(
                                        tr("Wiadomość jest zbyt długa", "Message is too long")
                                            .into(),
                                    );
                                } else if self.channel == 5
                                    && (self.recipient.trim().is_empty()
                                        || self.recipient.len()
                                            > oteryn_session::MAX_CHAT_NAME_BYTES)
                                {
                                    self.notice = Some(
                                        tr(
                                            "Podaj prawidłowego odbiorcę",
                                            "Enter a valid recipient",
                                        )
                                        .into(),
                                    );
                                } else {
                                    let intent = if self.channel == 5 {
                                        ChatIntent::Private {
                                            recipient_name: self.recipient.trim().into(),
                                            text,
                                        }
                                    } else if let Some(room) = room {
                                        ChatIntent::Room { room, text }
                                    } else {
                                        ChatIntent::Say {
                                            mode: ChatSpeechMode::Say,
                                            text,
                                        }
                                    };
                                    if link.send_chat(intent) {
                                        self.draft.clear();
                                        self.notice = None;
                                        edit.request_focus();
                                    } else {
                                        self.notice = Some(
                                            tr(
                                                "Poczekaj na poprzednią wiadomość",
                                                "Wait for the previous message",
                                            )
                                            .into(),
                                        );
                                    }
                                }
                            }
                        });
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
            let mut next = settings.clone();
            next.panel_shortcuts = shortcuts;
            self.preferences_changes = Some(next);
        }
        open_settings
    }
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
                .button(tr("Ustawienia · F10", "Settings · F10"))
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
        .inner_margin(1.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(label).size(10.0).strong());
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
    use super::{item_button, show_window_recovery};
    use crate::preferences_browser::test_support::{button_position, click, frame};

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
                let label = if english {
                    "Settings · F10"
                } else {
                    "Ustawienia · F10"
                };
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
