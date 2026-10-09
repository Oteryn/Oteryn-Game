//! Native panel navigation and future layouts; unknown server values stay unknown.
use egui::{Color32, RichText, Vec2};
use oteryn_client::{
    panel_catalog::{FieldKind, PANELS, PanelField, PanelLayout, fields_for_tab, panel},
    play::GameState,
};
use oteryn_session::{EntityDetail, EntityKind};
use std::collections::{BTreeMap, BTreeSet};

pub struct ClientPanels {
    pub manage: bool,
    pinned: BTreeSet<&'static str>,
    open: BTreeSet<&'static str>,
    tabs: BTreeMap<&'static str, usize>,
    searches: BTreeMap<&'static str, String>,
    initialized: bool,
    dirty: bool,
}

impl Default for ClientPanels {
    fn default() -> Self {
        Self {
            manage: false,
            pinned: PANELS
                .iter()
                .filter(|p| p.default_visible)
                .map(|p| p.id)
                .collect(),
            open: BTreeSet::new(),
            tabs: BTreeMap::new(),
            searches: BTreeMap::new(),
            initialized: false,
            dirty: false,
        }
    }
}

impl ClientPanels {
    pub fn initialize(&mut self, shortcuts: &[String]) {
        if !self.initialized {
            self.pinned = shortcuts
                .iter()
                .filter_map(|id| panel(id).map(|p| p.id))
                .collect();
            self.initialized = true;
        }
    }

    pub fn take_shortcuts(&mut self) -> Option<Vec<String>> {
        std::mem::take(&mut self.dirty)
            .then(|| self.pinned.iter().map(|id| (*id).to_owned()).collect())
    }
    pub fn shortcuts(&mut self, ui: &mut egui::Ui, english: bool) {
        ui.horizontal_wrapped(|ui| {
            for definition in PANELS.iter().filter(|p| self.pinned.contains(p.id)) {
                let label = definition.label(english);
                let icon: String = label
                    .split_whitespace()
                    .take(2)
                    .filter_map(|word| word.chars().next())
                    .collect();
                if ui
                    .add_sized([34.0, 30.0], egui::Button::new(icon))
                    .on_hover_text(label)
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
                .button("+")
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
                            self.pinned = PANELS.iter().map(|p| p.id).collect();
                            self.dirty = true;
                        }
                        if ui.button(tr("Domyślne", "Defaults")).clicked() {
                            self.pinned = PANELS
                                .iter()
                                .filter(|p| p.default_visible)
                                .map(|p| p.id)
                                .collect();
                            self.dirty = true;
                        }
                    });
                    egui::ScrollArea::vertical()
                        .id_salt("shortcut-manager-list")
                        .max_height((ui.available_height() - 8.0).max(40.0))
                        .show(ui, |ui| {
                            for definition in PANELS {
                                ui.horizontal(|ui| {
                                    let mut pinned = self.pinned.contains(definition.id);
                                    if ui
                                        .checkbox(&mut pinned, definition.label(english))
                                        .changed()
                                    {
                                        self.dirty = true;
                                        if pinned {
                                            self.pinned.insert(definition.id);
                                        } else {
                                            self.pinned.remove(definition.id);
                                        }
                                    }
                                    if definition.id != "shortcuts"
                                        && ui.button(tr("Otwórz", "Open")).clicked()
                                    {
                                        self.open.insert(definition.id);
                                    }
                                });
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
            egui::Window::new(definition.label(english))
                .id(egui::Id::new(("client-panel", id)))
                .open(&mut open)
                .constrain_to(bounds)
                .min_size([120.0, 100.0])
                .max_size(maximum)
                .default_size([520.0_f32.min(maximum.x), 420.0_f32.min(maximum.y)])
                .show(ctx, |ui| {
                    let selected = self.tabs.entry(id).or_default();
                    let search = self.searches.entry(id).or_default();
                    egui::ScrollArea::vertical()
                        .id_salt(("panel-content", id))
                        .max_height(ui.available_height().max(40.0))
                        .show(ui, |ui| {
                    if !definition.tabs.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            for (index, tab) in definition.tabs.iter().enumerate() { ui.selectable_value(selected, index, tab.label(english)); }
                        });
                        ui.separator();
                    }
                    let fields = definition.tabs.get(*selected).map_or(definition.fields, |tab| fields_for_tab(id, tab.id));
                    if definition.layout != PanelLayout::Documents {
                        ui.add(egui::TextEdit::singleline(search).desired_width(ui.available_width()).hint_text(tr("Filtruj pola i dostępne wpisy", "Filter fields and available entries")));
                        ui.add_space(8.0);
                    }
                    let query = search.trim().to_lowercase();
                    if definition.layout == PanelLayout::Documents {
                        ui.heading("Oteryn");
                        ui.label(tr("F10 otwiera ustawienia. Kliknięcie mapy wyznacza drogę, a kierunki zmienisz w Sterowaniu. Skróty paneli dodasz przyciskiem +.", "F10 opens settings. Click the map to walk; change movement keys in Controls. Add panel shortcuts using +."));
                        ui.separator();
                    }
                    if definition.layout == PanelLayout::Battle {
                        battle_entries(ui, state, &query, english);
                    } else {
                        let mut matched = false;
                        for field in fields {
                            if !query.is_empty() && !field.label(english).to_lowercase().contains(&query) {
                                continue;
                            }
                            matched = true;
                            let vitals_panel = id == "skills" || (id == "cyclopedia" && definition.tabs.get(*selected).is_some_and(|t| matches!(t.id, "character" | "character-stats")));
                            let values = vitals_panel.then_some(state.vitals).flatten().and_then(|v| match field.key {
                                "health" => Some((v.health, v.max_health)),
                                "mana" => Some((v.mana, v.max_mana)),
                                _ => None,
                            });
                            field_widget(ui, id, field, values, english);
                            ui.add_space(6.0);
                        }
                        if !matched { ui.label(tr("Brak pól pasujących do filtra.", "No fields match this filter.")); }
                    }
                        ui.separator();
                        ui.small(tr("Pola bez wartości oczekują na dane gry.", "Fields without values await game data."));
                    });
                });
            if !open {
                self.open.remove(id);
            }
        }
    }
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
                let (rect, _) = ui.allocate_exact_size(
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
                    || tr("Postęp nieznany", "Progress unknown").to_owned(),
                    |(current, maximum)| format!("{current} / {maximum}"),
                );
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(13.0),
                    Color32::from_gray(155),
                );
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
                ui.label(unknown());
                ui.small(tr(
                    "Wpisy pojawią się po otrzymaniu danych gry.",
                    "Entries will appear when game data arrives.",
                ));
            });
        }
        FieldKind::Details => {
            ui.group(|ui| {
                ui.label(RichText::new(field.label(english)).strong());
                ui.label(unknown());
                ui.add_space(8.0);
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

fn battle_entries(ui: &mut egui::Ui, state: &GameState, query: &str, english: bool) {
    let tr = |pl, en| if english { en } else { pl };
    ui.heading(tr("Widoczne postacie", "Visible actors"));
    let mut received = false;
    let mut matched = false;
    for entity in &state.entities {
        if !matches!(entity.detail, EntityDetail::Actor { .. }) {
            continue;
        }
        received = true;
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
        ui.group(|ui| {
            ui.label(RichText::new(label).strong());
            ui.small(tr(
                "Nazwa i wiarygodne zdrowie: brak danych",
                "Name and authoritative health: no data",
            ));
        });
        ui.add_space(4.0);
    }
    if !received {
        ui.label(tr(
            "Nie otrzymano wpisów postaci.",
            "No actor entries have been received.",
        ));
    } else if !matched {
        ui.label(tr(
            "Żadna postać nie pasuje do filtra.",
            "No actors match this filter.",
        ));
    }
}
