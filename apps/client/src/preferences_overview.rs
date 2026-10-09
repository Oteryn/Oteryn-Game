//! Readable Oteryn settings overview. Edits only the existing settings draft.
use super::PreferenceAction;
use egui::{Color32, RichText};
use oteryn_client::{settings::ClientSettings, settings_catalog::FutureValue};

pub(super) fn show(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    ui.spacing_mut().item_spacing = egui::vec2(12.0, 5.0);
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Body, egui::FontId::proportional(14.0));
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
    ui.add_space(8.0);
    ui.label(heading(
        ui,
        if en {
            "Quick settings"
        } else {
            "Szybkie ustawienia"
        },
        28.0,
    ));
    ui.label(if en {
        "Choose the information visible during play."
    } else {
        "Wybierz informacje widoczne podczas rozgrywki."
    });
    ui.add_space(8.0);
    let width = ui.available_width();
    if width >= 570.0 {
        let left = (width - 12.0) * 0.60;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(left);
                ui.set_max_width(left);
                controls(ui, draft, en);
                readability(ui, draft, en);
            });
            ui.vertical(|ui| {
                ui.set_width(width - left - 12.0);
                ui.set_max_width(width - left - 12.0);
                preview(ui, draft, en);
            });
        });
    } else {
        controls(ui, draft, en);
        readability(ui, draft, en);
        preview(ui, draft, en);
    }
}

fn controls(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    card(
        ui,
        if en {
            "Your character"
        } else {
            "Twoja postać"
        },
        |ui| {
            ui.weak(if en {
                "Health, mana and resource indicators."
            } else {
                "Zdrowie, mana i oznaczenia zasobów."
            });
            toggle(
                ui,
                draft,
                "hud.hud.player_enabled",
                if en { "Character HUD" } else { "HUD postaci" },
                en,
            );
            let active = selected(draft, "hud.hud.player_enabled");
            ui.add_enabled_ui(active, |ui| {
                toggle(
                    ui,
                    draft,
                    "hud.hud.resource_bars",
                    if en {
                        "Resource bars"
                    } else {
                        "Paski zasobów"
                    },
                    en,
                );
                toggle(
                    ui,
                    draft,
                    "hud.hud.owner_health",
                    if en { "Health" } else { "Zdrowie" },
                    en,
                );
                toggle(ui, draft, "hud.hud.owner_mana", "Mana", en);
                toggle(
                    ui,
                    draft,
                    "hud.hud.arcs",
                    if en {
                        "Resource arcs"
                    } else {
                        "Łuki zasobów"
                    },
                    en,
                );
            });
        },
    );
}

fn readability(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    card(ui, if en { "Readability" } else { "Czytelność" }, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(if en {
                "Interface size"
            } else {
                "Rozmiar interfejsu"
            });
            for (scale, label) in [(1.0, "100%"), (1.25, "125%"), (1.5, "150%")] {
                ui.selectable_value(&mut draft.ui_scale, scale, label);
            }
        });
        ui.checkbox(
            &mut draft.high_contrast,
            if en {
                "High contrast"
            } else {
                "Wysoki kontrast"
            },
        );
    });
}

fn preview(ui: &mut egui::Ui, draft: &ClientSettings, en: bool) {
    card(
        ui,
        if en {
            "Indicator preview"
        } else {
            "Podgląd oznaczeń"
        },
        |ui| {
            ui.weak(if en {
                "Illustrative proportions, not live character values."
            } else {
                "Przykładowe proporcje, nie bieżące wartości postaci."
            });
            let (rect, _) = ui
                .allocate_exact_size(egui::vec2(ui.available_width(), 66.0), egui::Sense::hover());
            if selected(draft, "hud.hud.player_enabled") {
                crate::actor_hud::preview(ui, rect, draft);
            } else {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    if en {
                        "Character HUD is off"
                    } else {
                        "HUD postaci jest wyłączony"
                    },
                    egui::FontId::proportional(12.0),
                    ui.visuals().text_color(),
                );
            }
        },
    );
}

fn heading(ui: &egui::Ui, text: &str, size: f32) -> RichText {
    let family = egui::FontFamily::Name("brand".into());
    if ui.fonts(|fonts| fonts.families().contains(&family)) {
        RichText::new(text).font(egui::FontId::new(size, family))
    } else {
        RichText::new(text).size(size)
    }
}

fn selected(draft: &ClientSettings, key: &str) -> bool {
    matches!(
        draft.future_preferences.get(key),
        Some(FutureValue::Bool(true))
    )
}

fn card(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().window_stroke)
        .corner_radius(5.0)
        .inner_margin(14.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(heading(ui, title, 20.0).color(ui.visuals().text_color()));
            ui.add_space(4.0);
            body(ui);
        });
}

fn toggle(ui: &mut egui::Ui, draft: &mut ClientSettings, key: &str, label: &str, en: bool) {
    let stored = draft.future_preferences.get(key);
    let value = selected(draft, key);
    let text = if value {
        if en { "On" } else { "Włączone" }
    } else if en {
        "Off"
    } else {
        "Wyłączone"
    };
    let unset = stored.is_none();
    ui.push_id(key, |ui| {
        ui.horizontal(|ui| {
            ui.set_min_height(26.0);
            ui.label(label);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let response = ui.add_sized(
                    [44.0, 24.0],
                    egui::Button::new("").corner_radius(12.0).selected(value),
                );
                let center = egui::pos2(
                    if value {
                        response.rect.right() - 12.0
                    } else {
                        response.rect.left() + 12.0
                    },
                    response.rect.center().y,
                );
                ui.painter()
                    .circle_filled(center, 8.0, ui.visuals().text_color());
                ui.label(text);
                if unset {
                    response.clone().on_hover_text(if en {
                        "Currently off. Your preference is saved only when changed."
                    } else {
                        "Obecnie wyłączone. Własny wybór zapiszesz po zmianie."
                    });
                }
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Checkbox,
                        ui.is_enabled(),
                        value,
                        label,
                    )
                });
                if response.clicked() {
                    draft
                        .future_preferences
                        .insert(key.into(), FutureValue::Bool(!value));
                    if key == "hud.hud.arcs" && !value {
                        draft
                            .future_preferences
                            .entry("hud.arc_size_preset".into())
                            .or_insert_with(|| FutureValue::Choice("default".into()));
                    }
                }
            });
        });
    });
    ui.separator();
}

pub(super) fn footer(ui: &mut egui::Ui, en: bool) -> (PreferenceAction, egui::Rect) {
    let available = ui.available_rect_before_wrap();
    let mut action = PreferenceAction::None;
    let footer = egui::Panel::bottom("oteryn-overview-footer")
        .frame(egui::Frame::NONE)
        .exact_size(50.0)
        .show(ui, |ui| {
            ui.spacing_mut().button_padding = egui::vec2(10.0, 6.0);
            ui.style_mut()
                .text_styles
                .insert(egui::TextStyle::Button, egui::FontId::proportional(14.0));
            ui.separator();
            ui.horizontal(|ui| {
                if ui
                    .button(if en { "Defaults" } else { "Domyślne" })
                    .clicked()
                {
                    action = PreferenceAction::Defaults;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(
                            egui::Button::new(if en { "Save changes" } else { "Zapisz zmiany" })
                                .fill(Color32::from_rgb(116, 78, 38)),
                        )
                        .clicked()
                    {
                        action = PreferenceAction::Ok;
                    }
                    if ui.button(if en { "Cancel" } else { "Anuluj" }).clicked() {
                        action = PreferenceAction::Cancel;
                    }
                });
            });
        });
    (
        action,
        egui::Rect::from_min_max(
            available.min,
            egui::pos2(
                available.right(),
                footer.response.rect.top().max(available.top()),
            ),
        ),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences_browser::{
        PreferencesBrowser,
        test_support::{button_position, click, frame},
    };

    #[test]
    fn overview_does_not_persist_defaults_and_save_cancel_survive_resize()
    -> Result<(), &'static str> {
        for en in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_theme(egui::Theme::Dark);
            let mut browser = PreferencesBrowser::new();
            let mut draft = ClientSettings {
                english: en,
                ..Default::default()
            };
            let original = draft.future_preferences.clone();
            for size in [egui::vec2(900.0, 620.0), egui::vec2(500.0, 360.0)] {
                let mut output = egui::FullOutput::default();
                for _ in 0..5 {
                    output = frame(&ctx, size, Vec::new(), |ctx| {
                        browser.show(ctx, &mut draft, None)
                    })
                    .1;
                }
                assert_eq!(draft.future_preferences, original);
                let save = button_position(
                    &output,
                    size,
                    if en { "Save changes" } else { "Zapisz zmiany" },
                )
                .ok_or("Save is clipped")?;
                let action = click(&ctx, size, save, |ctx| browser.show(ctx, &mut draft, None));
                assert!(matches!(action, PreferenceAction::Ok));
                let output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
                let cancel = button_position(&output, size, if en { "Cancel" } else { "Anuluj" })
                    .ok_or("Cancel is clipped")?;
                assert!(matches!(
                    click(&ctx, size, cancel, |ctx| browser
                        .show(ctx, &mut draft, None)),
                    PreferenceAction::Cancel
                ));
            }
        }
        Ok(())
    }
}
