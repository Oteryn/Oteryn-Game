//! Complete preference navigation with explicit future-consumer configuration.
#[path = "preferences_overview.rs"]
mod overview;
#[path = "preferences_pages.rs"]
mod pages;
#[path = "preferences_slider.rs"]
mod reference_slider;

use oteryn_client::{
    action_bar::ACTION_BAR_ROWS,
    settings::ClientSettings,
    settings_catalog::{
        FutureValue, Implementation, OptionKind, SETTINGS_SECTIONS, SettingOption, ShortcutBinding,
        reference_settings_sections,
    },
};

#[derive(Default)]
pub struct PreferencesBrowser {
    pub open: bool,
    section: usize,
    overview: bool,
    search: String,
    page_state: pages::PageState,
    action_bar_rows_available: [bool; ACTION_BAR_ROWS],
    compendium_available: bool,
    clear_action_row: Option<usize>,
    expanded: std::collections::BTreeSet<&'static str>,
    navigation_section: Option<usize>,
    /// A direct route such as the Custom Hotkeys action may reveal an advanced
    /// page without silently changing the user's saved discovery preference.
    forced_advanced: bool,
}

#[derive(Default)]
pub enum PreferenceAction {
    #[default]
    None,
    Ok,
    Apply,
    Cancel,
    Defaults,
    OpenCompendium,
    ExportOptions,
    ImportOptions,
    ResetOptions,
    OpenScreenshotFolder,
}

impl PreferencesBrowser {
    pub(crate) fn has_inner_dialog(&self) -> bool {
        self.open && self.page_state.has_inner_dialog()
    }

    /// Reset only locally implemented controls owned by the displayed page.
    /// Reference factory values and profile-table reset semantics remain unverified.
    pub(crate) fn reset_current_page(&self, draft: &mut ClientSettings) {
        let section = reference_settings_sections()[self.section].id;
        if !page_reset_available(section) || self.overview {
            return;
        }
        let defaults = ClientSettings::default();
        let basic_keys = [
            "controls.controls.mouse_preset",
            "gameplay.gameplay.inspect_permission",
            "gameplay.gameplay.cancel_chase",
            "gameplay.gameplay.nearby_corpses",
            "hud.hud.resource_bars",
            "hud.hud.arcs",
            "interface.interface.colourise_loot_value",
            "graphics.graphics.antialiasing",
            "sound.sound.master",
        ];
        let prefix = format!("{section}.");
        draft.future_preferences.retain(|key, _| {
            let owned = if section == "basic" {
                basic_keys.contains(&key.as_str())
            } else {
                (key.starts_with(&prefix) && !(section == "hud" && key == "hud.hud.cooldown_bar"))
                    || (section == "interface" && key == "hud.hud.cooldown_bar")
            };
            // An unavailable control has no qualified default or consumer. Preserve its intent.
            !owned || oteryn_client::settings_catalog::runtime_consumer(key).is_none()
        });
        if matches!(section, "basic" | "action_bars") {
            draft.action_bar.edge_enabled = defaults.action_bar.edge_enabled;
            draft.action_bar.visible = defaults.action_bar.visible;
            for (row, default_row) in draft
                .action_bar
                .extra_rows
                .iter_mut()
                .zip(defaults.action_bar.extra_rows)
            {
                row.visible = default_row.visible;
            }
            let action_bar = draft.action_bar.clone();
            if let Some(profile) = draft.hotkeys.active_mut() {
                profile.action_bar = action_bar;
            }
        }
        if matches!(section, "basic" | "graphics") {
            draft.fullscreen = defaults.fullscreen;
        }
        if section == "graphics" {
            draft.vsync = defaults.vsync;
            draft.fps = defaults.fps;
            draft.last_frame_limit = defaults.last_frame_limit;
        }
        if section == "shortcuts" {
            draft.panel_shortcuts = defaults.panel_shortcuts;
        }
    }

    pub(crate) fn clear_transient_route(&mut self) {
        self.forced_advanced = false;
        self.navigation_section = None;
    }

    pub fn open_section(&mut self, id: &str) -> bool {
        let Some(section) = reference_settings_sections()
            .iter()
            .position(|entry| entry.id == id)
        else {
            return false;
        };
        self.section = section;
        self.overview = false;
        self.forced_advanced = !category_visible(id, false);
        self.navigation_section = None;
        self.open = true;
        true
    }

    pub fn set_action_bar_rows_available(&mut self, available: [bool; ACTION_BAR_ROWS]) {
        self.action_bar_rows_available = available;
        if self
            .clear_action_row
            .is_some_and(|row| !available.get(row).copied().unwrap_or(false))
        {
            self.clear_action_row = None;
        }
    }
    pub fn set_compendium_available(&mut self, available: bool) {
        self.compendium_available = available;
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
        if ctx.memory(|memory| memory.focused().is_none())
            && !egui::Popup::is_any_open(ctx)
            && !self.page_state.has_inner_dialog()
        {
            // egui may focus/activate the first button on an unfocused Enter.
            // Options explicitly has no such default accept action.
            ctx.input_mut(|input| {
                input.consume_key(input.modifiers, egui::Key::Enter);
            });
        }
        let escape = ctx.input(|input| {
            if !input.focused {
                return None;
            }
            input.events.iter().find_map(|event| match event {
                egui::Event::Key {
                    key: egui::Key::Escape,
                    pressed: true,
                    modifiers,
                    ..
                } => Some(*modifiers),
                _ => None,
            })
        });
        // A popup/key-capture/confirmation owns Escape before the outer dialog.
        // The original Options dialog has no implicit Return-to-save handler.
        if let Some(modifiers) = escape
            && !egui::Popup::is_any_open(ctx)
            && !self.page_state.handle_escape()
        {
            ctx.input_mut(|input| input.consume_key(modifiers, egui::Key::Escape));
            return PreferenceAction::Cancel;
        }
        reference_slider::suppress_wheel_tail(ctx);
        let previous_style = ctx.style_of(egui::Theme::Dark);
        crate::client_chrome::install_preferences(ctx, draft.high_contrast);
        let mut action = PreferenceAction::None;
        let english = draft.english;
        let tr = |pl, en| if english { en } else { pl };
        let mut advanced =
            draft.stored_bool("basic.basic.advanced").unwrap_or(false) || self.forced_advanced;
        self.normalize_navigation(advanced);
        let bounds = ctx.content_rect().shrink(12.0);
        let maximum = bounds.size().max(egui::Vec2::splat(1.0));
        let size = egui::vec2(1180.0_f32.min(maximum.x), 880.0_f32.min(maximum.y));
        let style = ctx.style_of(egui::Theme::Dark);
        egui::Window::new(tr("Opcje", "Options"))
            .id("complete-preferences".into())
            .title_bar(false)
            .collapsible(false).resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .constrain_to(bounds).min_size(egui::Vec2::ZERO).max_size(maximum)
            .fixed_size(size)
            .frame(egui::Frame::new().fill(style.visuals.window_fill)
                .stroke(style.visuals.window_stroke).corner_radius(7.0))
            .show(ctx, |ui| {
                ui.set_min_size(size);
                ui.style_mut().spacing.scroll.floating = false;
                egui::Panel::top("preferences-options-title")
                    .exact_size(20.0)
                    .show(ui, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(tr("Opcje", "Options"));
                        });
                    });
                let mut advanced_setting = advanced;
                let (footer_action, body_rect) = primary_footer_impl(
                    ui,
                    "all-preferences-footer",
                    english,
                    Some(&mut advanced_setting),
                );
                if advanced_setting != advanced {
                    self.forced_advanced = false;
                    advanced = advanced_setting;
                    draft.future_preferences.insert(
                        "basic.basic.advanced".into(),
                        FutureValue::Bool(advanced),
                    );
                    self.section = section_index(if advanced { "controls" } else { "basic" });
                    self.navigation_section = None;
                    self.normalize_navigation(advanced);
                }
                action = footer_action;
                let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
                body_ui.set_clip_rect(body_rect.intersect(ui.clip_rect()));
                let nav_width = if size.x >= 1100.0 {222.0} else if size.x >= 700.0 {190.0} else {135.0};
                if size.x>=650.0 {egui::Panel::left("preferences-navigation").exact_size(nav_width).resizable(false)
                    .frame(egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(15,19,22,110)).inner_margin(12))
                    .show(&mut body_ui, |ui| {
                        ui.spacing_mut().item_spacing.y=2.0;
                        ui.add(egui::TextEdit::singleline(&mut self.search).desired_width(ui.available_width()).char_limit(120).hint_text(tr("Szukaj ustawienia…", "Search preferences…")));
                        ui.add_space(8.0);
                        egui::ScrollArea::vertical().id_salt("all-preferences-sections").auto_shrink([false,false]).show(ui, |ui| {
                            let search = self.search.trim().to_lowercase();
                            for (index, section) in reference_settings_sections().iter().enumerate() {
                                // The catalogue also retains compatibility-only options that are
                                // deliberately absent from the reference composition. Indexing
                                // those rows would reveal a page for a control the page cannot show.
                                // Until page compositions expose their own search terms, search
                                // only the exact visible page titles.
                                if !category_visible(section.id, advanced)
                                    || (!search.is_empty()
                                        && !section.text(english).to_lowercase().contains(&search))
                                    || (advanced
                                        && search.is_empty()
                                        && navigation_parent(section.id).is_some_and(|parent| {
                                            !self.expanded.contains(parent)
                                        }))
                                {
                                    continue;
                                }
                                if let Some(group) = match section.id {
                                    "basic" => Some(tr("NA SKRÓTY", "QUICK ACCESS")),
                                    "controls" => Some(tr("STEROWANIE", "CONTROLS")),
                                    "interface" => Some(tr("INTERFEJS", "INTERFACE")),
                                    "graphics" => Some(tr("OBRAZ I DŹWIĘK", "VIDEO AND AUDIO")),
                                    "miscellaneous" => Some(tr("POZOSTAŁE", "OTHER")),
                                    _ => None,
                                } {
                                    ui.add_space(16.0);
                                    ui.label(egui::RichText::new(group).size(10.0).color(egui::Color32::from_rgb(183,169,141)));
                                    ui.add_space(3.0);
                                }
                                let selected = !self.overview && self.section==index;
                                let label = navigation_label(
                                    section.id,
                                    section.text(english),
                                    english,
                                    advanced,
                                );
                                ui.horizontal(|ui| {
                                    if advanced
                                        && matches!(
                                            section.id,
                                            "controls"
                                                | "interface"
                                                | "graphics"
                                                | "sound"
                                                | "miscellaneous"
                                        )
                                    {
                                        let expanded = self.expanded.contains(section.id);
                                        if navigation_arrow(ui, section.id, expanded, english)
                                            .clicked()
                                        {
                                            self.toggle_navigation_group(section.id);
                                        }
                                    } else if advanced && navigation_parent(section.id).is_some() {
                                        ui.add_space(14.0);
                                    }
                                    let response = ui.add_sized([ui.available_width(),30.0],egui::Button::new("").selected(selected).frame(selected));
                                    ui.painter().text(response.rect.left_center()+egui::vec2(10.0,0.0),egui::Align2::LEFT_CENTER,label,egui::FontId::proportional(13.0),if selected {egui::Color32::from_rgb(240,211,148)} else {ui.visuals().text_color()});
                                    if selected {ui.painter().rect_filled(egui::Rect::from_min_size(response.rect.min,egui::vec2(3.0,response.rect.height())),1.0,ui.visuals().selection.stroke.color);}
                                    if response.clicked() {
                                        self.select_navigation(index);
                                    }
                                });
                            }

                        });
                    });
                } else {
                    egui::Panel::top("preferences-compact-navigation").show(&mut body_ui, |ui| {
                        let selected = &reference_settings_sections()[self.section];
                        egui::ComboBox::from_id_salt("preferences-category").selected_text(navigation_label(selected.id, selected.text(english), english, advanced)).show_ui(ui, |ui| {
                            for (index,section) in reference_settings_sections().iter().enumerate().filter(|(_, section)| category_visible(section.id, advanced)) {
                                let label = navigation_label(section.id, section.text(english), english, advanced);
                                if ui.selectable_label(self.section == index, label).clicked() {
                                    self.select_navigation(index);
                                }
                            }
                        });
                    });
                }
                egui::CentralPanel::default().frame(egui::Frame::new().inner_margin(if size.x<700.0 {12} else {22}))
                    .show(&mut body_ui, |ui| {
                        let available = ui.available_rect_before_wrap();
                        let reset_height = (ui.spacing().interact_size.y + 8.0)
                            .min(available.height().max(0.0));
                        let reset_top = (available.bottom() - reset_height).max(available.top());
                        let content_rect = egui::Rect::from_min_max(
                            available.min,
                            egui::pos2(
                                available.right(),
                                (reset_top - ui.spacing().item_spacing.y).max(available.top()),
                            ),
                        );
                        let reset_rect = egui::Rect::from_min_max(
                            egui::pos2(available.left(), reset_top),
                            available.max,
                        );
                        let mut reset_ui = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(reset_rect)
                                .layout(egui::Layout::right_to_left(egui::Align::Center)),
                        );
                        let can_reset = !self.overview && page_reset_available(reference_settings_sections()[self.section].id);
                        if reset_ui.add_enabled_ui(can_reset, |ui| reset_button(ui, english)).inner.clicked() {
                            action = PreferenceAction::Defaults;
                        }
                        let mut page_ui =
                            ui.new_child(egui::UiBuilder::new().max_rect(content_rect));
                        page_ui.set_clip_rect(content_rect.intersect(ui.clip_rect()));
                        let search = self.search.trim().to_lowercase();
                        let mut body_content_clip = egui::Rect::NOTHING;
                        let _body_output = egui::ScrollArea::vertical().id_salt(("all-preferences-options", reference_settings_sections()[self.section].id)).auto_shrink([false,false]).show(&mut page_ui, |ui| {
                            body_content_clip = ui.clip_rect();
                            ui.set_width(ui.available_width());
                        let section = &reference_settings_sections()[self.section];
                        if self.overview { overview::show(ui, draft, english); }
                        else if section.id == "basic" { basic_page(ui, draft, english); }
                        else if section.id == "action_bars" { action_bars_page(ui, draft, english, self.action_bar_rows_available, &mut self.clear_action_row); }
                        else if pages::show(ui, draft, section.id, english, self.compendium_available, &mut self.page_state) {
                            if let Some(help_action) = self.page_state.take_help_action() {
                                action = match help_action {
                                    pages::HelpAction::OpenCompendium => {
                                        PreferenceAction::OpenCompendium
                                    }
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
                        crate::settings_audit::record_settings_scroll(
                            ctx,
                            reference_settings_sections()[self.section].id,
                            body_content_clip,
                            &_body_output,
                        );
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

    fn normalize_navigation(&mut self, advanced: bool) {
        let current = reference_settings_sections()[self.section].id;
        if advanced && current == "basic" {
            self.section = section_index("controls");
        } else if !advanced && !category_visible(current, false) {
            self.section = section_index("basic");
        }
        if self.navigation_section == Some(self.section) {
            return;
        }
        let id = reference_settings_sections()[self.section].id;
        let group = if let Some(parent) = navigation_parent(id) {
            Some(parent)
        } else if matches!(
            id,
            "controls" | "interface" | "graphics" | "sound" | "miscellaneous"
        ) {
            Some(id)
        } else {
            None
        };
        if let Some(group) = group {
            self.expanded.clear();
            self.expanded.insert(group);
        }
        self.navigation_section = Some(self.section);
    }

    fn toggle_navigation_group(&mut self, group: &'static str) {
        if !self.expanded.remove(group) {
            self.expanded.clear();
            self.expanded.insert(group);
        }
    }

    fn select_navigation(&mut self, section: usize) {
        self.section = section;
        self.overview = false;
        self.navigation_section = None;
        if category_visible(reference_settings_sections()[section].id, false) {
            // A direct route may temporarily reveal the advanced tree. Choosing one
            // of the four basic destinations exits that transient view without
            // changing the saved discovery preference.
            self.forced_advanced = false;
        }
    }
}

fn section_index(id: &str) -> usize {
    reference_settings_sections()
        .iter()
        .position(|section| section.id == id)
        .unwrap_or(0)
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
    if advanced {
        id != "basic"
    } else {
        matches!(id, "basic" | "general_hotkeys" | "shortcuts" | "help")
    }
}

fn navigation_label<'a>(id: &str, fallback: &'a str, english: bool, advanced: bool) -> &'a str {
    if !advanced {
        match (id, english) {
            ("basic", true) => "Options",
            ("basic", false) => "Opcje",
            ("general_hotkeys", true) => "Hotkeys",
            ("general_hotkeys", false) => "Skróty",
            _ => fallback,
        }
    } else if id == "hud" {
        "HUD"
    } else if id == "miscellaneous" {
        "Misc."
    } else {
        fallback
    }
}

fn navigation_arrow(
    ui: &mut egui::Ui,
    _group: &'static str,
    expanded: bool,
    english: bool,
) -> egui::Response {
    let label = match (expanded, english) {
        (true, true) => "Collapse category",
        (false, true) => "Expand category",
        (true, false) => "Zwiń kategorię",
        (false, false) => "Rozwiń kategorię",
    };
    let response = ui
        .add_sized([12.0, 16.0], egui::Button::new("").frame(false))
        .on_hover_text(label);
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(
            egui::Id::new(("preferences-navigation-arrow", _group)),
            (response.rect, ui.clip_rect()),
        );
    });
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
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
                .inner_margin(egui::Margin::symmetric(12, 4))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    body(ui);
                });
        });
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
            ["Tryb pełnoekranowy", "Full Screen Mode"],
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
    let text = selected.map_or_else(|| "—".to_string(), |value| format!("{value} %"));
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
    available_rows: [bool; ACTION_BAR_ROWS],
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
        ui.horizontal(|ui| {
            ui.label(label);
            for index in 1..=3 {
                let row = edge * 3 + index - 1;
                let available = available_rows[row];
                let response = ui
                    .add_enabled(
                        available,
                        egui::Button::new(format!(
                            "{} {index}",
                            if english { "Bar" } else { "Pasek" }
                        )),
                    )
                    .on_hover_text(clear_action_row_tooltip(english, available));
                #[cfg(test)]
                ui.ctx().data_mut(|data| {
                    data.insert_temp(
                        egui::Id::new(("clear-action-row-enabled", row)),
                        (response.rect, response.enabled()),
                    );
                });
                if response.clicked() {
                    *clear_row = Some(row);
                }
            }
        });
    }
}

const fn clear_action_row_tooltip(english: bool, available: bool) -> &'static str {
    match (english, available) {
        (true, true) => {
            "Immediately clear this session’s assigned actions. Saved shortcuts and locks stay intact; Cancel does not restore assignments."
        }
        (false, true) => {
            "Natychmiast usuń czynności tej sesji. Skróty i blokady pozostają; Anuluj nie przywraca czynności."
        }
        (true, false) => "Available when this row contains a session assignment.",
        (false, false) => "Dostępne, gdy ten rząd zawiera przypisaną czynność sesji.",
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

fn reset_button(ui: &mut egui::Ui, english: bool) -> egui::Response {
    ui.button("Reset").on_hover_text(if english {
        "Restore Oteryn defaults for implemented controls on this page only"
    } else {
        "Przywróć domyślne wartości Oteryn tylko dla obsługiwanych kontrolek tej strony"
    })
}

fn page_reset_available(section: &str) -> bool {
    if matches!(
        section,
        "general_hotkeys" | "action_hotkeys" | "custom_hotkeys" | "help"
    ) {
        return false;
    }
    matches!(section, "basic" | "graphics" | "action_bars" | "shortcuts")
        || SETTINGS_SECTIONS
            .iter()
            .find(|entry| entry.id == section)
            .is_some_and(|entry| {
                entry.options.iter().any(|option| {
                    oteryn_client::settings_catalog::runtime_consumer(&format!(
                        "{section}.{}",
                        option.id
                    ))
                    .is_some()
                })
            })
}

fn primary_footer_impl(
    ui: &mut egui::Ui,
    id: &str,
    english: bool,
    mut advanced: Option<&mut bool>,
) -> (PreferenceAction, egui::Rect) {
    let mut action = PreferenceAction::None;
    let reset_in_page_body = advanced.is_some();
    let available = ui.available_rect_before_wrap();
    let compact_footer = available.width() < 520.0;
    let advanced_row = if compact_footer && advanced.is_some() {
        ui.spacing().interact_size.y + ui.spacing().item_spacing.y + 2.0
    } else {
        0.0
    };
    let height =
        ui.spacing().interact_size.y + if compact_footer { 14.0 } else { 24.0 } + advanced_row;
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
            if compact_footer {
                ui.spacing_mut().item_spacing.y = 0.0;
            }
            if available.width() < 300.0 {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.style_mut()
                    .text_styles
                    .insert(egui::TextStyle::Button, egui::FontId::proportional(8.0));
            }
            if compact_footer && let Some(advanced) = advanced.as_mut() {
                ui.add_sized(
                    [ui.available_width(), ui.spacing().interact_size.y],
                    egui::Checkbox::new(
                        advanced,
                        if english {
                            "Show Advanced Options"
                        } else {
                            "Pokaż opcje zaawansowane"
                        },
                    ),
                );
            }
            ui.horizontal(|ui| {
                let compact = ui.available_width() < 520.0;
                if !reset_in_page_body && reset_button(ui, english).clicked() {
                    action = PreferenceAction::Defaults;
                }
                if !compact && let Some(advanced) = advanced.as_mut() {
                    ui.checkbox(
                        advanced,
                        if english {
                            "Show Advanced Options"
                        } else {
                            "Pokaż opcje zaawansowane"
                        },
                    );
                }
                if compact {
                    ui.spacing_mut().button_padding.x = 1.0;
                    ui.spacing_mut().item_spacing.x = 1.0;
                }
                let apply = if english { "Apply" } else { "Zastosuj" };
                let action_width = if compact {
                    let row_width = ui.available_width().min(ui.clip_rect().width());
                    ((row_width - 2.0 * ui.spacing().item_spacing.x - 8.0) / 3.0).max(1.0)
                } else {
                    78.0
                };
                let group_width = 3.0 * action_width + 2.0 * ui.spacing().item_spacing.x;
                ui.add_space(
                    (ui.available_width() - group_width - ui.spacing().item_spacing.x).max(0.0),
                );
                // Keep visual and keyboard traversal order Ok / Apply / Cancel.
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    if ui
                        .add_sized(
                            [action_width, if compact { 18.0 } else { 32.0 }],
                            egui::Button::new(
                                egui::RichText::new("Ok")
                                    .color(egui::Color32::from_rgb(25, 24, 20))
                                    .strong(),
                            )
                            .truncate()
                            .fill(egui::Color32::from_rgb(207, 177, 119)),
                        )
                        .clicked()
                    {
                        action = PreferenceAction::Ok;
                    }
                    if ui
                        .add_sized(
                            [action_width, if compact { 18.0 } else { 32.0 }],
                            egui::Button::new(apply).truncate(),
                        )
                        .clicked()
                    {
                        action = PreferenceAction::Apply;
                    }
                    let cancel_response = ui.add_sized(
                        [action_width, if compact { 18.0 } else { 32.0 }],
                        egui::Button::new(if english { "Cancel" } else { "Anuluj" }).truncate(),
                    );
                    if cancel_response.clicked() {
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
                    for value in [0, 1, 30, 60, 90, 120, 144, 165, 200] {
                        let selected = draft.fps == value;
                        if ui.selectable_label(selected, value.to_string()).clicked() {
                            if value == 0 {
                                draft.set_frame_limit_enabled(false);
                            } else {
                                let _ = draft.set_frame_limit(value);
                                draft.set_frame_limit_enabled(true);
                            }
                        }
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
                    remember_movement_change(
                        ui.ctx(),
                        "complete-movement-conflict",
                        draft.set_movement_keys([82, 79, 81, 80]),
                    );
                }
                if ui.button("WASD").clicked() {
                    remember_movement_change(
                        ui.ctx(),
                        "complete-movement-conflict",
                        draft.set_movement_keys([26, 7, 22, 4]),
                    );
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
                let effective = draft.effective_movement_keys()[index];
                egui::ComboBox::from_id_salt(("complete-movement", index))
                    .selected_text(format!(
                        "{direction}: {}",
                        effective.map_or_else(
                            || if english {
                                "Unbound".into()
                            } else {
                                "Nieprzypisany".into()
                            },
                            shortcut_label,
                        )
                    ))
                    .show_ui(ui, |ui| {
                        for code in (4..=29).chain(79..=82) {
                            if ui
                                .selectable_label(effective == Some(code), shortcut_label(code))
                                .clicked()
                            {
                                remember_movement_change(
                                    ui.ctx(),
                                    "complete-movement-conflict",
                                    draft.set_movement_key(index, code),
                                );
                            }
                        }
                    });
            }
            movement_change_feedback(ui, "complete-movement-conflict", english);
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

pub(crate) fn remember_movement_change(
    ctx: &egui::Context,
    feedback_id: &'static str,
    result: std::io::Result<()>,
) {
    let feedback_id = egui::Id::new(feedback_id);
    ctx.data_mut(|data| {
        if result.is_err() {
            data.insert_temp(feedback_id, true);
        } else {
            data.remove::<bool>(feedback_id);
        }
    });
}

pub(crate) fn movement_change_feedback(
    ui: &mut egui::Ui,
    feedback_id: &'static str,
    english: bool,
) {
    if ui
        .ctx()
        .data(|data| data.get_temp::<bool>(egui::Id::new(feedback_id)))
        .unwrap_or(false)
    {
        ui.colored_label(
            egui::Color32::from_rgb(244, 112, 112),
            if english {
                "Shortcut conflict. Movement keys were not changed."
            } else {
                "Konflikt skrótów. Klawisze ruchu nie zostały zmienione."
            },
        );
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
            let mut chosen = match stored.as_ref() {
                Some(FutureValue::Choice(value)) => Some(value.clone()),
                _ => None,
            };
            let label = choices
                .iter()
                .find(|choice| chosen.as_deref() == Some(choice.id))
                .map_or("—", |choice| choice.text(english));
            let _choice_response = ui.add_enabled_ui(!choices.is_empty(), |ui| {
                egui::ComboBox::from_id_salt(&key)
                    .selected_text(label)
                    .show_ui(ui, |ui| {
                        for choice in choices {
                            ui.selectable_value(
                                &mut chosen,
                                Some(choice.id.into()),
                                choice.text(english),
                            );
                        }
                    });
            });
            #[cfg(test)]
            ui.ctx().data_mut(|data| {
                data.insert_temp(
                    egui::Id::new(("preference-choice", key.as_str())),
                    (
                        _choice_response.response.enabled(),
                        label.to_owned(),
                        choices
                            .iter()
                            .map(|choice| choice.text(english).to_owned())
                            .collect::<Vec<_>>(),
                    ),
                );
            });
            next = chosen.map(FutureValue::Choice);
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
    if !matches!(option.kind, OptionKind::Choice(_))
        && next.is_some()
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
        button_position_in_rect(output, Rect::from_min_size(Pos2::ZERO, size), label)
    }

    pub fn button_position_in_rect(
        output: &FullOutput,
        viewport: Rect,
        label: &str,
    ) -> Option<Pos2> {
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
        reveal_in_rect(
            ctx,
            size,
            Rect::from_min_size(Pos2::ZERO, size),
            label,
            pointer,
            &mut draw,
        )
    }

    pub fn reveal_in_rect<T>(
        ctx: &Context,
        size: Vec2,
        viewport: Rect,
        label: &str,
        pointer: Pos2,
        mut draw: impl FnMut(&Context) -> T,
    ) -> FullOutput {
        let mut output = frame(ctx, size, vec![], &mut draw).1;
        if button_position_in_rect(&output, viewport, label).is_some() {
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
            if button_position_in_rect(&output, viewport, label).is_some() {
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
    use test_support::{
        button_position, button_position_in_rect, click, frame, reveal, reveal_in_rect,
        scroll_geometry,
    };

    #[test]
    fn scoped_reset_keeps_bindings_locks_profiles_and_unavailable_intent() {
        let mut browser = PreferencesBrowser::new();
        let mut draft = ClientSettings::default();
        draft.action_bar.visible = false;
        draft.action_bar.edge_enabled = [false; 3];
        draft.action_bar.locked = true;
        draft.action_bar.shortcuts[0] = None;
        draft.action_bar.extra_rows[0].locked = false;
        draft.action_bar.extra_rows[0].visible = true;
        draft.future_preferences.extend([
            ("action_bars.bars.labels".into(), FutureValue::Bool(false)),
            ("sound.sound.master".into(), FutureValue::Int(65)),
            ("console.console.timestamps".into(), FutureValue::Bool(true)),
        ]);
        let before = draft.clone();
        assert!(browser.open_section("action_bars"));
        browser.reset_current_page(&mut draft);
        let mut expected = before.clone();
        let defaults = ClientSettings::default();
        expected.action_bar.visible = defaults.action_bar.visible;
        expected.action_bar.edge_enabled = defaults.action_bar.edge_enabled;
        for (row, default_row) in expected
            .action_bar
            .extra_rows
            .iter_mut()
            .zip(defaults.action_bar.extra_rows)
        {
            row.visible = default_row.visible;
        }
        expected
            .future_preferences
            .remove("action_bars.bars.labels");
        let action_bar = expected.action_bar.clone();
        if let Some(profile) = expected.hotkeys.active_mut() {
            profile.action_bar = action_bar;
        }
        assert_eq!(draft, expected);
        for section in [
            "general_hotkeys",
            "action_hotkeys",
            "custom_hotkeys",
            "help",
        ] {
            assert!(browser.open_section(section));
            browser.reset_current_page(&mut draft);
            assert_eq!(
                draft, expected,
                "unverified reset must not overwrite {section}"
            );
        }
        assert!(browser.open_section("basic"));
        browser.reset_current_page(&mut draft);
        assert_eq!(draft.stored_int("sound.sound.master"), Some(65));
        assert_eq!(draft.stored_bool("console.console.timestamps"), Some(true));
    }

    #[test]
    fn direct_section_navigation_accepts_only_the_twenty_reference_pages() {
        let size = egui::vec2(900.0, 620.0);
        for id in oteryn_client::settings_catalog::REFERENCE_SETTINGS_SECTION_IDS {
            let ctx = egui::Context::default();
            let mut browser = PreferencesBrowser::new();
            let mut draft = ClientSettings {
                english: true,
                ..Default::default()
            };
            assert!(browser.open_section(id), "direct route rejected {id}");
            let _ = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            });
            assert_eq!(
                reference_settings_sections()[browser.section].id,
                id,
                "direct route bounced after rendering"
            );
            assert_eq!(browser.forced_advanced, !category_visible(id, false));
            assert!(!browser.overview);
            assert!(browser.open);
            assert!(
                !draft
                    .future_preferences
                    .contains_key("basic.basic.advanced"),
                "direct route persisted discovery mode"
            );
        }

        let mut browser = PreferencesBrowser::new();
        assert!(browser.open_section("general_hotkeys"));
        assert!(!browser.open_section("not-a-settings-page"));
        for hidden in [
            "panels",
            "containers",
            "character_selection",
            "market",
            "floating_messages",
            "analyzers",
            "notifications",
        ] {
            assert!(!browser.open_section(hidden), "leaked section {hidden}");
        }
        assert_eq!(
            reference_settings_sections()[browser.section].id,
            "general_hotkeys"
        );
    }

    #[test]
    fn discovery_mode_exposes_the_exact_reference_navigation_sets() {
        let basic = reference_settings_sections()
            .iter()
            .filter(|section| category_visible(section.id, false))
            .map(|section| {
                (
                    section.id,
                    navigation_label(section.id, section.text(true), true, false),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            basic,
            vec![
                ("basic", "Options"),
                ("general_hotkeys", "Hotkeys"),
                ("shortcuts", "Shortcuts"),
                ("help", "Help"),
            ]
        );

        let advanced = reference_settings_sections()
            .iter()
            .filter(|section| category_visible(section.id, true))
            .map(|section| section.id)
            .collect::<Vec<_>>();
        assert_eq!(
            advanced,
            oteryn_client::settings_catalog::REFERENCE_SETTINGS_SECTION_IDS[1..]
        );
        assert_eq!(navigation_label("hud", "HUD", true, true), "HUD");
        assert_eq!(navigation_label("hud", "HUD", false, true), "HUD");
        assert_eq!(
            navigation_label("miscellaneous", "Miscellaneous", true, true),
            "Misc."
        );
    }

    #[test]
    fn basic_page_uses_the_reference_options_chrome_and_exact_labels() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(1440.0, 1000.0);
        let mut browser = PreferencesBrowser::new();
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        draft
            .future_preferences
            .insert("sound.sound.master".into(), FutureValue::Int(100));

        let mut output = egui::FullOutput::default();
        for _ in 0..4 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }

        for label in ["Options", "Full Screen Mode", "Master Volume: 100 %", "Ok"] {
            assert!(
                button_position(&output, size, label).is_some(),
                "reference Options label is missing: {label}"
            );
        }
        for legacy in [
            "OTERYN",
            "Settings",
            "Your world. Your way to play.",
            "CLIENT SETTINGS",
            "Fullscreen Mode",
            "Master Volume: 100%",
            "OK",
        ] {
            assert!(
                button_position(&output, size, legacy).is_none(),
                "non-reference Options chrome is still visible: {legacy}"
            );
        }
        let reset = button_position(&output, size, "Reset").ok_or("Reset button")?;
        let advanced =
            button_position(&output, size, "Show Advanced Options").ok_or("advanced checkbox")?;
        let ok = button_position(&output, size, "Ok").ok_or("Ok button")?;
        assert!(
            reset.y < ok.y,
            "Reset must stay in the page body above the footer"
        );
        assert!(
            advanced.x < ok.x && (advanced.y - ok.y).abs() < 12.0,
            "the advanced checkbox must be alone on the footer's left side"
        );
        Ok(())
    }

    #[test]
    fn transient_advanced_route_returns_to_basic_navigation_without_persisting()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(900.0, 620.0);
        let navigation = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(260.0, size.y));
        let mut browser = PreferencesBrowser::new();
        assert!(browser.open_section("custom_hotkeys"));
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };

        let mut output = egui::FullOutput::default();
        for _ in 0..4 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        assert!(browser.forced_advanced);
        assert_eq!(
            reference_settings_sections()[browser.section].id,
            "custom_hotkeys"
        );
        assert!(
            !draft
                .future_preferences
                .contains_key("basic.basic.advanced")
        );
        let basic_route = button_position_in_rect(&output, navigation, "General Hotkeys")
            .ok_or("basic-visible route missing from transient tree")?;
        let _ = click(&ctx, size, basic_route, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        for _ in 0..3 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }

        assert!(!browser.forced_advanced);
        assert_eq!(
            reference_settings_sections()[browser.section].id,
            "general_hotkeys"
        );
        assert!(button_position_in_rect(&output, navigation, "Hotkeys").is_some());
        assert!(button_position_in_rect(&output, navigation, "Controls").is_none());
        assert!(
            !draft
                .future_preferences
                .contains_key("basic.basic.advanced")
        );
        Ok(())
    }

    #[test]
    fn advanced_navigation_is_a_single_parent_accordion_and_routes_action_bars()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(1180.0, 880.0);
        let navigation = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(270.0, size.y));
        let mut browser = PreferencesBrowser::new();
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        draft
            .future_preferences
            .insert("basic.basic.advanced".into(), FutureValue::Bool(true));
        let arrow = |group: &'static str| {
            ctx.data(|data| {
                data.get_temp::<(egui::Rect, egui::Rect)>(egui::Id::new((
                    "preferences-navigation-arrow",
                    group,
                )))
            })
            .and_then(|(rect, clip)| clip.contains_rect(rect).then(|| rect.center()))
        };

        let mut output = egui::FullOutput::default();
        for _ in 0..4 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        assert_eq!(
            browser.expanded.iter().copied().collect::<Vec<_>>(),
            ["controls"]
        );
        assert!(button_position_in_rect(&output, navigation, "General Hotkeys").is_some());

        let controls_arrow = arrow("controls").ok_or("Controls collapse arrow")?;
        let _ = click(&ctx, size, controls_arrow, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        output = frame(&ctx, size, Vec::new(), |ctx| {
            browser.show(ctx, &mut draft, None)
        })
        .1;
        assert!(browser.expanded.is_empty());
        assert!(button_position_in_rect(&output, navigation, "General Hotkeys").is_none());

        let controls_arrow = arrow("controls").ok_or("Controls expand arrow")?;
        let _ = click(&ctx, size, controls_arrow, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        output = frame(&ctx, size, Vec::new(), |ctx| {
            browser.show(ctx, &mut draft, None)
        })
        .1;
        assert!(button_position_in_rect(&output, navigation, "General Hotkeys").is_some());

        let interface = button_position_in_rect(&output, navigation, "Interface")
            .ok_or("Interface navigation")?;
        let _ = click(&ctx, size, interface, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        for _ in 0..3 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        assert_eq!(
            browser.expanded.iter().copied().collect::<Vec<_>>(),
            ["interface"]
        );
        assert!(button_position_in_rect(&output, navigation, "HUD").is_some());
        assert!(button_position_in_rect(&output, navigation, "General Hotkeys").is_none());

        let action_bars = button_position_in_rect(&output, navigation, "Action Bars")
            .ok_or("Action Bars navigation")?;
        let _ = click(&ctx, size, action_bars, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        for _ in 0..3 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        assert_eq!(
            reference_settings_sections()[browser.section].id,
            "action_bars"
        );
        assert!(button_position(&output, size, "Show Bottom Action Bars:").is_some());

        let interface_arrow = arrow("interface").ok_or("Interface collapse arrow")?;
        let _ = click(&ctx, size, interface_arrow, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        output = frame(&ctx, size, Vec::new(), |ctx| {
            browser.show(ctx, &mut draft, None)
        })
        .1;
        assert!(browser.expanded.is_empty());
        assert!(button_position_in_rect(&output, navigation, "HUD").is_none());
        assert_eq!(
            reference_settings_sections()[browser.section].id,
            "action_bars"
        );
        Ok(())
    }

    #[test]
    fn failed_apply_path_keeps_a_direct_advanced_route_visible() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(900.0, 620.0);
        let mut browser = PreferencesBrowser::new();
        assert!(browser.open_section("custom_hotkeys"));
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let mut output = egui::FullOutput::default();
        for _ in 0..4 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        let ok = button_position(&output, size, "Ok").ok_or("Ok button")?;
        assert!(matches!(
            click(&ctx, size, ok, |ctx| browser.show(ctx, &mut draft, None)),
            PreferenceAction::Ok
        ));
        assert!(browser.forced_advanced);
        let _ = frame(&ctx, size, Vec::new(), |ctx| {
            browser.show(ctx, &mut draft, Some("Save failed"))
        });
        assert!(browser.forced_advanced);
        assert_eq!(
            reference_settings_sections()[browser.section].id,
            "custom_hotkeys"
        );
        assert_eq!(draft.stored_bool("basic.basic.advanced"), None);
        browser.clear_transient_route();
        assert!(!browser.forced_advanced);
        assert_eq!(browser.navigation_section, None);
        Ok(())
    }

    #[test]
    fn reference_navigation_hides_oteryn_only_destinations_and_shell_controls() {
        let ctx = egui::Context::default();
        let size = egui::vec2(1180.0, 880.0);
        let mut browser = PreferencesBrowser::new();
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let mut output = egui::FullOutput::default();
        for _ in 0..3 {
            output = frame(&ctx, size, vec![], |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        for forbidden in [
            "Quick settings",
            "Account/network",
            "Transparency",
            "Panels and Minimap",
            "Containers and Loot",
            "Character Selection",
            "Market",
            "Floating Messages",
            "Analyzers",
            "Notifications",
        ] {
            assert!(
                button_position(&output, size, forbidden).is_none(),
                "reference navigation leaked {forbidden}"
            );
        }
    }

    #[test]
    fn navigation_search_does_not_index_hidden_compatibility_controls() {
        let ctx = egui::Context::default();
        let size = egui::vec2(1180.0, 880.0);
        let mut browser = PreferencesBrowser::new();
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let navigation = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(260.0, size.y));
        for (query, leaked_page) in [
            ("Show Chat Panel", "Console"),
            ("Background FPS", "Graphics"),
            ("Mute in Background", "Sound"),
            ("English Interface", "Interface"),
        ] {
            browser.search = query.into();
            let mut output = egui::FullOutput::default();
            for _ in 0..3 {
                output = frame(&ctx, size, vec![], |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
            assert!(
                button_position_in_rect(&output, navigation, leaked_page).is_none(),
                "hidden query {query:?} leaked {leaked_page}"
            );
        }
    }

    #[test]
    fn choice_editors_expose_only_evidenced_entries_and_disable_unknown_dynamic_lists()
    -> Result<(), &'static str> {
        type ChoiceState = (bool, String, Vec<String>);

        let size = egui::vec2(900.0, 620.0);
        let ctx = egui::Context::default();
        ctx.set_theme(egui::Theme::Dark);
        crate::client_chrome::install(&ctx, false);
        let mut draft = ClientSettings::default();
        draft.future_preferences.insert(
            "game_window.window.target_marking".into(),
            FutureValue::Choice("frame_and_highlight".into()),
        );
        let mut draw = |ctx: &egui::Context| {
            egui::Window::new("Reference choices").show(ctx, |ui| {
                for (section, id, label) in [
                    ("controls", "controls.mouse_preset", "Mouse Preset:"),
                    (
                        "interface",
                        "interface.colourise_loot_value",
                        "Colourise Loot Value:",
                    ),
                    (
                        "game_window",
                        "window.target_marking",
                        "Mark Target Visually:",
                    ),
                    ("graphics", "graphics.antialiasing", "Antialiasing Mode:"),
                    ("graphics", "graphics.engine", "Graphics Engine:"),
                ] {
                    reference_option(ui, &mut draft, section, id, [label, label], true);
                }
            })
        };
        let mut output = egui::FullOutput::default();
        for _ in 0..4 {
            output = frame(&ctx, size, Vec::new(), &mut draw).1;
        }
        let choice_state = |key: &str| {
            ctx.data(|data| data.get_temp::<ChoiceState>(egui::Id::new(("preference-choice", key))))
        };
        for (key, enabled, label, choices) in [
            (
                "controls.controls.mouse_preset",
                false,
                "—",
                vec!["Classic Controls", "Regular Controls", "Left Smart-Click"],
            ),
            (
                "interface.interface.colourise_loot_value",
                false,
                "—",
                vec!["None", "Frames", "Corners"],
            ),
            (
                "game_window.window.target_marking",
                true,
                "Frame & Highlight",
                vec!["Frame & Highlight", "Frame Only", "Highlight Only", "None"],
            ),
            (
                "graphics.graphics.antialiasing",
                cfg!(windows),
                "—",
                vec!["None", "Antialiasing", "Smooth Retro"],
            ),
            ("graphics.graphics.engine", false, "—", vec![]),
        ] {
            let state = choice_state(key).ok_or("choice response")?;
            assert_eq!(state.0, enabled, "wrong enabled state for {key}");
            assert_eq!(state.1, label, "wrong selected label for {key}");
            assert_eq!(state.2, choices, "wrong menu entries for {key}");
        }
        assert!(button_position(&output, size, "Clear custom value").is_none());
        assert!(button_position(&output, size, "Unset").is_none());

        let target =
            button_position(&output, size, "Frame & Highlight").ok_or("target-marking choice")?;
        let _ = click(&ctx, size, target, &mut draw);
        output = frame(&ctx, size, Vec::new(), &mut draw).1;
        for label in ["Frame & Highlight", "Frame Only", "Highlight Only", "None"] {
            assert!(
                button_position(&output, size, label).is_some(),
                "missing target-marking choice {label}"
            );
        }
        assert!(button_position(&output, size, "Unset").is_none());
        assert!(button_position(&output, size, "Clear custom value").is_none());
        Ok(())
    }

    #[test]
    fn every_reference_settings_page_renders_in_the_compiled_client_at_native_size()
    -> Result<(), &'static str> {
        let size = egui::vec2(900.0, 620.0);
        for reference in crate::settings_audit::REFERENCE_SETTINGS_PAGES {
            let (section, marker) = (reference.id, reference.marker);
            let ctx = egui::Context::default();
            ctx.set_theme(egui::Theme::Dark);
            crate::client_chrome::install(&ctx, false);
            let mut browser = PreferencesBrowser::new();
            assert!(browser.open_section(section), "missing settings section");
            assert_eq!(
                browser.forced_advanced,
                !category_visible(section, false),
                "direct route chose the wrong navigation mode: {section}"
            );
            let mut draft = ClientSettings {
                english: true,
                ..Default::default()
            };
            let mut output = egui::FullOutput::default();
            for _ in 0..6 {
                output = frame(&ctx, size, Vec::new(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                })
                .1;
            }
            let (body, _) = scroll_geometry(&ctx, "all-preferences-body-geometry")
                .ok_or("missing settings body geometry")?;
            if button_position_in_rect(&output, body, marker).is_none() {
                output = reveal_in_rect(&ctx, size, body, marker, body.center(), |ctx| {
                    browser.show(ctx, &mut draft, None)
                });
            }
            assert!(
                button_position_in_rect(&output, body, marker).is_some(),
                "compiled page marker is not reachable: {section} / {marker}"
            );
            assert_eq!(
                reference_settings_sections()[browser.section].id,
                section,
                "direct route bounced to a different page"
            );
            assert!(
                !draft
                    .future_preferences
                    .contains_key("basic.basic.advanced"),
                "direct route persisted the transient advanced view"
            );
            for footer in ["Reset", "Ok", "Apply", "Cancel"] {
                assert!(
                    button_position(&output, size, footer).is_some(),
                    "compiled page footer is not reachable"
                );
            }
            let ok = button_position(&output, size, "Ok").ok_or("Ok footer")?;
            let apply = button_position(&output, size, "Apply").ok_or("Apply footer")?;
            let cancel = button_position(&output, size, "Cancel").ok_or("Cancel footer")?;
            assert!(
                ok.x < apply.x && apply.x < cancel.x,
                "reference footer order differs: {section}"
            );
        }
        Ok(())
    }

    #[test]
    fn effects_search_keeps_the_reference_composition_and_light_dependency()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        crate::client_chrome::install(&ctx, false);
        let size = egui::vec2(900.0, 620.0);
        let mut browser = PreferencesBrowser::new();
        browser.search = "ambient light".into();
        if !browser.open_section("effects") {
            return Err("effects page");
        }
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        for (key, value) in [
            ("effects.effects.ambient_light", FutureValue::Int(25)),
            ("effects.effects.own_opacity", FutureValue::Int(100)),
        ] {
            draft.future_preferences.insert(key.into(), value);
        }
        draft
            .future_preferences
            .insert("effects.effects.lighting".into(), FutureValue::Bool(false));

        let mut output = egui::FullOutput::default();
        for _ in 0..3 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        let ambient = ctx
            .data(|data| {
                data.get_temp::<(egui::Rect, egui::Rect, bool, bool)>(egui::Id::new((
                    "preference-percent",
                    "effects.effects.ambient_light",
                )))
            })
            .ok_or("ambient control")?;
        assert!(!ambient.2 && !ambient.3);
        assert!(button_position(&output, size, "Own Spell Effects: 100 %").is_some());
        assert!(button_position(&output, size, "Own spell opacity").is_none());
        Ok(())
    }

    #[test]
    fn screenshot_page_routes_the_real_folder_action() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(900.0, 620.0);
        let mut browser = PreferencesBrowser::new();
        if !browser.open_section("screenshots") {
            return Err("screenshots page");
        }
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
        draft
            .future_preferences
            .insert("basic.basic.advanced".into(), FutureValue::Bool(true));
        browser.section = section_index("interface");
        let mut output = egui::FullOutput::default();
        for _ in 0..5 {
            output = frame(&ctx, size, vec![], |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        let target = button_position(&output, size, "HUD").ok_or("HUD navigation missing")?;
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
                    "Powerless",
                    "Show Customisable Status Bars",
                    "Show Status Bars",
                    "Reset",
                    "Ok",
                    "Apply",
                    "Cancel",
                ]
            } else {
                [
                    "Bezsilność",
                    "Konfigurowalne paski stanu",
                    "Paski stanu",
                    "Reset",
                    "Ok",
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
    fn movement_editor_fits_a_settings_column_and_routes_preset_clicks()
    -> Result<(), Box<dyn std::error::Error>> {
        for english in [false, true] {
            let ctx = egui::Context::default();
            crate::client_chrome::install(&ctx, false);
            let size = egui::vec2(450.0, 350.0);
            let mut draft = ClientSettings::default();
            {
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
                    ["North: W", "East: D", "South: S", "West: A"]
                } else {
                    ["Północ: W", "Wschód: D", "Południe: S", "Zachód: A"]
                } {
                    assert!(
                        button_position(&output, size, label).is_some(),
                        "clipped {label}"
                    );
                }
                let preset = button_position(
                    &output,
                    size,
                    if english { "Arrow keys" } else { "Strzałki" },
                )
                .ok_or("preset visible")?;
                click(&ctx, size, preset, &mut draw);
            }
            assert_eq!(draft.movement_keys, [82, 79, 81, 80]);
            assert_eq!(
                draft.effective_movement_keys(),
                [Some(82), Some(79), Some(81), Some(80)]
            );
            assert_eq!(
                draft
                    .hotkeys
                    .active()
                    .and_then(|profile| profile.general_chat_off.get("movement.north"))
                    .and_then(|pair| pair.first)
                    .map(|binding| binding.key),
                Some(82)
            );
            let north = oteryn_input_actions::NormalizedInputEvent::Key {
                code: oteryn_input_actions::KeyCode::new(82)?,
                state: oteryn_input_actions::ButtonState::Pressed,
                modifiers: oteryn_input_actions::Modifiers::NONE,
                repeat: false,
            };
            assert_eq!(
                draft.hotkeys.resolve(std::slice::from_ref(&north), false),
                vec![oteryn_client::hotkeys::ResolvedHotkey::General(
                    "movement.north".into()
                )]
            );
            draft.validate()?;

            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "oteryn-complete-movement-{}-{nonce}-{}.json",
                std::process::id(),
                u8::from(english)
            ));
            draft.save(&path)?;
            let loaded = ClientSettings::load(&path)?;
            std::fs::remove_file(&path)?;
            assert_eq!(
                loaded.effective_movement_keys(),
                draft.effective_movement_keys()
            );
            assert_eq!(
                loaded.hotkeys.resolve(std::slice::from_ref(&north), false),
                vec![oteryn_client::hotkeys::ResolvedHotkey::General(
                    "movement.north".into()
                )]
            );

            let before_conflict = draft.clone();
            {
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
                let output = frame(&ctx, size, vec![], &mut draw).1;
                let north = button_position(
                    &output,
                    size,
                    if english { "North: Up" } else { "Północ: Up" },
                )
                .ok_or("north binding visible")?;
                click(&ctx, size, north, &mut draw);
                let popup = frame(&ctx, size, vec![], &mut draw).1;
                let conflicting = button_position(&popup, size, "E")
                    .ok_or("conflicting binding visible in popup")?;
                click(&ctx, size, conflicting, &mut draw);
                let output = frame(&ctx, size, vec![], &mut draw).1;
                assert!(
                    button_position(
                        &output,
                        size,
                        if english {
                            "Shortcut conflict. Movement keys were not changed."
                        } else {
                            "Konflikt skrótów. Klawisze ruchu nie zostały zmienione."
                        }
                    )
                    .is_some(),
                    "movement conflict feedback is not visible"
                );
            }
            assert_eq!(draft, before_conflict);
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
            browser.set_action_bar_rows_available([true; ACTION_BAR_ROWS]);
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
        let navigation = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(260.0, size.y));
        for label in ["Options", "Hotkeys", "Shortcuts", "Help"] {
            assert!(
                button_position_in_rect(&output, navigation, label).is_some(),
                "basic navigation is missing {label}"
            );
        }
        assert!(button_position_in_rect(&output, navigation, "Controls").is_none());
        assert!(button_position(&output, size, "Show Advanced Options").is_some());
        assert!(
            !draft
                .future_preferences
                .contains_key("basic.basic.advanced")
        );
        assert!(draft.validate().is_ok());
        Ok(())
    }

    #[test]
    fn advanced_checkbox_persists_and_switches_between_reference_navigation_modes()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(900.0, 620.0);
        let navigation = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(260.0, size.y));
        let mut browser = PreferencesBrowser::new();
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };

        let mut output = egui::FullOutput::default();
        for _ in 0..4 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        assert_eq!(draft.stored_bool("basic.basic.advanced"), None);
        for label in ["Options", "Hotkeys", "Shortcuts", "Help"] {
            assert!(button_position_in_rect(&output, navigation, label).is_some());
        }
        assert!(button_position_in_rect(&output, navigation, "Controls").is_none());

        let checkbox = button_position(&output, size, "Show Advanced Options")
            .ok_or("advanced checkbox missing")?;
        let _ = click(&ctx, size, checkbox, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        assert_eq!(draft.stored_bool("basic.basic.advanced"), Some(true));
        assert_eq!(
            reference_settings_sections()[browser.section].id,
            "controls"
        );
        for _ in 0..3 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        assert!(button_position_in_rect(&output, navigation, "Controls").is_some());
        assert!(button_position_in_rect(&output, navigation, "Options").is_none());

        let checkbox = button_position(&output, size, "Show Advanced Options")
            .ok_or("advanced checkbox missing after enabling")?;
        let _ = click(&ctx, size, checkbox, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        assert_eq!(draft.stored_bool("basic.basic.advanced"), Some(false));
        assert_eq!(reference_settings_sections()[browser.section].id, "basic");
        for _ in 0..3 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        for label in ["Options", "Hotkeys", "Shortcuts", "Help"] {
            assert!(button_position_in_rect(&output, navigation, label).is_some());
        }
        assert!(button_position_in_rect(&output, navigation, "Controls").is_none());
        Ok(())
    }

    #[test]
    fn compact_footer_keeps_advanced_checkbox_and_recovery_actions_reachable()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(500.0, 620.0);
        let mut browser = PreferencesBrowser::new();
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let mut output = egui::FullOutput::default();
        for _ in 0..4 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        for label in ["Show Advanced Options", "Reset", "Ok", "Apply", "Cancel"] {
            assert!(
                button_position(&output, size, label).is_some(),
                "compact footer clipped {label}"
            );
        }
        assert_eq!(draft.stored_bool("basic.basic.advanced"), None);
        assert!(button_position(&output, size, "Options").is_some());

        let checkbox = button_position(&output, size, "Show Advanced Options")
            .ok_or("compact advanced checkbox missing")?;
        let _ = click(&ctx, size, checkbox, |ctx| {
            browser.show(ctx, &mut draft, None)
        });
        assert_eq!(draft.stored_bool("basic.basic.advanced"), Some(true));
        assert_eq!(
            reference_settings_sections()[browser.section].id,
            "controls"
        );
        for _ in 0..3 {
            output = frame(&ctx, size, Vec::new(), |ctx| {
                browser.show(ctx, &mut draft, None)
            })
            .1;
        }
        assert!(button_position(&output, size, "Controls").is_some());
        Ok(())
    }

    #[test]
    fn clear_row_is_gated_per_row_and_queues_assignment_request_only() -> Result<(), &'static str> {
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
        let mut available = [false; ACTION_BAR_ROWS];
        available[8] = true;
        browser.set_action_bar_rows_available(available);
        {
            let mut show = |ctx: &egui::Context| {
                egui::Window::new("Action assignments")
                    .default_size([850.0, 580.0])
                    .show(ctx, |ui| {
                        action_bars_page(
                            ui,
                            &mut draft,
                            true,
                            browser.action_bar_rows_available,
                            &mut browser.clear_action_row,
                        )
                    });
            };
            for _ in 0..4 {
                let _ = frame(&ctx, size, Vec::new(), &mut show);
            }
            for row in 0..ACTION_BAR_ROWS {
                let (_, enabled) = ctx
                    .data(|data| {
                        data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                            "clear-action-row-enabled",
                            row,
                        )))
                    })
                    .ok_or("missing clear-row response")?;
                assert_eq!(enabled, row == 8, "row {row}");
            }
            let (disabled, _) = ctx
                .data(|data| {
                    data.get_temp::<(egui::Rect, bool)>(egui::Id::new((
                        "clear-action-row-enabled",
                        7,
                    )))
                })
                .ok_or("missing disabled clear-row response")?;
            click(&ctx, size, disabled.center(), &mut show);
        }
        assert_eq!(browser.take_clear_action_row(), None);

        let (enabled, _) = ctx
            .data(|data| {
                data.get_temp::<(egui::Rect, bool)>(egui::Id::new(("clear-action-row-enabled", 8)))
            })
            .ok_or("missing enabled clear-row response")?;
        let show = |ctx: &egui::Context| {
            egui::Window::new("Action assignments")
                .default_size([850.0, 580.0])
                .show(ctx, |ui| {
                    action_bars_page(
                        ui,
                        &mut draft,
                        true,
                        browser.action_bar_rows_available,
                        &mut browser.clear_action_row,
                    )
                });
        };
        click(&ctx, size, enabled.center(), show);
        assert_eq!(browser.take_clear_action_row(), Some(8));
        assert_eq!(browser.take_clear_action_row(), None);
        assert_eq!(draft.action_bar, before);

        browser.clear_action_row = Some(8);
        browser.set_action_bar_rows_available([false; ACTION_BAR_ROWS]);
        assert_eq!(browser.take_clear_action_row(), None);
        assert_eq!(
            clear_action_row_tooltip(true, false),
            "Available when this row contains a session assignment."
        );
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
                    assert!(button_position(&output, size, "Ok").is_some());
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
                    button_position(&output, size, "Ok").is_some(),
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
            browser.section = section_index("game_window");
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
                let ok = button_position(&output, size, "Ok")
                    .ok_or("Ok is clipped or outside viewport")?;
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
