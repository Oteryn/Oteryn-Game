//! Dedicated compositions from private reference captures; no reference assets shipped.
use super::{boxed, reference_option};
#[path = "preferences_display_pages.rs"]
mod display;
#[path = "preferences_hotkey_pages.rs"]
mod hotkeys;
#[path = "preferences_hud_pages.rs"]
mod hud;
use oteryn_client::{
    panel_catalog::{PANELS, ShortcutOrder, panel},
    settings::ClientSettings,
    settings_catalog::FutureValue,
};

#[derive(Default)]
pub(super) struct PageState {
    displayed: Option<String>,
    available: Option<String>,
    hotkeys: hotkeys::State,
    help_action: Option<HelpAction>,
    help_reset_confirmation: bool,
    help_info: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HelpAction {
    OpenCompendium,
    Export,
    Import,
    Reset,
    OpenScreenshotFolder,
}

impl PageState {
    pub(super) fn has_inner_dialog(&self) -> bool {
        self.help_reset_confirmation || self.hotkeys.has_inner_dialog()
    }

    pub(super) fn handle_escape(&mut self) -> bool {
        if self.help_reset_confirmation {
            self.help_reset_confirmation = false;
            return true;
        }
        self.hotkeys.handle_escape()
    }

    pub(super) fn take_help_action(&mut self) -> Option<HelpAction> {
        self.help_action.take()
    }
}

pub(super) fn show(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    section: &str,
    english: bool,
    compendium_available: bool,
    state: &mut PageState,
) -> bool {
    match section {
        "shortcuts" => shortcuts(ui, draft, english, state),
        "console" => console(ui, draft, english),
        "effects" => effects(ui, draft, english),
        "sound" => sound(ui, draft, english),
        "battle_sounds" => battle(ui, draft, english),
        "ui_sounds" => ui_sound(ui, draft, english),
        "help" => help(ui, english, compendium_available, state),
        _ => {
            return hotkeys::show(ui, draft, section, english, &mut state.hotkeys)
                || hud::show(ui, draft, section, english, state)
                || display::show(ui, draft, section, english);
        }
    }
    true
}

fn help(ui: &mut egui::Ui, en: bool, compendium_available: bool, state: &mut PageState) {
    let unavailable = if en {
        "No verified Oteryn destination is configured for this reference action."
    } else {
        "Dla tej czynności referencyjnej nie skonfigurowano zweryfikowanego celu Oteryn."
    };
    let local = if en {
        "This action is handled locally by Oteryn."
    } else {
        "Ta czynność jest obsługiwana lokalnie przez Oteryn."
    };
    let minimap_unavailable = if en {
        "No versioned minimap import/export format is available yet."
    } else {
        "Brak jeszcze wersjonowanego formatu importu/eksportu minimapy."
    };
    let import_supported = if en {
        "Imports only an Oteryn options ZIP into the draft. Apply or Ok saves it. Original-client and minimap archives are unsupported."
    } else {
        "Importuje tylko ZIP ustawień Oteryn do wersji roboczej. Apply lub Ok zapisuje zmiany. Archiwa oryginalnego klienta i minimapy są nieobsługiwane."
    };

    boxed(ui, "", |ui| {
        for (id, pl, english, enabled) in [
            ("client-help", "Pomoc klienta", "Client Help", false),
            (
                "compendium",
                "Kompendium",
                "Compendium",
                compendium_available,
            ),
            (
                "rule-violations",
                "Naruszenia zasad",
                "Rule Violations",
                false,
            ),
            ("manual", "Instrukcja", "Manual", false),
            ("faq", "Najczęstsze pytania", "FAQ", false),
        ] {
            let response = help_row(
                ui,
                id,
                if en { english } else { pl },
                enabled,
                if enabled { local } else { unavailable },
            );
            if id == "compendium" && response.clicked() {
                state.help_action = Some(HelpAction::OpenCompendium);
            }
        }
        if help_row(
            ui,
            "info",
            if en { "Info" } else { "Informacje" },
            true,
            local,
        )
        .clicked()
        {
            state.help_info = !state.help_info;
        }

        if state.help_info {
            ui.separator();
            ui.label(format!(
                "Oteryn {} · protocol-oteryn",
                env!("CARGO_PKG_VERSION")
            ));
            ui.weak(if en {
                "Identity and authoritative game state remain server-owned."
            } else {
                "Tożsamość i autorytatywny stan gry pozostają po stronie serwera."
            });
        }
    });

    boxed(ui, "", |ui| {
        if help_row(
            ui,
            "export-options",
            if en {
                "Export All Options"
            } else {
                "Eksportuj wszystkie ustawienia"
            },
            true,
            local,
        )
        .clicked()
        {
            state.help_action = Some(HelpAction::Export);
        }
        help_row(
            ui,
            "export-minimap",
            if en {
                "Export Minimap"
            } else {
                "Eksportuj minimapę"
            },
            false,
            minimap_unavailable,
        );
        if help_row(
            ui,
            "import-options-minimap",
            if en {
                "Import Options/Minimap"
            } else {
                "Importuj ustawienia/minimapę"
            },
            true,
            import_supported,
        )
        .clicked()
        {
            state.help_action = Some(HelpAction::Import);
        }
        if help_row(
            ui,
            "reset-options",
            if en {
                "Reset All Options"
            } else {
                "Przywróć wszystkie ustawienia"
            },
            true,
            local,
        )
        .clicked()
        {
            state.help_reset_confirmation = true;
        }
    });

    if state.help_reset_confirmation {
        egui::Modal::new(egui::Id::new("reset-all-options-confirmation")).show(ui.ctx(), |ui| {
            ui.heading(if en {
                "Reset All Options"
            } else {
                "Przywróć wszystkie ustawienia"
            });
            ui.label(if en {
                "Reset every local option?"
            } else {
                "Przywrócić wszystkie ustawienia lokalne?"
            });
            ui.horizontal(|ui| {
                if ui.button(if en { "Reset" } else { "Przywróć" }).clicked() {
                    state.help_action = Some(HelpAction::Reset);
                    state.help_reset_confirmation = false;
                }
                if ui.button(if en { "Cancel" } else { "Anuluj" }).clicked() {
                    state.help_reset_confirmation = false;
                }
            });
        });
    }
}

fn help_row(
    ui: &mut egui::Ui,
    _id: &'static str,
    label: &str,
    enabled: bool,
    explanation: &str,
) -> egui::Response {
    ui.horizontal(|ui| {
        let response = ui
            .add_enabled(
                enabled,
                egui::Button::new(label).min_size(egui::vec2(132.0, 22.0)),
            )
            .on_hover_text(explanation);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, info) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
            let color = ui.visuals().text_color();
            ui.painter()
                .circle_stroke(rect.center(), 5.0, egui::Stroke::new(1.0, color));
            ui.painter().text(
                rect.center() + egui::vec2(0.0, -0.5),
                egui::Align2::CENTER_CENTER,
                "i",
                egui::FontId::proportional(9.0),
                color,
            );
            let _ = info.on_hover_text(explanation);
            #[cfg(test)]
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    egui::Id::new(("help-info-glyph", _id)),
                    (rect, ui.clip_rect()),
                );
            });
        });
        #[cfg(test)]
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                egui::Id::new(("help-action-row", _id)),
                (response.rect, ui.clip_rect(), response.enabled()),
            );
        });
        response
    })
    .inner
}

fn row(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    section: &str,
    id: &str,
    labels: [&str; 2],
    en: bool,
) {
    let key = format!("{section}.{id}");
    let explanation = match (oteryn_client::settings_catalog::runtime_consumer(&key), en) {
        (Some(consumer), true) => format!("Applied by {consumer}."),
        (Some(consumer), false) => format!("Używane przez: {consumer}."),
        (None, true) => "Unavailable: this control has no runtime consumer.".into(),
        (None, false) => "Niedostępne: ta kontrolka nie ma obsługi wykonawczej.".into(),
    };
    let mut info_rect = egui::Rect::NOTHING;
    let _output = egui::Frame::new()
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(2.0)
        .inner_margin(egui::Margin::symmetric(5, 1))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().interact_size.y = 20.0;
            let (content_rect, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 22.0), egui::Sense::hover());
            info_rect = egui::Rect::from_center_size(
                egui::pos2(content_rect.right() - 8.0, content_rect.center().y),
                egui::vec2(16.0, 16.0),
            );
            let option_rect = egui::Rect::from_min_max(
                content_rect.min,
                egui::pos2(info_rect.left() - 4.0, content_rect.bottom()),
            );
            let mut option_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(option_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
            );
            option_ui.set_clip_rect(option_rect.intersect(ui.clip_rect()));
            reference_option(&mut option_ui, draft, section, id, labels, en);

            let info = ui.interact(
                info_rect,
                egui::Id::new(("reference-row-info", key.as_str())),
                egui::Sense::hover(),
            );
            let color = ui.visuals().text_color();
            ui.painter()
                .circle_stroke(info_rect.center(), 5.0, egui::Stroke::new(1.0, color));
            ui.painter().text(
                info_rect.center() + egui::vec2(0.0, -0.5),
                egui::Align2::CENTER_CENTER,
                "i",
                egui::FontId::proportional(9.0),
                color,
            );
            let _ = info.on_hover_text(explanation);
        });
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("reference-row-geometry", key.as_str())),
            (_output.response.rect, ui.clip_rect(), info_rect),
        );
    });
}

pub(super) fn retained_children(
    ui: &mut egui::Ui,
    enabled: bool,
    gate: &'static str,
    body: impl FnOnce(&mut egui::Ui),
) {
    ui.push_id(("retained-child-scope", gate), |ui| {
        ui.add_enabled_ui(enabled, |ui| {
            #[cfg(test)]
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    egui::Id::new(("retained-child-gate", gate)),
                    ui.is_enabled(),
                );
            });
            body(ui);
        });
    });
}

// Unset is visibly unknown, never silently saved as an invented reference default.
fn percent(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    section: &str,
    id: &str,
    labels: [&str; 2],
    en: bool,
) -> egui::Response {
    let key = format!("{section}.{id}");
    let selected = match draft.future_preferences.get(&key) {
        Some(FutureValue::Int(value)) => Some(*value),
        _ => None,
    };
    let mut value = selected.unwrap_or(0);
    let off_at_minimum =
        id == "effects.cloud_light" || matches!(section, "sound" | "battle_sounds" | "ui_sounds");
    let text = selected.map_or_else(
        || "—".into(),
        |v| {
            if v == 0 && off_at_minimum {
                if en { "0 % (Off)" } else { "0 % (Wył.)" }.into()
            } else {
                format!("{v} %")
            }
        },
    );
    let consumer = oteryn_client::settings_catalog::runtime_consumer(&key);
    let response = ui
        .push_id(&key, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{} {text}", labels[usize::from(en)]));
                ui.spacing_mut().slider_width = ui.available_width().clamp(45.0, 185.0);
                ui.add_enabled_ui(consumer.is_some(), |ui| {
                    crate::preferences_browser::reference_slider::show(ui, &mut value, 0..=100)
                })
                .inner
                .on_hover_text(match (consumer, en) {
                    (Some(consumer), true) => format!("Applied by {consumer}; — means unset."),
                    (Some(consumer), false) => {
                        format!("Używane przez: {consumer}; — oznacza brak wyboru.")
                    }
                    (None, true) => "Unavailable: this control has no runtime consumer.".into(),
                    (None, false) => "Niedostępne: ta kontrolka nie ma obsługi wykonawczej.".into(),
                })
            })
            .inner
        })
        .inner;
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("preference-percent", key.as_str())),
            (
                response.rect,
                ui.clip_rect(),
                ui.is_enabled(),
                response.enabled(),
            ),
        );
    });
    if consumer.is_some() && (response.changed() || (selected.is_none() && response.clicked())) {
        draft
            .future_preferences
            .insert(key, FutureValue::Int(value));
    }
    response
}

fn console(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    for (id, labels) in [
        (
            "console.information",
            ["Komunikaty informacyjne", "Show Info Messages"],
        ),
        (
            "console.events",
            ["Komunikaty zdarzeń", "Show Event Messages"],
        ),
        (
            "console.own_status",
            ["Komunikaty stanu", "Show Status Messages"],
        ),
        (
            "console.other_status",
            ["Komunikaty stanu innych", "Show Status Messages of Others"],
        ),
        (
            "console.private_tab",
            [
                "Otwieraj nowe karty po otrzymaniu prywatnych wiadomości",
                "Open New Tabs When Receiving Private Messages",
            ],
        ),
        ("console.timestamps", ["Znaczniki czasu", "Show Timestamps"]),
    ] {
        row(ui, draft, "console", id, labels, en);
    }
    retained_children(
        ui,
        draft.stored_bool("console.console.timestamps") == Some(true),
        "console-timestamp-seconds",
        |ui| {
            row(
                ui,
                draft,
                "console",
                "console.seconds",
                ["Sekundy w znacznikach czasu", "Show Seconds in Timestamps"],
                en,
            );
        },
    );
    row(
        ui,
        draft,
        "console",
        "console.levels",
        ["Poziomy postaci", "Show Levels"],
        en,
    );
}

fn effects(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    boxed(ui, "", |ui| {
        reference_option(
            ui,
            draft,
            "effects",
            "effects.lighting",
            ["Pokaż efekty światła", "Show Light Effects"],
            en,
        );
        let lighting = draft.stored_bool("effects.effects.lighting") == Some(true);
        ui.add_enabled_ui(lighting, |ui| {
            for (id, labels) in [
                (
                    "effects.ambient_light",
                    ["Światło otoczenia:", "Ambient Light:"],
                ),
                (
                    "effects.level_separator",
                    ["Oddzielenie pięter:", "Level Separator:"],
                ),
                (
                    "effects.cloud_light",
                    ["Efekt chmur i wnętrz:", "Clouds & Indoor Effect:"],
                ),
            ] {
                percent(ui, draft, "effects", id, labels, en);
            }
        });
    });
    boxed(
        ui,
        if en {
            "Opacity of Spell Effects"
        } else {
            "Widoczność efektów zaklęć"
        },
        |ui| {
            for (id, labels) in [
                (
                    "effects.own_opacity",
                    ["Efekty własnych zaklęć:", "Own Spell Effects:"],
                ),
                (
                    "effects.other_opacity",
                    ["Efekty innych graczy:", "Other Players' Effects:"],
                ),
                (
                    "effects.creature_opacity",
                    ["Efekty zaklęć stworzeń:", "Creature Spell Effects:"],
                ),
                (
                    "effects.boss_opacity",
                    [
                        "Efekty zaklęć stworzeń w obszarach bossów:",
                        "Boss Area Creature Spell Effects:",
                    ],
                ),
            ] {
                percent(ui, draft, "effects", id, labels, en);
            }
        },
    );
}

fn sound(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    boxed(ui, "", |ui| {
        reference_option(
            ui,
            draft,
            "sound",
            "sound.device",
            ["Urządzenie:", "Sound Device:"],
            en,
        );
    });
    for (id, labels) in [
        ("sound.master", ["Głośność główna:", "Master Volume:"]),
        ("sound.music", ["Muzyka:", "Music Volume:"]),
        ("sound.ambience", ["Otoczenie:", "Ambience Volume:"]),
        ("sound.items", ["Przedmioty:", "Item Volume:"]),
        ("sound.events", ["Zdarzenia:", "Event Volume:"]),
    ] {
        boxed(ui, "", |ui| {
            percent(ui, draft, "sound", id, labels, en);
            if id == "sound.music" {
                retained_children(
                    ui,
                    draft.stored_int("sound.sound.music").is_some_and(|v| v > 0),
                    "sound-music",
                    |ui| {
                        reference_option(
                            ui,
                            draft,
                            "sound",
                            "sound.anthem",
                            ["Hymn", "Anthem"],
                            en,
                        );
                    },
                );
            } else if id == "sound.items" {
                retained_children(
                    ui,
                    draft.stored_int("sound.sound.items").is_some_and(|v| v > 0),
                    "sound-items",
                    |ui| {
                        reference_option(
                            ui,
                            draft,
                            "sound",
                            "sound.eating",
                            ["Jedzenie i napoje", "Food and Beverages"],
                            en,
                        );
                        reference_option(
                            ui,
                            draft,
                            "sound",
                            "sound.item_movement",
                            ["Przenoszenie przedmiotów", "Move Item"],
                            en,
                        );
                    },
                );
            }
        });
    }
}

fn battle(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    for (own, volume, spells, attack, healing, support, weapons) in [
        (
            true,
            "sound.own_combat",
            "sound.own_spells",
            "sound.attack_spells",
            "sound.healing_spells",
            "sound.support_spells",
            "sound.weapons",
        ),
        (
            false,
            "sound.other_combat",
            "sound.other_spells",
            "sound.other_attack_spells",
            "sound.other_healing_spells",
            "sound.other_support_spells",
            "sound.other_weapons",
        ),
    ] {
        boxed(ui, "", |ui| {
            percent(
                ui,
                draft,
                "battle_sounds",
                volume,
                [
                    if own {
                        "Własne dźwięki walki:"
                    } else {
                        "Dźwięki innych graczy:"
                    },
                    if own {
                        "Own Battle Sounds:"
                    } else {
                        "Other Players:"
                    },
                ],
                en,
            );
            retained_children(
                ui,
                draft
                    .stored_int(&format!("battle_sounds.{volume}"))
                    .is_some_and(|v| v > 0),
                if own {
                    "battle-own-volume"
                } else {
                    "battle-other-volume"
                },
                |ui| {
                    reference_option(
                        ui,
                        draft,
                        "battle_sounds",
                        spells,
                        ["Zaklęcia", "Spells"],
                        en,
                    );
                    retained_children(
                        ui,
                        draft.stored_bool(&format!("battle_sounds.{spells}")) == Some(true),
                        if own {
                            "battle-own-spells"
                        } else {
                            "battle-other-spells"
                        },
                        |ui| {
                            ui.indent(("spells", own), |ui| {
                                for (id, labels) in [
                                    (attack, ["Atak", "Attack"]),
                                    (healing, ["Leczenie", "Healing"]),
                                    (support, ["Wsparcie", "Support"]),
                                ] {
                                    reference_option(ui, draft, "battle_sounds", id, labels, en);
                                }
                            });
                        },
                    );
                    reference_option(ui, draft, "battle_sounds", weapons, ["Broń", "Weapons"], en);
                },
            );
        });
    }
    boxed(ui, "", |ui| {
        percent(
            ui,
            draft,
            "battle_sounds",
            "sound.creature_combat",
            ["Stworzenia:", "Creatures:"],
            en,
        );
        retained_children(
            ui,
            draft
                .stored_int("battle_sounds.sound.creature_combat")
                .is_some_and(|v| v > 0),
            "battle-creature-volume",
            |ui| {
                for (id, labels) in [
                    (
                        "sound.creature_noise",
                        ["Odgłosy stworzeń", "Creature Noises"],
                    ),
                    ("sound.creature_death", ["Śmierć", "Death"]),
                    (
                        "sound.creature_attacks",
                        ["Ataki i zaklęcia", "Attacks and Spells"],
                    ),
                ] {
                    reference_option(ui, draft, "battle_sounds", id, labels, en);
                }
            },
        );
    });
}

fn ui_sound(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    let positive_volume = draft
        .stored_int("ui_sounds.sound.ui")
        .is_some_and(|v| v > 0);
    boxed(ui, "", |ui| {
        percent(
            ui,
            draft,
            "ui_sounds",
            "sound.ui",
            ["Głośność interfejsu:", "UI Volume:"],
            en,
        );
        retained_children(ui, positive_volume, "ui-social-volume", |ui| {
            for (id, labels) in [
                (
                    "sound.interactions",
                    ["Interakcje interfejsu", "UI interactions"],
                ),
                (
                    "sound.party_notice",
                    ["Wejście/wyjście z drużyny", "Join/Leave Party"],
                ),
                (
                    "sound.vip_notice",
                    ["Logowanie/wylogowanie VIP", "VIP login/logout"],
                ),
            ] {
                row(ui, draft, "ui_sounds", id, labels, en);
            }
        });
    });
    boxed(ui, "", |ui| {
        retained_children(ui, positive_volume, "ui-chat-volume", |ui| {
            reference_option(
                ui,
                draft,
                "ui_sounds",
                "sound.chat_enabled",
                ["Dźwięki wiadomości", "Console Messages"],
                en,
            );
            retained_children(
                ui,
                draft.stored_bool("ui_sounds.sound.chat_enabled") == Some(true),
                "ui-console-messages",
                |ui| {
                    ui.indent("console-audio", |ui| {
                        for (id, labels) in [
                            ("sound.party_chat", ["Drużyna", "Party"]),
                            ("sound.guild", ["Gildia", "Guild"]),
                            (
                                "sound.private_without_tab",
                                [
                                    "Prywatne w lokalnym czacie",
                                    "Private Messages in Local Chat",
                                ],
                            ),
                            ("sound.private", ["Wiadomości prywatne", "Private Messages"]),
                            ("sound.npc", ["NPC", "NPCs"]),
                            ("sound.global", ["Kanały globalne", "Global"]),
                            ("sound.team_finder", ["Wyszukiwarka drużyn", "Team Finder"]),
                            ("sound.raids", ["Najazdy", "Raid Announcements"]),
                            (
                                "sound.system",
                                ["Komunikaty systemowe", "System Announcements"],
                            ),
                        ] {
                            reference_option(ui, draft, "ui_sounds", id, labels, en);
                        }
                    });
                },
            );
        });
    });
}

fn step_button(ui: &mut egui::Ui, enabled: bool, earlier: bool, en: bool) -> egui::Response {
    let label = match (earlier, en) {
        (true, true) => "Move shortcut up",
        (false, true) => "Move shortcut down",
        (true, false) => "Przesuń skrót w górę",
        (false, false) => "Przesuń skrót w dół",
    };
    let response = ui.add_enabled(
        enabled,
        egui::Button::new("").min_size(egui::vec2(18.0, 18.0)),
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    let center = response.rect.center();
    let sign = if earlier { -1.0 } else { 1.0 };
    let color = if enabled {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            center + egui::vec2(-4.0, -sign * 2.0),
            center + egui::vec2(4.0, -sign * 2.0),
            center + egui::vec2(0.0, sign * 3.0),
        ],
        color,
        egui::Stroke::NONE,
    ));
    response.on_hover_text(label)
}

fn shortcuts(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool, state: &mut PageState) {
    const DISPLAYED_HEIGHT: f32 = 221.0;
    const AVAILABLE_HEIGHT: f32 = 70.0;

    let Ok(mut order) = ShortcutOrder::from_saved(&draft.panel_shortcuts) else {
        return;
    };
    if state
        .displayed
        .as_deref()
        .is_some_and(|id| !order.contains(id))
    {
        state.displayed = None;
    }
    if state.available.as_deref().is_some_and(|id| {
        order.contains(id)
            || panel(id).is_none()
            || !crate::client_panels::has_local_destination(id)
    }) {
        state.available = None;
    }
    let border = ui.visuals().widgets.noninteractive.bg_stroke.color;

    let mut displayed_list_rect = egui::Rect::NOTHING;
    let _displayed_group = egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(3.0)
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(if en {
                "Displayed Shortcuts"
            } else {
                "Wyświetlane skróty"
            });
            ui.horizontal_top(|ui| {
                let width = (ui.available_width()
                    - ui.spacing().interact_size.x
                    - ui.spacing().item_spacing.x)
                    .max(20.0);
                let list = ui.allocate_ui_with_layout(
                    egui::vec2(width, DISPLAYED_HEIGHT),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(width);
                        ui.set_min_height(DISPLAYED_HEIGHT);
                        ui.spacing_mut().interact_size.y = 18.0;
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.spacing_mut().button_padding = egui::vec2(4.0, 0.5);
                        crate::settings_audit::show_bounded_vertical(
                            ui,
                            "displayed-panel-shortcuts",
                            DISPLAYED_HEIGHT,
                            None,
                            |ui| {
                                for id in order.ids() {
                                    if let Some(definition) = panel(id)
                                        && ui
                                            .selectable_label(
                                                state.displayed.as_deref() == Some(id),
                                                definition.label(en),
                                            )
                                            .clicked()
                                    {
                                        state.displayed = Some((*id).into());
                                    }
                                }
                            },
                        );
                    },
                );
                displayed_list_rect = list.response.rect;
                ui.vertical(|ui| {
                    let index = state
                        .displayed
                        .as_deref()
                        .and_then(|id| order.ids().iter().position(|entry| *entry == id));
                    if ui
                        .add_enabled(index.is_some(), egui::Button::new("×"))
                        .on_hover_text(if en {
                            "Remove selected shortcut"
                        } else {
                            "Usuń wybrany skrót"
                        })
                        .clicked()
                        && let Some(id) = state.displayed.take()
                    {
                        order.set_visible(&id, false);
                    }
                    for earlier in [true, false] {
                        let can_move = index.is_some_and(|index| {
                            if earlier {
                                index > 0
                            } else {
                                index + 1 < order.ids().len()
                            }
                        });
                        if step_button(ui, can_move, earlier, en).clicked()
                            && let Some(index) = index
                        {
                            order.move_one(index, earlier);
                        }
                    }
                });
            });
        });
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("shortcut-group-geometry", "displayed")),
            (
                _displayed_group.response.rect,
                displayed_list_rect,
                ui.clip_rect(),
            ),
        );
    });
    let mut available_list_rect = egui::Rect::NOTHING;
    let _available_group = egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, border))
        .corner_radius(3.0)
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(if en {
                "Available Shortcuts"
            } else {
                "Dostępne skróty"
            });
            ui.horizontal_top(|ui| {
                let width = (ui.available_width()
                    - ui.spacing().interact_size.x
                    - ui.spacing().item_spacing.x)
                    .max(20.0);
                let list = ui.allocate_ui_with_layout(
                    egui::vec2(width, AVAILABLE_HEIGHT),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| {
                        ui.set_width(width);
                        ui.set_min_height(AVAILABLE_HEIGHT);
                        ui.spacing_mut().interact_size.y = 18.0;
                        ui.spacing_mut().item_spacing.y = 1.0;
                        ui.spacing_mut().button_padding = egui::vec2(4.0, 0.5);
                        crate::settings_audit::show_bounded_vertical(
                            ui,
                            "available-panel-shortcuts",
                            AVAILABLE_HEIGHT,
                            None,
                            |ui| {
                                for definition in
                                    PANELS.iter().filter(|entry| !order.contains(entry.id))
                                {
                                    let available =
                                        crate::client_panels::has_local_destination(definition.id);
                                    let response = ui
                                        .add_enabled(
                                            available,
                                            egui::Button::selectable(
                                                state.available.as_deref() == Some(definition.id),
                                                definition.label(en),
                                            ),
                                        )
                                        .on_hover_text(if available {
                                            if en {
                                                "Select this local shortcut"
                                            } else {
                                                "Wybierz ten lokalny skrót"
                                            }
                                        } else if en {
                                            "Unavailable: no verified local destination."
                                        } else {
                                            "Niedostępne: brak zweryfikowanego lokalnego celu."
                                        });
                                    #[cfg(test)]
                                    ui.ctx().data_mut(|data| {
                                        data.insert_temp(
                                            egui::Id::new((
                                                "available-shortcut-row",
                                                definition.id,
                                            )),
                                            (response.rect, response.enabled()),
                                        );
                                    });
                                    if response.clicked() {
                                        state.available = Some(definition.id.into());
                                    }
                                }
                            },
                        );
                    },
                );
                available_list_rect = list.response.rect;
                let can_add = state.available.as_deref().is_some_and(|id| {
                    !order.contains(id) && crate::client_panels::has_local_destination(id)
                });
                if ui
                    .add_enabled(can_add, egui::Button::new("+"))
                    .on_hover_text(if en {
                        "Add selected shortcut"
                    } else {
                        "Dodaj wybrany skrót"
                    })
                    .clicked()
                    && let Some(id) = state.available.take()
                    && crate::client_panels::has_local_destination(&id)
                {
                    order.set_visible(&id, true);
                    state.displayed = Some(id);
                }
            });
        });
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("shortcut-group-geometry", "available")),
            (
                _available_group.response.rect,
                available_list_rect,
                ui.clip_rect(),
            ),
        );
    });
    draft.panel_shortcuts = order.saved_ids();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences_browser::test_support::{button_position, click, frame};

    type Page = fn(&mut egui::Ui, &mut ClientSettings, bool);

    fn render_page(page: Page, draft: &mut ClientSettings) -> (egui::Vec2, egui::FullOutput) {
        let ctx = egui::Context::default();
        crate::client_chrome::install(&ctx, false);
        let size = egui::vec2(820.0, 760.0);
        let mut output = egui::FullOutput::default();
        for _ in 0..2 {
            output = frame(&ctx, size, vec![], |ctx| {
                egui::Window::new("reference-copy-test")
                    .fixed_size([760.0, 700.0])
                    .show(ctx, |ui| page(ui, draft, true));
            })
            .1;
        }
        (size, output)
    }

    fn painted_text_count(output: &egui::FullOutput, text: &str) -> usize {
        fn count(shape: &egui::epaint::Shape, text: &str) -> usize {
            match shape {
                egui::epaint::Shape::Text(found) if found.galley.job.text == text => 1,
                egui::epaint::Shape::Vec(shapes) => {
                    shapes.iter().map(|shape| count(shape, text)).sum()
                }
                _ => 0,
            }
        }
        output
            .shapes
            .iter()
            .map(|shape| count(&shape.shape, text))
            .sum()
    }

    fn one_pixel_frames(output: &egui::FullOutput) -> Vec<egui::Rect> {
        fn collect(shape: &egui::Shape, frames: &mut Vec<egui::Rect>) {
            match shape {
                egui::Shape::Rect(rect)
                    if (rect.stroke.width - 1.0).abs() < f32::EPSILON
                        && rect.rect.width() > 100.0 =>
                {
                    frames.push(rect.rect);
                }
                egui::Shape::Vec(shapes) => {
                    for shape in shapes {
                        collect(shape, frames);
                    }
                }
                _ => {}
            }
        }
        let mut frames = Vec::new();
        for shape in &output.shapes {
            collect(&shape.shape, &mut frames);
        }
        frames
    }

    #[test]
    fn generic_reference_rows_are_compact_framed_and_keep_info_at_the_right_edge()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        crate::client_chrome::install_preferences(&ctx, false);
        let size = egui::vec2(760.0, 560.0);
        let mut draft = ClientSettings::default();
        let mut state = PageState::default();
        let mut output = egui::FullOutput::default();
        for _ in 0..2 {
            output = frame(&ctx, size, vec![], |ctx| {
                egui::Window::new("compact-reference-rows")
                    .fixed_size([700.0, 500.0])
                    .show(ctx, |ui| {
                        assert!(show(ui, &mut draft, "interface", true, false, &mut state,));
                    });
            })
            .1;
        }

        let geometry = |key: &'static str| {
            ctx.data(|data| {
                data.get_temp::<(egui::Rect, egui::Rect, egui::Rect)>(egui::Id::new((
                    "reference-row-geometry",
                    key,
                )))
            })
        };
        let first = geometry("interface.interface.highlight_mouse").ok_or("first row")?;
        let second = geometry("interface.interface.system_cursor").ok_or("second row")?;
        let frames = one_pixel_frames(&output);
        for (name, (row, clip, info)) in [("first", first), ("second", second)] {
            assert!(clip.contains_rect(row), "{name}: {row:?} outside {clip:?}");
            assert!(clip.contains_rect(info), "{name} info glyph clipped");
            assert!(
                (25.0..=27.0).contains(&row.height()),
                "{name} row height was {}",
                row.height()
            );
            assert!(
                (4.0..=6.0).contains(&(row.right() - info.right())),
                "{name} info inset was {}",
                row.right() - info.right()
            );
            assert!((row.center().y - info.center().y).abs() < 0.1);
            assert!(frames.iter().any(|candidate| {
                (candidate.left() - row.left()).abs() <= 1.0
                    && (candidate.right() - row.right()).abs() <= 1.0
                    && (candidate.top() - row.top()).abs() <= 1.0
                    && (candidate.bottom() - row.bottom()).abs() <= 1.0
            }));
        }
        let gap = second.0.top() - first.0.bottom();
        assert!((3.0..=7.0).contains(&gap), "row gap was {gap}");
        Ok(())
    }

    #[test]
    fn shortcut_groups_match_reference_ratio_and_show_all_factory_default_rows()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        crate::client_chrome::install_preferences(&ctx, false);
        let size = egui::vec2(720.0, 520.0);
        let mut draft = ClientSettings::default();
        let defaults = draft.panel_shortcuts.clone();
        let mut state = PageState::default();
        let mut output = egui::FullOutput::default();
        for _ in 0..3 {
            output = frame(&ctx, size, vec![], |ctx| {
                egui::Window::new("shortcut-reference-geometry")
                    .fixed_size([660.0, 460.0])
                    .show(ctx, |ui| shortcuts(ui, &mut draft, true, &mut state));
            })
            .1;
        }
        let group = |id: &'static str| {
            ctx.data(|data| {
                data.get_temp::<(egui::Rect, egui::Rect, egui::Rect)>(egui::Id::new((
                    "shortcut-group-geometry",
                    id,
                )))
            })
        };
        let displayed = group("displayed").ok_or("displayed group")?;
        let available = group("available").ok_or("available group")?;
        assert!(displayed.2.contains_rect(displayed.0));
        assert!(available.2.contains_rect(available.0));
        assert!((displayed.1.height() - 221.0).abs() <= 1.0);
        assert!((available.1.height() - 70.0).abs() <= 1.0);
        assert!((displayed.1.height() / available.1.height() - 3.15).abs() < 0.1);
        let gap = available.0.top() - displayed.0.bottom();
        assert!((3.0..=7.0).contains(&gap), "shortcut group gap was {gap}");

        for id in defaults {
            let label = panel(&id).ok_or("default shortcut definition")?.label(true);
            let position = button_position(&output, size, label)
                .ok_or("one of the factory-default shortcut rows was clipped")?;
            assert!(displayed.0.contains(position), "{label}");
        }
        let visible_available = PANELS
            .iter()
            .filter(|definition| !draft.panel_shortcuts.iter().any(|id| id == definition.id))
            .filter(|definition| button_position(&output, size, definition.label(true)).is_some())
            .count();
        assert!(
            (3..=4).contains(&visible_available),
            "visible available rows: {visible_available}"
        );
        for (label, group_rect) in [
            ("Displayed Shortcuts", displayed.0),
            ("Available Shortcuts", available.0),
            ("×", displayed.0),
            ("+", available.0),
        ] {
            let position = button_position(&output, size, label).ok_or("group content")?;
            assert!(
                group_rect.contains(position),
                "{label} escaped group border"
            );
        }
        Ok(())
    }

    #[test]
    fn lenshelp_without_a_local_destination_cannot_be_added() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        crate::client_chrome::install_preferences(&ctx, false);
        let size = egui::vec2(720.0, 520.0);
        let mut draft = ClientSettings::default();
        assert!(!draft.panel_shortcuts.iter().any(|id| id == "help"));
        let before = draft.panel_shortcuts.clone();
        let mut state = PageState::default();
        {
            let mut draw = |ctx: &egui::Context| {
                egui::Window::new("unavailable-lenshelp")
                    .fixed_size([660.0, 460.0])
                    .show(ctx, |ui| shortcuts(ui, &mut draft, true, &mut state));
            };
            let mut output = egui::FullOutput::default();
            for _ in 0..3 {
                output = frame(&ctx, size, vec![], &mut draw).1;
            }
            let lenshelp = button_position(&output, size, "Lenshelp").ok_or("Lenshelp row")?;
            let (_, enabled) = ctx
                .data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "available-shortcut-row",
                        "help",
                    )))
                })
                .ok_or("Lenshelp availability")?;
            assert!(!enabled);
            click(&ctx, size, lenshelp, &mut draw);
            let output = frame(&ctx, size, vec![], &mut draw).1;
            let add = button_position(&output, size, "+").ok_or("add button")?;
            click(&ctx, size, add, &mut draw);
        }
        assert_eq!(draft.panel_shortcuts, before);
        assert!(state.available.is_none());
        Ok(())
    }

    #[test]
    fn saved_lenshelp_remains_visible_and_removable_for_compatibility() -> Result<(), &'static str>
    {
        let ctx = egui::Context::default();
        crate::client_chrome::install_preferences(&ctx, false);
        let size = egui::vec2(720.0, 520.0);
        let mut draft = ClientSettings::default();
        let without_help = draft.panel_shortcuts.clone();
        draft.panel_shortcuts.insert(4, "help".into());
        let mut state = PageState::default();
        {
            let mut draw = |ctx: &egui::Context| {
                egui::Window::new("retained-lenshelp")
                    .fixed_size([660.0, 460.0])
                    .show(ctx, |ui| shortcuts(ui, &mut draft, true, &mut state));
            };
            let mut output = egui::FullOutput::default();
            for _ in 0..3 {
                output = frame(&ctx, size, vec![], &mut draw).1;
            }
            let lenshelp =
                button_position(&output, size, "Lenshelp").ok_or("saved Lenshelp displayed row")?;
            let (displayed_group, _, _) = ctx
                .data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect, egui::Rect)>(egui::Id::new((
                        "shortcut-group-geometry",
                        "displayed",
                    )))
                })
                .ok_or("displayed shortcut group")?;
            assert!(displayed_group.contains(lenshelp));
            click(&ctx, size, lenshelp, &mut draw);
            let output = frame(&ctx, size, vec![], &mut draw).1;
            let remove = button_position(&output, size, "×").ok_or("remove button")?;
            click(&ctx, size, remove, &mut draw);
        }
        assert_eq!(draft.panel_shortcuts, without_help);
        assert!(state.displayed.is_none());
        Ok(())
    }

    #[test]
    fn shortcut_editor_changes_only_draft_and_keeps_order_and_selection() -> Result<(), &'static str>
    {
        for en in [false, true] {
            let ctx = egui::Context::default();
            crate::client_chrome::install(&ctx, false);
            let size = egui::vec2(600.0, 430.0);
            let committed = ClientSettings::default();
            let mut draft = committed.clone();
            let mut state = PageState::default();
            let added = PANELS
                .iter()
                .find(|p| {
                    !draft.panel_shortcuts.iter().any(|id| id == p.id)
                        && crate::client_panels::has_local_destination(p.id)
                })
                .ok_or("available panel")?;
            let mut draw = |ctx: &egui::Context| {
                egui::Window::new("page-test")
                    .default_size([500.0, 380.0])
                    .show(ctx, |ui| shortcuts(ui, &mut draft, en, &mut state));
            };
            for _ in 0..4 {
                let _ = frame(&ctx, size, vec![], &mut draw);
            }
            let (_, output) = frame(&ctx, size, vec![], &mut draw);
            let item =
                button_position(&output, size, added.label(en)).ok_or("available row visible")?;
            click(&ctx, size, item, &mut draw);
            let (_, output) = frame(&ctx, size, vec![], &mut draw);
            let add = button_position(&output, size, "+").ok_or("add visible")?;
            click(&ctx, size, add, &mut draw);
            assert_eq!(
                draft.panel_shortcuts.last().map(String::as_str),
                Some(added.id)
            );
            assert!(!committed.panel_shortcuts.iter().any(|id| id == added.id));
            assert_eq!(state.displayed.as_deref(), Some(added.id));
            assert!(state.available.is_none());
            ShortcutOrder::from_saved(&draft.panel_shortcuts).map_err(|_| "valid order")?;
        }
        Ok(())
    }

    #[test]
    fn drawing_dedicated_pages_does_not_invent_saved_defaults() -> Result<(), serde_json::Error> {
        for en in [false, true] {
            for section in [
                "console",
                "effects",
                "sound",
                "battle_sounds",
                "ui_sounds",
                "shortcuts",
                "controls",
                "interface",
                "graphics",
                "game_window",
                "gameplay",
                "hud",
                "miscellaneous",
                "screenshots",
            ] {
                let ctx = egui::Context::default();
                crate::client_chrome::install(&ctx, false);
                let mut draft = ClientSettings::default();
                let before = serde_json::to_value(&draft)?;
                let mut state = PageState::default();
                let _ = frame(&ctx, egui::vec2(560.0, 430.0), vec![], |ctx| {
                    egui::Window::new("page-test")
                        .default_size([500.0, 380.0])
                        .show(ctx, |ui| {
                            assert!(show(ui, &mut draft, section, en, false, &mut state));
                        });
                });
                assert_eq!(serde_json::to_value(&draft)?, before, "{section}");
            }
        }
        Ok(())
    }

    #[test]
    fn lighting_master_gates_children_retains_values_and_uses_the_combined_row()
    -> Result<(), &'static str> {
        for en in [false, true] {
            let ctx = egui::Context::default();
            crate::client_chrome::install(&ctx, false);
            let size = egui::vec2(560.0, 430.0);
            let mut draft = ClientSettings::default();
            for (key, value) in [
                ("effects.effects.ambient_light", 25),
                ("effects.effects.level_separator", 80),
                ("effects.effects.cloud_light", 50),
                ("effects.effects.indoor_light", 65),
                ("effects.effects.own_opacity", 100),
                ("effects.effects.other_opacity", 75),
                ("effects.effects.creature_opacity", 50),
                ("effects.effects.boss_opacity", 25),
            ] {
                draft
                    .future_preferences
                    .insert(key.into(), FutureValue::Int(value));
            }
            draft
                .future_preferences
                .insert("effects.effects.lighting".into(), FutureValue::Bool(false));
            let before = draft.future_preferences.clone();
            let render = |draft: &mut ClientSettings| {
                frame(&ctx, size, vec![], |ctx| {
                    egui::Window::new("effects-test")
                        .default_size([500.0, 380.0])
                        .show(ctx, |ui| effects(ui, draft, en));
                })
                .1
            };
            let _ = render(&mut draft);
            let mut output = render(&mut draft);
            let enabled = |key: &str| {
                ctx.data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect, bool, bool)>(egui::Id::new((
                        "preference-percent",
                        key,
                    )))
                })
                .map(|(_, _, parent, response)| (parent, response))
            };
            for key in [
                "effects.effects.ambient_light",
                "effects.effects.level_separator",
                "effects.effects.cloud_light",
            ] {
                assert_eq!(enabled(key), Some((false, false)), "{key}");
            }
            assert_eq!(enabled("effects.effects.own_opacity"), Some((true, false)));
            let combined = if en {
                "Clouds & Indoor Effect: 50 %"
            } else {
                "Efekt chmur i wnętrz: 50 %"
            };
            assert!(button_position(&output, size, combined).is_some());
            for absent in ["Cloud Effects: 50 %", "Indoor Effects: 65 %"] {
                assert!(button_position(&output, size, absent).is_none());
            }
            assert_eq!(draft.future_preferences, before);

            let (master, clip) = ctx
                .data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                        "preference-toggle",
                        "effects.effects.lighting",
                    )))
                })
                .ok_or("missing light master")?;
            assert!(clip.contains_rect(master));
            click(&ctx, size, master.center(), |ctx| {
                egui::Window::new("effects-test")
                    .default_size([500.0, 380.0])
                    .show(ctx, |ui| effects(ui, &mut draft, en));
            });
            assert_eq!(draft.stored_bool("effects.effects.lighting"), Some(true));
            output = render(&mut draft);
            assert_eq!(enabled("effects.effects.ambient_light"), Some((true, true)));
            for key in [
                "effects.effects.level_separator",
                "effects.effects.cloud_light",
            ] {
                assert_eq!(enabled(key), Some((true, false)), "{key}");
            }
            assert_eq!(enabled("effects.effects.own_opacity"), Some((true, false)));
            assert!(button_position(&output, size, combined).is_some());
            for label in if en {
                [
                    "Own Spell Effects: 100 %",
                    "Other Players' Effects: 75 %",
                    "Creature Spell Effects: 50 %",
                    "Boss Area Creature Spell Effects: 25 %",
                ]
            } else {
                [
                    "Efekty własnych zaklęć: 100 %",
                    "Efekty innych graczy: 75 %",
                    "Efekty zaklęć stworzeń: 50 %",
                    "Efekty zaklęć stworzeń w obszarach bossów: 25 %",
                ]
            } {
                assert!(button_position(&output, size, label).is_some(), "{label}");
            }
            assert_eq!(draft.stored_int("effects.effects.ambient_light"), Some(25));
            assert_eq!(draft.stored_int("effects.effects.indoor_light"), Some(65));
        }
        Ok(())
    }

    #[test]
    fn console_uses_the_exact_private_message_tab_label_without_inventing_behavior() {
        for en in [false, true] {
            let ctx = egui::Context::default();
            crate::client_chrome::install(&ctx, false);
            let size = egui::vec2(620.0, 480.0);
            let mut draft = ClientSettings::default();
            let before = draft.future_preferences.clone();
            let mut output = egui::FullOutput::default();
            for _ in 0..2 {
                output = frame(&ctx, size, vec![], |ctx| {
                    egui::Window::new("console-test")
                        .default_size([560.0, 440.0])
                        .show(ctx, |ui| console(ui, &mut draft, en));
                })
                .1;
            }
            let exact = if en {
                "Open New Tabs When Receiving Private Messages"
            } else {
                "Otwieraj nowe karty po otrzymaniu prywatnych wiadomości"
            };
            assert!(button_position(&output, size, exact).is_some());
            assert!(button_position(&output, size, "Open New Tabs for Private Messages").is_none());
            assert_eq!(draft.future_preferences, before);
        }
    }

    #[test]
    fn reference_backed_pages_use_exact_copy_and_percentage_spacing() -> Result<(), &'static str> {
        let mut console_draft = ClientSettings::default();
        let (size, output) = render_page(console, &mut console_draft);
        assert!(button_position(&output, size, "Show Levels").is_some());
        assert!(button_position(&output, size, "Text Size").is_none());

        let mut effects_draft = ClientSettings::default();
        effects_draft
            .future_preferences
            .insert("effects.effects.ambient_light".into(), FutureValue::Int(25));
        let (size, output) = render_page(effects, &mut effects_draft);
        assert!(button_position(&output, size, "Show Light Effects").is_some());
        assert!(button_position(&output, size, "Lighting").is_none());
        assert!(button_position(&output, size, "Ambient Light: 25 %").is_some());
        assert!(button_position(&output, size, "Ambient Light: 25%").is_none());

        let mut sound_draft = ClientSettings::default();
        sound_draft
            .future_preferences
            .insert("sound.sound.music".into(), FutureValue::Int(50));
        let (size, output) = render_page(sound, &mut sound_draft);
        let sound_device = button_position(&output, size, "Sound Device:")
            .ok_or("Sound Device group must be visible")?;
        assert!(
            one_pixel_frames(&output)
                .iter()
                .any(|frame| frame.width() > 500.0 && frame.contains(sound_device))
        );
        assert!(button_position(&output, size, "Music Volume: 50 %").is_some());
        assert!(button_position(&output, size, "Anthem").is_some());
        assert!(button_position(&output, size, "Play Anthem").is_none());

        let mut battle_draft = ClientSettings::default();
        for key in [
            "battle_sounds.sound.own_combat",
            "battle_sounds.sound.other_combat",
            "battle_sounds.sound.creature_combat",
        ] {
            battle_draft
                .future_preferences
                .insert(key.into(), FutureValue::Int(100));
        }
        let (size, output) = render_page(battle, &mut battle_draft);
        for label in [
            "Own Battle Sounds: 100 %",
            "Other Players: 100 %",
            "Creatures: 100 %",
        ] {
            assert!(button_position(&output, size, label).is_some(), "{label}");
        }
        assert!(button_position(&output, size, "Volume: 100 %").is_none());

        let mut ui_sound_draft = ClientSettings::default();
        ui_sound_draft
            .future_preferences
            .insert("ui_sounds.sound.ui".into(), FutureValue::Int(100));
        let (size, output) = render_page(ui_sound, &mut ui_sound_draft);
        let top_group_points = [
            button_position(&output, size, "UI Volume: 100 %").ok_or("UI Volume top-group row")?,
            button_position(&output, size, "UI interactions")
                .ok_or("UI interactions top-group row")?,
            button_position(&output, size, "VIP login/logout").ok_or("VIP top-group row")?,
        ];
        assert!(one_pixel_frames(&output).iter().any(|frame| {
            frame.width() > 500.0
                && top_group_points
                    .iter()
                    .all(|position| frame.contains(*position))
        }));
        assert!(button_position(&output, size, "UI Volume: 100 %").is_some());
        assert!(button_position(&output, size, "UI interactions").is_some());
        assert!(button_position(&output, size, "UI Interactions").is_none());
        assert!(button_position(&output, size, "VIP login/logout").is_some());
        assert!(button_position(&output, size, "VIP Login/Logout").is_none());
        assert_eq!(painted_text_count(&output, "Console Messages"), 1);

        let ctx = egui::Context::default();
        crate::client_chrome::install(&ctx, false);
        let size = egui::vec2(700.0, 560.0);
        let mut draft = ClientSettings::default();
        let mut state = PageState::default();
        let mut output = egui::FullOutput::default();
        for _ in 0..2 {
            output = frame(&ctx, size, vec![], |ctx| {
                egui::Window::new("shortcut-copy-test")
                    .fixed_size([640.0, 500.0])
                    .show(ctx, |ui| shortcuts(ui, &mut draft, true, &mut state));
            })
            .1;
        }
        for exact in ["Displayed Shortcuts", "Available Shortcuts"] {
            assert!(button_position(&output, size, exact).is_some(), "{exact}");
        }
        for old in ["Displayed Shortcuts:", "Available Shortcuts:"] {
            assert!(button_position(&output, size, old).is_none(), "{old}");
        }
        Ok(())
    }

    #[test]
    fn help_matches_the_ten_vertical_reference_actions_and_honest_availability()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        crate::client_chrome::install(&ctx, false);
        let size = egui::vec2(700.0, 620.0);
        let mut state = PageState::default();
        {
            let mut draw = |ctx: &egui::Context| {
                egui::Window::new("help-reference-test")
                    .fixed_size([640.0, 560.0])
                    .show(ctx, |ui| help(ui, true, true, &mut state));
            };
            let mut output = egui::FullOutput::default();
            for _ in 0..2 {
                output = frame(&ctx, size, vec![], &mut draw).1;
            }
            assert!(button_position(&output, size, "Support").is_none());
            assert!(button_position(&output, size, "Local Data").is_none());

            let rows = [
                ("client-help", "Client Help", false),
                ("compendium", "Compendium", true),
                ("rule-violations", "Rule Violations", false),
                ("manual", "Manual", false),
                ("faq", "FAQ", false),
                ("info", "Info", true),
                ("export-options", "Export All Options", true),
                ("export-minimap", "Export Minimap", false),
                ("import-options-minimap", "Import Options/Minimap", true),
                ("reset-options", "Reset All Options", true),
            ];
            let mut previous_y = f32::NEG_INFINITY;
            let mut row_left: Option<f32> = None;
            let mut info_row = None;
            let mut export_row = None;
            for (id, label, enabled) in rows {
                assert!(button_position(&output, size, label).is_some(), "{label}");
                let (rect, clip, actual_enabled) = ctx
                    .data(|data| {
                        data.get_temp::<(egui::Rect, egui::Rect, bool)>(egui::Id::new((
                            "help-action-row",
                            id,
                        )))
                    })
                    .ok_or("missing help row geometry")?;
                assert!(clip.contains_rect(rect), "{id}");
                assert_eq!(actual_enabled, enabled, "{id}");
                assert!(rect.center().y > previous_y, "{id}");
                previous_y = rect.center().y;
                if id == "info" {
                    info_row = Some(rect);
                } else if id == "export-options" {
                    export_row = Some(rect);
                }
                if let Some(left) = row_left {
                    assert!((rect.left() - left).abs() < 0.1, "{id}");
                } else {
                    row_left = Some(rect.left());
                }
                let (info, info_clip) = ctx
                    .data(|data| {
                        data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                            "help-info-glyph",
                            id,
                        )))
                    })
                    .ok_or("missing help info glyph")?;
                assert!(info_clip.contains_rect(info), "{id}");
                assert!(info.center().x > rect.right(), "{id}");
            }
            let boundary_gap =
                export_row.ok_or("export row")?.top() - info_row.ok_or("info row")?.bottom();
            assert!(
                (12.0..=16.0).contains(&boundary_gap),
                "Help group boundary gap was {boundary_gap}"
            );

            let compendium =
                button_position(&output, size, "Compendium").ok_or("missing Compendium action")?;
            click(&ctx, size, compendium, &mut draw);
        }
        assert_eq!(state.take_help_action(), Some(HelpAction::OpenCompendium));

        {
            let mut draw = |ctx: &egui::Context| {
                egui::Window::new("help-reference-disabled-test")
                    .fixed_size([640.0, 560.0])
                    .show(ctx, |ui| help(ui, true, true, &mut state));
            };
            let mut output = egui::FullOutput::default();
            for _ in 0..2 {
                output = frame(&ctx, size, vec![], &mut draw).1;
            }
            for label in ["Client Help", "Export Minimap"] {
                let position =
                    button_position(&output, size, label).ok_or("missing disabled Help action")?;
                click(&ctx, size, position, &mut draw);
            }
        }
        assert_eq!(state.take_help_action(), None);
        Ok(())
    }

    #[test]
    fn extracted_zero_volume_and_item_music_dependencies_preserve_saved_intent() {
        for en in [false, true] {
            for master in [0, 100] {
                for volume in [0, 50] {
                    let ctx = egui::Context::default();
                    crate::client_chrome::install(&ctx, false);
                    let size = egui::vec2(750.0, 800.0);
                    let mut draft = ClientSettings::default();
                    draft.future_preferences.extend([
                        ("sound.sound.master".into(), FutureValue::Int(master)),
                        ("sound.sound.music".into(), FutureValue::Int(volume)),
                        ("sound.sound.items".into(), FutureValue::Int(volume)),
                        ("sound.sound.anthem".into(), FutureValue::Bool(true)),
                        ("sound.sound.eating".into(), FutureValue::Bool(true)),
                        ("sound.sound.item_movement".into(), FutureValue::Bool(true)),
                    ]);
                    let before = draft.clone();
                    let mut output = egui::FullOutput::default();
                    for _ in 0..3 {
                        output = frame(&ctx, size, vec![], |ctx| {
                            egui::Window::new("extracted-sound")
                                .fixed_size([650.0, 700.0])
                                .show(ctx, |ui| sound(ui, &mut draft, en));
                        })
                        .1;
                    }
                    for gate in ["sound-music", "sound-items"] {
                        assert_eq!(
                            ctx.data(|data| data
                                .get_temp::<bool>(egui::Id::new(("retained-child-gate", gate)))),
                            Some(volume > 0)
                        );
                    }
                    if volume == 0 {
                        assert!(
                            button_position(
                                &output,
                                size,
                                if en {
                                    "Music Volume: 0 % (Off)"
                                } else {
                                    "Muzyka: 0 % (Wył.)"
                                }
                            )
                            .is_some()
                        );
                    }
                    // Reference dependencies do not invent a mixer/playback consumer.
                    for key in [
                        "sound.sound.master",
                        "sound.sound.music",
                        "sound.sound.anthem",
                        "sound.sound.eating",
                    ] {
                        assert!(oteryn_client::settings_catalog::runtime_consumer(key).is_none());
                    }
                    assert_eq!(draft, before);
                }
            }
        }
    }

    #[test]
    fn extracted_battle_and_ui_volumes_gate_filters_without_erasing_selections() {
        for (page, volume, gates) in [
            (
                battle as Page,
                "battle_sounds.sound.own_combat",
                &["battle-own-volume", "battle-own-spells"][..],
            ),
            (
                battle as Page,
                "battle_sounds.sound.other_combat",
                &["battle-other-volume", "battle-other-spells"][..],
            ),
            (
                battle as Page,
                "battle_sounds.sound.creature_combat",
                &["battle-creature-volume"][..],
            ),
            (
                ui_sound as Page,
                "ui_sounds.sound.ui",
                &["ui-social-volume", "ui-chat-volume", "ui-console-messages"][..],
            ),
        ] {
            for selection in [None, Some(0), Some(1), Some(100)] {
                let ctx = egui::Context::default();
                let mut draft = ClientSettings::default();
                for key in [
                    "battle_sounds.sound.own_spells",
                    "battle_sounds.sound.other_spells",
                    "battle_sounds.sound.attack_spells",
                    "battle_sounds.sound.creature_noise",
                    "ui_sounds.sound.chat_enabled",
                    "ui_sounds.sound.party_chat",
                ] {
                    draft
                        .future_preferences
                        .insert(key.into(), FutureValue::Bool(true));
                }
                if let Some(value) = selection {
                    draft
                        .future_preferences
                        .insert(volume.into(), FutureValue::Int(value));
                }
                let before = draft.clone();
                for _ in 0..2 {
                    let _ = frame(&ctx, egui::vec2(750.0, 850.0), vec![], |ctx| {
                        egui::Window::new("audio-volume-gates")
                            .show(ctx, |ui| page(ui, &mut draft, true));
                    });
                }
                for gate in gates {
                    assert_eq!(
                        ctx.data(|data| data
                            .get_temp::<bool>(egui::Id::new(("retained-child-gate", *gate)))),
                        Some(selection.is_some_and(|v| v > 0)),
                        "{volume}/{gate}/{selection:?}"
                    );
                }
                assert_eq!(draft, before);
                assert!(oteryn_client::settings_catalog::runtime_consumer(volume).is_none());
            }
        }
    }

    #[test]
    fn retained_reference_masters_gate_children_without_erasing_saved_intent() {
        for (page, parent, children, gate) in [
            (
                console as Page,
                "console.console.timestamps",
                &["console.console.seconds"][..],
                "console-timestamp-seconds",
            ),
            (
                battle as Page,
                "battle_sounds.sound.own_spells",
                &[
                    "battle_sounds.sound.attack_spells",
                    "battle_sounds.sound.healing_spells",
                    "battle_sounds.sound.support_spells",
                ][..],
                "battle-own-spells",
            ),
            (
                battle as Page,
                "battle_sounds.sound.other_spells",
                &[
                    "battle_sounds.sound.other_attack_spells",
                    "battle_sounds.sound.other_healing_spells",
                    "battle_sounds.sound.other_support_spells",
                ][..],
                "battle-other-spells",
            ),
            (
                ui_sound as Page,
                "ui_sounds.sound.chat_enabled",
                &[
                    "ui_sounds.sound.party_chat",
                    "ui_sounds.sound.guild",
                    "ui_sounds.sound.private_without_tab",
                    "ui_sounds.sound.private",
                    "ui_sounds.sound.npc",
                    "ui_sounds.sound.global",
                    "ui_sounds.sound.team_finder",
                    "ui_sounds.sound.raids",
                    "ui_sounds.sound.system",
                ][..],
                "ui-console-messages",
            ),
        ] {
            let ctx = egui::Context::default();
            crate::client_chrome::install(&ctx, false);
            let size = egui::vec2(720.0, 720.0);
            let mut draft = ClientSettings::default();
            for key in [
                "battle_sounds.sound.own_combat",
                "battle_sounds.sound.other_combat",
                "ui_sounds.sound.ui",
            ] {
                draft
                    .future_preferences
                    .insert(key.into(), FutureValue::Int(100));
            }
            draft
                .future_preferences
                .insert(parent.into(), FutureValue::Bool(false));
            for child in children {
                draft
                    .future_preferences
                    .insert((*child).into(), FutureValue::Bool(true));
            }
            let draw = |ctx: &egui::Context, draft: &mut ClientSettings| {
                egui::Window::new(gate)
                    .fixed_size([660.0, 660.0])
                    .show(ctx, |ui| page(ui, draft, true));
            };
            for _ in 0..2 {
                let _ = frame(&ctx, size, vec![], |ctx| draw(ctx, &mut draft));
            }
            assert_eq!(
                ctx.data(|data| {
                    data.get_temp::<bool>(egui::Id::new(("retained-child-gate", gate)))
                }),
                Some(false),
                "{gate}"
            );
            for child in children {
                assert_eq!(
                    draft.future_preferences.get(*child),
                    Some(&FutureValue::Bool(true)),
                    "disabled child {child} retains its saved intent"
                );
            }

            draft
                .future_preferences
                .insert(parent.into(), FutureValue::Bool(true));
            let _ = frame(&ctx, size, vec![], |ctx| draw(ctx, &mut draft));
            assert_eq!(
                ctx.data(|data| {
                    data.get_temp::<bool>(egui::Id::new(("retained-child-gate", gate)))
                }),
                Some(true),
                "{gate}"
            );
            for child in children {
                assert_eq!(
                    draft.future_preferences.get(*child),
                    Some(&FutureValue::Bool(true)),
                    "reenabling a master restores retained child {child}"
                );
            }
        }
    }
}
