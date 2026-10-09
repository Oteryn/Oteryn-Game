//! In-world UI consumes session projections; it never invents character state.
use egui::{Color32, RichText};
use oteryn_client::action_bar::{
    ACTION_BAR_SLOTS, ActionBar, ActionBarCommand, ActionBarOutcome, SlotAssignment,
};
use oteryn_client::minimap::{LoadedMinimap, MINIMAP_SIDE, MinimapZoom};
use oteryn_client::{
    play::{PlayLink, PlayView},
    settings::ClientSettings,
};
use oteryn_session::{ChatIntent, ChatLine, ChatRoom, ChatSpeechMode, EntityDetail, ItemEntry};

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
                let _ = link.send_action(command);
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
        if settings.action_bar.visible {
            let chat_height = if settings.show_chat { 160.0 } else { 12.0 };
            let commands = {
                let bar = self.bar(settings);
                let mut commands = Vec::new();
                egui::Area::new("game-actionbar".into())
                    .fixed_pos(egui::pos2(
                        rect.left() + 12.0,
                        rect.bottom() - chat_height - 50.0,
                    ))
                    .show(ctx, |ui| {
                        egui::Frame::group(ui.style())
                            .fill(Color32::from_rgb(12, 20, 26))
                            .show(ui, |ui| {
                                if let Some(bar) = bar {
                                    ui.horizontal(|ui| {
                                        for slot in 0..ACTION_BAR_SLOTS {
                                            let label = bar
                                                .assignment(slot)
                                                .map_or("—", |a| a.label.as_str());
                                            let response = ui
                                                .add_sized(
                                                    [38.0, 34.0],
                                                    egui::Button::new(format!("{}", slot + 1)),
                                                )
                                                .on_hover_text(label);
                                            if response.clicked()
                                                && let Some(command) = bar.activate(slot)
                                            {
                                                commands.push(command);
                                            }
                                            response.context_menu(|ui| {
                                                ui.label(tr("Przypisz akcję", "Assign action"));
                                                if bar.preferences().locked {
                                                    ui.disable();
                                                }
                                                for (name, room) in [
                                                    (tr("Pomoc", "Help"), ChatRoom::Help),
                                                    (tr("Świat", "World"), ChatRoom::World),
                                                    ("English", ChatRoom::English),
                                                    (
                                                        tr("Reklamy", "Advertising"),
                                                        ChatRoom::Advertising,
                                                    ),
                                                ] {
                                                    if ui.button(name).clicked() {
                                                        let _ = bar.assign(
                                                            slot,
                                                            Some(SlotAssignment {
                                                                label: name.into(),
                                                                command: ActionBarCommand::Chat(
                                                                    ChatIntent::OpenRoom(room),
                                                                ),
                                                            }),
                                                        );
                                                        ui.close();
                                                    }
                                                }
                                                if ui.button(tr("Wyczyść", "Clear")).clicked() {
                                                    let _ = bar.assign(slot, None);
                                                    ui.close();
                                                }
                                            });
                                        }
                                    });
                                }
                            });
                    });
                commands
            };
            for command in commands {
                let _ = link.send_action(command);
            }
        }
        egui::Area::new("game-header".into())
            .fixed_pos(rect.min)
            .show(ctx, |ui| {
                egui::Frame::group(ui.style())
                    .fill(Color32::from_rgb(12, 20, 26))
                    .show(ui, |ui| {
                        ui.set_width((rect.width() - 20.0).max(1.0));
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new("OTERYN")
                                    .strong()
                                    .color(Color32::from_rgb(220, 183, 111)),
                            );
                            if let Some(vitals) = state.vitals {
                                ui.add(
                                    egui::ProgressBar::new(ratio(vitals.health, vitals.max_health))
                                        .desired_width(145.0)
                                        .fill(Color32::from_rgb(174, 55, 58))
                                        .text(format!(
                                            "HP {} / {}",
                                            vitals.health, vitals.max_health
                                        )),
                                );
                                ui.add(
                                    egui::ProgressBar::new(ratio(vitals.mana, vitals.max_mana))
                                        .desired_width(145.0)
                                        .fill(Color32::from_rgb(56, 100, 179))
                                        .text(format!("MP {} / {}", vitals.mana, vitals.max_mana)),
                                );
                                ui.small(format!(
                                    "Soul {} · Harmony {}",
                                    vitals.soul, vitals.harmony
                                ));
                            }
                            if ui
                                .button(tr("Ustawienia · F10", "Settings · F10"))
                                .clicked()
                            {
                                open_settings = true;
                            }
                            if ui
                                .button("+")
                                .on_hover_text(tr("Panele i skróty", "Panels and shortcuts"))
                                .clicked()
                            {
                                self.panels.manage = true;
                            }
                            if ui.button(tr("Więcej", "More")).clicked() {
                                self.dialogs.manage = true;
                            }
                        });
                    });
            });
        egui::Area::new("game-sidebar".into())
            .fixed_pos(egui::pos2(rect.right() - 220.0, rect.top() + 52.0))
            .show(ctx, |ui| {
                egui::Frame::group(ui.style())
                    .fill(Color32::from_rgb(12, 20, 26))
                    .show(ui, |ui| {
                        ui.set_width(198.0);
                        egui::ScrollArea::vertical()
                            .max_height((rect.height() - 70.0).max(10.0))
                            .show(ui, |ui| {
                                self.panels.shortcuts(ui, en);
                                if settings.show_minimap {
                                    ui.heading(tr("Minimapa", "Minimap"));
                                    egui::ComboBox::from_id_salt("minimap-zoom")
                                        .selected_text(
                                            [
                                                tr("Blisko", "Nearby"),
                                                tr("Okolica", "Region"),
                                                tr("Wczytany obszar", "Loaded area"),
                                            ][self.minimap_zoom],
                                        )
                                        .show_ui(ui, |ui| {
                                            for (index, label) in [
                                                tr("Blisko", "Nearby"),
                                                tr("Okolica", "Region"),
                                                tr("Wczytany obszar", "Loaded area"),
                                            ]
                                            .into_iter()
                                            .enumerate()
                                            {
                                                ui.selectable_value(
                                                    &mut self.minimap_zoom,
                                                    index,
                                                    label,
                                                );
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
                                            egui::vec2(180.0, 180.0),
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
                                if settings.show_inventory {
                                    ui.heading(tr("Ekwipunek", "Inventory"));
                                    if let Some(inventory) = &state.inventory {
                                        for equipped in &inventory.equipment {
                                            ui.small(format!("{:?}", equipped.slot));
                                            item(ui, &equipped.item, link);
                                        }
                                        if let Some(backpack) = &inventory.main_backpack {
                                            item(ui, backpack, link);
                                        }
                                        ui.separator();
                                        ui.strong(tr("Plecak", "Backpack"));
                                        if inventory.entries.is_empty() {
                                            ui.small(tr("Pusty", "Empty"));
                                        }
                                        for entry in &inventory.entries {
                                            item(ui, entry, link);
                                        }
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
                                        ui.strong(tr("Otwarty kontener", "Open container"));
                                        for entry in &container.entries {
                                            item(ui, entry, link);
                                        }
                                    }
                                }
                                if settings.show_battle {
                                    ui.separator();
                                    ui.heading(tr("Widoczne postacie", "Visible actors"));
                                    for entity in &state.entities {
                                        if let EntityDetail::Actor { .. } = entity.detail {
                                            ui.label(format!(
                                                "{:?} · {}, {}",
                                                entity.kind, entity.position.x, entity.position.y
                                            ));
                                        }
                                    }
                                }
                            });
                    });
            });
        if settings.show_chat {
            egui::Area::new("game-console".into())
                .fixed_pos(egui::pos2(rect.left(), rect.bottom() - 160.0))
                .show(ctx, |ui| {
                    egui::Frame::group(ui.style())
                        .fill(Color32::from_rgb(12, 20, 26))
                        .show(ui, |ui| {
                            ui.set_width((rect.width() - 240.0).max(100.0));
                            let Some(chat) = &state.chat else {
                                ui.label(tr(
                                    "Czat nie jest dostępny w tej sesji",
                                    "Chat is unavailable in this session",
                                ));
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
                                .max_height(65.0)
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
                                    "{:?} · {} s",
                                    result.disposition, result.wait_seconds
                                ));
                            }
                        });
                });
        }
        egui::Area::new("game-feedback".into())
            .anchor(egui::Align2::LEFT_TOP, [12.0, 42.0])
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
                            format!("{:?}", result.disposition)
                        }
                        ActionBarOutcome::Moved(result) => {
                            format!("{:?}", result.outcome)
                        }
                        ActionBarOutcome::Chat(result) => {
                            format!("{:?}", result.disposition)
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
        self.dialogs.show(ctx, en);
        if let Some(shortcuts) = self.panels.take_shortcuts() {
            let mut next = settings.clone();
            next.panel_shortcuts = shortcuts;
            self.preferences_changes = Some(next);
        }
        open_settings
    }
}

fn ratio(value: u32, maximum: u32) -> f32 {
    if maximum == 0 {
        0.0
    } else {
        (value as f32 / maximum as f32).clamp(0.0, 1.0)
    }
}

fn item(ui: &mut egui::Ui, item: &ItemEntry, link: &PlayLink) {
    ui.label(format!("#{} × {}", item.item_definition_ref, item.count))
        .on_hover_text(format!("Subtype: {}", item.sub_type))
        .context_menu(|ui| {
            if ui
                .button("Przenieś do plecaka / Move to backpack")
                .clicked()
            {
                let _ = link.send_action(ActionBarCommand::MoveToBackpack(item.handle));
                ui.close();
            }
        });
}
