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
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
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
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                        ui.set_width(198.0);
                        egui::ScrollArea::vertical()
                            .max_height((rect.height() - 70.0).max(10.0))
                            .show(ui, |ui| {
                                if settings.show_minimap {
                                    ui.strong(tr("Minimapa", "Minimap"));
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
                                            egui::vec2(128.0, 128.0),
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
                                    ui.strong(tr("Ekwipunek", "Inventory"));
                                    equipment(ui, state.inventory.as_ref(), link, en);
                                    if let Some(inventory) = &state.inventory {
                                        ui.separator();
                                        ui.strong(tr("Plecak", "Backpack"));
                                        if inventory.entries.is_empty() {
                                            ui.small(tr("Pusty", "Empty"));
                                        }
                                        item_grid(
                                            ui,
                                            "main-backpack",
                                            &inventory.entries,
                                            link,
                                            en,
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
                                        ui.strong(tr("Otwarty kontener", "Open container"));
                                        item_grid(
                                            ui,
                                            "open-container",
                                            &container.entries,
                                            link,
                                            en,
                                        );
                                    }
                                }
                                if settings.show_battle {
                                    ui.separator();
                                    ui.strong(tr("Lista walki", "Battle list"));
                                    for entity in &state.entities {
                                        if let EntityDetail::Actor { .. } = entity.detail {
                                            ui.label(format!(
                                                "{} · {}, {}",
                                                actor_kind(entity.kind, en),
                                                entity.position.x,
                                                entity.position.y
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
                                ui.set_min_height(135.0);
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
                                ui.allocate_ui(egui::vec2(ui.available_width(), 55.0), |ui| {
                                    ui.weak(tr(
                                        "Kanały czatu są niedostępne w tej sesji.",
                                        "Chat channels are unavailable in this session.",
                                    ));
                                });
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
                                    "{} · {} s",
                                    chat_feedback(result.disposition, en),
                                    result.wait_seconds
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

fn ratio(value: u32, maximum: u32) -> f32 {
    if maximum == 0 {
        0.0
    } else {
        (value as f32 / maximum as f32).clamp(0.0, 1.0)
    }
}

fn equipment(
    ui: &mut egui::Ui,
    inventory: Option<&oteryn_session::CharacterInventory>,
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
        .spacing([4.0, 4.0])
        .show(ui, |ui| {
            for (index, slot) in slots.into_iter().enumerate() {
                if slot.is_none() && index != 2 {
                    ui.allocate_space(egui::vec2(42.0, 38.0));
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
                        ui.painter().text(
                            response.rect.right_bottom() - egui::vec2(3.0, 3.0),
                            egui::Align2::RIGHT_BOTTOM,
                            item.count.to_string(),
                            egui::FontId::proportional(11.0),
                            Color32::from_rgb(231, 210, 162),
                        );
                        response
                            .clone()
                            .on_hover_text(item_description(item, english));
                        item_menu(response, item, link, english);
                    } else {
                        response.on_hover_text(if english {
                            "Item data is unavailable."
                        } else {
                            "Dane przedmiotu są niedostępne."
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
        .spacing([4.0, 4.0])
        .show(ui, |ui| {
            for (index, item) in items.iter().enumerate() {
                let response = ui
                    .add_sized([42.0, 34.0], egui::Button::new(item.count.to_string()))
                    .on_hover_text(item_description(item, english));
                item_menu(response, item, link, english);
                if index % 4 == 3 {
                    ui.end_row();
                }
            }
        });
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
