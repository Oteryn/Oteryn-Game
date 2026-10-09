//! HUD, confirmation and capture page compositions. Server state is never fabricated.
use super::{boxed, percent, reference_option, row};
use oteryn_client::settings::ClientSettings;
use oteryn_client::settings_catalog::FutureValue;

const CONDITIONS: [(&str, [&str; 2]); 6] = [
    ("poison", ["Zatrucie", "Poisoned"]),
    ("burning", ["Płonięcie", "Burning"]),
    ("electric", ["Porażenie", "Electrified"]),
    ("bleeding", ["Krwawienie", "Bleeding"]),
    ("agony", ["Agonia", "Agony"]),
    ("powerless", ["Bezsilność", "Powerless"]),
];

pub(super) fn show(ui: &mut egui::Ui, draft: &mut ClientSettings, section: &str, en: bool) -> bool {
    match section {
        "hud" => {
            ui.scope(|ui| {
                ui.spacing_mut().interact_size.y = 18.0;
                ui.spacing_mut().item_spacing = egui::vec2(8.0, 1.0);
                ui.spacing_mut().icon_width = 14.0;
                ui.spacing_mut().icon_width_inner = 9.0;
                for text in [egui::TextStyle::Body, egui::TextStyle::Button] {
                    ui.style_mut()
                        .text_styles
                        .insert(text, egui::FontId::proportional(13.0));
                }
                hud(ui, draft, en);
            });
        }
        "miscellaneous" => miscellaneous(ui, draft, en),
        "screenshots" => screenshots(ui, draft, en),
        _ => return false,
    }
    true
}

fn hud_frame(ui: &mut egui::Ui, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().window_stroke)
        .corner_radius(5.0)
        .inner_margin(6.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
    ui.add_space(5.0);
}

fn hud(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    ui.label(
        egui::RichText::new(if en {
            "HUD and indicators"
        } else {
            "HUD i wskaźniki"
        })
        .size(21.0)
        .color(ui.visuals().text_color()),
    );
    ui.add_space(4.0);
    hud_frame(ui, |ui| {
        let top = ui.cursor().top();
        let divider = ui.max_rect().center().x;
        ui.columns(2, |columns| {
            reference_option(
                &mut columns[0],
                draft,
                "hud",
                "hud.player_enabled",
                ["HUD własnej postaci", "Show HUD for Own Character"],
                en,
            );
            dependents(
                &mut columns[0],
                enabled(draft, "hud.player_enabled"),
                |ui| {
                    for (id, labels) in [
                        ("hud.resource_bars", ["Paski zasobów", "Show Bars"]),
                        ("hud.owner_name", ["Nazwa", "Show Name"]),
                        ("hud.owner_health", ["Zdrowie", "Show Health"]),
                        ("hud.owner_mana", ["Mana", "Show Mana"]),
                        ("show_harmony", ["Harmonia", "Show Harmony"]),
                    ] {
                        reference_option(ui, draft, "hud", id, labels, en);
                    }
                    dependents(ui, enabled(draft, "show_harmony"), |ui| {
                        let key = "hud.harmony_position";
                        let stored = draft.future_preferences.get(key).cloned();
                        for (id, labels) in [
                            ("health", ["Obok łuku zdrowia", "next to Health Arc"]),
                            ("mana", ["Obok łuku many", "next to Mana Arc"]),
                        ] {
                            let selected =
                                matches!(&stored, Some(FutureValue::Choice(value)) if value == id);
                            if ui.radio(selected, labels[usize::from(en)]).clicked() {
                                draft
                                    .future_preferences
                                    .insert(key.into(), FutureValue::Choice(id.into()));
                            }
                        }
                    });
                    reference_option(
                        ui,
                        draft,
                        "hud",
                        "hud.marks",
                        ["Znaczniki", "Show Marks"],
                        en,
                    );
                },
            );
            reference_option(
                &mut columns[1],
                draft,
                "hud",
                "hud.creatures_enabled",
                ["HUD innych stworzeń", "Show HUD for Other Creatures"],
                en,
            );
            dependents(
                &mut columns[1],
                enabled(draft, "hud.creatures_enabled"),
                |ui| {
                    for (id, labels) in [
                        ("hud.other_names", ["Nazwa", "Show Name"]),
                        ("hud.other_health", ["Zdrowie", "Show Health"]),
                        ("other_marks", ["Znaczniki", "Show Marks"]),
                        ("hud.npc_icons", ["Ikony NPC", "Show NPC Icons"]),
                    ] {
                        reference_option(ui, draft, "hud", id, labels, en);
                    }
                },
            );
        });
        ui.painter().vline(
            divider,
            top..=ui.min_rect().bottom(),
            ui.visuals().widgets.noninteractive.bg_stroke,
        );
    });
    hud_frame(ui, |ui| {
        ui.columns(2, |columns| {
            reference_option(
                &mut columns[0],
                draft,
                "hud",
                "hud.arcs",
                ["Pokaż łuki", "Show Arcs"],
                en,
            );
            // Default/Small/Large verified in the private optionsmenu_hud_arc_size translations.
            let arcs_enabled = enabled(draft, "hud.arcs");
            dependents(&mut columns[0], arcs_enabled, |ui| {
                reference_option(ui, draft, "hud", "arc_size_preset", ["", ""], en);
            });
            columns[1].add_enabled_ui(arcs_enabled, |ui| {
                percent(
                    ui,
                    draft,
                    "hud",
                    "hud.arc_distance",
                    ["Odległość:", "Distance:"],
                    en,
                );
                percent(
                    ui,
                    draft,
                    "hud",
                    "hud.arc_opacity",
                    ["Widoczność:", "Opacity:"],
                    en,
                );
            });
        });
    });
    hud_frame(ui, |ui| {
        let widths = [
            ui.available_width() * 0.48,
            ui.available_width() * 0.25,
            ui.available_width() * 0.27,
        ];
        let row_height = 19.0;
        let cell = |ui: &mut egui::Ui, column: usize, draw: &mut dyn FnMut(&mut egui::Ui)| {
            ui.allocate_ui_with_layout(
                egui::vec2((widths[column] - 3.0).max(1.0), row_height),
                if column == 0 {
                    egui::Layout::left_to_right(egui::Align::Center)
                } else {
                    egui::Layout::right_to_left(egui::Align::Center)
                },
                |ui| {
                    ui.set_min_size(egui::vec2((widths[column] - 3.0).max(1.0), row_height));
                    draw(ui);
                },
            );
        };
        ui.horizontal(|ui| {
            cell(ui, 0, &mut |ui| {
                ui.label(if en {
                    "Special Condition"
                } else {
                    "Stan postaci"
                });
            });
            cell(ui, 1, &mut |ui| {
                condition_column_toggle(ui, draft, "hud", en);
                ui.label(if en { "Show in HUD" } else { "Pokaż w HUD" });
            });
            cell(ui, 2, &mut |ui| {
                condition_column_toggle(ui, draft, "bar", en);
                ui.label(if en { "Show in Bar" } else { "Pokaż na pasku" });
            });
        });
        for (index, (id, labels)) in CONDITIONS.into_iter().enumerate() {
            ui.horizontal(|ui| {
                if index % 2 == 0 {
                    let rect = egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(ui.available_width(), row_height),
                    );
                    ui.painter()
                        .rect_filled(rect, 0.0, ui.visuals().faint_bg_color);
                }
                cell(ui, 0, &mut |ui| {
                    ui.label(labels[usize::from(en)]);
                });
                for (column, where_) in [(1, "hud"), (2, "bar")] {
                    cell(ui, column, &mut |ui| {
                        reference_option(
                            ui,
                            draft,
                            "hud",
                            &format!("condition_{id}_{where_}"),
                            ["", ""],
                            en,
                        );
                    });
                }
            });
        }
    });
    hud_frame(ui, |ui| {
        ui.columns(2, |columns| {
            reference_option(
                &mut columns[0],
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
                &mut columns[1],
                draft,
                "hud",
                "status_bars",
                ["Paski stanu", "Show Status Bars"],
                en,
            );
        });
    });
}

fn condition_column_toggle(ui: &mut egui::Ui, draft: &mut ClientSettings, where_: &str, en: bool) {
    let keys = CONDITIONS.map(|(id, _)| format!("hud.condition_{id}_{where_}"));
    let all = |value| {
        keys.iter()
            .all(|key| draft.future_preferences.get(key) == Some(&FutureValue::Bool(value)))
    };
    let mut selected = all(true);
    let mixed = !selected && !all(false);
    let response = ui.push_id(where_, |ui| {
        ui.add(egui::Checkbox::new(&mut selected, "").indeterminate(mixed))
    }).inner.on_hover_text(if en {
        "Set this column for the six listed conditions. Saved selections; gameplay support is pending."
    } else {
        "Ustaw tę kolumnę dla sześciu widocznych stanów. Zapisane wybory; obsługa w grze jest w przygotowaniu."
    });
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new(("condition-column", where_)), response.rect)
    });
    if response.changed() {
        for key in keys {
            draft
                .future_preferences
                .insert(key, FutureValue::Bool(selected));
        }
    }
}

fn enabled(draft: &ClientSettings, id: &str) -> bool {
    matches!(
        draft.future_preferences.get(&format!("hud.{id}")),
        Some(FutureValue::Bool(true))
    )
}

fn dependents(ui: &mut egui::Ui, enabled: bool, body: impl FnOnce(&mut egui::Ui)) {
    ui.add_enabled_ui(enabled, |ui| {
        ui.horizontal_top(|ui| {
            ui.add_space(14.0);
            ui.vertical(body);
        });
    });
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

fn screenshots(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
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
    ui.add_enabled(
        false,
        egui::Button::new(if en {
            "Open Screenshot Folder"
        } else {
            "Otwórz folder zrzutów"
        }),
    )
    .on_hover_text(if en {
        "Capture storage is not configured yet."
    } else {
        "Zapis zrzutów nie został jeszcze podłączony."
    });
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
                            show(ui, draft, "hud", en);
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
    fn hud_condition_headers_edit_only_the_six_existing_column_flags() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(560.0, 500.0);
        let mut draft = ClientSettings::default();
        let draw = |ctx: &egui::Context, draft: &mut ClientSettings| {
            egui::Window::new("HUD test")
                .default_size([510.0, 450.0])
                .show(ctx, |ui| {
                    show(ui, draft, "hud", true);
                });
        };
        for _ in 0..3 {
            let _ = frame(&ctx, size, vec![], |ctx| draw(ctx, &mut draft));
        }
        assert!(draft.future_preferences.is_empty());
        for value in [true, false] {
            let rect = ctx
                .data(|data| {
                    data.get_temp::<egui::Rect>(egui::Id::new(("condition-column", "hud")))
                })
                .ok_or("header")?;
            click(&ctx, size, rect.center(), |ctx| draw(ctx, &mut draft));
            assert_eq!(draft.future_preferences.len(), CONDITIONS.len());
            for (id, _) in CONDITIONS {
                assert_eq!(
                    draft
                        .future_preferences
                        .get(&format!("hud.condition_{id}_hud")),
                    Some(&FutureValue::Bool(value))
                );
            }
        }
        draft.validate().map_err(|_| "valid condition flags")?;
        Ok(())
    }
}
