//! Complete preference navigation with explicit future-consumer configuration.
#[path = "preferences_overview.rs"]
mod overview;
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
    overview: bool,
    search: String,
    page_state: pages::PageState,
    action_bar_available: bool,
    clear_action_row: Option<usize>,
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
    ExportOptions,
    ImportOptions,
    ResetOptions,
    OpenScreenshotFolder,
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
            overview: false,
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
        crate::client_chrome::install_preferences(
            ctx,
            draft.high_contrast,
            draft.settings_transparency,
        );
        let mut action = PreferenceAction::None;
        let english = draft.english;
        let tr = |pl, en| if english { en } else { pl };
        let bounds = ctx.content_rect().shrink(12.0);
        let maximum = bounds.size().max(egui::Vec2::splat(1.0));
        let size = egui::vec2(1180.0_f32.min(maximum.x), 880.0_f32.min(maximum.y));
        let style = ctx.style_of(egui::Theme::Dark);
        egui::Window::new(tr("Ustawienia", "Settings"))
            .id("complete-preferences".into())
            .title_bar(false).collapsible(false).resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .constrain_to(bounds).min_size(egui::Vec2::ZERO).max_size(maximum)
            .fixed_size(size)
            .frame(egui::Frame::new().fill(style.visuals.window_fill)
                .stroke(style.visuals.window_stroke).corner_radius(7.0))
            .show(ctx, |ui| {
                ui.set_min_size(size);
                ui.style_mut().spacing.scroll.floating = false;
                let (footer_action, body_rect) = primary_footer_impl(ui, "all-preferences-footer", english, None);
                action = footer_action;
                let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
                body_ui.set_clip_rect(body_rect.intersect(ui.clip_rect()));
                egui::Panel::top("preferences-brand-header").exact_size(70.0)
                    .frame(egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(35,39,42,100)).inner_margin(egui::Margin::symmetric(20,12)))
                    .show(&mut body_ui, |ui| {
                        ui.horizontal_centered(|ui| {
                            if size.x>650.0 {ui.label(egui::RichText::new("OTERYN").size(26.0).color(egui::Color32::from_rgb(231,205,153)));
                            ui.add_space(12.0); ui.separator(); ui.add_space(12.0);}
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(tr("Ustawienia", "Settings")).size(20.0).strong());
                                if size.x>650.0 {ui.weak(tr("Twój świat. Twój sposób gry.", "Your world. Your way to play."));}
                            });
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.add_sized([30.0,30.0], egui::Button::new("×")).clicked() { action = PreferenceAction::Cancel; }
                                if size.x > 650.0 {
                                    ui.spacing_mut().slider_width = 90.0;
                                    ui.add(egui::Slider::new(&mut draft.settings_transparency, 0..=70).suffix("%").text(tr("Przezroczystość", "Transparency")));
                                }
                            });
                        });
                    });
                let nav_width = if size.x >= 1100.0 {222.0} else if size.x >= 700.0 {190.0} else {135.0};
                if size.x>=650.0 {egui::Panel::left("preferences-navigation").exact_size(nav_width).resizable(false)
                    .frame(egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(15,19,22,110)).inner_margin(12))
                    .show(&mut body_ui, |ui| {
                        ui.spacing_mut().item_spacing.y=2.0;
                        ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(ui.available_width()).char_limit(120).hint_text(tr("Szukaj ustawienia…", "Search preferences…")));
                        ui.add_space(8.0);
                        egui::Panel::bottom("preferences-account-link").exact_size(34.0).show(ui, |ui| {
                            if ui.button(tr("Konto i sieć", "Account/network")).clicked() {action=PreferenceAction::QuickSettings;}
                        });
                        egui::ScrollArea::vertical().id_salt("all-preferences-sections").auto_shrink([false,false]).show(ui, |ui| {
                            if ui.selectable_label(self.overview,tr("Szybkie ustawienia", "Quick settings")).clicked() {self.overview=true;}
                            let search = self.search.trim().to_lowercase();
                            for (index, section) in SETTINGS_SECTIONS.iter().enumerate() {
                                if !search.is_empty() && !section.text(english).to_lowercase().contains(&search) && !section.options.iter().any(|option| option.text(english).to_lowercase().contains(&search)) {continue;}
                                if let Some(group) = match section.id {
                                    "basic" => Some(tr("NA SKRÓTY", "QUICK ACCESS")),
                                    "controls" => Some(tr("STEROWANIE", "CONTROLS")),
                                    "interface" => Some(tr("INTERFEJS", "INTERFACE")),
                                    "graphics" => Some(tr("OBRAZ I DŹWIĘK", "VIDEO AND AUDIO")),
                                    "miscellaneous" => Some(tr("POZOSTAŁE", "OTHER")),
                                    "panels" => Some("OTERYN"), _ => None,
                                } {
                                    ui.add_space(16.0);
                                    ui.label(egui::RichText::new(group).size(10.0).color(egui::Color32::from_rgb(183,169,141)));
                                    ui.add_space(3.0);
                                }
                                let selected = !self.overview && self.section==index;
                                let label = if section.id=="hud" {tr("HUD postaci", "Character HUD")} else {section.text(english)};
                                let response = ui.add_sized([ui.available_width(),30.0],egui::Button::new("").selected(selected).frame(selected));
                                ui.painter().text(response.rect.left_center()+egui::vec2(10.0,0.0),egui::Align2::LEFT_CENTER,label,egui::FontId::proportional(13.0),if selected {egui::Color32::from_rgb(240,211,148)} else {ui.visuals().text_color()});
                                if selected {ui.painter().rect_filled(egui::Rect::from_min_size(response.rect.min,egui::vec2(3.0,response.rect.height())),1.0,ui.visuals().selection.stroke.color);}
                                if response.clicked() {self.section=index;self.overview=false;}
                            }

                        });
                    });
                } else {
                    egui::Panel::top("preferences-compact-navigation").show(&mut body_ui, |ui| {
                        egui::ComboBox::from_id_salt("preferences-category").selected_text(SETTINGS_SECTIONS[self.section].text(english)).show_ui(ui, |ui| {
                            for (index,section) in SETTINGS_SECTIONS.iter().enumerate() {if ui.selectable_value(&mut self.section,index,section.text(english)).clicked() {self.overview=false;}}
                        });
                    });
                }
                egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(if size.x<700.0 {12} else {22}))
                    .show(&mut body_ui, |ui| {
                        let search = self.search.trim().to_lowercase();
                        let _body_output = egui::ScrollArea::vertical().id_salt(("all-preferences-options", SETTINGS_SECTIONS[self.section].id)).auto_shrink([false,false]).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.label(egui::RichText::new(tr("USTAWIENIA KLIENTA", "CLIENT SETTINGS")).size(10.0).color(egui::Color32::from_rgb(185,164,130)));
                            ui.add_space(3.0);
                            if !self.overview {ui.label(egui::RichText::new(if SETTINGS_SECTIONS[self.section].id=="hud" {tr("HUD postaci", "Character HUD")} else {SETTINGS_SECTIONS[self.section].text(english)}).size(24.0));}
                            ui.add_space(16.0);
                        let section = &SETTINGS_SECTIONS[self.section];
                        if self.overview { overview::show(ui, draft, english); }
                        else if section.id == "basic" { basic_page(ui, draft, english); }
                        else if section.id == "action_bars" { action_bars_page(ui, draft, english, self.action_bar_available, &mut self.clear_action_row); }
                        else if search.is_empty() && pages::show(ui, draft, section.id, english, &mut self.page_state) {
                            if let Some(help_action) = self.page_state.take_help_action() {
                                action = match help_action {
                                    pages::HelpAction::Export => PreferenceAction::ExportOptions,
                                    pages::HelpAction::Import => PreferenceAction::ImportOptions,
                                    pages::HelpAction::Reset => PreferenceAction::ResetOptions,
                                    pages::HelpAction::OpenScreenshotFolder => PreferenceAction::OpenScreenshotFolder,
                                };
                            }
                        }
                        else {

                        let mut matching = 0;
                        for option in section.options {
                            if !search.is_empty() && !section.text(english).to_lowercase().contains(&search) && !option.text(english).to_lowercase().contains(&search) { continue; }
                            matching += 1;
                            // Former family-level scalar bindings had no slot identity.
                            // Their saved intent stays intact; real rows use the typed editor.
                            if section.id == "action_hotkeys" { continue; }
                            match option.implementation {
                                Implementation::Implemented{field} => implemented(ui, draft, option, field, english),
                                Implementation::Stored { consumer } => {
                                    let key = format!("{}.{}", section.id, option.id);
                                    ui.push_id(&key, |ui| {
                                        ui.label(option.text(english)).on_hover_text(if english {
                                            format!("Applied by {consumer}.")
                                        } else {
                                            format!("Używane przez: {consumer}.")
                                        });
                                        stored_value(ui, draft, option, key.clone(), english);
                                    });
                                    ui.separator();
                                }
                                Implementation::Pending{ consumer } => {
                                    let key = format!("{}.{}", section.id, option.id);
                                    ui.push_id(&key, |ui| {
                                        ui.label(option.text(english)).on_hover_text(if english {
                                            format!("Unavailable: missing {consumer}.")
                                        } else {
                                            format!("Niedostępne: brak obsługi przez {consumer}.")
                                        });
                                        unavailable_value(ui, draft, option, key.clone(), english);
                                    });
                                    ui.separator();
                                }
                            }
                        }
                        if section.id == "help" {
                            ui.collapsing(tr("Licencje ikon interfejsu", "Interface icon licenses"), |ui| {
                                ui.label(include_str!("../assets/icons/LICENSE.txt"));
                            });
                        }
                        if matching == 0 { ui.weak(tr("Brak wyników w tej kategorii. Wybierz kategorię z listy.", "No matches in this category. Choose a category from the list.")); }
                        if section.id == "action_hotkeys" && (matching > 0 || search.is_empty()) {
                            ui.separator();
                            ui.heading(tr("Dziewięć pasków i ich skróty", "Nine action rows and their shortcuts"));
                            crate::action_bar_ui::preferences_editor(ui, &mut draft.action_bar, english);
                        }
                        }
                            if let Some(message) = message {ui.add_space(12.0);ui.label(message);}
                        });
                        #[cfg(test)]
                        test_support::record_scroll_geometry(ctx, "all-preferences-body-geometry", &_body_output);
                    });
            });
        let active_action_bar = draft.action_bar.clone();
        if let Some(profile) = draft.hotkeys.active_mut() {
            profile.action_bar = active_action_bar;
        }
        ctx.set_style_of(egui::Theme::Dark, previous_style);
        action
    }
}

fn boxed(ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
    let line = egui::Color32::from_rgb(58, 61, 61);
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, line))
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !title.is_empty() {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(35, 39, 42, 160))
                    .inner_margin(egui::Margin::symmetric(12, 8))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new(title).size(13.0).strong());
                    });
            }
            egui::Frame::new()
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    body(ui);
                });
        });
    ui.add_space(12.0);
}

fn future_toggle(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    key: &str,
    label: &str,
    english: bool,
) {
    let consumer = oteryn_client::settings_catalog::runtime_consumer(key);
    let stored = draft.future_preferences.get(key);
    let mut value = matches!(stored, Some(FutureValue::Bool(true)));
    let response = ui
        .add_enabled(
            consumer.is_some(),
            egui::Checkbox::new(&mut value, label).indeterminate(stored.is_none()),
        )
        .on_hover_text(match (consumer, english) {
            (Some(consumer), true) => format!("Applied by {consumer}."),
            (Some(consumer), false) => format!("Używane przez: {consumer}."),
            (None, true) => "Unavailable: this control has no runtime consumer.".into(),
            (None, false) => "Niedostępne: ta kontrolka nie ma obsługi wykonawczej.".into(),
        });
    // Keep egui's input/accessibility behavior and paint the project's compact square control.
    let icon = egui::Rect::from_center_size(
        egui::pos2(response.rect.left() + 8.0, response.rect.center().y),
        egui::vec2(16.0, 16.0),
    );
    let accent = ui.visuals().selection.stroke.color;
    let fill = if value {
        accent
    } else {
        egui::Color32::from_rgb(18, 21, 24)
    };
    ui.painter().rect(
        icon,
        2.0,
        fill,
        egui::Stroke::new(
            1.0,
            if value {
                accent
            } else {
                egui::Color32::from_rgb(92, 97, 89)
            },
        ),
        egui::StrokeKind::Inside,
    );
    if value {
        ui.painter().add(egui::Shape::line(
            vec![
                icon.left_center() + egui::vec2(3.0, 0.0),
                icon.center() + egui::vec2(-1.0, 4.0),
                icon.right_top() + egui::vec2(-3.0, 4.0),
            ],
            egui::Stroke::new(2.0, egui::Color32::from_rgb(45, 43, 34)),
        ));
    } else if stored.is_none() {
        ui.painter().hline(
            icon.left() + 4.0..=icon.right() - 4.0,
            icon.center().y,
            egui::Stroke::new(1.0, ui.visuals().weak_text_color()),
        );
    }
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("preference-toggle", key)),
            (response.rect, ui.clip_rect()),
        )
    });
    if consumer.is_some() && response.changed() {
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
        if matches!(
            option.implementation,
            Implementation::Stored { .. } | Implementation::Pending { .. }
        ) && option.kind == OptionKind::Toggle
        {
            future_toggle(ui, draft, &key, label, english);
        } else if let Implementation::Implemented {
            field: "movement_keys",
        } = option.implementation
        {
            ui.vertical(|ui| implemented(ui, draft, option, "movement_keys", english));
        } else {
            ui.horizontal(|ui| match option.implementation {
                Implementation::Implemented { field } => {
                    if field == "fullscreen" {
                        ui.checkbox(&mut draft.fullscreen, label);
                    } else {
                        implemented(ui, draft, option, field, english);
                    }
                }
                Implementation::Stored { .. } => {
                    ui.label(label);
                    stored_value(ui, draft, option, key.clone(), english);
                }
                Implementation::Pending { consumer } => {
                    ui.label(label).on_hover_text(if english {
                        format!("Unavailable: missing {consumer}.")
                    } else {
                        format!("Niedostępne: brak obsługi przez {consumer}.")
                    });
                    unavailable_value(ui, draft, option, key.clone(), english);
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
        ui.horizontal_wrapped(|ui| {
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
    let consumer = oteryn_client::settings_catalog::runtime_consumer(key);
    let response = ui
        .horizontal(|ui| {
            ui.label(format!(
                "{} {text}",
                if english {
                    "Master Volume:"
                } else {
                    "Głośność główna:"
                }
            ));
            ui.spacing_mut().slider_width = ui.available_width().clamp(50.0, 180.0);
            ui.add_enabled(
                consumer.is_some(),
                egui::Slider::new(&mut value, 0..=100).show_value(false),
            )
            .on_hover_text(if english {
                "Unavailable: audio playback has no runtime backend."
            } else {
                "Niedostępne: odtwarzanie dźwięku nie ma jeszcze warstwy wykonawczej."
            })
        })
        .inner;
    // Choosing zero explicitly also changes the selection from unknown to known.
    if consumer.is_some() && (response.changed() || (selected.is_none() && response.clicked())) {
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
    let compact_footer = available.width() < 520.0;
    let height = ui.spacing().interact_size.y + if compact_footer { 8.0 } else { 24.0 };
    let footer = egui::Panel::bottom(egui::Id::new(id))
        .frame(
            egui::Frame::new()
                .fill(egui::Color32::from_rgba_unmultiplied(29, 32, 33, 140))
                .inner_margin(egui::Margin::symmetric(
                    if compact_footer { 2 } else { 18 },
                    if compact_footer { 2 } else { 10 },
                )),
        )
        .exact_size(height)
        .show(ui, |ui| {
            ui.spacing_mut().button_padding =
                egui::vec2(if compact_footer { 2.0 } else { 6.0 }, 3.0);
            if available.width() < 300.0 {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.style_mut()
                    .text_styles
                    .insert(egui::TextStyle::Button, egui::FontId::proportional(10.0));
            }
            ui.horizontal(|ui| {
                let compact = ui.available_width() < 520.0;
                if ui
                    .button("Reset")
                    .on_hover_text(if english {
                        "Restore defaults in the settings draft"
                    } else {
                        "Przywróć domyślne ustawienia w wersji roboczej"
                    })
                    .clicked()
                {
                    action = PreferenceAction::Defaults;
                }
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
                        .add_sized(
                            [
                                if compact { 0.0 } else { 78.0 },
                                if compact { 18.0 } else { 32.0 },
                            ],
                            egui::Button::new(
                                egui::RichText::new("OK")
                                    .color(egui::Color32::from_rgb(25, 24, 20))
                                    .strong(),
                            )
                            .fill(egui::Color32::from_rgb(207, 177, 119)),
                        )
                        .clicked()
                    {
                        action = PreferenceAction::Ok;
                    }
                    if ui
                        .add_sized(
                            [
                                if compact { 0.0 } else { 78.0 },
                                if compact { 18.0 } else { 32.0 },
                            ],
                            egui::Button::new(apply),
                        )
                        .clicked()
                    {
                        action = PreferenceAction::Apply;
                    }
                    if ui
                        .add_sized(
                            [
                                if compact { 0.0 } else { 78.0 },
                                if compact { 18.0 } else { 32.0 },
                            ],
                            egui::Button::new(if english { "Cancel" } else { "Anuluj" }),
                        )
                        .clicked()
                    {
                        action = PreferenceAction::Cancel;
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

fn stored_value(
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

fn unavailable_value(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    option: &SettingOption,
    key: String,
    english: bool,
) {
    ui.add_enabled_ui(false, |ui| {
        stored_value(ui, draft, option, key, english);
    });
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

    pub fn reveal<T>(
        ctx: &Context,
        size: Vec2,
        label: &str,
        pointer: Pos2,
        mut draw: impl FnMut(&Context) -> T,
    ) -> FullOutput {
        let mut output = frame(ctx, size, vec![], &mut draw).1;
        if button_position(&output, size, label).is_some() {
            return output;
        }
        for delta in std::iter::once(4000.0).chain(std::iter::repeat_n(-100.0, 40)) {
            output = frame(
                ctx,
                size,
                vec![
                    Event::PointerMoved(pointer),
                    Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Point,
                        delta: egui::vec2(0.0, delta),
                        phase: egui::TouchPhase::Move,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                &mut draw,
            )
            .1;
            for _ in 0..12 {
                output = frame(ctx, size, vec![], &mut draw).1;
            }
            if button_position(&output, size, label).is_some() {
                break;
            }
        }
        output
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
    use test_support::{button_position, click, frame, reveal, scroll_geometry};

    #[test]
    fn every_reference_settings_page_renders_in_the_compiled_client_at_native_size()
    -> Result<(), &'static str> {
        let pages = [
            ("basic", "Gameplay"),
            ("controls", "Mouse Preset:"),
            ("general_hotkeys", "Auto-Switch Hotkey Preset"),
            ("action_hotkeys", "Bottom Action Bar: Action Button 1.01"),
            ("custom_hotkeys", "New Action"),
            ("interface", "Highlight Mouse Target"),
            ("hud", "Special conditions · 35"),
            ("console", "Show Info Messages"),
            ("game_window", "Show Textual Effects"),
            ("action_bars", "Show Bottom Action Bars:"),
            ("shortcuts", "Displayed Shortcuts:"),
            ("graphics", "Graphics Engine:"),
            ("effects", "Lighting"),
            ("sound", "Sound Device:"),
            ("battle_sounds", "Own Battle Sounds"),
            ("ui_sounds", "UI Volume: —"),
            ("miscellaneous", "Ask Before Buying Products"),
            ("gameplay", "Allow Others to Inspect Your Character"),
            ("screenshots", "Only Capture Game Window"),
            ("help", "Client Help"),
        ];
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);
        crate::client_chrome::install(&ctx, false);
        let size = egui::vec2(900.0, 620.0);
        let mut browser = PreferencesBrowser::new();
        browser.overview = false;
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        for (section, marker) in pages {
            browser.section = SETTINGS_SECTIONS
                .iter()
                .position(|entry| entry.id == section)
                .ok_or("missing settings section")?;
            let mut output = egui::FullOutput::default();
            for _ in 0..6 {
                output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
            assert!(
                button_position(&output, size, marker).is_some(),
                "compiled page marker is not reachable: {section} / {marker}"
            );
            for footer in ["Reset", "OK", "Apply", "Cancel"] {
                assert!(
                    button_position(&output, size, footer).is_some(),
                    "compiled page footer is not reachable"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn screenshot_page_routes_the_real_folder_action() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(900.0, 620.0);
        let mut browser = PreferencesBrowser::new();
        browser.section = SETTINGS_SECTIONS
            .iter()
            .position(|section| section.id == "screenshots")
            .ok_or("screenshots page")?;
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let output = reveal(
            &ctx,
            size,
            "Open Screenshot Folder",
            egui::pos2(size.x * 0.7, size.y * 0.7),
            |ctx| browser.show(ctx, &mut draft, None),
        );
        let button = button_position(&output, size, "Open Screenshot Folder")
            .ok_or("folder action clipped")?;
        assert!(matches!(
            click(&ctx, size, button, |ctx| browser
                .show(ctx, &mut draft, None)),
            PreferenceAction::OpenScreenshotFolder
        ));
        Ok(())
    }

    #[test]
    fn quick_settings_hud_navigation_opens_the_requested_page() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(900.0, 620.0);
        let mut browser = PreferencesBrowser::new();
        browser.overview = true;
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let mut output = egui::FullOutput::default();
        for _ in 0..5 {
            output = frame(&ctx, size, vec![], |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        let target =
            button_position(&output, size, "Character HUD").ok_or("HUD navigation missing")?;
        let _ = click(&ctx, size, target, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        for _ in 0..3 {
            let _ = frame(&ctx, size, vec![], |ctx| {
                browser.show(ctx, &mut draft, None)
            });
        }
        assert!(!browser.overview);
        assert_eq!(SETTINGS_SECTIONS[browser.section].id, "hud");
        Ok(())
    }

    #[test]
    fn native_default_hud_fits_status_flags_and_routes_independent_condition_cells()
    -> Result<(), &'static str> {
        for english in [false, true] {
            let ctx = egui::Context::default();
            let size = egui::vec2(1440.0, 1000.0);
            let mut browser = PreferencesBrowser::new();
            browser.overview = false;
            browser.section = SETTINGS_SECTIONS
                .iter()
                .position(|s| s.id == "hud")
                .ok_or("HUD page")?;
            let mut draft = ClientSettings {
                english,
                ..Default::default()
            };
            draft
                .future_preferences
                .insert("basic.basic.advanced".into(), FutureValue::Bool(true));
            draft
                .future_preferences
                .insert("hud.conditions_hud_enabled".into(), FutureValue::Bool(true));
            draft
                .future_preferences
                .insert("hud.conditions_bar_enabled".into(), FutureValue::Bool(true));
            let mut output = egui::FullOutput::default();
            for _ in 0..4 {
                output = frame(&ctx, size, vec![], |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
            for label in if english {
                [
                    "Account/network",
                    "Powerless",
                    "Show Customisable Status Bars",
                    "Show Status Bars",
                    "Reset",
                    "OK",
                    "Apply",
                    "Cancel",
                ]
            } else {
                [
                    "Konto i sieć",
                    "Bezsilność",
                    "Konfigurowalne paski stanu",
                    "Paski stanu",
                    "Reset",
                    "OK",
                    "Zastosuj",
                    "Anuluj",
                ]
            } {
                output = reveal(
                    &ctx,
                    size,
                    label,
                    egui::pos2(size.x * 0.7, size.y * 0.7),
                    |ctx| browser.show(ctx, &mut draft, None),
                );
                assert!(
                    button_position(&output, size, label).is_some(),
                    "default HUD clips {label}"
                );
            }
            // Short and long labels must not move either checkbox column.
            let condition_cell = |key: &str| {
                ctx.data(|data| {
                    data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                        "preference-toggle",
                        key,
                    )))
                })
                .map(|(rect, _)| rect)
            };
            let poison = condition_cell("hud.condition_poison_hud").ok_or("poison HUD cell")?;
            let powerless =
                condition_cell("hud.condition_powerless_hud").ok_or("powerless HUD cell")?;
            let poison_bar = condition_cell("hud.condition_poison_bar").ok_or("poison bar cell")?;
            let powerless_bar =
                condition_cell("hud.condition_powerless_bar").ok_or("powerless bar cell")?;
            assert!((poison.left() - powerless.left()).abs() < 0.5);
            assert!((poison_bar.left() - powerless_bar.left()).abs() < 0.5);
            assert!(poison_bar.left() - poison.left() > 70.0);
            let account = button_position(
                &output,
                size,
                if english {
                    "Account/network"
                } else {
                    "Konto i sieć"
                },
            )
            .ok_or("account navigation clipped")?;
            assert!(matches!(
                click(&ctx, size, account, |ctx| browser
                    .show(ctx, &mut draft, None)),
                PreferenceAction::QuickSettings
            ));
            let _ = reveal(
                &ctx,
                size,
                if english { "Powerless" } else { "Bezsilność" },
                egui::pos2(size.x * 0.7, size.y * 0.7),
                |ctx| browser.show(ctx, &mut draft, None),
            );
            for where_ in ["hud", "bar"] {
                let key = format!("hud.condition_powerless_{where_}");
                let (rect, clip) = ctx
                    .data(|data| {
                        data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                            "preference-toggle",
                            key.as_str(),
                        )))
                    })
                    .ok_or("condition cell")?;
                assert!(clip.contains_rect(rect));
                let _ = click(&ctx, size, rect.center(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                });
                assert!(!draft.future_preferences.contains_key(&key));
            }
            assert!(
                !draft
                    .future_preferences
                    .contains_key("hud.condition_agony_hud")
            );
            draft.validate().map_err(|_| "valid draft")?;
            assert!(
                button_position(
                    &output,
                    size,
                    if english {
                        "Search preferences…"
                    } else {
                        "Szukaj ustawienia…"
                    }
                )
                .is_some()
            );
        }
        Ok(())
    }

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
    fn advanced_action_bars_scroll_to_clear_buttons_and_route_actions() -> Result<(), &'static str>
    {
        for english in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_theme(egui::Theme::Dark);
            let size = egui::vec2(900.0, 620.0);
            let viewport = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
            let mut browser = PreferencesBrowser::new();
            browser.overview = false;
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
            let mut output;
            for _ in 0..4 {
                let _ = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
            output = reveal(
                &ctx,
                size,
                if english {
                    "Clear Right Action Bars:"
                } else {
                    "Wyczyść prawe paski:"
                },
                egui::pos2(size.x * 0.75, size.y * 0.6),
                |ctx| browser.show(ctx, &mut draft, None),
            );
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
    fn master_volume_without_an_audio_backend_is_disabled_and_never_saved()
    -> Result<(), &'static str> {
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
        assert!(!draft.future_preferences.contains_key("sound.sound.master"));
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
        browser.overview = false;
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
        let mut output;
        for _ in 0..4 {
            let _ = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        for title in ["Gameplay", "Interface", "Graphics", "Sound"] {
            output = reveal(
                &ctx,
                size,
                title,
                egui::pos2(size.x * 0.75, size.y * 0.6),
                |ctx| browser.show(ctx, &mut draft, None),
            );
            assert!(
                button_position(&output, size, title).is_some(),
                "reference group {title} unreachable"
            );
        }
        output = reveal(
            &ctx,
            size,
            "Bar 3",
            egui::pos2(size.x * 0.75, size.y * 0.6),
            |ctx| browser.show(ctx, &mut draft, None),
        );
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
        // Full settings no longer need the old advanced-mode toggle.
        assert!(button_position(&output, size, "Controls").is_some());
        assert!(
            !draft
                .future_preferences
                .contains_key("basic.basic.advanced")
        );
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
        browser.overview = false;
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
    fn basic_preferences_remain_reachable_at_native_and_scaled_desktop_sizes()
    -> Result<(), &'static str> {
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
                browser.overview = false;
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
                    // Larger type may require scrolling at high UI scale, but every
                    // option must remain reachable without moving the fixed footer.
                    for _ in 0..8 {
                        if button_position(&output, size, label).is_some() {
                            break;
                        }
                        output = frame(
                            &ctx,
                            size,
                            vec![
                                egui::Event::PointerMoved(egui::pos2(size.x * 0.75, size.y * 0.6)),
                                egui::Event::MouseWheel {
                                    unit: egui::MouseWheelUnit::Point,
                                    delta: egui::vec2(0.0, -100.0),
                                    phase: egui::TouchPhase::Move,
                                    modifiers: egui::Modifiers::NONE,
                                },
                            ],
                            |ctx| browser.show(ctx, &mut draft, None),
                        )
                        .1;
                        for _ in 0..10 {
                            output = frame(&ctx, size, vec![], |ctx| {
                                browser.show(ctx, &mut draft, None)
                            })
                            .1;
                        }
                    }
                    assert!(
                        button_position(&output, size, label).is_some(),
                        "Basic control unreachable: {label}, {size:?}"
                    );
                    assert!(button_position(&output, size, "OK").is_some());
                }
                let (inner, content) = scroll_geometry(&ctx, "all-preferences-body-geometry")
                    .ok_or("missing preferences body geometry")?;
                assert!(
                    content.x <= inner.width() + 1.0,
                    "Basic preferences unnecessarily scroll horizontally: {content:?}, {inner:?}"
                );
                assert!(
                    inner.bottom() < size.y,
                    "Page viewport must stay within the window"
                );
                assert!(
                    button_position(&output, size, "OK").is_some(),
                    "Fixed footer must remain reachable after page scrolling"
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
            browser.overview = false;
            let mut draft = ClientSettings {
                english,
                ..Default::default()
            };
            draft
                .future_preferences
                .insert("basic.basic.advanced".into(), FutureValue::Bool(true));
            let effects = SETTINGS_SECTIONS
                .iter()
                .find(|section| section.id == "game_window")
                .ok_or("missing effects")?;
            let action_bars = SETTINGS_SECTIONS
                .iter()
                .find(|section| section.id == "action_bars")
                .ok_or("missing action bars")?;
            let mut output;
            for _ in 0..4 {
                let _ = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
            output = reveal(
                &ctx,
                size,
                effects.text(english),
                egui::pos2(110.0, size.y * 0.6),
                |ctx| browser.show(ctx, &mut draft, None),
            );
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
                "Show Textual Effects"
            } else {
                "Efekty tekstowe"
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
            output = reveal(
                &ctx,
                size,
                action_bars.text(english),
                egui::pos2(110.0, size.y * 0.6),
                |ctx| browser.show(ctx, &mut draft, None),
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
            browser.overview = false;
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
