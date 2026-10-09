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
}

impl PreferencesBrowser {
    pub fn show(&mut self, ctx: &egui::Context, draft: &mut ClientSettings) {
        if !self.open {
            return;
        }
        let english = draft.english;
        let tr = |pl, en| if english { en } else { pl };
        egui::Window::new(tr("Wszystkie ustawienia", "All preferences"))
            .id("complete-preferences".into()).open(&mut self.open).default_size([820.0, 560.0])
            .show(ctx, |ui| {
                ui.horizontal_top(|ui| {
                    egui::ScrollArea::vertical().id_salt("all-preferences-sections").max_height(440.0).show(ui, |ui| {
                        ui.set_width(190.0);
                        for (index, section) in SETTINGS_SECTIONS.iter().enumerate() { ui.selectable_value(&mut self.section, index, section.text(english)); }
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().id_salt("all-preferences-options").max_height(440.0).show(ui, |ui| {
                        ui.set_min_width(430.0);
                        let section = &SETTINGS_SECTIONS[self.section];
                        ui.heading(section.text(english));
                        for option in section.options {
                            match option.implementation {
                                Implementation::Implemented{field} => implemented(ui, draft, option, field, english),
                                Implementation::Pending{..} => {
                                    let key = format!("{}.{}", section.id, option.id);
                                    ui.group(|ui| { ui.label(option.text(english)); pending(ui, draft, option, key, english); ui.small(tr("Ustawienie przygotowane; obsługa funkcji jest w trakcie prac.", "Preference prepared; this feature is still being implemented.")); });
                                }
                            }
                        }
                    });
                });
                ui.separator();
                ui.label(tr("Zapisz zmiany przyciskiem Zastosuj i zapisz w oknie ustawień.", "Use Apply and save in the settings window to save changes."));
            });
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
            ui.small(if english {
                "Configure each direction in Controls."
            } else {
                "Skonfiguruj kierunki na stronie Sterowanie."
            });
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
        79 => "→".into(),
        80 => "←".into(),
        81 => "↓".into(),
        82 => "↑".into(),
        _ => "?".into(),
    }
}
