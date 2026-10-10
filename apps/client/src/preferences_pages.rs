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
    last_frame_limit: Option<u16>,
    hotkeys: hotkeys::State,
    help_action: Option<HelpAction>,
    help_reset_confirmation: bool,
    help_info: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HelpAction {
    Export,
    Import,
    Reset,
    OpenScreenshotFolder,
}

impl PageState {
    pub(super) fn take_help_action(&mut self) -> Option<HelpAction> {
        self.help_action.take()
    }
}

pub(super) fn show(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    section: &str,
    english: bool,
    state: &mut PageState,
) -> bool {
    match section {
        "shortcuts" => shortcuts(ui, draft, english, state),
        "console" => console(ui, draft, english),
        "effects" => effects(ui, draft, english),
        "sound" => sound(ui, draft, english),
        "battle_sounds" => battle(ui, draft, english),
        "ui_sounds" => ui_sound(ui, draft, english),
        "help" => help(ui, english, state),
        _ => {
            return hotkeys::show(ui, draft, section, english, &mut state.hotkeys)
                || hud::show(ui, draft, section, english, state)
                || display::show(ui, draft, section, english, state);
        }
    }
    true
}

fn help(ui: &mut egui::Ui, en: bool, state: &mut PageState) {
    boxed(ui, if en { "Support" } else { "Pomoc" }, |ui| {
        egui::Grid::new("help-support-actions")
            .num_columns(2)
            .show(ui, |ui| {
                for (index, (pl, english)) in [
                    ("Pomoc klienta", "Client Help"),
                    ("Kompendium", "Compendium"),
                    ("Naruszenia zasad", "Rule Violations"),
                    ("Instrukcja", "Manual"),
                    ("Najczęstsze pytania", "FAQ"),
                ]
                .into_iter()
                .enumerate()
                {
                    ui.add_enabled(false, egui::Button::new(if en { english } else { pl }))
                        .on_hover_text(if en {
                            "No verified Oteryn destination is configured for this reference action."
                        } else {
                            "Dla tej czynności referencyjnej nie skonfigurowano zweryfikowanego celu Oteryn."
                        });
                    if index % 2 == 1 {
                        ui.end_row();
                    }
                }
                if ui.button(if en { "Info" } else { "Informacje" }).clicked() {
                    state.help_info = !state.help_info;
                }
                ui.end_row();
            });
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
    boxed(ui, if en { "Local Data" } else { "Dane lokalne" }, |ui| {
        if ui
            .button(if en {
                "Export All Options"
            } else {
                "Eksportuj wszystkie ustawienia"
            })
            .clicked()
        {
            state.help_action = Some(HelpAction::Export);
        }
        if ui
            .button(if en {
                "Import Options"
            } else {
                "Importuj ustawienia"
            })
            .clicked()
        {
            state.help_action = Some(HelpAction::Import);
        }
        for label in if en {
            ["Export Minimap", "Import Minimap"]
        } else {
            ["Eksportuj minimapę", "Importuj minimapę"]
        } {
            ui.add_enabled(false, egui::Button::new(label))
                .on_hover_text(if en {
                    "No versioned minimap import/export format is available yet."
                } else {
                    "Brak jeszcze wersjonowanego formatu importu/eksportu minimapy."
                });
        }
        if !state.help_reset_confirmation {
            if ui
                .button(if en {
                    "Reset All Options"
                } else {
                    "Przywróć wszystkie ustawienia"
                })
                .clicked()
            {
                state.help_reset_confirmation = true;
            }
        } else {
            egui::Modal::new(egui::Id::new("reset-all-options-confirmation")).show(
                ui.ctx(),
                |ui| {
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
                },
            );
        }
    });
}

fn row(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    section: &str,
    id: &str,
    labels: [&str; 2],
    en: bool,
) {
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(0, 3))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.set_min_height(28.0);
            reference_option(ui, draft, section, id, labels, en);
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
    let text = selected.map_or_else(|| "—".into(), |v| format!("{v}%"));
    let consumer = oteryn_client::settings_catalog::runtime_consumer(&key);
    let response = ui
        .push_id(&key, |ui| {
            ui.horizontal(|ui| {
                ui.label(format!("{} {text}", labels[usize::from(en)]));
                ui.spacing_mut().slider_width = ui.available_width().clamp(45.0, 185.0);
                ui.add_enabled(
                    consumer.is_some(),
                    egui::Slider::new(&mut value, 0..=100).show_value(false),
                )
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
                "Nowe karty prywatnych rozmów",
                "Open New Tabs for Private Messages",
            ],
        ),
        ("console.timestamps", ["Znaczniki czasu", "Show Timestamps"]),
        (
            "console.seconds",
            ["Sekundy w znacznikach czasu", "Show Seconds in Timestamps"],
        ),
        ("console.levels", ["Poziomy postaci", "Show Levels"]),
    ] {
        row(ui, draft, "console", id, labels, en);
    }
    boxed(ui, "OTERYN", |ui| {
        ui.checkbox(
            &mut draft.show_chat,
            if en {
                "Show Chat Panel"
            } else {
                "Pokaż panel czatu"
            },
        );
        reference_option(
            ui,
            draft,
            "console",
            "console.font_size",
            ["Rozmiar tekstu", "Text Size"],
            en,
        );
    });
}

fn effects(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    boxed(ui, if en { "Lighting" } else { "Oświetlenie" }, |ui| {
        reference_option(
            ui,
            draft,
            "effects",
            "effects.lighting",
            ["Pokaż efekty światła", "Show Light Effects"],
            en,
        );
        for (id, labels) in [
            (
                "effects.ambient_light",
                ["Światło otoczenia:", "Ambient Light:"],
            ),
            (
                "effects.level_separator",
                ["Oddzielenie pięter:", "Level Separator:"],
            ),
            ("effects.cloud_light", ["Chmury:", "Cloud Effects:"]),
            ("effects.indoor_light", ["Wnętrza:", "Indoor Effects:"]),
        ] {
            percent(ui, draft, "effects", id, labels, en);
        }
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
                ("effects.own_opacity", ["Własne:", "Own:"]),
                ("effects.other_opacity", ["Inni gracze:", "Other Players:"]),
                ("effects.creature_opacity", ["Stworzenia:", "Creatures:"]),
                (
                    "effects.boss_opacity",
                    ["Obszary zaklęć bossów:", "Boss Area Creature Spells:"],
                ),
            ] {
                percent(ui, draft, "effects", id, labels, en);
            }
        },
    );
}

fn sound(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    reference_option(
        ui,
        draft,
        "sound",
        "sound.device",
        ["Urządzenie:", "Sound Device:"],
        en,
    );
    ui.separator();
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
                reference_option(
                    ui,
                    draft,
                    "sound",
                    "sound.anthem",
                    ["Odtwarzaj hymn", "Play Anthem"],
                    en,
                );
            } else if id == "sound.items" {
                // These keys retain their existing storage section; their visual group
                // is Sound, as observed, rather than UI Sounds.
                reference_option(
                    ui,
                    draft,
                    "ui_sounds",
                    "sound.eating",
                    ["Jedzenie i napoje", "Food and Beverages"],
                    en,
                );
                reference_option(
                    ui,
                    draft,
                    "ui_sounds",
                    "sound.item_movement",
                    ["Przenoszenie przedmiotów", "Move Item"],
                    en,
                );
            }
        });
    }
    boxed(ui, "OTERYN", |ui| {
        reference_option(
            ui,
            draft,
            "sound",
            "sound.background_mute",
            ["Wycisz w tle", "Mute in Background"],
            en,
        );
    });
}

fn battle(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    for (own, title, volume, spells, attack, healing, support, weapons) in [
        (
            true,
            ["Własne dźwięki walki", "Own Battle Sounds"],
            "sound.own_combat",
            "sound.own_spells",
            "sound.attack_spells",
            "sound.healing_spells",
            "sound.support_spells",
            "sound.weapons",
        ),
        (
            false,
            ["Dźwięki innych graczy", "Other Players"],
            "sound.other_combat",
            "sound.other_spells",
            "sound.other_attack_spells",
            "sound.other_healing_spells",
            "sound.other_support_spells",
            "sound.other_weapons",
        ),
    ] {
        boxed(ui, title[usize::from(en)], |ui| {
            percent(
                ui,
                draft,
                "battle_sounds",
                volume,
                ["Głośność:", "Volume:"],
                en,
            );
            reference_option(
                ui,
                draft,
                "battle_sounds",
                spells,
                ["Zaklęcia", "Spells"],
                en,
            );
            ui.indent(("spells", own), |ui| {
                for (id, labels) in [
                    (attack, ["Atak", "Attack"]),
                    (healing, ["Leczenie", "Healing"]),
                    (support, ["Wsparcie", "Support"]),
                ] {
                    reference_option(ui, draft, "battle_sounds", id, labels, en);
                }
            });
            reference_option(ui, draft, "battle_sounds", weapons, ["Broń", "Weapons"], en);
        });
    }
    boxed(ui, if en { "Creatures" } else { "Stworzenia" }, |ui| {
        percent(
            ui,
            draft,
            "battle_sounds",
            "sound.creature_combat",
            ["Głośność:", "Volume:"],
            en,
        );
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
    });
}

fn ui_sound(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    percent(
        ui,
        draft,
        "ui_sounds",
        "sound.ui",
        ["Głośność interfejsu:", "UI Volume:"],
        en,
    );
    for (id, labels) in [
        (
            "sound.interactions",
            ["Interakcje interfejsu", "UI Interactions"],
        ),
        (
            "sound.party_notice",
            ["Wejście/wyjście z drużyny", "Join/Leave Party"],
        ),
        (
            "sound.vip_notice",
            ["Logowanie/wylogowanie VIP", "VIP Login/Logout"],
        ),
    ] {
        row(ui, draft, "ui_sounds", id, labels, en);
    }
    boxed(
        ui,
        if en {
            "Console Messages"
        } else {
            "Wiadomości konsoli"
        },
        |ui| {
            reference_option(
                ui,
                draft,
                "ui_sounds",
                "sound.chat_enabled",
                ["Dźwięki wiadomości", "Console Messages"],
                en,
            );
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
    if state
        .available
        .as_deref()
        .is_some_and(|id| order.contains(id))
    {
        state.available = None;
    }
    ui.label(if en {
        "Displayed Shortcuts:"
    } else {
        "Wyświetlane skróty:"
    });
    ui.horizontal_top(|ui| {
        let width = (ui.available_width() - 32.0).max(20.0);
        egui::Frame::new()
            .stroke(egui::Stroke::new(
                1.0,
                ui.visuals().widgets.noninteractive.bg_stroke.color,
            ))
            .inner_margin(3.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(width);
                    egui::ScrollArea::vertical()
                        .id_salt("displayed-panel-shortcuts")
                        .auto_shrink([false, false])
                        .max_height(130.0)
                        .show(ui, |ui| {
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
                        });
                });
            });
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
    ui.add_space(5.0);
    ui.label(if en {
        "Available Shortcuts:"
    } else {
        "Dostępne skróty:"
    });
    ui.horizontal_top(|ui| {
        let width = (ui.available_width() - 32.0).max(20.0);
        egui::Frame::new()
            .stroke(egui::Stroke::new(
                1.0,
                ui.visuals().widgets.noninteractive.bg_stroke.color,
            ))
            .inner_margin(3.0)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.set_width(width);
                    egui::ScrollArea::vertical()
                        .id_salt("available-panel-shortcuts")
                        .auto_shrink([false, false])
                        .max_height(110.0)
                        .show(ui, |ui| {
                            for definition in
                                PANELS.iter().filter(|entry| !order.contains(entry.id))
                            {
                                if ui
                                    .selectable_label(
                                        state.available.as_deref() == Some(definition.id),
                                        definition.label(en),
                                    )
                                    .clicked()
                                {
                                    state.available = Some(definition.id.into());
                                }
                            }
                        });
                });
            });
        if ui
            .add_enabled(state.available.is_some(), egui::Button::new("+"))
            .on_hover_text(if en {
                "Add selected shortcut"
            } else {
                "Dodaj wybrany skrót"
            })
            .clicked()
            && let Some(id) = state.available.take()
        {
            order.set_visible(&id, true);
            state.displayed = Some(id);
        }
    });
    draft.panel_shortcuts = order.saved_ids();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences_browser::test_support::{button_position, click, frame};

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
                .find(|p| !draft.panel_shortcuts.iter().any(|id| id == p.id))
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
                            assert!(show(ui, &mut draft, section, en, &mut state));
                        });
                });
                assert_eq!(serde_json::to_value(&draft)?, before, "{section}");
            }
        }
        Ok(())
    }
}
