//! HUD, confirmation and capture page compositions. Server state is never fabricated.
use super::{HelpAction, PageState, boxed, percent, reference_option, row};
use oteryn_client::settings::ClientSettings;
use oteryn_client::settings_catalog::FutureValue;

const CONDITIONS: [(&str, [&str; 2]); 35] = [
    ("poison", ["Zatrucie", "Poisoned"]),
    ("burning", ["Płonięcie", "Burning"]),
    ("electric", ["Porażenie", "Electrified"]),
    ("bleeding", ["Krwawienie", "Bleeding"]),
    ("agony", ["Agonia", "Agony"]),
    ("powerless", ["Bezsilność", "Powerless"]),
    ("rooted", ["Unieruchomienie", "Rooted"]),
    ("feared", ["Strach", "Feared"]),
    ("drunk", ["Upicie", "Drunk"]),
    ("magic_shield", ["Tarcza magiczna", "Magic Shield"]),
    ("monk_virtue", ["Bonus cnót mnicha", "Monk’s Virtue bonus"]),
    ("slowed", ["Spowolnienie", "Slowed"]),
    ("haste", ["Przyspieszenie", "Haste"]),
    ("logout_block", ["Blokada wylogowania", "Logout Block"]),
    ("drowning", ["Tonięcie", "Drowning"]),
    ("freezing", ["Zamarzanie", "Freezing"]),
    ("dazzled", ["Oślepienie", "Dazzled"]),
    ("cursed", ["Klątwa", "Cursed"]),
    ("strengthened", ["Wzmocnienie", "Strengthened"]),
    (
        "pz_block",
        ["Blokada strefy ochronnej", "Protection Zone Block"],
    ),
    ("in_pz", ["W strefie ochronnej", "In Protection Zone"]),
    ("resting", ["Strefa odpoczynku", "Resting Area"]),
    ("lesser_hex", ["Słabszy urok", "Lesser Hex"]),
    ("intense_hex", ["Silny urok", "Intense Hex"]),
    ("greater_hex", ["Potężny urok", "Greater Hex"]),
    ("goshnar_taint", ["Skaza Goshnara", "Goshnar’s Taint"]),
    ("bakragore_taint", ["Skaza Bakragore", "Bakragore’s Taint"]),
    ("yellow_skull", ["Żółta czaszka", "Yellow Skull"]),
    ("party_mode", ["Tryb drużyny", "Party Mode"]),
    ("white_skull", ["Biała czaszka", "White Skull"]),
    ("red_skull", ["Czerwona czaszka", "Red Skull"]),
    ("black_skull", ["Czarna czaszka", "Black Skull"]),
    ("orange_skull", ["Pomarańczowa czaszka", "Orange Skull"]),
    ("guild_war", ["Wojna gildii", "In Guild War"]),
    ("hungry", ["Głód", "Hungry"]),
];

pub(super) fn show(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    section: &str,
    en: bool,
    state: &mut PageState,
) -> bool {
    match section {
        "hud" => hud(ui, draft, en),
        "miscellaneous" => miscellaneous(ui, draft, en),
        "screenshots" => screenshots(ui, draft, en, state),
        _ => return false,
    }
    true
}

fn hud_card(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    id: &str,
    labels: [&str; 2],
    en: bool,
    body: impl FnOnce(&mut egui::Ui, &mut ClientSettings),
) {
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 61, 61)))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(35, 39, 42, 160))
                .inner_margin(egui::Margin::symmetric(12, 5))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    reference_option(ui, draft, "hud", id, labels, en);
                });
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    body(ui, draft);
                });
        });
    ui.add_space(8.0);
}

fn hud(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    let tr = |pl, en_| if en { en_ } else { pl };
    ui.columns(2, |columns| {
        hud_card(
            &mut columns[0],
            draft,
            "hud.player_enabled",
            ["Własna postać", "Own character"],
            en,
            |ui, draft| {
                ui.add_enabled_ui(enabled(draft, "hud.player_enabled"), |ui| {
                    ui.columns(2, |cols| {
                        for (column, id, labels) in [
                            (0, "hud.resource_bars", ["Paski zasobów", "Show Bars"]),
                            (1, "hud.owner_name", ["Nazwa", "Show Name"]),
                            (0, "hud.owner_health", ["Zdrowie", "Show Health"]),
                            (1, "hud.owner_mana", ["Mana", "Show Mana"]),
                            (0, "show_harmony", ["Harmonia", "Show Harmony"]),
                            (1, "hud.marks", ["Znaczniki", "Show Marks"]),
                        ] {
                            reference_option(&mut cols[column], draft, "hud", id, labels, en);
                        }
                    });
                    ui.add_enabled_ui(enabled(draft, "show_harmony"), |ui| {
                        reference_option(
                            ui,
                            draft,
                            "hud",
                            "harmony_position",
                            ["Harmonia obok", "Harmony beside"],
                            en,
                        );
                    });
                });
            },
        );
        hud_card(
            &mut columns[1],
            draft,
            "hud.creatures_enabled",
            ["Inne stworzenia", "Other creatures"],
            en,
            |ui, draft| {
                ui.add_enabled_ui(enabled(draft, "hud.creatures_enabled"), |ui| {
                    ui.columns(2, |cols| {
                        for (column, id, labels) in [
                            (0, "hud.other_names", ["Nazwa", "Show Name"]),
                            (1, "hud.other_health", ["Zdrowie", "Show Health"]),
                            (0, "other_marks", ["Znaczniki", "Show Marks"]),
                            (1, "hud.npc_icons", ["Ikony NPC", "Show NPC Icons"]),
                        ] {
                            reference_option(&mut cols[column], draft, "hud", id, labels, en);
                        }
                    });
                });
                ui.add_space(8.0);
                ui.weak(tr(
                    "Postacie, potwory i NPC widoczne na ekranie.",
                    "Characters, monsters and NPCs on screen.",
                ));
            },
        );
    });
    hud_card(
        ui,
        draft,
        "hud.arcs",
        ["Łuki zasobów", "Resource arcs"],
        en,
        |ui, draft| {
            let active = enabled(draft, "hud.arcs") || enabled(draft, "conditions_hud_enabled");
            ui.add_enabled_ui(active, |ui| {
                ui.columns(3, |cols| {
                    reference_option(
                        &mut cols[0],
                        draft,
                        "hud",
                        "arc_size_preset",
                        ["Rozmiar", "Size"],
                        en,
                    );
                    percent(
                        &mut cols[1],
                        draft,
                        "hud",
                        "hud.arc_distance",
                        ["Odległość:", "Distance:"],
                        en,
                    );
                    percent(
                        &mut cols[2],
                        draft,
                        "hud",
                        "hud.arc_opacity",
                        ["Krycie:", "Opacity:"],
                        en,
                    );
                });
            });
        },
    );
    boxed(
        ui,
        tr("Stany specjalne · 35", "Special conditions · 35"),
        |ui| {
            let selection_id = egui::Id::new("selected-hud-condition");
            let mut selected = ui
                .ctx()
                .data(|d| d.get_temp::<String>(selection_id))
                .unwrap_or_else(|| "poison".into());
            let mut order: Vec<&str> = match draft.future_preferences.get("hud.condition_order") {
                Some(FutureValue::Text(text)) => text
                    .split(',')
                    .filter(|id| CONDITIONS.iter().any(|(known, _)| known == id))
                    .collect(),
                _ => Vec::new(),
            };
            let mut unique = Vec::new();
            for id in order.drain(..).chain(CONDITIONS.iter().map(|(id, _)| *id)) {
                if !unique.contains(&id) {
                    unique.push(id);
                }
            }
            let mut order: Vec<String> = unique.into_iter().map(str::to_owned).collect();
            let table_width = (ui.available_width() - 44.0).max(100.0);
            let col_width = table_width * 0.25;
            let row = |ui: &mut egui::Ui,
                       label: &str,
                       draft: &mut ClientSettings,
                       id: Option<&str>,
                       selected: &mut String| {
                ui.horizontal(|ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(table_width * 0.5 - 16.0, 28.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_min_width(table_width * 0.5 - 16.0);
                            if let Some(id) = id {
                                if ui.selectable_label(selected == id, label).clicked() {
                                    *selected = id.into();
                                }
                            } else {
                                ui.label(label);
                            }
                        },
                    );
                    for where_ in ["hud", "bar"] {
                        ui.allocate_ui_with_layout(
                            egui::vec2(col_width - 8.0, 28.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.set_min_width(col_width - 8.0);
                                if let Some(id) = id {
                                    ui.add_enabled_ui(
                                        enabled(draft, &format!("conditions_{where_}_enabled")),
                                        |ui| {
                                            reference_option(
                                                ui,
                                                draft,
                                                "hud",
                                                &format!("condition_{id}_{where_}"),
                                                ["", ""],
                                                en,
                                            );
                                        },
                                    );
                                } else {
                                    reference_option(
                                        ui,
                                        draft,
                                        "hud",
                                        &format!("conditions_{where_}_enabled"),
                                        if where_ == "hud" {
                                            ["Na HUD", "In HUD"]
                                        } else {
                                            ["Na pasku", "In bar"]
                                        },
                                        en,
                                    );
                                }
                            },
                        );
                    }
                });
            };
            row(ui, tr("Stan", "Condition"), draft, None, &mut selected);
            ui.horizontal_top(|ui| {
                egui::ScrollArea::vertical()
                    .id_salt("hud-conditions")
                    .max_height(180.0)
                    .max_width(table_width)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.vertical(|ui| {
                            ui.set_width(table_width);
                            for (index, id) in order.iter().enumerate() {
                                if let Some((_, labels)) =
                                    CONDITIONS.iter().find(|(known, _)| *known == id)
                                {
                                    if index % 2 == 1 {
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_size(
                                                ui.cursor().min,
                                                egui::vec2(table_width, 34.0),
                                            ),
                                            0.0,
                                            ui.visuals().faint_bg_color,
                                        );
                                    }
                                    row(
                                        ui,
                                        labels[usize::from(en)],
                                        draft,
                                        Some(id),
                                        &mut selected,
                                    );
                                }
                            }
                        });
                    });
                ui.vertical(|ui| {
                    ui.add_space(45.0);
                    let index = order.iter().position(|id| id == &selected).unwrap_or(0);
                    let order_supported =
                        oteryn_client::settings_catalog::runtime_consumer("hud.condition_order")
                            .is_some();
                    for (label, target, allowed) in [
                        ("↑", index.saturating_sub(1), index > 0),
                        (
                            "↓",
                            (index + 1).min(order.len() - 1),
                            index + 1 < order.len(),
                        ),
                    ] {
                        let response = ui.add_enabled(
                            allowed && order_supported,
                            egui::Button::new("").min_size(egui::vec2(30.0, 30.0)),
                        );
                        #[cfg(test)]
                        ui.ctx().data_mut(|d| {
                            d.insert_temp(egui::Id::new(("condition-order", label)), response.rect)
                        });
                        let center = response.rect.center();
                        let direction = if label == "↑" { -1.0 } else { 1.0 };
                        let color = if allowed && order_supported {
                            ui.visuals().text_color()
                        } else {
                            ui.visuals().weak_text_color()
                        };
                        ui.painter().add(egui::Shape::line(
                            vec![
                                center + egui::vec2(-4.0, -2.0 * direction),
                                center + egui::vec2(0.0, 2.0 * direction),
                                center + egui::vec2(4.0, -2.0 * direction),
                            ],
                            egui::Stroke::new(1.5, color),
                        ));
                        if response
                            .on_hover_text(if !order_supported {
                                tr(
                                    "Niedostępne bez projekcji stanów HUD",
                                    "Unavailable without a HUD condition projection",
                                )
                            } else if label == "↑" {
                                tr("Przenieś w górę", "Move up")
                            } else {
                                tr("Przenieś w dół", "Move down")
                            })
                            .clicked()
                        {
                            order.swap(index, target);
                            draft.future_preferences.insert(
                                "hud.condition_order".into(),
                                FutureValue::Text(order.join(",")),
                            );
                        }
                    }
                });
            });
            ui.ctx().data_mut(|d| d.insert_temp(selection_id, selected));
        },
    );
    ui.columns(2, |cols| {
        reference_option(
            &mut cols[0],
            draft,
            "hud",
            "custom_status_bars",
            [
                "Konfigurowalne paski stanu",
                "Show Customisable Status Bars",
            ],
            en,
        );
        reference_option(
            &mut cols[1],
            draft,
            "hud",
            "status_bars",
            ["Paski stanu", "Show Status Bars"],
            en,
        );
    });
}

fn enabled(draft: &ClientSettings, id: &str) -> bool {
    matches!(
        draft.future_preferences.get(&format!("hud.{id}")),
        Some(FutureValue::Bool(true))
    )
}

fn miscellaneous(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    for (id, labels) in [
        (
            "buy_confirmation",
            [
                "Pytaj przed zakupem produktów",
                "Ask Before Buying Products",
            ],
        ),
        (
            "stow_confirmation",
            [
                "Pytaj przed schowaniem zawartości",
                "Ask Before Stowing Container Content",
            ],
        ),
        (
            "sort_confirmation",
            [
                "Pytaj przed sortowaniem zagnieżdżonych pojemników",
                "Ask Before Sorting Nested Containers",
            ],
        ),
        (
            "move_confirmation",
            [
                "Pytaj przed przenoszeniem zawartości pojemników",
                "Ask Before Moving Contents of Nested Containers",
            ],
        ),
        (
            "stay_logged_in",
            [
                "Pozostań zalogowany w tej sesji",
                "Stay Logged In for Session",
            ],
        ),
        (
            "optimise_connection",
            [
                "Optymalizuj stabilność połączenia",
                "Optimise Connection Stability",
            ],
        ),
        ("misc.quick_login", ["Szybkie logowanie", "Quick Login"]),
    ] {
        row(ui, draft, "miscellaneous", id, labels, en);
    }
    ui.weak(if en {
        "Session selections do not store credentials or enable remembered sign-in."
    } else {
        "Te wybory nie zapisują danych logowania ani nie włączają zapamiętanej sesji."
    });
}

fn screenshots(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool, state: &mut PageState) {
    for (id, labels) in [
        (
            "capture.game_only",
            ["Tylko okno gry", "Only Capture Game Window"],
        ),
        (
            "capture.backlog",
            ["Bufor ostatnich 5 sekund", "Keep Backlog of Last 5 Seconds"],
        ),
        ("auto", ["Automatyczne zrzuty", "Enable Auto Screenshots"]),
    ] {
        row(ui, draft, "screenshots", id, labels, en);
    }
    boxed(
        ui,
        if en {
            "Capture Events"
        } else {
            "Zdarzenia do zapisu"
        },
        |ui| {
            ui.columns(2, |columns| {
                for (id, labels) in [
                    ("level", ["Awans poziomu", "Level Up"]),
                    ("skill", ["Awans umiejętności", "Skill Up"]),
                    ("capture.achievements", ["Osiągnięcie", "Achievement"]),
                    (
                        "bestiary_unlocked",
                        ["Odblokowany wpis bestiariusza", "Bestiary Entry Unlocked"],
                    ),
                    (
                        "bestiary_completed",
                        ["Ukończony wpis bestiariusza", "Bestiary Entry Completed"],
                    ),
                    ("capture.treasure", ["Znaleziony skarb", "Treasure Found"]),
                    ("capture.loot", ["Cenny łup", "Valuable Loot"]),
                    ("capture.boss", ["Pokonany boss", "Boss Defeated"]),
                ] {
                    reference_option(&mut columns[0], draft, "screenshots", id, labels, en);
                }
                for (id, labels) in [
                    ("death_pve", ["Śmierć PvE", "Death PvE"]),
                    ("death_pvp", ["Śmierć PvP", "Death PvP"]),
                    ("player_kill", ["Zabójstwo gracza", "Player Kill"]),
                    (
                        "player_assist",
                        ["Asysta przy zabójstwie", "Player Kill Assist"],
                    ),
                    ("player_attack", ["Atak gracza", "Player Attacking"]),
                    (
                        "capture.damage",
                        ["Najwyższe obrażenia", "Highest Damage Dealt"],
                    ),
                    (
                        "capture.healing",
                        ["Najwyższe leczenie", "Highest Healing Done"],
                    ),
                    ("capture.low_health", ["Niskie zdrowie", "Low Health"]),
                    ("gift_of_life", ["Dar życia", "Gift of Life Triggered"]),
                ] {
                    reference_option(&mut columns[1], draft, "screenshots", id, labels, en);
                }
            });
        },
    );
    if ui
        .button(if en {
            "Open Screenshot Folder"
        } else {
            "Otwórz folder zrzutów"
        })
        .on_hover_text(if en {
            "Create and open Oteryn's local screenshot folder."
        } else {
            "Utwórz i otwórz lokalny folder zrzutów Oteryn."
        })
        .clicked()
    {
        state.help_action = Some(HelpAction::OpenScreenshotFolder);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences_browser::test_support::{click, frame};

    #[test]
    fn hud_masters_gate_edits_without_erasing_saved_children() -> Result<(), &'static str> {
        for en in [false, true] {
            for (master, child) in [
                ("hud.player_enabled", "hud.owner_health"),
                ("hud.creatures_enabled", "hud.other_health"),
            ] {
                let ctx = egui::Context::default();
                let size = egui::vec2(560.0, 500.0);
                let mut draft = ClientSettings::default();
                let draw = |ctx: &egui::Context, draft: &mut ClientSettings| {
                    egui::Window::new("HUD test")
                        .default_size([510.0, 450.0])
                        .show(ctx, |ui| {
                            show(ui, draft, "hud", en, &mut PageState::default());
                        });
                };
                for _ in 0..3 {
                    let _ = frame(&ctx, size, vec![], |ctx| draw(ctx, &mut draft));
                }
                for (id, expected) in [
                    (child, None),
                    (master, Some(true)),
                    (child, Some(true)),
                    (master, Some(false)),
                    (child, Some(true)),
                ] {
                    let key = format!("hud.{id}");
                    let rect = ctx
                        .data(|data| {
                            data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                                "preference-toggle",
                                key.as_str(),
                            )))
                        })
                        .ok_or("toggle")?
                        .0;
                    click(&ctx, size, rect.center(), |ctx| draw(ctx, &mut draft));
                    assert_eq!(
                        draft.future_preferences.get(&key),
                        expected.map(FutureValue::Bool).as_ref()
                    );
                }
                assert!(
                    !draft
                        .future_preferences
                        .contains_key("hud.harmony_position")
                );
            }
        }
        Ok(())
    }

    #[test]
    fn unavailable_condition_controls_preserve_existing_choices_without_saving_more()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(1100.0, 900.0);
        let mut draft = ClientSettings::default();
        for (index, (id, _)) in CONDITIONS.iter().enumerate() {
            draft.future_preferences.insert(
                format!("hud.condition_{id}_hud"),
                FutureValue::Bool(index % 2 == 0),
            );
        }
        let choices = draft.future_preferences.clone();
        let draw = |ctx: &egui::Context, draft: &mut ClientSettings| {
            egui::Window::new("HUD test")
                .fixed_size([1050.0, 850.0])
                .show(ctx, |ui| {
                    show(ui, draft, "hud", true, &mut PageState::default());
                });
        };
        for _ in 0..3 {
            let _ = frame(&ctx, size, vec![], |ctx| draw(ctx, &mut draft));
        }
        for _ in 0..3 {
            let (rect, clip) = ctx
                .data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                        "preference-toggle",
                        "hud.conditions_hud_enabled",
                    )))
                })
                .ok_or("header")?;
            assert!(clip.contains_rect(rect));
            click(&ctx, size, rect.center(), |ctx| draw(ctx, &mut draft));
            assert!(
                !draft
                    .future_preferences
                    .contains_key("hud.conditions_hud_enabled")
            );
            for (key, expected) in &choices {
                assert_eq!(draft.future_preferences.get(key), Some(expected));
            }
        }
        let down = ctx
            .data(|d| d.get_temp::<egui::Rect>(egui::Id::new(("condition-order", "↓"))))
            .ok_or("reorder control")?;
        click(&ctx, size, down.center(), |ctx| draw(ctx, &mut draft));
        assert!(!draft.future_preferences.contains_key("hud.condition_order"));
        for (key, expected) in &choices {
            assert_eq!(draft.future_preferences.get(key), Some(expected));
        }
        draft.validate().map_err(|_| "valid condition flags")?;
        let restored: ClientSettings =
            serde_json::from_str(&serde_json::to_string(&draft).map_err(|_| "serialize")?)
                .map_err(|_| "deserialize")?;
        assert_eq!(restored, draft);
        Ok(())
    }
}
