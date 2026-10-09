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
        let bounds = ctx.content_rect().shrink(4.0);
        let maximum = bounds.size().max(egui::Vec2::splat(1.0));
        egui::Window::new(if maximum.x < 520.0 { tr("Ustawienia", "Preferences") } else { tr("Wszystkie ustawienia", "All preferences") })
            .id("complete-preferences".into()).open(&mut self.open)
            .collapsible(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .constrain_to(bounds).min_size(egui::Vec2::ZERO).max_size(maximum)
            .default_size([820.0_f32.min(maximum.x), 560.0_f32.min(maximum.y)])
            .show(ctx, |ui| {
                ui.style_mut().spacing.item_spacing = egui::vec2(7.0, 5.0);
                ui.style_mut().spacing.button_padding = egui::vec2(8.0, 4.0);
                ui.style_mut().spacing.scroll.floating = false;
                let (footer_action, body_rect) = primary_footer(ui, "all-preferences-footer", english);
                action = footer_action;
                let body_size = body_rect.size().max(egui::Vec2::splat(1.0));
                let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
                body_ui.set_clip_rect(ui.clip_rect().intersect(body_rect));
                egui::ScrollArea::both().id_salt(("all-preferences-body", SETTINGS_SECTIONS[self.section].id))
                    .auto_shrink([false, false]).max_width(body_size.x).max_height(body_size.y).show(&mut body_ui, |ui| {
                let body_width = body_size.x.max(1.0);
                ui.set_width(body_width);
                let compact = body_width < 520.0;
                ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(body_width).char_limit(120).hint_text(tr("Szukaj ustawienia…", "Search preferences…")));
                ui.separator();
                let sections_width = (body_width * 0.28).min(190.0);
                let options_width = if compact { body_width } else { (body_width - sections_width - 28.0).max(1.0) };
                let column_height = (maximum.y - 140.0).max(1.0);
                let search = self.search.trim().to_lowercase();
                if compact {
                    egui::ComboBox::from_id_salt("all-preferences-category").width(body_width).truncate()
                        .selected_text(SETTINGS_SECTIONS[self.section].text(english)).show_ui(ui, |ui| {
                            for (index, section) in SETTINGS_SECTIONS.iter().enumerate() {
                                if search.is_empty() || section.text(english).to_lowercase().contains(&search) || section.options.iter().any(|option| option.text(english).to_lowercase().contains(&search)) {
                                    ui.selectable_value(&mut self.section, index, section.text(english));
                                }
                            }
                        });
                }
                ui.horizontal_top(|ui| {
                    if !compact {
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
                    }
                    egui::ScrollArea::vertical().id_salt(("all-preferences-options", SETTINGS_SECTIONS[self.section].id)).max_height(if compact { f32::INFINITY } else { column_height }).show(ui, |ui| {
                        ui.vertical(|ui| {
                        ui.set_width(options_width);
                        let section = &SETTINGS_SECTIONS[self.section];
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
                        if matches!(section.id, "action_hotkeys" | "action_bars") && (matching > 0 || search.is_empty()) {
                            ui.separator();
                            ui.heading(tr("Dziewięć pasków i ich skróty", "Nine action rows and their shortcuts"));
                            crate::action_bar_ui::preferences_editor(ui, &mut draft.action_bar, english);
                        }
                        });
                    });
                });
                ui.separator();
                ui.small(tr("Opcje bez obsługi zapisują wybór do przyszłego użycia; nie zmieniają jeszcze gry.", "Preferences awaiting support save your choice for future use; they do not change gameplay yet."));
                if let Some(message) = message { ui.label(message); }
                ui.horizontal_wrapped(|ui| {
                    if ui.button(tr("Domyślne", "Defaults")).clicked() { action = PreferenceAction::Defaults; }
                    if ui.button(tr("Połączenie i konto", "Connection and account")).clicked() { action = PreferenceAction::QuickSettings; }
                });
                });
            });
        action
    }
}

/// Reserve recovery controls before allocating scrollable content, including long errors.
pub(crate) fn primary_footer(
    ui: &mut egui::Ui,
    id: &str,
    english: bool,
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
                let apply = match (english, compact) {
                    (true, true) => "Apply",
                    (false, true) => "Zapisz",
                    (true, false) => "Apply and save",
                    (false, false) => "Zastosuj i zapisz",
                };
                let apply_response = ui.button(apply).on_hover_text(if english {
                    "Apply and save preferences"
                } else {
                    "Zastosuj i zapisz ustawienia"
                });
                if apply_response.clicked() {
                    action = PreferenceAction::Apply;
                }
                if ui
                    .button(if english { "Cancel" } else { "Anuluj" })
                    .clicked()
                {
                    action = PreferenceAction::Cancel;
                }
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
        79 => "→".into(),
        80 => "←".into(),
        81 => "↓".into(),
        82 => "↑".into(),
        _ => "?".into(),
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use egui::{Context, Event, FullOutput, Pos2, Rect, Vec2, epaint::Shape};

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
    use test_support::{button_position, click, frame};

    #[test]
    fn category_navigation_does_not_inherit_other_options_scroll() -> Result<(), &'static str> {
        for english in [false, true] {
            let ctx = egui::Context::default();
            let size = egui::vec2(900.0, 620.0);
            let mut browser = PreferencesBrowser::new();
            let mut draft = ClientSettings {
                english,
                ..Default::default()
            };
            let effects = SETTINGS_SECTIONS
                .iter()
                .find(|section| section.id == "effects")
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
            let first_effect = effects.options[0].text(english);
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
            for option in &action_bars.options[..3] {
                assert!(
                    button_position(&output, size, option.text(english)).is_some(),
                    "new category hides its first row-count control: {}",
                    option.id
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
                let apply_label = match (english, size.x < 560.0) {
                    (true, true) => "Apply",
                    (false, true) => "Zapisz",
                    (true, false) => "Apply and save",
                    (false, false) => "Zastosuj i zapisz",
                };
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
