//! HUD, confirmation and capture page compositions. Server state is never fabricated.
use super::{HelpAction, PageState, percent, reference_option, retained_children, row};
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

const SCREENSHOT_EVENTS_LEFT: &[(&str, [&str; 2])] = &[
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
];

const SCREENSHOT_EVENTS_RIGHT: &[(&str, [&str; 2])] = &[
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
];

const CONDITION_ROW_HEIGHT: f32 = 16.5;
const CONDITION_VIEWPORT_HEIGHT: f32 = 100.0;
const CONDITION_REORDER_SIZE: f32 = 20.0;

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

fn hud(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    let tr = |pl, en_| if en { en_ } else { pl };
    hud_boxed(ui, "hud-master-frame", |ui| {
        let columns = ui.scope(|ui| {
            ui.spacing_mut().item_spacing.y = 1.0;
            ui.spacing_mut().interact_size.y = 16.0;
            ui.spacing_mut().button_padding.y = 0.0;
            ui.columns(2, |columns| {
                let ui = &mut columns[0];
                hud_master_row(
                    ui,
                    draft,
                    "hud.player_enabled",
                    ["Pokaż HUD własnej postaci", "Show HUD for Own Character"],
                    [
                        "Pokaż lub ukryj elementy HUD własnej postaci.",
                        "Show or hide HUD elements for your own character.",
                    ],
                    en,
                );
                ui.indent("own-hud-children", |ui| {
                    ui.add_enabled_ui(enabled(draft, "hud.player_enabled"), |ui| {
                        for (id, labels) in [
                            ("hud.resource_bars", ["Pokaż paski", "Show Bars"]),
                            ("hud.owner_name", ["Pokaż nazwę", "Show Name"]),
                            ("hud.owner_health", ["Pokaż zdrowie", "Show Health"]),
                            ("hud.owner_mana", ["Pokaż manę", "Show Mana"]),
                            ("show_harmony", ["Pokaż harmonię", "Show Harmony"]),
                        ] {
                            reference_option(ui, draft, "hud", id, labels, en);
                        }
                        ui.add_enabled_ui(enabled(draft, "show_harmony"), |ui| {
                            ui.indent("harmony-position", |ui| harmony_position(ui, draft, en));
                        });
                        reference_option(
                            ui,
                            draft,
                            "hud",
                            "hud.marks",
                            ["Pokaż znaczniki", "Show Marks"],
                            en,
                        );
                    });
                });

                let ui = &mut columns[1];
                hud_master_row(
                    ui,
                    draft,
                    "hud.creatures_enabled",
                    ["Pokaż HUD innych stworzeń", "Show HUD for Other Creatures"],
                    [
                        "Pokaż lub ukryj elementy HUD innych stworzeń.",
                        "Show or hide HUD elements for other creatures.",
                    ],
                    en,
                );
                ui.indent("other-hud-children", |ui| {
                    ui.add_enabled_ui(enabled(draft, "hud.creatures_enabled"), |ui| {
                        for (id, labels) in [
                            ("hud.other_names", ["Pokaż nazwę", "Show Name"]),
                            ("hud.other_health", ["Pokaż zdrowie", "Show Health"]),
                            ("other_marks", ["Pokaż znaczniki", "Show Marks"]),
                            ("hud.npc_icons", ["Pokaż ikony NPC", "Show NPC Icons"]),
                        ] {
                            reference_option(ui, draft, "hud", id, labels, en);
                        }
                    });
                });
            });
        });
        let columns_rect = columns.response.rect;
        let divider_x = columns_rect.center().x;
        ui.painter().vline(
            divider_x,
            columns_rect.y_range(),
            egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 61, 61)),
        );
        #[cfg(test)]
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                egui::Id::new("hud-columns-divider"),
                (divider_x, columns_rect),
            )
        });
    });
    hud_boxed(ui, "hud-arcs-frame", |ui| {
        ui.spacing_mut().item_spacing.y = 1.0;
        ui.spacing_mut().interact_size.y = 16.0;
        ui.spacing_mut().button_padding.y = 0.0;
        reference_option(
            ui,
            draft,
            "hud",
            "hud.arcs",
            ["Pokaż łuki", "Show Arcs"],
            en,
        );
        let active = enabled(draft, "hud.arcs");
        ui.add_enabled_ui(active, |ui| {
            ui.columns(2, |cols| {
                reference_option(&mut cols[0], draft, "hud", "arc_size_preset", ["", ""], en);
                cols[1].vertical(|ui| {
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
                        ["Krycie:", "Opacity:"],
                        en,
                    );
                });
            });
        });
    });
    let condition_frame = egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 61, 61)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(12, 5))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.spacing_mut().interact_size.y = 16.0;
            ui.spacing_mut().button_padding.y = 0.0;
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
            let order_supported =
                oteryn_client::settings_catalog::runtime_consumer("hud.condition_order").is_some();
            #[cfg(test)]
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    egui::Id::new("hud-condition-selection-enabled"),
                    order_supported,
                )
            });
            let table_width = (ui.available_width() - 44.0).max(100.0);
            let col_width = table_width * 0.25;
            let row = |ui: &mut egui::Ui,
                       label: &str,
                       draft: &mut ClientSettings,
                       id: Option<&str>,
                       selected: &mut String| {
                let response = ui
                    .allocate_ui_with_layout(
                        egui::vec2(table_width, CONDITION_ROW_HEIGHT),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.spacing_mut().button_padding.y = 0.0;
                            ui.allocate_ui_with_layout(
                                egui::vec2(table_width * 0.5 - 16.0, CONDITION_ROW_HEIGHT),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_min_width(table_width * 0.5 - 16.0);
                                    if let Some(id) = id {
                                        ui.add_enabled_ui(order_supported, |ui| {
                                            let icon = condition_icon(ui, id, label);
                                            if icon.clicked()
                                                || ui
                                                    .selectable_label(selected == id, label)
                                                    .clicked()
                                            {
                                                *selected = id.into();
                                            }
                                        })
                                        .response
                                        .on_disabled_hover_text(tr(
                                            "Niedostępne bez projekcji stanów HUD",
                                            "Unavailable without a HUD condition projection",
                                        ));
                                    } else {
                                        ui.label(label);
                                    }
                                },
                            );
                            for where_ in ["hud", "bar"] {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(col_width - 8.0, CONDITION_ROW_HEIGHT),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.set_min_width(col_width - 8.0);
                                        if let Some(id) = id {
                                            ui.add_enabled_ui(
                                                enabled(
                                                    draft,
                                                    &format!("conditions_{where_}_enabled"),
                                                ),
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
                                            condition_header_toggle(ui, draft, where_, en);
                                        }
                                    },
                                );
                            }
                        },
                    )
                    .response;
                #[cfg(test)]
                ui.ctx().data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new(("hud-condition-row", id.unwrap_or("header"))),
                        (response.rect, ui.clip_rect()),
                    )
                });
            };
            row(
                ui,
                tr("Stan specjalny", "Special Condition"),
                draft,
                None,
                &mut selected,
            );
            ui.horizontal_top(|ui| {
                crate::settings_audit::show_bounded_vertical(
                    ui,
                    "hud-conditions",
                    CONDITION_VIEWPORT_HEIGHT,
                    Some(table_width),
                    |ui| {
                        ui.spacing_mut().item_spacing.y = 0.0;
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.set_width(table_width);
                            for (index, id) in order.iter().enumerate() {
                                if let Some((_, labels)) =
                                    CONDITIONS.iter().find(|(known, _)| *known == id)
                                {
                                    if index % 2 == 1 {
                                        ui.painter().rect_filled(
                                            egui::Rect::from_min_size(
                                                ui.cursor().min,
                                                egui::vec2(table_width, CONDITION_ROW_HEIGHT),
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
                    },
                );
                ui.vertical(|ui| {
                    let index = order.iter().position(|id| id == &selected).unwrap_or(0);
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
                            egui::Button::new("").min_size(egui::vec2(
                                CONDITION_REORDER_SIZE,
                                CONDITION_REORDER_SIZE,
                            )),
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
        });
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new("hud-condition-frame"),
            condition_frame.response.rect,
        )
    });
    hud_status_row(
        ui,
        draft,
        "custom_status_bars",
        [
            "Konfigurowalne paski stanu",
            "Show Customisable Status Bars",
        ],
        en,
    );
    hud_status_row(
        ui,
        draft,
        "status_bars",
        ["Paski stanu", "Show Status Bars"],
        en,
    );
}

fn hud_boxed(ui: &mut egui::Ui, test_id: &'static str, body: impl FnOnce(&mut egui::Ui)) {
    let frame = egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 61, 61)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(12, 5))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            body(ui);
        });
    #[cfg(test)]
    ui.ctx()
        .data_mut(|data| data.insert_temp(egui::Id::new(test_id), frame.response.rect));
}

fn condition_header_toggle(ui: &mut egui::Ui, draft: &mut ClientSettings, where_: &str, en: bool) {
    let labels = if where_ == "hud" {
        ["Pokaż w HUD", "Show in HUD"]
    } else {
        ["Pokaż na pasku", "Show in Bar"]
    };
    ui.horizontal(|ui| {
        ui.label(labels[usize::from(en)]);
        reference_option(
            ui,
            draft,
            "hud",
            &format!("conditions_{where_}_enabled"),
            ["", ""],
            en,
        );
    });
}

fn hud_master_row(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    id: &'static str,
    labels: [&str; 2],
    info: [&str; 2],
    en: bool,
) {
    let row = ui.horizontal(|ui| {
        reference_option(ui, draft, "hud", id, labels, en);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            info_glyph(ui, egui::Id::new(("hud-master-info", id)), info, en);
        });
    });
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new(("hud-master-row", id)), row.response.rect)
    });
}

fn info_glyph(ui: &mut egui::Ui, id: egui::Id, labels: [&str; 2], en: bool) {
    let response = ui
        .allocate_response(egui::vec2(18.0, 18.0), egui::Sense::hover())
        .on_hover_text(labels[usize::from(en)]);
    let color = ui.visuals().weak_text_color();
    ui.painter()
        .circle_stroke(response.rect.center(), 7.0, egui::Stroke::new(1.0, color));
    ui.painter().text(
        response.rect.center(),
        egui::Align2::CENTER_CENTER,
        "i",
        egui::FontId::proportional(11.0),
        color,
    );
    #[cfg(test)]
    ui.ctx()
        .data_mut(|data| data.insert_temp(id, (response.rect, labels[usize::from(en)].to_owned())));
}

fn hud_status_row(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    id: &'static str,
    labels: [&str; 2],
    en: bool,
) {
    let _row = egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(58, 61, 61)))
        .corner_radius(4.0)
        .inner_margin(egui::Margin::symmetric(12, 4))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().interact_size.y = 16.0;
            ui.spacing_mut().button_padding.y = 0.0;
            ui.horizontal(|ui| {
                reference_option(ui, draft, "hud", id, labels, en);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    info_glyph(
                        ui,
                        egui::Id::new(("hud-status-info", id)),
                        [
                            "Niedostępne bez projekcji aktora i pasków stanu.",
                            "Unavailable without an actor/status-bar projection.",
                        ],
                        en,
                    );
                });
            });
        });
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new(("hud-status-row", id)), _row.response.rect)
    });
}

fn enabled(draft: &ClientSettings, id: &str) -> bool {
    matches!(
        draft.future_preferences.get(&format!("hud.{id}")),
        Some(FutureValue::Bool(true))
    )
}

fn harmony_position(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    let key = "hud.harmony_position";
    let selected = match draft.future_preferences.get(key) {
        Some(FutureValue::Choice(value)) => Some(value.clone()),
        _ => None,
    };
    ui.vertical(|ui| {
        for (id, labels) in [
            ("health", ["obok łuku zdrowia", "next to Health Arc"]),
            ("mana", ["obok łuku many", "next to Mana Arc"]),
        ] {
            let response = ui.radio(selected.as_deref() == Some(id), labels[usize::from(en)]);
            #[cfg(test)]
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    egui::Id::new(("harmony-position", id)),
                    (response.rect, ui.clip_rect()),
                )
            });
            if response.clicked() {
                draft
                    .future_preferences
                    .insert(key.into(), FutureValue::Choice(id.into()));
            }
        }
    });
}

fn condition_icon(ui: &mut egui::Ui, id: &str, label: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::click());
    let painter = ui.painter();
    let frame = rect.shrink(1.0);
    painter.rect_filled(frame, 2.0, egui::Color32::from_rgb(31, 34, 35));
    painter.rect_stroke(
        frame,
        2.0,
        egui::Stroke::new(0.7, egui::Color32::from_rgb(79, 83, 82)),
        egui::StrokeKind::Inside,
    );
    paint_condition_icon(painter, frame.shrink(2.0), id);
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("condition-icon", id)),
            (rect, ui.clip_rect()),
        )
    });
    response.on_hover_text(label)
}

#[rustfmt::skip]
fn paint_condition_icon(painter: &egui::Painter, rect: egui::Rect, id: &str) {
    use egui::{Color32, Pos2, Shape, Stroke};

    let point = |x: f32, y: f32| Pos2::new(rect.left() + rect.width() * x, rect.top() + rect.height() * y);
    let line = |points: &[[f32; 2]], color: Color32, width: f32| {
        painter.add(Shape::line(points.iter().map(|p| point(p[0], p[1])).collect(), Stroke::new(width, color)));
    };
    let polygon = |points: &[[f32; 2]], color: Color32| {
        painter.add(Shape::convex_polygon(points.iter().map(|p| point(p[0], p[1])).collect(), color, Stroke::NONE));
    };
    let circle = |x: f32, y: f32, radius: f32, color: Color32| {
        painter.circle_filled(point(x, y), rect.width() * radius, color);
    };
    let green = Color32::from_rgb(82, 222, 83);
    let red = Color32::from_rgb(238, 55, 62);
    let blue = Color32::from_rgb(92, 191, 246);
    let yellow = Color32::from_rgb(246, 199, 53);
    let orange = Color32::from_rgb(240, 127, 39);
    let violet = Color32::from_rgb(192, 97, 232);
    let white = Color32::from_rgb(224, 230, 229);
    let grey = Color32::from_rgb(147, 153, 154);
    let ink = match id {
        "poison" | "haste" | "strengthened" | "in_pz" | "party_mode" => green,
        "burning" | "bleeding" | "agony" | "pz_block" | "red_skull" => red,
        "electric" | "dazzled" | "lesser_hex" | "intense_hex" | "greater_hex" | "yellow_skull" | "hungry" => yellow,
        "rooted" | "logout_block" | "orange_skull" => orange,
        "magic_shield" | "monk_virtue" | "drowning" | "freezing" => blue,
        "feared" | "cursed" | "goshnar_taint" => violet,
        "powerless" | "drunk" | "slowed" | "resting" | "bakragore_taint" | "black_skull" | "guild_war" => grey,
        _ => white,
    };
    match id {
        "poison" | "bleeding" | "drowning" => {
            polygon(&[[0.50,0.08],[0.77,0.50],[0.70,0.77],[0.50,0.91],[0.30,0.77],[0.23,0.50]], ink);
            if id == "poison" { circle(0.49,0.57,0.10,Color32::from_rgb(24,91,35)); }
            if id == "drowning" { line(&[[0.29,0.63],[0.43,0.57],[0.57,0.63],[0.71,0.57]],white,1.0); }
        }
        "burning" => polygon(&[[0.53,0.04],[0.67,0.35],[0.78,0.49],[0.72,0.76],[0.51,0.92],[0.28,0.77],[0.25,0.55],[0.39,0.34],[0.42,0.61],[0.57,0.50]], ink),
        "electric" => polygon(&[[0.55,0.04],[0.24,0.55],[0.47,0.54],[0.39,0.94],[0.76,0.40],[0.53,0.42]], ink),
        "agony" => { circle(0.50,0.50,0.14,ink); for p in [[[0.50,0.05],[0.50,0.28]],[[0.50,0.72],[0.50,0.95]],[[0.05,0.50],[0.28,0.50]],[[0.72,0.50],[0.95,0.50]],[[0.18,0.18],[0.34,0.34]],[[0.66,0.66],[0.82,0.82]]] { line(&p,ink,1.2); } }
        "powerless" => { painter.rect_stroke(egui::Rect::from_min_max(point(0.18,0.26),point(0.76,0.77)),1.0,Stroke::new(1.2,ink),egui::StrokeKind::Inside); line(&[[0.76,0.42],[0.90,0.42],[0.90,0.61],[0.76,0.61]],ink,1.2); line(&[[0.55,0.31],[0.40,0.49],[0.57,0.49],[0.42,0.72]],red,1.2); }
        "rooted" => { line(&[[0.50,0.09],[0.50,0.62],[0.22,0.90]],ink,1.4); line(&[[0.50,0.62],[0.78,0.90]],ink,1.4); line(&[[0.50,0.46],[0.24,0.27]],green,1.2); line(&[[0.50,0.38],[0.74,0.20]],green,1.2); }
        "feared" => { line(&[[0.10,0.51],[0.29,0.32],[0.50,0.25],[0.71,0.32],[0.90,0.51],[0.71,0.69],[0.50,0.76],[0.29,0.69],[0.10,0.51]],ink,1.2); circle(0.50,0.51,0.14,ink); circle(0.50,0.51,0.055,Color32::BLACK); }
        "drunk" => { circle(0.31,0.64,0.15,ink); circle(0.58,0.38,0.12,orange); circle(0.74,0.67,0.09,yellow); }
        "magic_shield" | "in_pz" | "pz_block" => {
            line(&[[0.50,0.08],[0.79,0.20],[0.75,0.62],[0.50,0.90],[0.25,0.62],[0.21,0.20],[0.50,0.08]],ink,1.4);
            if id == "in_pz" { line(&[[0.32,0.52],[0.46,0.66],[0.70,0.36]],green,1.6); }
            if id == "pz_block" { line(&[[0.18,0.82],[0.82,0.18]],red,1.8); }
        }
        "monk_virtue" => { circle(0.50,0.50,0.18,ink); for p in [[[0.50,0.06],[0.50,0.22]],[[0.50,0.78],[0.50,0.94]],[[0.06,0.50],[0.22,0.50]],[[0.78,0.50],[0.94,0.50]]] { line(&p,ink,1.2); } }
        "slowed" | "haste" | "strengthened" => {
            let up = id != "slowed";
            let ys = if up { [[0.72,0.45,0.25],[0.48,0.21,0.49]] } else { [[0.28,0.55,0.75],[0.52,0.79,0.51]] };
            for [tip,left_y,right_y] in ys { line(&[[0.20,left_y],[0.50,tip],[0.80,right_y]],ink,1.5); }
        }
        "logout_block" => { painter.rect_stroke(egui::Rect::from_min_max(point(0.22,0.44),point(0.78,0.86)),1.0,Stroke::new(1.3,ink),egui::StrokeKind::Inside); line(&[[0.34,0.44],[0.34,0.31],[0.42,0.18],[0.58,0.18],[0.66,0.31],[0.66,0.44]],ink,1.3); circle(0.50,0.63,0.055,red); }
        "freezing" => { for p in [[[0.50,0.06],[0.50,0.94]],[[0.12,0.28],[0.88,0.72]],[[0.12,0.72],[0.88,0.28]]] { line(&p,ink,1.15); } circle(0.50,0.50,0.07,white); }
        "dazzled" => polygon(&[[0.50,0.06],[0.61,0.37],[0.94,0.39],[0.68,0.58],[0.76,0.91],[0.50,0.71],[0.24,0.91],[0.32,0.58],[0.06,0.39],[0.39,0.37]],ink),
        "cursed" | "yellow_skull" | "white_skull" | "red_skull" | "black_skull" | "orange_skull" => {
            circle(0.50,0.43,0.28,ink); painter.rect_filled(egui::Rect::from_min_max(point(0.34,0.55),point(0.66,0.80)),1.0,ink); circle(0.40,0.41,0.055,Color32::BLACK); circle(0.60,0.41,0.055,Color32::BLACK); line(&[[0.43,0.65],[0.43,0.79]],Color32::BLACK,1.0); line(&[[0.57,0.65],[0.57,0.79]],Color32::BLACK,1.0);
        }
        "resting" => { circle(0.47,0.47,0.31,ink); circle(0.59,0.37,0.27,Color32::from_rgb(31,34,35)); circle(0.70,0.72,0.045,blue); }
        "lesser_hex" | "intense_hex" | "greater_hex" => {
            line(&[[0.50,0.08],[0.84,0.28],[0.84,0.68],[0.50,0.89],[0.16,0.68],[0.16,0.28],[0.50,0.08]],ink,1.2);
            let dots = match id { "lesser_hex" => 1, "intense_hex" => 2, _ => 3 };
            for index in 0..dots { circle(0.38 + index as f32 * 0.12,0.50,0.045,ink); }
        }
        "goshnar_taint" | "bakragore_taint" => { circle(0.50,0.50,0.27,ink); circle(0.50,0.50,0.15,Color32::from_rgb(31,34,35)); line(&[[0.12,0.18],[0.33,0.34]],red,1.2); line(&[[0.88,0.18],[0.67,0.34]],blue,1.2); line(&[[0.12,0.82],[0.33,0.66]],yellow,1.2); }
        "party_mode" => { circle(0.50,0.32,0.12,ink); circle(0.26,0.43,0.09,ink); circle(0.74,0.43,0.09,ink); line(&[[0.22,0.79],[0.29,0.60],[0.42,0.57],[0.50,0.76],[0.58,0.57],[0.71,0.60],[0.78,0.79]],ink,1.4); }
        "guild_war" => { line(&[[0.16,0.16],[0.84,0.84]],ink,1.5); line(&[[0.84,0.16],[0.16,0.84]],yellow,1.5); line(&[[0.12,0.28],[0.28,0.12]],ink,1.2); line(&[[0.72,0.12],[0.88,0.28]],yellow,1.2); }
        "hungry" => { line(&[[0.28,0.10],[0.28,0.90]],ink,1.2); line(&[[0.18,0.10],[0.18,0.39],[0.38,0.39],[0.38,0.10]],ink,1.1); line(&[[0.67,0.10],[0.67,0.90]],ink,1.2); line(&[[0.67,0.10],[0.82,0.28],[0.67,0.44]],ink,1.2); }
        _ => { circle(0.50,0.50,0.21,ink); }
    }
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
}

fn screenshots(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool, state: &mut PageState) {
    for (id, labels) in [
        (
            "capture.game_only",
            ["Tylko okno gry", "Only Capture Game Window"],
        ),
        (
            "capture.backlog",
            [
                "Zachowaj historię zrzutów z ostatnich 5 sekund",
                "Keep Backlog of the Screenshots of the Last 5 Seconds",
            ],
        ),
        ("auto", ["Automatyczne zrzuty", "Enable Auto Screenshots"]),
    ] {
        row(ui, draft, "screenshots", id, labels, en);
    }
    ui.label(if en {
        "Select all events that should trigger auto screenshots:"
    } else {
        "Wybierz wszystkie zdarzenia, które powinny wywołać automatyczny zrzut ekranu:"
    });
    retained_children(
        ui,
        draft.stored_bool("screenshots.auto") == Some(true),
        "screenshot-automatic-events",
        |ui| {
            ui.columns(2, |columns| {
                for (id, labels) in SCREENSHOT_EVENTS_LEFT {
                    reference_option(&mut columns[0], draft, "screenshots", id, *labels, en);
                }
                for (id, labels) in SCREENSHOT_EVENTS_RIGHT {
                    reference_option(&mut columns[1], draft, "screenshots", id, *labels, en);
                }
            });
        },
    );
    ui.add_space(8.0);
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
    use crate::preferences_browser::{
        PreferencesBrowser,
        test_support::{button_position, click, frame, scroll_geometry},
    };

    #[test]
    fn hud_uses_reference_masters_radio_harmony_and_all_condition_icons() -> Result<(), &'static str>
    {
        for en in [false, true] {
            let ctx = egui::Context::default();
            let size = egui::vec2(1100.0, 900.0);
            let mut draft = ClientSettings::default();
            draft
                .future_preferences
                .insert("hud.hud.player_enabled".into(), FutureValue::Bool(true));
            draft
                .future_preferences
                .insert("hud.show_harmony".into(), FutureValue::Bool(true));
            let draw = |ctx: &egui::Context, draft: &mut ClientSettings| {
                egui::Window::new("HUD test")
                    .fixed_size([1050.0, 850.0])
                    .show(ctx, |ui| {
                        show(ui, draft, "hud", en, &mut PageState::default());
                    });
            };
            let mut output = egui::FullOutput::default();
            for _ in 0..3 {
                output = frame(&ctx, size, vec![], |ctx| draw(ctx, &mut draft)).1;
            }
            for label in if en {
                [
                    "Show HUD for Own Character",
                    "Show HUD for Other Creatures",
                    "Show Arcs",
                    "Special Condition",
                    "Show in HUD",
                    "Show in Bar",
                    "next to Health Arc",
                    "next to Mana Arc",
                ]
            } else {
                [
                    "Pokaż HUD własnej postaci",
                    "Pokaż HUD innych stworzeń",
                    "Pokaż łuki",
                    "Stan specjalny",
                    "Pokaż w HUD",
                    "Pokaż na pasku",
                    "obok łuku zdrowia",
                    "obok łuku many",
                ]
            } {
                assert!(
                    button_position(&output, size, label).is_some(),
                    "missing {label}"
                );
            }
            for stale in [
                "Own character",
                "Other creatures",
                "Harmony beside",
                "Resource arcs",
                "Special conditions · 35",
                "Condition",
                "In HUD",
                "In bar",
            ] {
                assert!(
                    button_position(&output, size, stale).is_none(),
                    "stale {stale}"
                );
            }
            let toggle = |key: &str| {
                ctx.data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                        "preference-toggle",
                        key,
                    )))
                    .map(|(rect, _)| rect)
                })
            };
            let radio = |id: &'static str| {
                ctx.data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                        "harmony-position",
                        id,
                    )))
                })
            };
            let own_order = [
                toggle("hud.hud.player_enabled").ok_or("own HUD master")?,
                toggle("hud.hud.resource_bars").ok_or("own bars")?,
                toggle("hud.hud.owner_name").ok_or("own name")?,
                toggle("hud.hud.owner_health").ok_or("own health")?,
                toggle("hud.hud.owner_mana").ok_or("own mana")?,
                toggle("hud.show_harmony").ok_or("own harmony")?,
                radio("health").ok_or("health radio")?.0,
                radio("mana").ok_or("mana radio")?.0,
                toggle("hud.hud.marks").ok_or("own marks")?,
            ];
            assert!(
                own_order
                    .windows(2)
                    .all(|pair| pair[0].center().y < pair[1].center().y),
                "own-character HUD controls must follow the captured vertical order"
            );
            let other_order = [
                toggle("hud.hud.creatures_enabled").ok_or("other HUD master")?,
                toggle("hud.hud.other_names").ok_or("other name")?,
                toggle("hud.hud.other_health").ok_or("other health")?,
                toggle("hud.other_marks").ok_or("other marks")?,
                toggle("hud.hud.npc_icons").ok_or("NPC icons")?,
            ];
            assert!(
                other_order
                    .windows(2)
                    .all(|pair| pair[0].center().y < pair[1].center().y),
                "other-creature HUD controls must follow the captured vertical order"
            );
            assert!(
                own_order[0].center().x < other_order[0].center().x,
                "the own and other creature groups must share one two-column frame"
            );
            assert!(
                own_order[1].left() >= own_order[0].left() + 10.0,
                "own-character children must be indented under their master"
            );
            assert!(
                other_order[1].left() >= other_order[0].left() + 10.0,
                "other-creature children must be indented under their master"
            );
            let (divider_x, columns_rect) = ctx
                .data(|data| {
                    data.get_temp::<(f32, egui::Rect)>(egui::Id::new("hud-columns-divider"))
                })
                .ok_or("HUD column divider")?;
            assert!((divider_x - columns_rect.center().x).abs() <= 0.5);
            assert!(own_order[0].center().x < divider_x);
            assert!(other_order[0].center().x > divider_x);
            assert!(columns_rect.top() <= own_order[0].top());
            assert!(columns_rect.bottom() >= own_order.last().ok_or("own last")?.bottom());
            for id in ["hud.player_enabled", "hud.creatures_enabled"] {
                let row = ctx
                    .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("hud-master-row", id))))
                    .ok_or("HUD master row")?;
                let (info, tooltip) = ctx
                    .data(|data| {
                        data.get_temp::<(egui::Rect, String)>(egui::Id::new((
                            "hud-master-info",
                            id,
                        )))
                    })
                    .ok_or("HUD master info glyph")?;
                assert!(row.contains_rect(info));
                assert!(
                    info.center().x > row.center().x,
                    "HUD master info glyph must remain on the right"
                );
                let expected = match (en, id) {
                    (true, "hud.player_enabled") => {
                        "Show or hide HUD elements for your own character."
                    }
                    (true, _) => "Show or hide HUD elements for other creatures.",
                    (false, "hud.player_enabled") => {
                        "Pokaż lub ukryj elementy HUD własnej postaci."
                    }
                    (false, _) => "Pokaż lub ukryj elementy HUD innych stworzeń.",
                };
                assert_eq!(tooltip, expected);
            }
            assert!(
                toggle("hud.custom_status_bars")
                    .ok_or("custom status bars")?
                    .center()
                    .y
                    < toggle("hud.status_bars").ok_or("status bars")?.center().y,
                "status-bar controls must remain vertically ordered"
            );
            let custom_row = ctx
                .data(|data| {
                    data.get_temp::<egui::Rect>(egui::Id::new((
                        "hud-status-row",
                        "custom_status_bars",
                    )))
                })
                .ok_or("custom status row")?;
            let status_row = ctx
                .data(|data| {
                    data.get_temp::<egui::Rect>(egui::Id::new(("hud-status-row", "status_bars")))
                })
                .ok_or("status row")?;
            assert!(custom_row.bottom() < status_row.top());
            assert!(custom_row.width() > 900.0, "status row is not full-width");
            assert!((custom_row.width() - status_row.width()).abs() <= 1.0);
            for (id, row_rect) in [
                ("custom_status_bars", custom_row),
                ("status_bars", status_row),
            ] {
                let (info, tooltip) = ctx
                    .data(|data| {
                        data.get_temp::<(egui::Rect, String)>(egui::Id::new((
                            "hud-status-info",
                            id,
                        )))
                    })
                    .ok_or("status info glyph")?;
                assert!(row_rect.contains_rect(info));
                assert!(
                    info.center().x > row_rect.center().x,
                    "status info glyph must remain on the right"
                );
                assert_eq!(
                    tooltip,
                    if en {
                        "Unavailable without an actor/status-bar projection."
                    } else {
                        "Niedostępne bez projekcji aktora i pasków stanu."
                    }
                );
            }
            for (where_, label) in if en {
                [("hud", "Show in HUD"), ("bar", "Show in Bar")]
            } else {
                [("hud", "Pokaż w HUD"), ("bar", "Pokaż na pasku")]
            } {
                let label = button_position(&output, size, label).ok_or("condition header")?;
                let toggle = toggle(&format!("hud.conditions_{where_}_enabled"))
                    .ok_or("condition header toggle")?;
                assert!(
                    label.x < toggle.center().x,
                    "condition header checkbox must follow its label"
                );
            }
            let poison = ctx
                .data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                        "condition-icon",
                        "poison",
                    )))
                })
                .ok_or("poison icon")?
                .0;
            let condition_frame = ctx
                .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("hud-condition-frame")))
                .ok_or("condition frame")?;
            assert!(
                (126.0..=133.0).contains(&condition_frame.height()),
                "condition header and viewport must retain the captured compact height: {condition_frame:?}"
            );
            let condition_row = |id: &'static str| {
                ctx.data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                        "hud-condition-row",
                        id,
                    )))
                })
            };
            let (header, _) = condition_row("header").ok_or("condition header geometry")?;
            let (poison_row, condition_clip) =
                condition_row("poison").ok_or("poison row geometry")?;
            let (burning_row, _) = condition_row("burning").ok_or("burning row geometry")?;
            assert!((16.0..=17.0).contains(&header.height()), "{header:?}");
            assert!(
                (16.0..=17.0).contains(&poison_row.height()),
                "{poison_row:?}"
            );
            assert!(
                (16.0..=17.0).contains(&(burning_row.top() - poison_row.top())),
                "condition rows must use the captured compact pitch"
            );
            assert!(
                (99.0..=101.0).contains(&condition_clip.height()),
                "condition list must expose the captured six-row viewport: {condition_clip:?}"
            );
            let up = ctx
                .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("condition-order", "↑"))))
                .ok_or("up order control")?;
            assert_eq!(up.size(), egui::vec2(20.0, 20.0));
            assert!(
                (poison.center().y - up.center().y).abs() <= 3.0,
                "first order control must align with the first condition row"
            );
            assert!(
                !draft
                    .future_preferences
                    .contains_key("hud.harmony_position")
            );
            let (mana, clip) = radio("mana").ok_or("mana radio")?;
            assert!(clip.contains_rect(mana));
            click(&ctx, size, mana.center(), |ctx| draw(ctx, &mut draft));
            assert_eq!(
                draft.future_preferences.get("hud.harmony_position"),
                Some(&FutureValue::Choice("mana".into()))
            );
            draft
                .future_preferences
                .insert("hud.show_harmony".into(), FutureValue::Bool(false));
            let _ = frame(&ctx, size, vec![], |ctx| draw(ctx, &mut draft));
            let (health, _) = radio("health").ok_or("health radio")?;
            click(&ctx, size, health.center(), |ctx| draw(ctx, &mut draft));
            assert_eq!(
                draft.future_preferences.get("hud.harmony_position"),
                Some(&FutureValue::Choice("mana".into())),
                "disabled radio must retain the saved choice"
            );
            let (powerless, clip) = ctx
                .data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                        "condition-icon",
                        "powerless",
                    )))
                })
                .ok_or("powerless icon")?;
            assert!(
                clip.contains_rect(powerless),
                "sixth condition row must remain inside the compact viewport: {poison:?} {powerless:?} {clip:?}"
            );
            assert_eq!(
                ctx.data(|data| {
                    data.get_temp::<bool>(egui::Id::new("hud-condition-selection-enabled"))
                }),
                Some(false)
            );
            click(&ctx, size, powerless.center(), |ctx| draw(ctx, &mut draft));
            assert_eq!(
                ctx.data(|data| {
                    data.get_temp::<String>(egui::Id::new("selected-hud-condition"))
                })
                .as_deref(),
                Some("poison"),
                "unavailable condition rows must not advertise a selectable no-op"
            );
            for (id, _) in CONDITIONS {
                let (rect, _) = ctx
                    .data(|data| {
                        data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                            "condition-icon",
                            id,
                        )))
                    })
                    .ok_or("condition icon")?;
                assert_eq!(rect.size(), egui::vec2(16.0, 16.0), "{id}");
            }
        }
        Ok(())
    }

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
    fn arc_children_depend_only_on_show_arcs() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(900.0, 700.0);
        let mut draft = ClientSettings::default();
        draft
            .future_preferences
            .insert("hud.conditions_hud_enabled".into(), FutureValue::Bool(true));
        draft
            .future_preferences
            .insert("hud.hud.arcs".into(), FutureValue::Bool(false));
        let draw = |ctx: &egui::Context, draft: &mut ClientSettings| {
            egui::Window::new("HUD arc dependency")
                .fixed_size([850.0, 650.0])
                .show(ctx, |ui| {
                    show(ui, draft, "hud", true, &mut PageState::default());
                });
        };
        for _ in 0..3 {
            let _ = frame(&ctx, size, vec![], |ctx| draw(ctx, &mut draft));
        }
        let choice_enabled = || {
            ctx.data(|data| {
                data.get_temp::<(bool, String, Vec<String>)>(egui::Id::new((
                    "preference-choice",
                    "hud.arc_size_preset",
                )))
                .map(|state| state.0)
            })
        };
        let percent_enabled = |key: &'static str| {
            ctx.data(|data| {
                data.get_temp::<(egui::Rect, egui::Rect, bool, bool)>(egui::Id::new((
                    "preference-percent",
                    key,
                )))
                .map(|state| state.3)
            })
        };
        assert_eq!(choice_enabled(), Some(false));
        assert_eq!(percent_enabled("hud.hud.arc_distance"), Some(false));
        assert_eq!(percent_enabled("hud.hud.arc_opacity"), Some(false));

        draft
            .future_preferences
            .insert("hud.hud.arcs".into(), FutureValue::Bool(true));
        for _ in 0..2 {
            let _ = frame(&ctx, size, vec![], |ctx| draw(ctx, &mut draft));
        }
        assert_eq!(choice_enabled(), Some(true));
        assert_eq!(percent_enabled("hud.hud.arc_distance"), Some(true));
        assert_eq!(percent_enabled("hud.hud.arc_opacity"), Some(true));
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

    #[test]
    fn nested_condition_scroll_reaches_hungry_without_moving_the_fixed_footer()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);
        crate::client_chrome::install(&ctx, false);
        let size = egui::vec2(900.0, 620.0);
        let mut browser = PreferencesBrowser::new();
        assert!(browser.open_section("hud"));
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let draw =
            |ctx: &egui::Context, browser: &mut PreferencesBrowser, draft: &mut ClientSettings| {
                browser.show(ctx, draft, None)
            };
        let mut output = egui::FullOutput::default();
        for _ in 0..30 {
            output = frame(&ctx, size, vec![], |ctx| {
                draw(ctx, &mut browser, &mut draft)
            })
            .1;
        }
        for footer in ["Reset", "Cancel", "Apply", "Ok"] {
            assert!(
                button_position(&output, size, footer).is_some(),
                "fixed footer control is not reachable: {footer}"
            );
        }
        assert!(button_position(&output, size, "Hungry").is_none());
        let (body, _) = scroll_geometry(&ctx, "all-preferences-body-geometry")
            .ok_or("settings body geometry")?;
        let master_frame = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("hud-master-frame")))
            .ok_or("HUD master frame")?;
        let arcs_frame = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("hud-arcs-frame")))
            .ok_or("HUD arcs frame")?;
        let conditions_frame = ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new("hud-condition-frame")))
            .ok_or("HUD conditions frame")?;
        let condition_clip = ctx
            .data(|data| {
                data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                    "condition-icon",
                    "poison",
                )))
            })
            .ok_or("condition viewport")?
            .1;
        assert!(
            (99.0..=101.0).contains(&condition_clip.height()),
            "condition viewport is not the captured compact height: {condition_clip:?}"
        );
        for (key, label) in [
            ("hud.custom_status_bars", "Show Customisable Status Bars"),
            ("hud.status_bars", "Show Status Bars"),
        ] {
            let id = key.strip_prefix("hud.").ok_or("status key")?;
            let (rect, clip) = ctx
                .data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                        "preference-toggle",
                        key,
                    )))
                })
                .ok_or("status toggle")?;
            let row_rect = ctx
                .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("hud-status-row", id))))
                .ok_or("status row")?;
            assert!(
                clip.contains_rect(row_rect) && body.contains_rect(row_rect),
                "status row must be visible without scrolling: {key} row={row_rect:?} clip={clip:?} master={master_frame:?} arcs={arcs_frame:?} conditions={conditions_frame:?}"
            );
            assert!(row_rect.contains_rect(rect));
            assert!(
                button_position(&output, size, label).is_some(),
                "initial status label is clipped: {label}"
            );
        }

        let hungry_visible = |ctx: &egui::Context| {
            ctx.data(|data| {
                data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                    "condition-icon",
                    "hungry",
                )))
                .is_some_and(|(rect, clip)| clip.contains_rect(rect))
            })
        };
        assert!(!hungry_visible(&ctx));
        let pointer = condition_clip.center();
        for _ in 0..12 {
            output = frame(
                &ctx,
                size,
                vec![
                    egui::Event::PointerMoved(pointer),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0.0, -220.0),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                |ctx| draw(ctx, &mut browser, &mut draft),
            )
            .1;
            for _ in 0..6 {
                output = frame(&ctx, size, vec![], |ctx| {
                    draw(ctx, &mut browser, &mut draft)
                })
                .1;
            }
            if hungry_visible(&ctx) && button_position(&output, size, "Hungry").is_some() {
                break;
            }
        }
        assert!(hungry_visible(&ctx), "Hungry remained clipped");
        assert!(button_position(&output, size, "Hungry").is_some());
        for key in ["hud.custom_status_bars", "hud.status_bars"] {
            let (rect, clip) = ctx
                .data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                        "preference-toggle",
                        key,
                    )))
                })
                .ok_or("status toggle after nested scroll")?;
            assert!(
                clip.contains_rect(rect),
                "inner condition scroll moved the fixed status row: {key}"
            );
        }
        for footer in ["Reset", "Cancel", "Apply", "Ok"] {
            assert!(
                button_position(&output, size, footer).is_some(),
                "fixed footer control moved or clipped: {footer}"
            );
        }
        Ok(())
    }

    #[test]
    fn automatic_screenshot_master_gates_all_seventeen_retained_triggers() {
        assert_eq!(
            SCREENSHOT_EVENTS_LEFT.len() + SCREENSHOT_EVENTS_RIGHT.len(),
            17
        );
        let ctx = egui::Context::default();
        crate::client_chrome::install(&ctx, false);
        let size = egui::vec2(760.0, 720.0);
        let mut draft = ClientSettings::default();
        draft
            .future_preferences
            .insert("screenshots.auto".into(), FutureValue::Bool(false));
        let retained: Vec<_> = SCREENSHOT_EVENTS_LEFT
            .iter()
            .chain(SCREENSHOT_EVENTS_RIGHT)
            .enumerate()
            .map(|(index, (id, _))| {
                let key = format!("screenshots.{id}");
                let value = FutureValue::Bool(index % 2 == 0);
                draft.future_preferences.insert(key.clone(), value.clone());
                (key, value)
            })
            .collect();
        let mut state = PageState::default();
        let draw = |ctx: &egui::Context, draft: &mut ClientSettings, state: &mut PageState| {
            egui::Window::new("screenshots-test")
                .fixed_size([700.0, 660.0])
                .show(ctx, |ui| screenshots(ui, draft, true, state));
        };
        let mut output = egui::FullOutput::default();
        for _ in 0..2 {
            output = frame(&ctx, size, vec![], |ctx| draw(ctx, &mut draft, &mut state)).1;
        }
        for label in [
            "Keep Backlog of the Screenshots of the Last 5 Seconds",
            "Select all events that should trigger auto screenshots:",
        ] {
            assert!(button_position(&output, size, label).is_some(), "{label}");
        }
        for stale in ["Keep Backlog of Last 5 Seconds", "Capture Events"] {
            assert!(button_position(&output, size, stale).is_none(), "{stale}");
        }
        assert_eq!(
            ctx.data(|data| {
                data.get_temp::<bool>(egui::Id::new((
                    "retained-child-gate",
                    "screenshot-automatic-events",
                )))
            }),
            Some(false)
        );
        for (key, value) in &retained {
            assert_eq!(draft.future_preferences.get(key), Some(value), "{key}");
        }

        draft
            .future_preferences
            .insert("screenshots.auto".into(), FutureValue::Bool(true));
        let _ = frame(&ctx, size, vec![], |ctx| draw(ctx, &mut draft, &mut state));
        assert_eq!(
            ctx.data(|data| {
                data.get_temp::<bool>(egui::Id::new((
                    "retained-child-gate",
                    "screenshot-automatic-events",
                )))
            }),
            Some(true)
        );
        for (key, value) in &retained {
            assert_eq!(draft.future_preferences.get(key), Some(value), "{key}");
        }
    }
}
