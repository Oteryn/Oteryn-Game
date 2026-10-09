//! Native panel navigation and future layouts; unknown server values stay unknown.
use egui::{Color32, RichText, Vec2};
use oteryn_client::{
    panel_catalog::{
        FieldKind, PANELS, PanelDefinition, PanelField, PanelLayout, ShortcutOrder, fields_for_tab,
        panel,
    },
    play::GameState,
};
use oteryn_session::{EntityDetail, EntityKind};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub struct ClientPanels {
    pub manage: bool,
    pinned: ShortcutOrder,
    open: BTreeSet<&'static str>,
    contents: BTreeMap<&'static str, PanelState>,
    views: crate::panel_views::PanelViews,
    initialized: bool,
    dirty: bool,
}

impl ClientPanels {
    pub fn initialize(&mut self, shortcuts: &[String]) {
        if !self.initialized {
            self.pinned = ShortcutOrder::from_saved(shortcuts).unwrap_or_default();
            self.initialized = true;
        }
    }

    pub fn take_shortcuts(&mut self) -> Option<Vec<String>> {
        std::mem::take(&mut self.dirty).then(|| self.pinned.saved_ids())
    }
    pub fn shortcuts(&mut self, ui: &mut egui::Ui, english: bool) {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(4.0);
            for definition in self.pinned.ids().iter().filter_map(|id| panel(id)) {
                let label = definition.label(english);
                let selected = if definition.id == "shortcuts" {
                    self.manage
                } else {
                    self.open.contains(definition.id)
                };
                if crate::panel_icons::shortcut_selected(ui, definition.id, label, selected)
                    .clicked()
                {
                    if definition.id == "shortcuts" {
                        self.manage = true;
                    } else if !self.open.remove(definition.id) {
                        self.open.insert(definition.id);
                    }
                }
            }
            if ui
                .add_sized([34.0, 30.0], egui::Button::new("+"))
                .on_hover_text(if english {
                    "Manage shortcuts"
                } else {
                    "Dodaj skróty"
                })
                .clicked()
            {
                self.manage = true;
            }
        });
    }

    pub fn show(&mut self, ctx: &egui::Context, state: &GameState, english: bool) {
        let tr = |pl, en| if english { en } else { pl };
        let bounds = ctx.content_rect().shrink(12.0);
        // Window sizes describe content; reserve title/frame space inside the viewport.
        let maximum = Vec2::new(
            (bounds.width() - 28.0).max(120.0),
            (bounds.height() - 58.0).max(100.0),
        );
        if self.manage {
            egui::Window::new(tr("Panele i skróty", "Panels and shortcuts"))
                .id("panel-shortcuts-manager".into())
                .open(&mut self.manage)
                .constrain_to(bounds)
                .min_size([120.0, 100.0])
                .max_size(maximum)
                .default_size([440.0_f32.min(maximum.x), 420.0_f32.min(maximum.y)])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(tr("Wszystkie", "All")).clicked() {
                            self.pinned = ShortcutOrder::all();
                            self.dirty = true;
                        }
                        if ui.button(tr("Domyślne", "Defaults")).clicked() {
                            self.pinned = ShortcutOrder::default();
                            self.dirty = true;
                        }
                    });
                    egui::ScrollArea::vertical()
                        .id_salt("shortcut-manager-list")
                        .max_height((ui.available_height() - 8.0).max(40.0))
                        .show(ui, |ui| {
                            let displayed = self.pinned.ids().to_vec();
                            let mut change = None;
                            ui.heading(tr("Wyświetlane skróty", "Displayed shortcuts"));
                            for (index, id) in displayed.iter().enumerate() {
                                let Some(definition) = panel(id) else {
                                    continue;
                                };
                                ui.horizontal(|ui| {
                                    if crate::panel_icons::shortcut(
                                        ui,
                                        definition.id,
                                        definition.label(english),
                                    )
                                    .clicked()
                                        && definition.id != "shortcuts"
                                    {
                                        self.open.insert(definition.id);
                                    }
                                    ui.label(definition.label(english));
                                    if ui
                                        .add_enabled(index > 0, egui::Button::new("↑"))
                                        .on_hover_text(tr(
                                            "Przenieś skrót wcześniej",
                                            "Move shortcut earlier",
                                        ))
                                        .clicked()
                                    {
                                        change = Some(ShortcutChange::Move(index, true));
                                    }
                                    if ui
                                        .add_enabled(
                                            index + 1 < displayed.len(),
                                            egui::Button::new("↓"),
                                        )
                                        .on_hover_text(tr(
                                            "Przenieś skrót później",
                                            "Move shortcut later",
                                        ))
                                        .clicked()
                                    {
                                        change = Some(ShortcutChange::Move(index, false));
                                    }
                                    if ui
                                        .button(tr("Usuń", "Remove"))
                                        .on_hover_text(tr(
                                            "Ukryj skrót; panel pozostaje dostępny",
                                            "Hide shortcut; the panel stays available",
                                        ))
                                        .clicked()
                                    {
                                        change =
                                            Some(ShortcutChange::Visible(definition.id, false));
                                    }
                                });
                            }
                            ui.separator();
                            ui.heading(tr("Dostępne skróty", "Available shortcuts"));
                            let available: Vec<_> = PANELS
                                .iter()
                                .filter(|p| !self.pinned.contains(p.id))
                                .collect();
                            if available.is_empty() {
                                ui.weak(tr(
                                    "Wszystkie skróty są wyświetlane.",
                                    "All shortcuts are displayed.",
                                ));
                            }
                            for definition in available {
                                ui.horizontal(|ui| {
                                    if crate::panel_icons::shortcut(
                                        ui,
                                        definition.id,
                                        definition.label(english),
                                    )
                                    .clicked()
                                        && definition.id != "shortcuts"
                                    {
                                        self.open.insert(definition.id);
                                    }
                                    ui.label(definition.label(english));
                                    if ui
                                        .button(tr("Dodaj", "Add"))
                                        .on_hover_text(tr(
                                            "Dodaj skrót na końcu paska",
                                            "Append shortcut to the bar",
                                        ))
                                        .clicked()
                                    {
                                        change = Some(ShortcutChange::Visible(definition.id, true));
                                    }
                                });
                            }
                            if let Some(change) = change {
                                self.dirty |= match change {
                                    ShortcutChange::Move(index, earlier) => {
                                        self.pinned.move_one(index, earlier)
                                    }
                                    ShortcutChange::Visible(id, visible) => {
                                        self.pinned.set_visible(id, visible)
                                    }
                                };
                            }
                        });
                });
        }
        let opened: Vec<_> = self.open.iter().copied().collect();
        for id in opened {
            let Some(definition) = panel(id) else {
                continue;
            };
            let mut open = true;
            let size = panel_size(definition.layout).min(maximum);
            egui::Window::new(definition.label(english))
                .id(egui::Id::new(("client-panel", id)))
                .open(&mut open)
                .constrain_to(bounds)
                .min_size([120.0, 100.0])
                .max_size(maximum)
                .default_size(size)
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);
                    egui::ScrollArea::vertical()
                        .id_salt(("panel-content", id))
                        .max_height(ui.available_height().max(40.0))
                        .show(ui, |ui| {
                            if !self.views.show(ui, id, english, state) {
                                panel_content(
                                    ui,
                                    definition,
                                    self.contents.entry(id).or_default(),
                                    state,
                                    english,
                                );
                            }
                        });
                });
            if !open {
                self.open.remove(id);
            }
        }
        if let Some(destination) = self.views.take_navigation()
            && panel(destination).is_some()
        {
            self.open.insert(destination);
        }
    }
}

enum ShortcutChange {
    Move(usize, bool),
    Visible(&'static str, bool),
}

#[derive(Default)]
struct PanelState {
    tab: usize,
    query: String,
    actor_filter: usize,
    sort: usize,
}

fn panel_size(layout: PanelLayout) -> Vec2 {
    match layout {
        PanelLayout::Statistics => Vec2::new(300.0, 390.0),
        PanelLayout::Battle
        | PanelLayout::Contacts
        | PanelLayout::Party
        | PanelLayout::Tracker
        | PanelLayout::Thresholds
        | PanelLayout::EquipmentEffects => Vec2::new(290.0, 320.0),
        PanelLayout::Cyclopedia
        | PanelLayout::Wheel
        | PanelLayout::Forge
        | PanelLayout::TaskBoard
        | PanelLayout::Analytics => Vec2::new(600.0, 420.0),
        _ => Vec2::new(440.0, 350.0),
    }
}

fn panel_content(
    ui: &mut egui::Ui,
    definition: &PanelDefinition,
    contents: &mut PanelState,
    state: &GameState,
    english: bool,
) {
    let tr = |pl, en| if english { en } else { pl };
    if !definition.tabs.is_empty() {
        if definition.tabs.len() > 7 {
            // Keep the twenty-page Cyclopedia navigator compact instead of consuming
            // the viewport with wrapped tab rows.
            let groups: &[&[&str]] = &[
                &["map", "items", "houses"],
                &[
                    "character",
                    "character-stats",
                    "offence",
                    "defence",
                    "blessings",
                    "deaths",
                    "pvp-kills",
                    "achievements",
                    "item-summary",
                    "appearances",
                    "store-summary",
                    "titles",
                ],
                &["bestiary", "charms", "bosses", "boss-slots", "archive"],
            ];
            if definition.id == "cyclopedia" {
                ui.horizontal_wrapped(|ui| {
                    for (group, label) in groups.iter().zip([
                        tr("Świat", "World"),
                        tr("Postać", "Character"),
                        tr("Stworzenia i magia", "Creatures and magic"),
                    ]) {
                        let current = definition.tabs.get(contents.tab).map(|tab| tab.id);
                        if ui
                            .selectable_label(current.is_some_and(|id| group.contains(&id)), label)
                            .clicked()
                            && let Some(index) = definition
                                .tabs
                                .iter()
                                .position(|tab| group.contains(&tab.id))
                        {
                            contents.tab = index;
                        }
                    }
                });
            }
            egui::ComboBox::from_id_salt((definition.id, "section"))
                .selected_text(
                    definition
                        .tabs
                        .get(contents.tab)
                        .map_or("", |tab| tab.label(english)),
                )
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for (index, tab) in definition.tabs.iter().enumerate() {
                        ui.selectable_value(&mut contents.tab, index, tab.label(english));
                    }
                });
        } else {
            ui.horizontal_wrapped(|ui| {
                for (index, tab) in definition.tabs.iter().enumerate() {
                    ui.selectable_value(&mut contents.tab, index, tab.label(english));
                }
            });
        }
        ui.separator();
    }
    let tab = definition.tabs.get(contents.tab).map(|tab| tab.id);
    if definition.layout == PanelLayout::Documents {
        document(ui, definition.id, tab, english);
        return;
    }
    ui.add(
        egui::TextEdit::singleline(&mut contents.query)
            .desired_width(ui.available_width())
            .hint_text(tr("Szukaj", "Search")),
    );
    let query = contents.query.trim().to_lowercase();
    if definition.layout == PanelLayout::Battle {
        battle_entries(
            ui,
            state,
            &query,
            &mut contents.actor_filter,
            &mut contents.sort,
            english,
        );
        return;
    }
    let fields = tab.map_or(definition.fields, |tab| fields_for_tab(definition.id, tab));
    let vitals_panel = definition.id == "skills"
        || (definition.id == "cyclopedia" && matches!(tab, Some("character" | "character-stats")));
    let vitals = vitals_panel.then_some(state.vitals).flatten();
    if let Some(vitals) = vitals {
        ui.horizontal_wrapped(|ui| {
            for (label, value) in [
                ("Soul", vitals.soul),
                (tr("Harmonia", "Harmony"), vitals.harmony),
            ] {
                if query.is_empty() || label.to_lowercase().contains(&query) {
                    ui.label(format!("{label}: {value}"));
                }
            }
            if query.is_empty() || "serene".contains(&query) {
                ui.label(if vitals.serene {
                    tr("Serene: aktywne", "Serene: active")
                } else {
                    tr("Serene: nieaktywne", "Serene: inactive")
                });
            }
        });
    }
    let filtered: Vec<_> = fields
        .iter()
        .filter(|field| query.is_empty() || field.label(english).to_lowercase().contains(&query))
        .collect();
    let resource_match = vitals.is_some()
        && ["Soul", tr("Harmonia", "Harmony"), "Serene"]
            .iter()
            .any(|label| label.to_lowercase().contains(&query));
    if filtered.is_empty() && !query.is_empty() && !resource_match {
        ui.label(tr(
            "Brak pól pasujących do filtra.",
            "No fields match this filter.",
        ));
        return;
    }
    ui.add_space(4.0);
    // Scalar statistics belong in compact rows; progress, lists and descriptions retain
    // their own presentation instead of one framed placeholder per statistic.
    egui::Grid::new((definition.id, contents.tab, "statistics"))
        .num_columns(2)
        .spacing([16.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            for field in filtered.iter().filter(|field| {
                matches!(
                    field.kind,
                    FieldKind::Value | FieldKind::Duration | FieldKind::Status
                )
            }) {
                ui.label(field.label(english));
                ui.label(RichText::new("—").color(Color32::from_gray(145)));
                ui.end_row();
            }
        });
    for field in filtered.iter().filter(|field| {
        !matches!(
            field.kind,
            FieldKind::Value | FieldKind::Duration | FieldKind::Status
        )
    }) {
        let measured = vitals.and_then(|v| match field.key {
            "health" => Some((v.health, v.max_health)),
            "mana" => Some((v.mana, v.max_mana)),
            _ => None,
        });
        field_widget(ui, definition.id, field, measured, english);
        ui.add_space(4.0);
    }
    ui.separator();
    ui.small(tr(
        "— oznacza brak danych gry.",
        "— means game data is unavailable.",
    ));
}

fn document(ui: &mut egui::Ui, id: &str, tab: Option<&str>, english: bool) {
    let tr = |pl, en| if english { en } else { pl };
    let text = match (id, tab) {
        (_, Some("account")) => tr(
            "Wybierz konto i postać na ekranie logowania. Dane świata i dostępność postaci pochodzą z Platformy Oteryn.",
            "Choose your account and character on the sign-in screen. World information and character availability come from Oteryn Platform.",
        ),
        (_, Some("world")) => tr(
            "Minimapa pokazuje teren wczytany przez klienta. Świat, postacie i wyniki wykonywanych czynności aktualizuje serwer gry.",
            "The minimap shows terrain loaded by the client. The game server updates the world, actors and action outcomes.",
        ),
        (_, Some("features")) => tr(
            "Skróty otwierają umiejętności, listę walki, dzienniki, cyklopedię i pozostałe panele. Przycisk + wybiera widoczne skróty.",
            "Shortcuts open skills, the battle list, journals, Cyclopedia and other panels. Use + to choose visible shortcuts.",
        ),
        (_, Some("support")) => tr(
            "Przy zgłoszeniu problemu podaj wersję klienta, świat i widoczny komunikat błędu. Nie udostępniaj haseł ani biletów logowania.",
            "When reporting a problem, include the client version, world and displayed error. Keep passwords and sign-in tickets private.",
        ),
        (_, Some("updates")) => {
            ui.label(format!("Oteryn {}", env!("CARGO_PKG_VERSION")));
            return;
        }
        _ => tr(
            "F10 otwiera ustawienia. Kierunki ruchu zmienisz w Sterowaniu. Kliknięcie terenu wyznacza drogę; kliknięcie obiektu wybiera go. Prawy przycisk na polu paska akcji otwiera menu przypisania.",
            "F10 opens settings. Change movement keys in Controls. Clicking terrain sets a walking goal; clicking an object selects it. Right-click an action slot to open its assignment menu.",
        ),
    };
    ui.label(text);
}

fn field_widget(
    ui: &mut egui::Ui,
    panel_id: &str,
    field: &PanelField,
    measured: Option<(u32, u32)>,
    english: bool,
) {
    let tr = |pl, en| if english { en } else { pl };
    let unknown = || RichText::new(tr("Brak danych", "No data")).color(Color32::from_gray(155));
    match field.kind {
        FieldKind::Value => {
            ui.horizontal_wrapped(|ui| {
                ui.label(field.label(english));
                ui.label(RichText::new("—").strong().color(Color32::from_gray(155)));
            });
        }
        FieldKind::Progress => {
            ui.label(RichText::new(field.label(english)).strong());
            if let Some((current, maximum)) = measured.filter(|(_, maximum)| *maximum > 0) {
                ui.add(
                    egui::ProgressBar::new((current as f32 / maximum as f32).clamp(0.0, 1.0))
                        .desired_width(ui.available_width())
                        .fill(if field.key == "mana" {
                            Color32::from_rgb(56, 100, 179)
                        } else {
                            Color32::from_rgb(174, 55, 58)
                        })
                        .text(format!("{current} / {maximum}")),
                );
            } else {
                // An outline with no fill represents missing data, never a fabricated 0%.
                let (rect, response) = ui.allocate_exact_size(
                    Vec2::new(ui.available_width(), 22.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_stroke(
                    rect,
                    4.0,
                    egui::Stroke::new(1.0, Color32::from_gray(85)),
                    egui::StrokeKind::Inside,
                );
                let label = measured.map_or_else(
                    || "—".to_owned(),
                    |(current, maximum)| format!("{current} / {maximum}"),
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(13.0),
                    Color32::from_gray(155),
                );
                response.on_hover_text(tr(
                    "Nie otrzymano danych postępu.",
                    "Progress data has not been received.",
                ));
            }
        }
        FieldKind::Duration => {
            ui.group(|ui| {
                ui.label(field.label(english));
                ui.horizontal_wrapped(|ui| {
                    ui.monospace("—");
                    ui.label(unknown());
                });
            });
        }
        FieldKind::Status => {
            ui.horizontal_wrapped(|ui| {
                ui.label(field.label(english));
                egui::Frame::new()
                    .stroke(egui::Stroke::new(1.0, Color32::from_gray(85)))
                    .corner_radius(6.0)
                    .inner_margin(egui::Margin::symmetric(7, 3))
                    .show(ui, |ui| {
                        ui.label(unknown());
                    });
            });
        }
        FieldKind::Table => {
            ui.group(|ui| {
                ui.label(RichText::new(field.label(english)).strong());
                let columns = table_columns(panel_id, field.key, english);
                let column_width = ((ui.available_width()
                    - 18.0 * (columns.len().saturating_sub(1)) as f32)
                    / columns.len() as f32)
                    .max(24.0);
                egui::Grid::new(("planned-panel-table", panel_id, field.key))
                    .num_columns(columns.len())
                    .min_col_width(0.0)
                    .max_col_width(column_width)
                    .spacing([18.0, 8.0])
                    .show(ui, |ui| {
                        for column in columns {
                            ui.add(egui::Label::new(RichText::new(*column).strong()).wrap());
                        }
                        ui.end_row();
                    });
                ui.separator();
                ui.add_space(8.0);
                ui.label(RichText::new("—").color(Color32::from_gray(145)));
            });
        }
        FieldKind::Details => {
            egui::CollapsingHeader::new(field.label(english))
                .id_salt((panel_id, field.key))
                .show(ui, |ui| {
                    ui.label(RichText::new("—").color(Color32::from_gray(145)));
                });
        }
    }
}

fn table_columns(panel_id: &str, field: &str, english: bool) -> &'static [&'static str] {
    match (panel_id, field, english) {
        (_, "skills", false) => &["Umiejętność", "Poziom", "Postęp"],
        (_, "skills", true) => &["Skill", "Level", "Progress"],
        (_, "items" | "resources" | "requirements", false) => &["Przedmiot lub zasób", "Ilość"],
        (_, "items" | "resources" | "requirements", true) => &["Item or resource", "Quantity"],
        (_, "members", false) => &["Postać", "Stan", "Wynik"],
        (_, "members", true) => &["Character", "Status", "Result"],
        (_, "records" | "history", false) => &["Data", "Zdarzenie"],
        (_, "records" | "history", true) => &["Date", "Event"],
        (_, "types" | "sources" | "breakdown", false) => &["Rodzaj lub źródło", "Wartość"],
        (_, "types" | "sources" | "breakdown", true) => &["Type or source", "Value"],
        (_, "offers", false) => &["Oferta", "Cena", "Dostępność"],
        (_, "offers", true) => &["Offer", "Price", "Availability"],
        (_, _, false) => &["Wpis", "Szczegóły"],
        (_, _, true) => &["Entry", "Details"],
    }
}

fn battle_entries(
    ui: &mut egui::Ui,
    state: &GameState,
    query: &str,
    filter: &mut usize,
    sort: &mut usize,
    english: bool,
) {
    let tr = |pl, en| if english { en } else { pl };
    ui.horizontal_wrapped(|ui| {
        for (index, label) in [
            tr("Wszystkie", "All"),
            tr("Gracze", "Players"),
            tr("Stworzenia", "Creatures"),
            "NPC",
        ]
        .into_iter()
        .enumerate()
        {
            ui.selectable_value(filter, index, label);
        }
    });
    egui::ComboBox::from_id_salt("battle-sort")
        .selected_text(
            [
                tr("Według rodzaju", "By type"),
                tr("Według położenia", "By position"),
            ][*sort],
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(sort, 0, tr("Według rodzaju", "By type"));
            ui.selectable_value(sort, 1, tr("Według położenia", "By position"));
        });
    ui.separator();
    let mut actors: Vec<_> = state
        .entities
        .iter()
        .filter(|entity| {
            matches!(entity.detail, EntityDetail::Actor { .. })
                && match *filter {
                    1 => entity.kind == EntityKind::Player,
                    2 => entity.kind == EntityKind::Creature,
                    3 => entity.kind == EntityKind::Npc,
                    _ => true,
                }
        })
        .collect();
    actors.sort_by_key(|entity| {
        if *sort == 0 {
            (
                entity.kind as i32,
                entity.position.x,
                entity.position.y,
                entity.position.floor,
            )
        } else {
            (
                i32::from(entity.position.floor),
                entity.position.y,
                entity.position.x,
                entity.kind as i16,
            )
        }
    });
    let mut matched = false;
    for entity in &actors {
        let kind = match entity.kind {
            EntityKind::Player => tr("Gracz", "Player"),
            EntityKind::Creature => tr("Stworzenie", "Creature"),
            EntityKind::Npc => "NPC",
            EntityKind::Corpse | EntityKind::GroundItem => continue,
        };
        let label = format!(
            "{kind} · {}, {} · {}",
            entity.position.x, entity.position.y, entity.position.floor
        );
        if !query.is_empty() && !label.to_lowercase().contains(query) {
            continue;
        }
        matched = true;
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(kind).strong());
            ui.monospace(format!(
                "{}, {} / {}",
                entity.position.x, entity.position.y, entity.position.floor
            ));
        });
    }
    if !matched {
        ui.label(RichText::new("—").color(Color32::from_gray(145)));
    }
    ui.separator();
    ui.small(tr(
        "Nazwy i zdrowie oczekują na dane gry.",
        "Names and health await game data.",
    ));
}
