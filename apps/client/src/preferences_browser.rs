//! Complete preference navigation with explicit future-consumer configuration.
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
}

#[derive(Default)]
pub enum PreferenceAction {
    #[default]
    None,
    Apply,
    Cancel,
    Defaults,
    QuickSettings,
}

impl PreferencesBrowser {
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
        let mut action = PreferenceAction::None;
        let english = draft.english;
        let tr = |pl, en| if english { en } else { pl };
        let maximum =
            (ctx.content_rect().size() - egui::vec2(40.0, 80.0)).max(egui::vec2(280.0, 220.0));
        egui::Window::new(tr("Wszystkie ustawienia", "All preferences"))
            .id("complete-preferences".into()).open(&mut self.open)
            .collapsible(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .constrain_to(ctx.content_rect()).max_size(maximum)
            .default_size([820.0_f32.min(maximum.x), 560.0_f32.min(maximum.y)])
            .show(ctx, |ui| {
                ui.style_mut().spacing.item_spacing = egui::vec2(7.0, 5.0);
                ui.style_mut().spacing.button_padding = egui::vec2(8.0, 4.0);
                ui.style_mut().spacing.scroll.floating = false;
                ui.add(egui::TextEdit::singleline(&mut self.search).char_limit(120).hint_text(tr("Szukaj ustawienia…", "Search preferences…")));
                ui.separator();
                let sections_width = (ui.available_width() * 0.28).clamp(120.0, 190.0);
                let options_width = (ui.available_width() - sections_width - 28.0).max(120.0);
                let column_height = (maximum.y - 215.0).clamp(80.0, 350.0);
                let search = self.search.trim().to_lowercase();
                ui.horizontal_top(|ui| {
                    egui::ScrollArea::vertical().id_salt("all-preferences-sections").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible).max_height(column_height).show(ui, |ui| {
                        ui.vertical(|ui| {
                        ui.set_width(sections_width);
                        for (index, section) in SETTINGS_SECTIONS.iter().enumerate() {
                            if search.is_empty() || section.text(english).to_lowercase().contains(&search) || section.options.iter().any(|option| option.text(english).to_lowercase().contains(&search)) {
                                ui.selectable_value(&mut self.section, index, section.text(english));
                            }
                        }
                        });
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().id_salt("all-preferences-options").max_height(column_height).show(ui, |ui| {
                        ui.vertical(|ui| {
                        ui.set_width(options_width);
                        let section = &SETTINGS_SECTIONS[self.section];
                        ui.heading(section.text(english));
                        let mut matching = 0;
                        for option in section.options {
                            if !search.is_empty() && !section.text(english).to_lowercase().contains(&search) && !option.text(english).to_lowercase().contains(&search) { continue; }
                            matching += 1;
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
                        if section.id == "action_hotkeys" && search.is_empty() { action_shortcuts(ui, draft, english); }
                        });
                    });
                });
                ui.separator();
                ui.small(tr("Opcje bez obsługi zapisują wybór do przyszłego użycia; nie zmieniają jeszcze gry.", "Preferences awaiting support save your choice for future use; they do not change gameplay yet."));
                if let Some(message) = message { ui.label(message); }
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr("Domyślne", "Defaults")).clicked() { action = PreferenceAction::Defaults; }
                    if ui.button(tr("Połączenie i konto", "Connection and account")).clicked() { action = PreferenceAction::QuickSettings; }
                    if ui.button(tr("Anuluj", "Cancel")).clicked() { action = PreferenceAction::Cancel; }
                    if ui.button(tr("Zastosuj i zapisz", "Apply and save")).clicked() { action = PreferenceAction::Apply; }
                });
            });
        action
    }
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
        _ => {
            ui.label(label);
        }
    }
}

fn action_shortcuts(ui: &mut egui::Ui, draft: &mut ClientSettings, english: bool) {
    ui.separator();
    ui.checkbox(
        &mut draft.action_bar.visible,
        if english {
            "Show action bar"
        } else {
            "Pokaż pasek akcji"
        },
    );
    ui.checkbox(
        &mut draft.action_bar.locked,
        if english {
            "Lock assignments"
        } else {
            "Zablokuj przypisania"
        },
    );
    for (index, shortcut) in draft.action_bar.shortcuts.iter_mut().enumerate() {
        egui::ComboBox::from_id_salt(("complete-action", index))
            .selected_text(format!(
                "{}: {}",
                index + 1,
                shortcut.map_or_else(
                    || if english {
                        "None".into()
                    } else {
                        "Brak".into()
                    },
                    |s| shortcut_label(s.key)
                )
            ))
            .show_ui(ui, |ui| {
                ui.selectable_value(shortcut, None, if english { "None" } else { "Brak" });
                for key in (4..=39).chain(58..=66).chain(68..=69) {
                    ui.selectable_value(
                        shortcut,
                        Some(oteryn_client::action_bar::SlotShortcut { key, modifiers: 0 }),
                        shortcut_label(key),
                    );
                }
            });
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
        79 => "→".into(),
        80 => "←".into(),
        81 => "↓".into(),
        82 => "↑".into(),
        _ => "?".into(),
    }
}
