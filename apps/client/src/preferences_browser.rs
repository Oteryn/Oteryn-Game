//! Complete preference navigation with explicit future-consumer configuration.
#[path = "preferences_pages.rs"]
mod pages;

use oteryn_client::{
    settings::ClientSettings,
    settings_catalog::{
        FutureValue, Implementation, OptionKind, SETTINGS_SECTIONS, SettingOption, ShortcutBinding,
    },
};

#[derive(Default)]
pub struct PreferencesBrowser {
    pub open: bool,
    section: usize,
    search: String,
    page_state: pages::PageState,
    action_bar_available: bool,
    clear_action_row: Option<usize>,
    expanded: std::collections::BTreeSet<&'static str>,
    navigation_section: Option<usize>,
}

#[derive(Default)]
pub enum PreferenceAction {
    #[default]
    None,
    Ok,
    Apply,
    Cancel,
    Defaults,
    QuickSettings,
}

impl PreferencesBrowser {
    pub fn set_action_bar_available(&mut self, available: bool) {
        self.action_bar_available = available;
        if !available {
            self.clear_action_row = None;
        }
    }
    pub fn take_clear_action_row(&mut self) -> Option<usize> {
        self.clear_action_row.take()
    }
    pub fn new() -> Self {
        Self {
            open: true,
            ..Self::default()
        }
    }
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        draft: &mut ClientSettings,
        message: Option<&str>,
    ) -> PreferenceAction {
        if !self.open {
            return PreferenceAction::None;
        }
        let previous_style = ctx.style_of(egui::Theme::Dark);
        crate::client_chrome::install(ctx, draft.high_contrast);
        let mut action = PreferenceAction::None;
        let english = draft.english;
        let tr = |pl, en| if english { en } else { pl };
        let advanced = matches!(
            draft.future_preferences.get("basic.basic.advanced"),
            Some(FutureValue::Bool(true))
        );
        if !advanced && !category_visible(SETTINGS_SECTIONS[self.section].id, false) {
            self.section = 0;
        }
        if self.navigation_section != Some(self.section) {
            let id = SETTINGS_SECTIONS[self.section].id;
            if let Some(parent) = navigation_parent(id) {
                self.expanded.insert(parent);
            } else if matches!(
                id,
                "controls" | "interface" | "graphics" | "sound" | "miscellaneous"
            ) {
                self.expanded.insert(id);
            }
            self.navigation_section = Some(self.section);
        }
        let bounds = ctx.content_rect().shrink(4.0);
        let maximum = bounds.size().max(egui::Vec2::splat(1.0));
        egui::Window::new(tr("Opcje", "Options"))
            .id("complete-preferences".into()).open(&mut self.open)
            .collapsible(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .constrain_to(bounds).min_size(egui::Vec2::ZERO).max_size(maximum)
            .default_size([560.0_f32.min(maximum.x), 430.0_f32.min(maximum.y)])
            .show(ctx, |ui| {
                crate::client_chrome::surface(ui, ui.max_rect(), false);
                ui.style_mut().spacing.item_spacing = egui::vec2(3.0, 2.0);
                ui.style_mut().spacing.button_padding = egui::vec2(4.0, 2.0);
                ui.style_mut().spacing.scroll.floating = false;
                let mut advanced_setting = advanced;
                let (footer_action, body_rect) = primary_footer_impl(ui, "all-preferences-footer", english, Some(&mut advanced_setting));
                if advanced_setting != advanced { draft.future_preferences.insert("basic.basic.advanced".into(), FutureValue::Bool(advanced_setting)); }
                action = footer_action;
                let body_size = body_rect.size().max(egui::Vec2::splat(1.0));
                let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
                body_ui.set_clip_rect(ui.clip_rect().intersect(body_rect));
                let _body_output = egui::ScrollArea::both().id_salt(("all-preferences-body", SETTINGS_SECTIONS[self.section].id))
                    .auto_shrink([false, false]).max_width(body_size.x).max_height(body_size.y).show(&mut body_ui, |ui| {
                let body_width = ui.available_width().max(1.0);
                ui.set_width(body_width);
                let compact = body_width < 520.0;
                let basic = SETTINGS_SECTIONS[self.section].id == "basic";
                if !basic { ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(body_width).char_limit(120).hint_text(tr("Szukaj ustawienia…", "Search preferences…"))); ui.separator(); }
                let sections_width = 105.0;
                let search = if basic { String::new() } else { self.search.trim().to_lowercase() };
                if compact {
                    egui::ComboBox::from_id_salt("all-preferences-category").width(body_width).truncate()
                        .selected_text(SETTINGS_SECTIONS[self.section].text(english)).show_ui(ui, |ui| {
                            for (index, section) in SETTINGS_SECTIONS.iter().enumerate() {
                                if category_visible(section.id, advanced) && (search.is_empty() || section.text(english).to_lowercase().contains(&search) || section.options.iter().any(|option| option.text(english).to_lowercase().contains(&search))) {
                                    ui.selectable_value(&mut self.section, index, section.text(english));
                                }
                            }
                        });
                }
                // Keep the columns inside the current dialog body, reserving room for
                // availability text and secondary controls below them.
                let column_height = (ui.available_height() - if basic && !compact { 30.0 } else if basic { 50.0 } else { 45.0 }).max(1.0);
                ui.horizontal_top(|ui| {
                    if !compact {
                        ui.set_max_height(column_height);
                    }
                    if !compact {
                    egui::ScrollArea::vertical().id_salt("all-preferences-sections").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded).max_height(column_height).show(ui, |ui| {
                        ui.vertical(|ui| {
                        ui.set_width(sections_width);
                        for (index, section) in SETTINGS_SECTIONS.iter().enumerate() {
                            let parent = navigation_parent(section.id);
                            if advanced && search.is_empty() && parent.is_some_and(|parent| !self.expanded.contains(parent)) { continue; }
                            if category_visible(section.id, advanced) && (search.is_empty() || section.text(english).to_lowercase().contains(&search) || section.options.iter().any(|option| option.text(english).to_lowercase().contains(&search))) {
                                if section.id == "panels" { ui.separator(); ui.small("OTERYN"); }
                                let label = if !advanced && section.id == "basic" { tr("Opcje", "Options") } else if !advanced && section.id == "general_hotkeys" { tr("Skróty", "Hotkeys") } else { section.text(english) };
                                ui.horizontal(|ui| {
                                    if advanced && matches!(section.id, "controls" | "interface" | "graphics" | "sound" | "miscellaneous") {
                                        if navigation_arrow(ui, self.expanded.contains(section.id), english).clicked() && !self.expanded.remove(section.id) { self.expanded.insert(section.id); }
                                    } else if advanced && parent.is_some() { ui.add_space(14.0); }
                                    ui.selectable_value(&mut self.section, index, label);
                                });
                            }
                        }
                        });
                    });
                    ui.separator();
                    }
                    let options_width = ui.available_width().max(1.0);
                    egui::ScrollArea::vertical().id_salt(("all-preferences-options", SETTINGS_SECTIONS[self.section].id)).max_width(options_width).max_height(if compact { f32::INFINITY } else { column_height }).show(ui, |ui| {
                        ui.vertical(|ui| {
                        ui.set_width(ui.available_width().max(1.0));
                        let section = &SETTINGS_SECTIONS[self.section];
                        if section.id == "basic" { basic_page(ui, draft, english); }
                        else if section.id == "action_bars" { action_bars_page(ui, draft, english, self.action_bar_available, &mut self.clear_action_row); }
                        else if search.is_empty() && pages::show(ui, draft, section.id, english, &mut self.page_state) {}
                        else {
                        ui.heading(section.text(english));
                        let mut matching = 0;
                        for option in section.options {
                            if !search.is_empty() && !section.text(english).to_lowercase().contains(&search) && !option.text(english).to_lowercase().contains(&search) { continue; }
                            matching += 1;
                            // Former family-level scalar bindings had no slot identity.
                            // Their saved intent stays intact; real rows use the typed editor.
                            if section.id == "action_hotkeys" { continue; }
                            match option.implementation {
                                Implementation::Implemented{field} => implemented(ui, draft, option, field, english),
                                Implementation::Pending{..} => {
                                    let key = format!("{}.{}", section.id, option.id);
                                    ui.push_id(&key, |ui| {
                                        ui.label(option.text(english)).on_hover_text(tr("Możesz zapisać swój wybór. Działanie tej funkcji nie jest jeszcze dostępne.", "Your selection can be saved. This feature is not available yet."));
                                        pending(ui, draft, option, key.clone(), english);
                                    });
                                    ui.separator();
                                }
                            }
                        }
                        if matching == 0 { ui.weak(tr("Brak wyników w tej kategorii. Wybierz kategorię z listy.", "No matches in this category. Choose a category from the list.")); }
                        if section.id == "action_hotkeys" && (matching > 0 || search.is_empty()) {
                            ui.separator();
                            ui.heading(tr("Dziewięć pasków i ich skróty", "Nine action rows and their shortcuts"));
                            crate::action_bar_ui::preferences_editor(ui, &mut draft.action_bar, english);
                        }
                        }
                        });
                    });
                });
                ui.separator();
                if !basic && SETTINGS_SECTIONS[self.section].id != "shortcuts" { ui.small(tr("Opcje bez obsługi zapisują wybór do przyszłego użycia; nie zmieniają jeszcze gry.", "Preferences awaiting support save your choice for future use; they do not change gameplay yet.")); }
                if let Some(message) = message { ui.label(message); }
                let mut show_advanced = advanced;
                if compact && ui.checkbox(&mut show_advanced, tr("Pokaż opcje zaawansowane", "Show advanced options")).changed() { draft.future_preferences.insert("basic.basic.advanced".into(), FutureValue::Bool(show_advanced)); }
                if basic && !compact { ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Reset").clicked() { action = PreferenceAction::Defaults; }
                    if ui.small_button(tr("OTERYN: połączenie i konto", "OTERYN: connection and account")).clicked() { action = PreferenceAction::QuickSettings; }
                }); } else { ui.horizontal_wrapped(|ui| {
                    if ui.button(tr("Domyślne", "Defaults")).clicked() { action = PreferenceAction::Defaults; }
                    if ui.button(tr("Połączenie i konto", "Connection and account")).clicked() { action = PreferenceAction::QuickSettings; }
                }); }
                });
                #[cfg(test)]
                test_support::record_scroll_geometry(ctx, "all-preferences-body-geometry", &_body_output);
            });
        ctx.set_style_of(egui::Theme::Dark, previous_style);
        action
    }
}

fn navigation_arrow(ui: &mut egui::Ui, expanded: bool, english: bool) -> egui::Response {
    let label = match (expanded, english) {
        (true, true) => "Collapse category",
        (false, true) => "Expand category",
        (true, false) => "Zwiń kategorię",
        (false, false) => "Rozwiń kategorię",
    };
    let response = ui
        .add_sized([12.0, 16.0], egui::Button::new("").frame(false))
        .on_hover_text(label);
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    // Replace the label with an independently drawn arrow: the bundled font does
    // not provide these triangle glyphs on every native platform.
    let centre = response.rect.center();
    let offsets = if expanded {
        [
            egui::vec2(-3.0, -2.0),
            egui::vec2(3.0, -2.0),
            egui::vec2(0.0, 3.0),
        ]
    } else {
        [
            egui::vec2(-2.0, -3.0),
            egui::vec2(-2.0, 3.0),
            egui::vec2(3.0, 0.0),
        ]
    };
    ui.painter().add(egui::epaint::PathShape::convex_polygon(
        offsets.map(|offset| centre + offset).to_vec(),
        ui.visuals().text_color(),
        egui::Stroke::NONE,
    ));
    response
}

fn navigation_parent(id: &str) -> Option<&'static str> {
    match id {
        "general_hotkeys" | "action_hotkeys" | "custom_hotkeys" => Some("controls"),
        "hud" | "console" | "game_window" | "action_bars" | "shortcuts" => Some("interface"),
        "effects" => Some("graphics"),
        "battle_sounds" | "ui_sounds" => Some("sound"),
        "gameplay" | "screenshots" | "help" => Some("miscellaneous"),
        _ => None,
    }
}

fn category_visible(id: &str, advanced: bool) -> bool {
    advanced || matches!(id, "basic" | "general_hotkeys" | "shortcuts" | "help")
}

fn boxed(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::group(ui.style())
        .inner_margin(2.0)
        .show(ui, |ui| {
            crate::client_chrome::surface(ui, ui.max_rect(), true);
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(title).strong());
                });
            }
            body(ui);
        });
    ui.add_space(1.0);
}

fn future_toggle(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    key: &str,
    label: &str,
    english: bool,
) {
    let stored = draft.future_preferences.get(key);
    let mut value = matches!(stored, Some(FutureValue::Bool(true)));
    if ui
        .add(egui::Checkbox::new(&mut value, label).indeterminate(stored.is_none()))
        .on_hover_text(if english {
            "Saved selection; gameplay support is pending."
        } else {
            "Zapisany wybór; działanie w grze oczekuje na obsługę."
        })
        .changed()
    {
        draft
            .future_preferences
            .insert(key.into(), FutureValue::Bool(value));
    }
}

fn reference_option(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    section: &str,
    id: &str,
    labels: [&str; 2],
    english: bool,
) {
    let label = labels[usize::from(english)];
    let Some(option) = SETTINGS_SECTIONS
        .iter()
        .find(|entry| entry.id == section)
        .and_then(|section| section.options.iter().find(|option| option.id == id))
    else {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add_enabled(
                false,
                egui::Button::new(if english {
                    "Not available"
                } else {
                    "Niedostępne"
                }),
            );
        });
        return;
    };
    let key = format!("{section}.{id}");
    ui.push_id(&key, |ui| {
        if matches!(option.implementation, Implementation::Pending { .. })
            && option.kind == OptionKind::Toggle
        {
            future_toggle(ui, draft, &key, label, english);
        } else if let Implementation::Implemented {
            field: "movement_keys",
        } = option.implementation
        {
            ui.vertical(|ui| implemented(ui, draft, option, "movement_keys", english));
        } else {
            ui.horizontal(|ui| {
                if let Implementation::Implemented { field } = option.implementation {
                    if field == "fullscreen" {
                        ui.checkbox(&mut draft.fullscreen, label);
                    } else {
                        implemented(ui, draft, option, field, english);
                    }
                } else {
                    ui.label(label).on_hover_text(if english {
                        "Saved selection; gameplay support is pending."
                    } else {
                        "Zapisany wybór; działanie w grze oczekuje na obsługę."
                    });
                    pending(ui, draft, option, key.clone(), english);
                }
            });
        }
    });
}

fn edge_rows(ui: &mut egui::Ui, draft: &mut ClientSettings, english: bool) {
    let labels = if english {
        [
            "Show Bottom Action Bars:",
            "Show Left Action Bars:",
            "Show Right Action Bars:",
        ]
    } else {
        [
            "Pokaż dolne paski akcji:",
            "Pokaż lewe paski akcji:",
            "Pokaż prawe paski akcji:",
        ]
    };
    for (edge, label) in labels.into_iter().enumerate() {
        ui.push_id(edge, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(label);
                ui.checkbox(
                    &mut draft.action_bar.edge_enabled[edge],
                    if english { "All" } else { "Wszystkie" },
                );
                ui.add_enabled_ui(draft.action_bar.edge_enabled[edge], |ui| {
                    for index in 0..3 {
                        if let Some(mut row) = draft.action_bar.row(edge * 3 + index) {
                            let text =
                                format!("{} {}", if english { "Bar" } else { "Pasek" }, index + 1);
                            if ui.checkbox(&mut row.visible, text).changed() {
                                let _ = draft.action_bar.set_row(edge * 3 + index, row);
                            }
                        }
                    }
                });
            });
        });
    }
}

fn basic_page(ui: &mut egui::Ui, draft: &mut ClientSettings, english: bool) {
    boxed(ui, if english { "Gameplay" } else { "Rozgrywka" }, |ui| {
        for (section, id, labels) in [
            (
                "controls",
                "controls.mouse_preset",
                ["Schemat myszy:", "Mouse Preset:"],
            ),
            (
                "gameplay",
                "gameplay.inspect_permission",
                ["Pozwól wszystkim oglądać postać", "Allow All to Inspect Me"],
            ),
            (
                "gameplay",
                "gameplay.cancel_chase",
                ["Automatycznie przerywaj pościg", "Auto Chase Off"],
            ),
            (
                "gameplay",
                "gameplay.nearby_corpses",
                [
                    "Szybkie łupienie pobliskich ciał",
                    "Quick Loot Nearby Corpses",
                ],
            ),
        ] {
            reference_option(ui, draft, section, id, labels, english);
        }
    });
    boxed(ui, if english { "Interface" } else { "Interfejs" }, |ui| {
        ui.horizontal(|ui| {
            ui.label(if english { "HUD Style:" } else { "Styl HUD:" });
            future_toggle(
                ui,
                draft,
                "hud.hud.resource_bars",
                if english { "Show Bars" } else { "Pokaż paski" },
                english,
            );
            future_toggle(
                ui,
                draft,
                "hud.hud.arcs",
                if english { "Show Arcs" } else { "Pokaż łuki" },
                english,
            );
        });
        edge_rows(ui, draft, english);
        reference_option(
            ui,
            draft,
            "interface",
            "interface.colourise_loot_value",
            ["Kolorowanie wartości łupów:", "Colourise Loot Value:"],
            english,
        );
    });
    boxed(ui, if english { "Graphics" } else { "Grafika" }, |ui| {
        reference_option(
            ui,
            draft,
            "graphics",
            "graphics.antialiasing",
            ["Wygładzanie obrazu:", "Antialiasing Mode:"],
            english,
        );
        reference_option(
            ui,
            draft,
            "graphics",
            "graphics.fullscreen",
            ["Pełny ekran", "Fullscreen Mode"],
            english,
        );
    });
    boxed(ui, if english { "Sound" } else { "Dźwięk" }, |ui| {
        let _ = master_volume(ui, draft, english);
    });
}

fn master_volume(ui: &mut egui::Ui, draft: &mut ClientSettings, english: bool) -> egui::Response {
    let key = "sound.sound.master";
    let selected = match draft.future_preferences.get(key) {
        Some(FutureValue::Int(value)) => Some(*value),
        _ => None,
    };
    let mut value = selected.unwrap_or(0);
    let text = selected.map_or_else(|| "—".to_string(), |value| format!("{value}%"));
    let response = ui.horizontal(|ui| {
        ui.label(format!("{} {text}", if english { "Master Volume:" } else { "Głośność główna:" }));
        ui.spacing_mut().slider_width = ui.available_width().clamp(50.0, 180.0);
        ui.add(egui::Slider::new(&mut value, 0..=100).show_value(false)).on_hover_text(if english {
            "Choose a saved volume preference. — means unset. Audio playback is not yet supported."
        } else { "Wybierz zapisaną głośność. — oznacza brak wyboru. Odtwarzanie dźwięku oczekuje na obsługę." })
    }).inner;
    // Choosing zero explicitly also changes the selection from unknown to known.
    if response.changed() || (selected.is_none() && response.clicked()) {
        draft
            .future_preferences
            .insert(key.into(), FutureValue::Int(value));
    }
    response
}

fn action_bars_page(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    english: bool,
    available: bool,
    clear_row: &mut Option<usize>,
) {
    edge_rows(ui, draft, english);
    ui.separator();
    for (id, labels) in [
        (
            "bars.labels",
            [
                "Pokaż przypisany skrót",
                "Show Assigned Hotkey for Action Button",
            ],
        ),
        (
            "bars.item_amounts",
            [
                "Pokaż ilość przypisanych przedmiotów",
                "Show Amount of Assigned Objects",
            ],
        ),
        (
            "bars.spell_parameters",
            ["Pokaż parametry zaklęć", "Show Spell Parameters"],
        ),
        (
            "bars.graphic_cooldown",
            ["Pokaż graficzny czas odnowienia", "Show Graphical Cooldown"],
        ),
        (
            "bars.numeric_cooldown",
            [
                "Pokaż czas odnowienia w sekundach",
                "Show Cooldown in Seconds",
            ],
        ),
        (
            "bars.tooltips",
            ["Pokaż podpowiedź przycisku", "Show Action Button Tooltip"],
        ),
        (
            "bars.auto_spells",
            [
                "Automatycznie dodawaj nowe zaklęcia",
                "Auto-Insert New Spells",
            ],
        ),
    ] {
        reference_option(ui, draft, "action_bars", id, labels, english);
    }
    ui.separator();
    for (edge, label) in (if english {
        [
            "Clear Bottom Action Bars:",
            "Clear Left Action Bars:",
            "Clear Right Action Bars:",
        ]
    } else {
        [
            "Wyczyść dolne paski:",
            "Wyczyść lewe paski:",
            "Wyczyść prawe paski:",
        ]
    })
    .into_iter()
    .enumerate()
    {
        ui.horizontal(|ui| { ui.label(label); for index in 1..=3 {
            if ui.add_enabled(available, egui::Button::new(format!("{} {index}", if english { "Bar" } else { "Pasek" })))
                .on_hover_text(if available { if english { "Immediately clear this session’s assigned actions. Saved shortcuts and locks stay intact; Cancel does not restore assignments." } else { "Natychmiast usuń czynności tej sesji. Skróty i blokady pozostają; Anuluj nie przywraca czynności." } } else if english { "Available after entering the game." } else { "Dostępne po wejściu do gry." }).clicked() { *clear_row = Some(edge * 3 + index - 1); }
        } });
    }
}

/// Reserve recovery controls before allocating scrollable content, including long errors.
pub(crate) fn primary_footer(
    ui: &mut egui::Ui,
    id: &str,
    english: bool,
) -> (PreferenceAction, egui::Rect) {
    primary_footer_impl(ui, id, english, None)
}

fn primary_footer_impl(
    ui: &mut egui::Ui,
    id: &str,
    english: bool,
    advanced: Option<&mut bool>,
) -> (PreferenceAction, egui::Rect) {
    let mut action = PreferenceAction::None;
    let available = ui.available_rect_before_wrap();
    let height = ui.spacing().interact_size.y + 8.0;
    let footer = egui::Panel::bottom(egui::Id::new(id))
        .frame(egui::Frame::NONE)
        .exact_size(height)
        .show(ui, |ui| {
            ui.spacing_mut().button_padding = egui::vec2(6.0, 3.0);
            ui.horizontal(|ui| {
                let compact = ui.available_width() < 520.0;
                if !compact && let Some(advanced) = advanced {
                    ui.checkbox(
                        advanced,
                        if english {
                            "Show advanced options"
                        } else {
                            "Pokaż opcje zaawansowane"
                        },
                    );
                }
                if compact {
                    ui.spacing_mut().button_padding.x = 3.0;
                }
                let apply = if english { "Apply" } else { "Zastosuj" };
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(if english { "Cancel" } else { "Anuluj" })
                        .clicked()
                    {
                        action = PreferenceAction::Cancel;
                    }
                    if ui
                        .button(apply)
                        .on_hover_text(if english {
                            "Apply and save preferences"
                        } else {
                            "Zastosuj i zapisz ustawienia"
                        })
                        .clicked()
                    {
                        action = PreferenceAction::Apply;
                    }
                    if ui
                        .button("OK")
                        .on_hover_text(if english {
                            "Save preferences and close"
                        } else {
                            "Zapisz ustawienia i zamknij"
                        })
                        .clicked()
                    {
                        action = PreferenceAction::Ok;
                    }
                });
            });
        });
    let body = egui::Rect::from_min_max(
        available.min,
        egui::pos2(
            available.right(),
            footer.response.rect.top().max(available.top()),
        ),
    );
    (action, body)
}

fn implemented(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    option: &SettingOption,
    field: &str,
    english: bool,
) {
    let label = option.text(english);
    let toggle = match field {
        "click_to_walk" => Some(&mut draft.click_to_walk),
        "english" => Some(&mut draft.english),
        "high_contrast" => Some(&mut draft.high_contrast),
        "reduced_motion" => Some(&mut draft.reduced_motion),
        "show_chat" => Some(&mut draft.show_chat),
        "show_minimap" => Some(&mut draft.show_minimap),
        "fullscreen" => Some(&mut draft.fullscreen),
        "vsync" => Some(&mut draft.vsync),
        "show_inventory" => Some(&mut draft.show_inventory),
        "show_battle" => Some(&mut draft.show_battle),
        _ => None,
    };
    if let Some(value) = toggle {
        ui.checkbox(value, label);
        return;
    }
    match field {
        "ui_scale" => {
            ui.add(egui::Slider::new(&mut draft.ui_scale, 0.8..=1.8).text(label));
        }
        "window_width" => {
            ui.add(egui::Slider::new(&mut draft.window_width, 800..=3840).text(label));
        }
        "window_height" => {
            ui.add(egui::Slider::new(&mut draft.window_height, 600..=2160).text(label));
        }
        "fps" => {
            egui::ComboBox::from_id_salt(field)
                .selected_text(format!("{label}: {}", draft.fps))
                .show_ui(ui, |ui| {
                    for value in [0, 30, 60, 90, 120, 144, 165, 240, 360] {
                        ui.selectable_value(&mut draft.fps, value, value.to_string());
                    }
                });
        }
        "background_fps" => {
            ui.add(egui::Slider::new(&mut draft.background_fps, 5..=60).text(label));
        }
        "movement_keys" => {
            ui.label(label);
            ui.horizontal(|ui| {
                if ui
                    .button(if english { "Arrow keys" } else { "Strzałki" })
                    .clicked()
                {
                    draft.movement_keys = [82, 79, 81, 80];
                }
                if ui.button("WASD").clicked() {
                    draft.movement_keys = [26, 7, 22, 4];
                }
            });
            for (index, direction) in if english {
                ["North", "East", "South", "West"]
            } else {
                ["Północ", "Wschód", "Południe", "Zachód"]
            }
            .iter()
            .enumerate()
            {
                egui::ComboBox::from_id_salt(("complete-movement", index))
                    .selected_text(format!(
                        "{direction}: {}",
                        shortcut_label(draft.movement_keys[index])
                    ))
                    .show_ui(ui, |ui| {
                        for code in (4..=29).chain(79..=82) {
                            ui.selectable_value(
                                &mut draft.movement_keys[index],
                                code,
                                shortcut_label(code),
                            );
                        }
                    });
            }
        }
        "action_bottom_rows" | "action_left_rows" | "action_right_rows" => {
            let edge = match field {
                "action_bottom_rows" => 0,
                "action_left_rows" => 1,
                _ => 2,
            };
            let mut count = draft.action_bar.visible_rows()[edge];
            if ui
                .add(egui::Slider::new(&mut count, 0..=3).text(label))
                .changed()
                && draft.set_action_row_count(edge, count).is_err()
            {
                ui.label(if english {
                    "Could not change action rows."
                } else {
                    "Nie zmieniono pasków akcji."
                });
            }
        }
        _ => {
            ui.label(label);
        }
    }
}

fn pending(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    option: &SettingOption,
    key: String,
    english: bool,
) {
    let tr = |pl, en| if english { en } else { pl };
    let stored = draft.future_preferences.get(&key).cloned();
    let baseline = stored.clone();
    let mut next = stored.clone();
    match option.kind {
        OptionKind::Toggle => {
            let mut value = matches!(stored, Some(FutureValue::Bool(true)));
            if ui
                .add(
                    egui::Checkbox::new(&mut value, tr("Włączone", "Enabled"))
                        .indeterminate(stored.is_none()),
                )
                .changed()
            {
                next = Some(FutureValue::Bool(value));
            }
        }
        OptionKind::Integer { min, max } => {
            if let Some(FutureValue::Int(mut value)) = stored {
                if ui
                    .add(egui::DragValue::new(&mut value).range(min..=max))
                    .changed()
                {
                    next = Some(FutureValue::Int(value));
                }
            } else if ui.button(tr("Ustaw wartość", "Set value")).clicked() {
                next = Some(FutureValue::Int(min));
            }
        }
        OptionKind::Decimal { min, max } => {
            if let Some(FutureValue::Decimal(mut value)) = stored {
                if ui.add(egui::Slider::new(&mut value, min..=max)).changed() {
                    next = Some(FutureValue::Decimal(value));
                }
            } else if ui.button(tr("Ustaw wartość", "Set value")).clicked() {
                next = Some(FutureValue::Decimal(min));
            }
        }
        OptionKind::Choice(choices) => {
            let mut chosen = if let Some(FutureValue::Choice(value)) = stored {
                value
            } else {
                String::new()
            };
            let label = choices
                .iter()
                .find(|c| c.id == chosen)
                .map_or(tr("Nieustawione", "Unset"), |c| c.text(english));
            egui::ComboBox::from_id_salt(&key)
                .selected_text(label)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut chosen, String::new(), tr("Nieustawione", "Unset"));
                    for choice in choices {
                        ui.selectable_value(&mut chosen, choice.id.into(), choice.text(english));
                    }
                });
            next = (!chosen.is_empty()).then_some(FutureValue::Choice(chosen));
        }
        OptionKind::Text { max_bytes } => {
            let mut value = if let Some(FutureValue::Text(value)) = stored {
                value
            } else {
                String::new()
            };
            if ui
                .add(egui::TextEdit::singleline(&mut value).char_limit(max_bytes))
                .changed()
                && value.len() <= max_bytes
            {
                next = Some(FutureValue::Text(value));
            }
        }
        OptionKind::Binding => {
            if let Some(FutureValue::Binding(mut binding)) = stored {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut binding.ctrl, "Ctrl");
                    ui.checkbox(&mut binding.alt, "Alt");
                    ui.checkbox(&mut binding.shift, "Shift");
                    ui.checkbox(&mut binding.meta, "Super");
                });
                egui::ComboBox::from_id_salt(&key)
                    .selected_text(shortcut_label(binding.key))
                    .show_ui(ui, |ui| {
                        for code in (4..=39).chain(58..=69).chain(79..=82) {
                            ui.selectable_value(&mut binding.key, code, shortcut_label(code));
                        }
                    });
                next = Some(FutureValue::Binding(binding));
            } else if ui.button(tr("Przypisz klawisz", "Assign key")).clicked() {
                next = Some(FutureValue::Binding(ShortcutBinding {
                    key: 30,
                    ctrl: false,
                    alt: false,
                    shift: false,
                    meta: false,
                }));
            }
        }
        OptionKind::Action => {
            ui.add_enabled(false, egui::Button::new(tr("Niedostępne", "Unavailable")));
        }
    }
    if next.is_some()
        && ui
            .small_button(tr("Wyczyść własną wartość", "Clear custom value"))
            .clicked()
    {
        next = None;
    }
    if next != baseline {
        if let Some(value) = next {
            draft.future_preferences.insert(key, value);
        } else {
            draft.future_preferences.remove(&key);
        }
    }
}

fn shortcut_label(code: u16) -> String {
    match code {
        4..=29 => char::from_u32(u32::from(code) + 61)
            .unwrap_or('?')
            .to_string(),
        30..=38 => (code - 29).to_string(),
        39 => "0".into(),
        58..=69 => format!("F{}", code - 57),
        79 => "Right".into(),
        80 => "Left".into(),
        81 => "Down".into(),
        82 => "Up".into(),
        _ => "?".into(),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use egui::{Context, Event, FullOutput, Pos2, Rect, Vec2, epaint::Shape};

    pub fn record_scroll_geometry(
        ctx: &Context,
        id: &str,
        output: &egui::scroll_area::ScrollAreaOutput<()>,
    ) {
        ctx.data_mut(|data| {
            data.insert_temp(egui::Id::new(id), (output.inner_rect, output.content_size));
        });
    }

    pub fn scroll_geometry(ctx: &Context, id: &str) -> Option<(Rect, Vec2)> {
        ctx.data(|data| data.get_temp(egui::Id::new(id)))
    }

    pub fn frame<T>(
        ctx: &Context,
        size: Vec2,
        events: Vec<Event>,
        mut draw: impl FnMut(&Context) -> T,
    ) -> (T, FullOutput) {
        ctx.begin_pass(egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            time: Some(ctx.cumulative_pass_nr() as f64 / 60.0),
            events,
            ..Default::default()
        });
        let result = draw(ctx);
        let mut output = ctx.end_pass();
        output.textures_delta.clear();
        (result, output)
    }

    pub fn button_position(output: &FullOutput, size: Vec2, label: &str) -> Option<Pos2> {
        fn find(shape: &Shape, clip: Rect, viewport: Rect, label: &str) -> Option<Pos2> {
            match shape {
                Shape::Text(text) if text.galley.job.text == label => {
                    let bounds = text.visual_bounding_rect();
                    (clip.contains_rect(bounds) && viewport.contains_rect(bounds))
                        .then(|| bounds.center())
                }
                Shape::Vec(shapes) => shapes
                    .iter()
                    .find_map(|shape| find(shape, clip, viewport, label)),
                _ => None,
            }
        }
        let viewport = Rect::from_min_size(Pos2::ZERO, size);
        output
            .shapes
            .iter()
            .find_map(|shape| find(&shape.shape, shape.clip_rect, viewport, label))
    }

    pub fn click<T>(
        ctx: &Context,
        size: Vec2,
        pos: Pos2,
        mut draw: impl FnMut(&Context) -> T,
    ) -> T {
        let _ = frame(ctx, size, vec![Event::PointerMoved(pos)], &mut draw);
        let _ = frame(
            ctx,
            size,
            vec![Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            &mut draw,
        );
        frame(
            ctx,
            size,
            vec![Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
            draw,
        )
        .0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_support::{button_position, click, frame, scroll_geometry};

    #[test]
    fn movement_editor_fits_a_settings_column_and_routes_preset_clicks() -> Result<(), &'static str>
    {
        for english in [false, true] {
            let ctx = egui::Context::default();
            crate::client_chrome::install(&ctx, false);
            let size = egui::vec2(450.0, 350.0);
            let mut draft = ClientSettings::default();
            let mut draw = |ctx: &egui::Context| {
                egui::Window::new("movement-test")
                    .default_size([350.0, 260.0])
                    .show(ctx, |ui| {
                        reference_option(
                            ui,
                            &mut draft,
                            "controls",
                            "controls.movement",
                            ["Klawisze ruchu", "Movement Keys"],
                            english,
                        );
                    });
            };
            for _ in 0..4 {
                let _ = frame(&ctx, size, vec![], &mut draw);
            }
            let (_, output) = frame(&ctx, size, vec![], &mut draw);
            for label in if english {
                ["North: Up", "East: Right", "South: Down", "West: Left"]
            } else {
                [
                    "Północ: Up",
                    "Wschód: Right",
                    "Południe: Down",
                    "Zachód: Left",
                ]
            } {
                assert!(
                    button_position(&output, size, label).is_some(),
                    "clipped {label}"
                );
            }
            let wasd = button_position(&output, size, "WASD").ok_or("preset visible")?;
            click(&ctx, size, wasd, &mut draw);
            assert_eq!(draft.movement_keys, [26, 7, 22, 4]);
            draft.validate().map_err(|_| "valid preset")?;
        }
        Ok(())
    }

    #[test]
    fn advanced_action_bars_fit_all_clear_buttons_without_scrolling() -> Result<(), &'static str> {
        for english in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_theme(egui::Theme::Dark);
            let size = egui::vec2(900.0, 620.0);
            let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
            let mut browser = PreferencesBrowser::new();
            browser.section = SETTINGS_SECTIONS
                .iter()
                .position(|section| section.id == "action_bars")
                .ok_or("missing action bars")?;
            browser.set_action_bar_available(true);
            let mut draft = ClientSettings {
                english,
                ..Default::default()
            };
            draft
                .future_preferences
                .insert("basic.basic.advanced".into(), FutureValue::Bool(true));
            let mut output = egui::FullOutput::default();
            for _ in 0..4 {
                output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
            assert!(
                button_position(
                    &output,
                    size,
                    if english {
                        "Clear Right Action Bars:"
                    } else {
                        "Wyczyść prawe paski:"
                    }
                )
                .is_some(),
                "right clear row is clipped"
            );
            for index in 1..=3 {
                let label = format!("{} {index}", if english { "Bar" } else { "Pasek" });
                let (bounds, clip) = output
                    .shapes
                    .iter()
                    .rev()
                    .find_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Text(text) if text.galley.job.text == label => {
                            Some((text.visual_bounding_rect(), shape.clip_rect))
                        }
                        _ => None,
                    })
                    .ok_or("right clear button missing")?;
                assert!(
                    clip.contains_rect(bounds) && viewport.contains_rect(bounds),
                    "right clear button is clipped: {label}"
                );
                let _ = click(&ctx, size, bounds.center(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                });
                assert_eq!(browser.take_clear_action_row(), Some(5 + index));
                let hovered = ctx.interaction_snapshot(|snapshot| {
                    snapshot.hovered.iter().copied().collect::<Vec<_>>()
                });
                assert!(
                    hovered
                        .iter()
                        .filter_map(|id| ctx.read_response(*id))
                        .any(|response| response.clicked()
                            && clip.contains_rect(response.rect)
                            && viewport.contains_rect(response.rect)),
                    "clear button frame is clipped"
                );
                output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
        }
        Ok(())
    }

    #[test]
    fn master_volume_remains_unset_until_slider_input() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);
        crate::client_chrome::install(&ctx, false);
        let size = egui::vec2(600.0, 300.0);
        let mut draft = ClientSettings::default();
        let mut slider = egui::Rect::NOTHING;
        for _ in 0..4 {
            slider = frame(&ctx, size, Vec::new(), |ctx| {
                egui::Window::new("Sound").show(ctx, |ui| master_volume(ui, &mut draft, true))
            })
            .0
            .ok_or("sound window missing")?
            .inner
            .ok_or("sound collapsed")?
            .rect;
        }
        assert!(!draft.future_preferences.contains_key("sound.sound.master"));
        let position = egui::pos2(slider.left() + slider.width() * 0.75, slider.center().y);
        let _ = click(&ctx, size, position, |ctx| {
            egui::Window::new("Sound").show(ctx, |ui| master_volume(ui, &mut draft, true))
        });
        assert!(matches!(
            draft.future_preferences.get("sound.sound.master"),
            Some(FutureValue::Int(1..=100))
        ));
        let saved = draft.future_preferences.clone();
        let _ = frame(&ctx, size, Vec::new(), |ctx| {
            egui::Window::new("Sound").show(ctx, |ui| master_volume(ui, &mut draft, true))
        });
        assert_eq!(draft.future_preferences, saved);
        assert!(draft.validate().is_ok());
        Ok(())
    }

    #[test]
    fn reference_basic_rows_and_advanced_discovery_are_real_local_controls()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);
        crate::client_chrome::install(&ctx, false);
        let size = egui::vec2(900.0, 620.0);
        let mut browser = PreferencesBrowser::new();
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let mut third = draft.action_bar.row(2).ok_or("missing row")?;
        third.locked = true;
        third.shortcuts[0] = Some(oteryn_client::action_bar::SlotShortcut {
            key: 58,
            modifiers: 2,
        });
        draft
            .action_bar
            .set_row(2, third)
            .map_err(|_| "invalid row")?;
        let mut output = egui::FullOutput::default();
        for _ in 0..4 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        for title in ["Gameplay", "Interface", "Graphics", "Sound"] {
            assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.job.text == title)), "reference group {title} not rendered");
        }
        let position =
            button_position(&output, size, "Bar 3").ok_or("individual row control clipped")?;
        let _ = click(&ctx, size, position, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        let updated = draft.action_bar.row(2).ok_or("missing row")?;
        assert!(updated.visible && updated.locked);
        assert_eq!(updated.shortcuts, third.shortcuts);
        assert!(draft.action_bar.row(0).is_some_and(|row| row.visible));
        assert!(draft.action_bar.row(1).is_some_and(|row| !row.visible));
        for enable in [false, true] {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
            let all = button_position(&output, size, "All").ok_or("edge master clipped")?;
            let _ = click(&ctx, size, all, |ctx| browser.show(ctx, &mut draft, None));
            assert_eq!(draft.action_bar.edge_enabled[0], enable);
            assert_eq!(draft.action_bar.row_is_visible(2), enable);
            assert_eq!(draft.action_bar.row(2), Some(updated));
            if !enable {
                output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
                let disabled =
                    button_position(&output, size, "Bar 3").ok_or("disabled row clipped")?;
                let _ = click(&ctx, size, disabled, |ctx| {
                    browser.show(ctx, &mut draft, None)
                });
                assert_eq!(draft.action_bar.row(2), Some(updated));
            }
        }
        output = frame(&ctx, size, Vec::new(), |ctx| {
            browser.show(ctx, &mut draft, None)
        })
        .1;
        assert!(button_position(&output, size, "Effects").is_none());
        let advanced = button_position(&output, size, "Show advanced options")
            .ok_or("advanced switch clipped")?;
        let _ = click(&ctx, size, advanced, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        assert_eq!(
            draft.future_preferences.get("basic.basic.advanced"),
            Some(&FutureValue::Bool(true))
        );
        for _ in 0..4 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        assert!(button_position(&output, size, "Controls").is_some());
        assert!(button_position(&output, size, "Effects").is_none());
        assert!(draft.validate().is_ok());
        Ok(())
    }

    #[test]
    fn clear_row_is_session_gated_and_queues_assignment_request_only() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);
        crate::client_chrome::install(&ctx, false);
        let size = egui::vec2(900.0, 620.0);
        let mut browser = PreferencesBrowser::new();
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let before = draft.action_bar.clone();
        for available in [false, true] {
            browser.set_action_bar_available(available);
            let mut show = |ctx: &egui::Context| {
                egui::Window::new("Action assignments")
                    .default_size([850.0, 580.0])
                    .show(ctx, |ui| {
                        action_bars_page(
                            ui,
                            &mut draft,
                            true,
                            browser.action_bar_available,
                            &mut browser.clear_action_row,
                        )
                    });
            };
            let mut output = egui::FullOutput::default();
            for _ in 0..4 {
                output = frame(&ctx, size, Vec::new(), &mut show).1;
            }
            let position = output
                .shapes
                .iter()
                .rev()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text)
                        if text.galley.job.text == "Bar 3"
                            && shape.clip_rect.contains_rect(text.visual_bounding_rect()) =>
                    {
                        Some(text.visual_bounding_rect().center())
                    }
                    _ => None,
                })
                .ok_or("clear button missing")?;
            click(&ctx, size, position, show);
            assert_eq!(browser.take_clear_action_row(), available.then_some(8));
            assert_eq!(browser.take_clear_action_row(), None);
            assert_eq!(draft.action_bar, before);
        }
        Ok(())
    }

    #[test]
    fn basic_preferences_fit_the_body_at_native_and_tall_desktop_sizes() -> Result<(), &'static str>
    {
        for english in [false, true] {
            for (physical_size, scale) in [
                (egui::vec2(1143.0, 814.0), 1.0),
                (egui::vec2(1143.0, 814.0), 1.8),
                (egui::vec2(1920.0, 1440.0), 1.0),
            ] {
                let ctx = egui::Context::default();
                ctx.set_theme(egui::Theme::Dark);
                crate::client_chrome::install(&ctx, false);
                let size = physical_size / scale;
                let mut browser = PreferencesBrowser::new();
                let mut draft = ClientSettings {
                    english,
                    ui_scale: scale,
                    ..Default::default()
                };
                let mut output = egui::FullOutput::default();
                for _ in 0..30 {
                    output = frame(&ctx, size, Vec::new(), |ctx| {
                        browser.show(ctx, &mut draft, None)
                    })
                    .1;
                }
                for label in if english {
                    [
                        "Gameplay",
                        "Interface",
                        "Graphics",
                        "Sound",
                        "Master Volume: —",
                    ]
                } else {
                    [
                        "Rozgrywka",
                        "Interfejs",
                        "Grafika",
                        "Dźwięk",
                        "Głośność główna: —",
                    ]
                } {
                    assert!(
                        button_position(&output, size, label).is_some(),
                        "Basic control is clipped: {label}, {size:?}"
                    );
                }
                let (inner, content) = scroll_geometry(&ctx, "all-preferences-body-geometry")
                    .ok_or("missing preferences body geometry")?;
                assert!(
                    content.x <= inner.width() + 1.0,
                    "Basic preferences unnecessarily scroll horizontally: {content:?}, {inner:?}"
                );
                assert!(
                    content.y <= inner.height() + 1.0,
                    "Basic preferences overflow the dialog body: {content:?}, {inner:?}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn category_navigation_does_not_inherit_other_options_scroll() -> Result<(), &'static str> {
        for english in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_theme(egui::Theme::Dark);
            crate::client_chrome::install(&ctx, false);
            let size = egui::vec2(900.0, 620.0);
            let mut browser = PreferencesBrowser::new();
            let mut draft = ClientSettings {
                english,
                ..Default::default()
            };
            draft
                .future_preferences
                .insert("basic.basic.advanced".into(), FutureValue::Bool(true));
            browser.expanded.extend(["graphics", "interface"]);
            let effects = SETTINGS_SECTIONS
                .iter()
                .find(|section| section.id == "hud")
                .ok_or("missing effects")?;
            let action_bars = SETTINGS_SECTIONS
                .iter()
                .find(|section| section.id == "action_bars")
                .ok_or("missing action bars")?;
            let mut output = egui::FullOutput::default();
            for _ in 0..4 {
                output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
            let position = button_position(&output, size, effects.text(english))
                .ok_or("effects navigation not visible")?;
            let _ = click(&ctx, size, position, |ctx| {
                browser.show(ctx, &mut draft, None)
            });
            for _ in 0..4 {
                output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
            let first_effect = if english {
                "Show HUD for Own Character"
            } else {
                "HUD własnej postaci"
            };
            assert!(button_position(&output, size, first_effect).is_some());
            let _ = frame(
                &ctx,
                size,
                vec![
                    egui::Event::PointerMoved(egui::pos2(600.0, 300.0)),
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0.0, -350.0),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                |ctx| browser.show(ctx, &mut draft, None),
            );
            for _ in 0..30 {
                output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
            assert!(
                button_position(&output, size, first_effect).is_none(),
                "effects did not scroll"
            );
            let position = button_position(&output, size, action_bars.text(english))
                .ok_or("action bars navigation not visible")?;
            let _ = click(&ctx, size, position, |ctx| {
                browser.show(ctx, &mut draft, None)
            });
            for _ in 0..4 {
                output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
            assert_eq!(SETTINGS_SECTIONS[browser.section].id, "action_bars");
            for label in if english {
                [
                    "Show Bottom Action Bars:",
                    "Show Left Action Bars:",
                    "Show Right Action Bars:",
                ]
            } else {
                [
                    "Pokaż dolne paski akcji:",
                    "Pokaż lewe paski akcji:",
                    "Pokaż prawe paski akcji:",
                ]
            } {
                assert!(
                    button_position(&output, size, label).is_some(),
                    "new category hides its first visibility control: {label}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn apply_and_cancel_remain_clickable_after_resize_at_high_ui_scale() -> Result<(), &'static str>
    {
        for english in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_theme(egui::Theme::Dark);
            crate::client_chrome::install(&ctx, false);
            let mut browser = PreferencesBrowser::new();
            browser.section = SETTINGS_SECTIONS
                .iter()
                .position(|section| section.id == "action_hotkeys")
                .ok_or("missing action hotkeys")?;
            let mut draft = ClientSettings {
                english,
                ui_scale: 1.8,
                ..Default::default()
            };
            draft
                .future_preferences
                .insert("basic.basic.advanced".into(), FutureValue::Bool(true));
            let message =
                "A long settings error must scroll without hiding recovery controls. ".repeat(40);
            // RawInput is already logical: the native adapter divides physical size by UI scale.
            for size in [
                egui::vec2(900.0, 620.0),
                egui::vec2(300.0, 200.0) / 1.8,
                egui::vec2(800.0, 600.0) / 1.8,
            ] {
                let mut output = egui::FullOutput::default();
                for _ in 0..4 {
                    output = frame(&ctx, size, Vec::new(), |ctx| {
                        browser.show(ctx, &mut draft, Some(&message))
                    })
                    .1;
                }
                let apply_label = if english { "Apply" } else { "Zastosuj" };
                let apply = button_position(&output, size, apply_label)
                    .ok_or("Apply is clipped or outside viewport")?;
                assert!(
                    matches!(
                        click(&ctx, size, apply, |ctx| browser.show(
                            ctx,
                            &mut draft,
                            Some(&message)
                        )),
                        PreferenceAction::Apply
                    ),
                    "Apply click failed at {size:?}, english={english}, position={apply:?}"
                );
                let output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, Some(&message))
                })
                .1;
                let ok = button_position(&output, size, "OK")
                    .ok_or("OK is clipped or outside viewport")?;
                assert!(matches!(
                    click(&ctx, size, ok, |ctx| browser.show(
                        ctx,
                        &mut draft,
                        Some(&message)
                    )),
                    PreferenceAction::Ok
                ));
                let output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, Some(&message))
                })
                .1;
                let cancel =
                    button_position(&output, size, if english { "Cancel" } else { "Anuluj" })
                        .ok_or("Cancel is clipped or outside viewport")?;
                assert!(matches!(
                    click(&ctx, size, cancel, |ctx| browser.show(
                        ctx,
                        &mut draft,
                        Some(&message)
                    )),
                    PreferenceAction::Cancel
                ));
            }
        }
        Ok(())
    }
}
