//! In-world UI consumes session projections; it never invents character state.
use egui::{Color32, RichText};
use oteryn_client::{play::PlayLink, settings::ClientSettings};
use oteryn_session::{ChatIntent, ChatLine, ChatRoom, ChatSpeechMode, EntityDetail, ItemEntry};

#[derive(Default)]
pub struct GameUi {
    draft: String,
    recipient: String,
    channel: usize,
    notice: Option<String>,
}

impl GameUi {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        link: &PlayLink,
        settings: &ClientSettings,
    ) -> bool {
        let state = link.game_state();
        let rect = ctx.content_rect();
        let en = settings.english;
        let tr = |pl, english| if en { english } else { pl };
        let mut open_settings = false;
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
                                if settings.show_inventory {
                                    ui.heading(tr("Ekwipunek", "Inventory"));
                                    if let Some(inventory) = &state.inventory {
                                        for equipped in &inventory.equipment {
                                            ui.small(format!("{:?}", equipped.slot));
                                            item(ui, &equipped.item);
                                        }
                                        if let Some(backpack) = &inventory.main_backpack {
                                            item(ui, backpack);
                                        }
                                        ui.separator();
                                        ui.strong(tr("Plecak", "Backpack"));
                                        if inventory.entries.is_empty() {
                                            ui.small(tr("Pusty", "Empty"));
                                        }
                                        for entry in &inventory.entries {
                                            item(ui, entry);
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
                                            item(ui, entry);
                                        }
                                    }
                                }
                                if settings.show_battle {
                                    ui.separator();
                                    ui.heading(tr("Widoczne postacie", "Visible actors"));
                                    for entity in &state.entities {
                                        if let EntityDetail::Actor { health_percent, .. } =
                                            entity.detail
                                        {
                                            ui.label(format!(
                                                "{:?} · {}, {}",
                                                entity.kind, entity.position.x, entity.position.y
                                            ));
                                            ui.add(
                                                egui::ProgressBar::new(
                                                    f32::from(health_percent) / 100.0,
                                                )
                                                .text(format!("{health_percent}%")),
                                            );
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
                            if let Some(notice) = &self.notice {
                                ui.small(notice);
                            }
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

fn item(ui: &mut egui::Ui, item: &ItemEntry) {
    ui.label(format!("#{} × {}", item.item_definition_ref, item.count))
        .on_hover_text(format!("Subtype: {}", item.sub_type));
}
