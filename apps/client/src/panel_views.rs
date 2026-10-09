//! Dedicated native panel compositions. Form inputs are session-local drafts, never game facts.
//! Recreate PanelViews on admission; selected item handles must not cross session generations.
use egui::{Color32, RichText, Stroke, Vec2};
use oteryn_client::play::GameState;
use std::collections::BTreeMap;

type Label = (&'static str, &'static str);
type Route = (&'static str, Option<usize>);

/// Client-owned names only. No allocation, gem or server preset is inferred from these drafts.
#[derive(Default)]
struct LocalPresets {
    names: Vec<String>,
    selected: Option<usize>,
}
impl LocalPresets {
    fn create(&mut self, name: &str) -> bool {
        let name: String = name.trim().chars().take(96).collect();
        if name.is_empty() || self.names.len() >= 16 {
            return false;
        }
        self.selected = Some(self.names.len());
        self.names.push(name);
        true
    }
    fn rename(&mut self, name: &str) -> bool {
        let name: String = name.trim().chars().take(96).collect();
        let Some(current) = self.selected.and_then(|index| self.names.get_mut(index)) else {
            return false;
        };
        if name.is_empty() {
            return false;
        }
        *current = name;
        true
    }
    fn copy(&mut self) -> bool {
        let Some(name) = self
            .selected
            .and_then(|index| self.names.get(index))
            .cloned()
        else {
            return false;
        };
        self.create(&name)
    }
    fn remove(&mut self) {
        if let Some(index) = self.selected.filter(|index| *index < self.names.len()) {
            self.names.remove(index);
            self.selected = (!self.names.is_empty()).then(|| index.min(self.names.len() - 1));
        }
    }
}

#[derive(Default)]
struct Draft {
    tab: usize,
    sector: usize,
    texts: BTreeMap<&'static str, String>,
    choices: BTreeMap<&'static str, usize>,
    flags: BTreeMap<&'static str, bool>,
    items: BTreeMap<&'static str, Option<u64>>,
    wheel: LocalPresets,
}

#[derive(Default)]
pub struct PanelViews {
    drafts: BTreeMap<String, Draft>,
    navigation: Option<&'static str>,
}

impl PanelViews {
    /// One local panel-navigation request, consumed by the shell without a Game command.
    pub fn take_navigation(&mut self) -> Option<&'static str> {
        self.navigation.take()
    }

    /// Returns false for panels drawn by the existing live HUD/help/shortcut renderer.
    pub fn show(&mut self, ui: &mut egui::Ui, id: &str, english: bool, state: &GameState) -> bool {
        if !matches!(
            id,
            "spells"
                | "vip"
                | "quest-log"
                | "highscores"
                | "wheel"
                | "prey"
                | "forge"
                | "task-board"
                | "party"
                | "social"
                | "reward-wall"
                | "analytics"
                | "bosstiary"
                | "boss-slots"
                | "weapon-proficiency"
                | "imbuement-tracker"
                | "quest-tracker"
                | "kill-tracker"
                | "bosstiary-tracker"
                | "bestiary-tracker"
                | "unjustified-points"
                | "cyclopedia"
        ) {
            return false;
        }
        let draft = self.drafts.entry(id.to_owned()).or_default();
        let mut route: Option<Route> = None;
        ui.push_id(("dedicated-panel", id), |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 4.0);
            match id {
                "spells" => spells(ui, draft, english),
                "vip" => vip(ui, draft, english),
                "quest-log" => {
                    if quests(ui, draft, english) {
                        route = Some(("quest-tracker", None));
                    }
                }
                "highscores" => highscores(ui, draft, english),
                "wheel" => wheel(ui, draft, english),
                "prey" => prey(ui, draft, english),
                "forge" => forge(ui, draft, english, state),
                "task-board" => tasks(ui, draft, english),
                "party" | "social" => social(ui, draft, english, id == "party"),
                "reward-wall" => rewards(ui, draft, english),
                "analytics" => {
                    if analytics(ui, draft, english) {
                        route = Some(("cyclopedia", Some(1)));
                    }
                }
                "bosstiary" => bestiary(ui, draft, english, true),
                "boss-slots" => boss_slots(ui, english),
                "weapon-proficiency" => proficiency(ui, draft, english, state),
                "imbuement-tracker" => imbuements(ui, draft, english, state),
                "unjustified-points" => unjustified(ui, english),
                "cyclopedia" => cyclopedia(ui, draft, english, state),
                _ => route = tracker(ui, draft, english, id),
            }
        });
        if let Some((target, page)) = route {
            if let Some(page) = page {
                self.drafts
                    .entry("cyclopedia".to_owned())
                    .or_default()
                    .choices
                    .insert("cyclopedia.page", page);
            }
            // Local navigation never assigns a tracked item/quest or sends a Game command.
            self.navigation = Some(target);
        }
        true
    }
}

fn tr(english: bool, label: Label) -> &'static str {
    if english { label.1 } else { label.0 }
}

fn tabs(ui: &mut egui::Ui, draft: &mut Draft, english: bool, labels: &[Label]) {
    ui.horizontal_wrapped(|ui| {
        for (index, label) in labels.iter().enumerate() {
            ui.selectable_value(&mut draft.tab, index, tr(english, *label));
        }
    });
    ui.separator();
}

fn text(ui: &mut egui::Ui, draft: &mut Draft, key: &'static str, english: bool, label: Label) {
    ui.label(tr(english, label));
    ui.add(
        egui::TextEdit::singleline(draft.texts.entry(key).or_default())
            .char_limit(96)
            .desired_width(ui.available_width()),
    );
}

fn choice(
    ui: &mut egui::Ui,
    draft: &mut Draft,
    key: &'static str,
    english: bool,
    label: Label,
    choices: &[Label],
) {
    if choices.is_empty() {
        return;
    }
    let selected = draft.choices.entry(key).or_default();
    *selected = (*selected).min(choices.len() - 1);
    egui::ComboBox::from_id_salt(key)
        .width(ui.available_width())
        .truncate()
        .selected_text(format!(
            "{}: {}",
            tr(english, label),
            tr(english, choices[*selected])
        ))
        .show_ui(ui, |ui| {
            for (index, label) in choices.iter().enumerate() {
                ui.selectable_value(selected, index, tr(english, *label));
            }
        });
}

fn flag(ui: &mut egui::Ui, draft: &mut Draft, key: &'static str, english: bool, label: Label) {
    ui.checkbox(draft.flags.entry(key).or_default(), tr(english, label));
}

fn action(ui: &mut egui::Ui, english: bool, label: Label) {
    ui.add_enabled(false, egui::Button::new(tr(english, label)))
        .on_disabled_hover_text(tr(
            english,
            (
                "Ta czynność jest obecnie niedostępna.",
                "This action is currently unavailable.",
            ),
        ));
}

fn values(ui: &mut egui::Ui, key: &str, english: bool, labels: &[Label]) {
    egui::Grid::new(key)
        .num_columns(2)
        .striped(true)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            for label in labels {
                ui.label(tr(english, *label));
                ui.label(RichText::new("—").color(Color32::GRAY));
                ui.end_row();
            }
        });
}

fn table(ui: &mut egui::Ui, key: &str, english: bool, columns: &[Label]) {
    if columns.is_empty() {
        return;
    }
    ui.group(|ui| {
        let width = ((ui.available_width() - 8.0 * (columns.len() - 1) as f32)
            / columns.len() as f32)
            .max(24.0);
        egui::Grid::new(key)
            .num_columns(columns.len())
            .min_col_width(0.0)
            .max_col_width(width)
            .spacing([8.0, 4.0])
            .show(ui, |ui| {
                for column in columns {
                    ui.add(egui::Label::new(RichText::new(tr(english, *column)).strong()).wrap());
                }
                ui.end_row();
            });
        ui.separator();
        ui.add_space(8.0);
        ui.label(
            RichText::new(tr(
                english,
                ("Nie otrzymano wpisów.", "No entries have been received."),
            ))
            .color(Color32::GRAY),
        );
        ui.add_space(8.0);
    });
}

fn progress(ui: &mut egui::Ui, english: bool, label: Label) {
    ui.label(tr(english, label));
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 18.0), egui::Sense::hover());
    ui.painter().rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, Color32::from_gray(85)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "—",
        egui::FontId::proportional(12.0),
        Color32::GRAY,
    );
    response.on_hover_text(tr(
        english,
        (
            "Nie otrzymano danych postępu.",
            "Progress data has not been received.",
        ),
    ));
}

fn details(ui: &mut egui::Ui, key: &str, english: bool, label: Label) {
    egui::CollapsingHeader::new(tr(english, label))
        .id_salt(key)
        .default_open(true)
        .show(ui, |ui| {
            ui.label("—");
        });
}

fn item_slot(
    ui: &mut egui::Ui,
    draft: &mut Draft,
    key: &'static str,
    english: bool,
    label: Label,
    state: &GameState,
) {
    let chosen = draft.items.entry(key).or_default();
    let inventory = state.inventory.as_ref();
    let items: Vec<_> = inventory
        .into_iter()
        .flat_map(|view| {
            view.entries
                .iter()
                .chain(view.main_backpack.iter())
                .chain(view.equipment.iter().map(|entry| &entry.item))
        })
        .collect();
    if chosen.is_some_and(|handle| !items.iter().any(|item| item.handle.get() == handle)) {
        *chosen = None;
    }
    ui.label(tr(english, label));
    let selected = items
        .iter()
        .find(|item| Some(item.handle.get()) == *chosen)
        .map_or_else(
            || "—".to_owned(),
            |item| {
                format!(
                    "{} #{} × {}",
                    tr(english, ("Przedmiot", "Item")),
                    item.item_definition_ref,
                    item.count
                )
            },
        );
    egui::ComboBox::from_id_salt(key)
        .width(ui.available_width())
        .truncate()
        .selected_text(selected)
        .show_ui(ui, |ui| {
            ui.selectable_value(chosen, None, "—");
            for item in items {
                ui.selectable_value(
                    chosen,
                    Some(item.handle.get()),
                    format!("#{} × {}", item.item_definition_ref, item.count),
                );
            }
        });
}

const ALL: Label = ("Wszystkie", "All");
const VOCATIONS: &[Label] = &[
    ALL,
    ("Rycerz", "Knight"),
    ("Paladyn", "Paladin"),
    ("Druid", "Druid"),
    ("Czarodziej", "Sorcerer"),
    ("Mnich", "Monk"),
];

#[rustfmt::skip]
fn spells(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    text(ui,d,"spell.search",en,("Wyszukaj czar lub formułę","Search spell or formula"));
    choice(ui,d,"spell.vocation",en,("Profesja","Vocation"),VOCATIONS);
    choice(ui,d,"spell.group",en,("Grupa","Group"),&[ALL,("Atak","Attack"),("Leczenie","Healing"),("Wsparcie","Support")]);
    ui.menu_button(tr(en,("Filtry i sortowanie","Filters and sorting")),|ui| {
        flag(ui,d,"spell.usable",en,("Tylko możliwe do użycia","Only usable spells"));
        choice(ui,d,"spell.sort",en,("Sortuj","Sort"),&[("Nazwa","Name"),("Formuła","Formula"),("Poziom","Level"),("Mana","Mana")]);
        flag(ui,d,"spell.descending",en,("Malejąco","Descending"));
    });
    ui.columns(2,|columns| {
        table(&mut columns[0],"spell.list",en,&[("Czar","Spell"),("Formuła","Formula")]);
        values(&mut columns[1],"spell.detail",en,&[("Wybrany czar","Selected spell"),("Formuła","Formula"),("Profesja","Vocation"),("Grupa","Group"),("Wymagany poziom","Required level"),("Mana","Mana"),("Odnowienie","Cooldown")]);
        details(&mut columns[1],"spell.description",en,("Opis czaru","Spell description"));
    });
    ui.horizontal_wrapped(|ui| { action(ui,en,("Rzuć czar","Cast spell")); action(ui,en,("Przypisz do paska akcji","Assign to action bar")); });
}

#[rustfmt::skip]
fn vip(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    text(ui,d,"vip.search",en,("Wyszukaj kontakt","Search contact"));
    ui.menu_button(tr(en,("Lista i grupy","List and groups")),|ui| {
        flag(ui,d,"vip.offline",en,("Pokaż kontakty offline","Show offline contacts"));
        flag(ui,d,"vip.groups",en,("Grupuj kontakty","Group contacts"));
        choice(ui,d,"vip.sort",en,("Sortuj","Sort"),&[("Nazwa","Name"),("Dostępność","Online status")]);
    });
    table(ui,"vip.list",en,&[("Postać","Character"),("Dostępność","Online status"),("Grupa","Group")]);
    egui::CollapsingHeader::new(tr(en,("Dodaj lub edytuj kontakt","Add or edit contact"))).show(ui,|ui| {
        text(ui,d,"vip.name",en,("Nazwa postaci","Character name")); text(ui,d,"vip.group",en,("Grupa","Group"));
        text(ui,d,"vip.note",en,("Opis kontaktu","Contact description")); flag(ui,d,"vip.notify",en,("Powiadamiaj o logowaniu","Notify on sign-in"));
        ui.horizontal_wrapped(|ui| { action(ui,en,("Dodaj kontakt","Add contact")); action(ui,en,("Zapisz kontakt","Save contact")); action(ui,en,("Usuń kontakt","Remove contact")); });
    });
}

#[rustfmt::skip]
fn quests(ui: &mut egui::Ui, d: &mut Draft, en: bool) -> bool {
    text(ui,d,"quest.search",en,("Wyszukaj zadanie lub misję","Search quest or mission"));
    choice(ui,d,"quest.state",en,("Stan","State"),&[ALL,("Aktywne","Active"),("Ukończone","Completed")]);
    choice(ui,d,"quest.sort",en,("Kolejność","Order"),&[("Nazwa","Name"),("Stan","State")]);
    ui.horizontal_wrapped(|ui| { flag(ui,d,"quest.completed",en,("Pokaż ukończone","Show completed")); flag(ui,d,"quest.hidden",en,("Pokaż ukryte","Show hidden")); });
    values(ui,"quest.counts",en,&[("Ukończone zadania","Completed quests"),("Ukryte zadania","Hidden quests")]);
    ui.columns(2,|columns| {
        table(&mut columns[0],"quest.tree",en,&[("Zadanie","Quest"),("Stan","State")]);
        table(&mut columns[1],"quest.missions",en,&[("Misja","Mission"),("Postęp","Progress")]);
    });
    details(ui,"quest.description",en,("Opis wybranej misji","Selected mission description"));
    ui.horizontal_wrapped(|ui| { action(ui,en,("Poprzednia misja","Previous mission")); action(ui,en,("Następna misja","Next mission")); action(ui,en,("Śledź misję","Track mission")); action(ui,en,("Usuń śledzenie","Stop tracking")); });
    ui.button(tr(en,("Śledzenie zadań","Quest Tracker"))).clicked()
}

#[rustfmt::skip]
fn highscores(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    text(ui,d,"ranking.world",en,("Świat","World"));
    choice(ui,d,"ranking.vocation",en,("Profesja","Vocation"),VOCATIONS);
    text(ui,d,"ranking.category",en,("Kategoria rankingu","Ranking category"));
    choice(ui,d,"ranking.pvp",en,("Tryb PvP","PvP filter"),&[ALL,("PvP","PvP"),("Bez PvP","Non-PvP")]);
    table(ui,"ranking.list",en,&[("Miejsce","Rank"),("Postać","Character"),("Profesja","Vocation"),("Świat","World"),("Wynik","Score")]);
    values(ui,"ranking.own",en,&[("Twoje miejsce","Your rank")]);
    ui.horizontal_wrapped(|ui| { action(ui,en,("Poprzednia strona","Previous page")); action(ui,en,("Następna strona","Next page")); action(ui,en,("Pokaż moją postać","Show my character")); });
}

#[rustfmt::skip]
fn wheel(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    tabs(ui,d,en,&[("Koło","Wheel"),("Pracownia klejnotów","Gem Atelier"),("Warsztat fragmentów","Fragment Workshop")]);
    match d.tab {
        0 => {
            local_presets(ui,d,en);
            table(ui,"wheel.server-presets",en,&[("Zapisany profil koła","Saved wheel preset"),("Dostępność","Availability")]);
            values(ui,"wheel.points",en,&[("Dostępne punkty awansu","Available promotion points"),("Przydzielone punkty","Allocated points")]);
            choice(ui,d,"wheel.view",en,("Widok","View"),&[("Układ koła","Wheel layout"),("Korzyści","Perks"),("Naczynia","Vessels")]);
            match d.choices["wheel.view"] {
                0 => {
                    ui.group(|ui| { ui.strong(tr(en,("Układ koła","Wheel layout"))); ui.add_space(35.0);
                        ui.weak(tr(en,("Nie otrzymano układu koła.","No wheel layout has been received."))); ui.add_space(35.0); });
                    values(ui,"wheel.summary",en,&[("Wybrana korzyść","Selected perk"),("Ranga korzyści","Perk rank")]);
                }
                1 => {
                    text(ui,d,"wheel.perk-search",en,("Wyszukaj korzyść","Search perk"));
                    table(ui,"wheel.perks",en,&[("Korzyść","Perk"),("Ranga","Rank"),("Punkty","Points")]);
                    details(ui,"wheel.perk-details",en,("Opis i wymagania wybranej korzyści","Selected-perk description and requirements"));
                }
                _ => {
                    table(ui,"wheel.vessels",en,&[("Naczynie","Vessel"),("Klejnot","Gem"),("Dostępność","Availability")]);
                    details(ui,"wheel.vessel-details",en,("Wybrane naczynie i wymagania","Selected vessel and requirements"));
                    if ui.button(tr(en,("Otwórz pracownię klejnotów","Open Gem Atelier"))).clicked() { d.tab=1; }
                }
            }
            ui.horizontal_wrapped(|ui| { action(ui,en,("Przydziel punkty","Allocate points")); action(ui,en,("Zastosuj profil","Apply preset")); action(ui,en,("Wyzeruj koło","Reset wheel")); });
        }
        1 => atelier_columns(ui,d,en,145.0,gem_revelation,gem_collection),
        _ => atelier_columns(ui,d,en,(ui.available_width()*0.34).clamp(145.0,230.0),fragment_grades,fragment_collection),
    }
}

fn local_presets(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    egui::CollapsingHeader::new(tr(en, ("Szkice profili", "Preset drafts")))
        .id_salt("wheel.local-presets")
        .default_open(true)
        .show(ui, |ui| {
            let previous = d.wheel.selected;
            let selected = previous
                .and_then(|index| d.wheel.names.get(index))
                .map_or("—", String::as_str);
            egui::ComboBox::from_id_salt("wheel.local-preset")
                .width(ui.available_width())
                .truncate()
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut d.wheel.selected, None, "—");
                    for (index, name) in d.wheel.names.iter().enumerate() {
                        ui.selectable_value(
                            &mut d.wheel.selected,
                            Some(index),
                            format!("{} · {name}", index + 1),
                        );
                    }
                });
            if previous != d.wheel.selected {
                d.texts.insert(
                    "wheel.preset",
                    d.wheel
                        .selected
                        .and_then(|index| d.wheel.names.get(index))
                        .cloned()
                        .unwrap_or_default(),
                );
            }
            text(ui, d, "wheel.preset", en, ("Nazwa szkicu", "Draft name"));
            let name = d.texts.get("wheel.preset").cloned().unwrap_or_default();
            let has_selected = d.wheel.selected.is_some();
            let valid_name = !name.trim().is_empty();
            let can_create = d.wheel.names.len() < 16;
            let mut changed = false;
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(
                        valid_name && can_create,
                        egui::Button::new(tr(en, ("Dodaj", "Add"))),
                    )
                    .clicked()
                {
                    changed = d.wheel.create(&name);
                }
                if ui
                    .add_enabled(
                        has_selected && can_create,
                        egui::Button::new(tr(en, ("Kopiuj", "Copy"))),
                    )
                    .clicked()
                {
                    changed = d.wheel.copy();
                }
                if ui
                    .add_enabled(
                        has_selected && valid_name,
                        egui::Button::new(tr(en, ("Zmień nazwę", "Rename"))),
                    )
                    .clicked()
                {
                    changed = d.wheel.rename(&name);
                }
                if ui
                    .add_enabled(has_selected, egui::Button::new(tr(en, ("Usuń", "Remove"))))
                    .clicked()
                {
                    d.wheel.remove();
                    changed = true;
                }
            });
            if changed {
                d.texts.insert(
                    "wheel.preset",
                    d.wheel
                        .selected
                        .and_then(|index| d.wheel.names.get(index))
                        .cloned()
                        .unwrap_or_default(),
                );
            }
        });
}

// Layout follows the privately verified Gem/Fragment captures, not the unverified wheel.
fn atelier_columns(
    ui: &mut egui::Ui,
    d: &mut Draft,
    en: bool,
    left_width: f32,
    left: fn(&mut egui::Ui, &mut Draft, bool),
    right: fn(&mut egui::Ui, &mut Draft, bool),
) {
    // Leave room for frame/font pixel rounding at narrow scales.
    let width = (ui.available_width() - 2.0).max(1.0);
    ui.scope(|ui| {
        ui.set_max_width(width);
        if width < 360.0 {
            left(ui, d, en);
            ui.separator();
            right(ui, d, en);
            return;
        }
        let right_width = (width - left_width - ui.spacing().item_spacing.x).max(1.0);
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                Vec2::new(left_width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(left_width);
                    left(ui, d, en);
                },
            );
            ui.allocate_ui_with_layout(
                Vec2::new(right_width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(right_width);
                    right(ui, d, en);
                },
            );
        });
    });
}

fn atelier_search(ui: &mut egui::Ui, d: &mut Draft, key: &'static str, en: bool) {
    ui.add(
        egui::TextEdit::singleline(d.texts.entry(key).or_default())
            .hint_text(tr(en, ("Szukaj", "Search")))
            .char_limit(96)
            .desired_width(ui.available_width().min(155.0)),
    );
    if ui
        .button("×")
        .on_hover_text(tr(en, ("Wyczyść wyszukiwanie", "Clear search")))
        .clicked()
    {
        d.texts.remove(key);
    }
}

fn atelier_filter(
    ui: &mut egui::Ui,
    d: &mut Draft,
    key: &'static str,
    en: bool,
    all: Label,
    label: Label,
) {
    // The captures expose only the closed "All" selector. Exact enum choices are unknown.
    let selected = d
        .texts
        .get(key)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .unwrap_or_else(|| tr(en, all).to_owned());
    let width = ui.available_width().min(125.0);
    ui.allocate_ui_with_layout(
        Vec2::new(width, ui.spacing().interact_size.y),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            egui::ComboBox::from_id_salt(key)
                .width(width)
                .truncate()
                .selected_text(selected)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(!d.texts.contains_key(key), tr(en, all))
                        .clicked()
                    {
                        d.texts.remove(key);
                        ui.close();
                    }
                    text(ui, d, key, en, label);
                });
        },
    );
}

fn unknown_region(ui: &mut egui::Ui, en: bool, title: Label, height: f32) {
    ui.group(|ui| {
        ui.set_min_width((ui.available_width() - 1.0).max(1.0));
        ui.strong(tr(en, title));
        ui.allocate_ui_with_layout(
            Vec2::new(ui.available_width(), height),
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
            |ui| {
                ui.weak(tr(
                    en,
                    ("Nie otrzymano danych.", "No data has been received."),
                ));
            },
        );
    });
}

fn gem_revelation(ui: &mut egui::Ui, _d: &mut Draft, en: bool) {
    ui.group(|ui| {
        ui.strong(tr(en, ("Naczynia", "Vessels")));
        let side = ui.available_width().min(92.0);
        let (rect, response) = ui.allocate_exact_size(Vec2::splat(side), egui::Sense::hover());
        let color = Color32::from_rgb(184, 154, 101);
        let painter = ui.painter();
        let c = rect.center();
        for offset in [
            Vec2::new(-0.32, -0.32),
            Vec2::new(0.32, -0.32),
            Vec2::new(-0.32, 0.32),
            Vec2::new(0.32, 0.32),
        ] {
            painter.circle_stroke(c + offset * side, side * 0.13, Stroke::new(1.0, color));
        }
        painter.text(
            c,
            egui::Align2::CENTER_CENTER,
            "—",
            egui::FontId::proportional(16.0),
            Color32::GRAY,
        );
        response.on_hover_text(tr(
            en,
            ("Dane naczyń są niedostępne.", "Vessel data is unavailable."),
        ));
    });
    ui.strong(tr(en, ("Odkrywanie klejnotów", "Gem Revelation")));
    // Three observed revelation tiers; target item identities and quantities are not known.
    for tier in [
        ("Mniejszy klejnot", "Lesser Gem"),
        ("Klejnot", "Gem"),
        ("Większy klejnot", "Greater Gem"),
    ] {
        ui.push_id(tier.1, |ui| {
            ui.group(|ui| {
                ui.add(egui::Label::new(tr(en, tier)).wrap());
                values(
                    ui,
                    "gem.revelation",
                    en,
                    &[("Ilość", "Quantity"), ("Koszt", "Cost")],
                );
                action(ui, en, ("Odkryj", "Reveal"));
            });
        });
    }
}

fn gem_collection(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    unknown_region(
        ui,
        en,
        ("Modyfikatory wybranego klejnotu", "Selected gem modifiers"),
        65.0,
    );
    ui.horizontal_wrapped(|ui| {
        atelier_search(ui, d, "gems.search", en);
        atelier_filter(
            ui,
            d,
            "gems.affinity",
            en,
            ("Wszystkie powinowactwa", "All affinities"),
            ("Filtr powinowactwa", "Affinity filter"),
        );
        atelier_filter(
            ui,
            d,
            "gems.quality",
            en,
            ("Wszystkie jakości", "All qualities"),
            ("Filtr jakości", "Quality filter"),
        );
        flag(
            ui,
            d,
            "gems.locked",
            en,
            ("Tylko zablokowane", "Locked only"),
        );
    });
    pagination(ui, d, "gems.page", en);
    unknown_region(ui, en, ("Kolekcja klejnotów", "Gem collection"), 170.0);
    if ui
        .button(tr(
            en,
            ("Otwórz warsztat fragmentów", "Open Fragment Workshop"),
        ))
        .clicked()
    {
        d.tab = 2;
    }
}

fn fragment_grades(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    ui.strong(tr(
        en,
        ("Ulepszanie klasy modyfikatora", "Enhance modifier grade"),
    ));
    ui.weak(tr(en, ("Wybrany modyfikator: —", "Selected modifier: —")));
    let mut centers = Vec::new();
    for (ordinal, roman) in [(4, "IV"), (3, "III"), (2, "II"), (1, "I")] {
        ui.horizontal(|ui| {
            let (rect, response) = ui.allocate_exact_size(Vec2::splat(32.0), egui::Sense::click());
            let center = rect.center();
            centers.push(center);
            let chosen = d.choices.get("fragments.grade") == Some(&ordinal);
            let color = if chosen {
                Color32::from_rgb(184, 154, 101)
            } else {
                Color32::from_gray(100)
            };
            ui.painter()
                .circle_stroke(center, 12.0, Stroke::new(1.5, color));
            ui.painter().text(
                center,
                egui::Align2::CENTER_CENTER,
                "—",
                egui::FontId::proportional(11.0),
                Color32::GRAY,
            );
            if response
                .on_hover_text(tr(
                    en,
                    ("Wybierz klasę do podglądu", "Select grade to preview"),
                ))
                .clicked()
            {
                d.choices.insert("fragments.grade", ordinal);
            }
            ui.vertical(|ui| {
                ui.label(format!("{} {roman}", tr(en, ("Klasa", "Grade"))));
                ui.weak("—");
            });
        });
        ui.add_space(25.0);
    }
    for segment in centers.windows(2) {
        ui.painter().line_segment(
            [
                segment[0] + Vec2::new(0.0, 13.0),
                segment[1] - Vec2::new(0.0, 13.0),
            ],
            Stroke::new(1.5, Color32::from_gray(80)),
        );
    }
    values(
        ui,
        "fragment.enhancement-cost",
        en,
        &[
            ("Koszt ulepszenia", "Enhancement cost"),
            ("Wymagane zasoby", "Required resources"),
        ],
    );
    action(ui, en, ("Ulepsz", "Enhance"));
}

fn fragment_collection(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    ui.horizontal_wrapped(|ui| {
        atelier_search(ui, d, "fragments.search", en);
        atelier_filter(
            ui,
            d,
            "fragments.effect",
            en,
            ALL,
            ("Filtr modyfikatora", "Modifier filter"),
        );
    });
    pagination(ui, d, "fragments.page", en);
    unknown_region(ui, en, ("Modyfikatory", "Modifiers"), 270.0);
}

#[rustfmt::skip]
fn prey(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    ui.horizontal_wrapped(|ui| { for index in 0..3 { ui.selectable_value(&mut d.sector,index,format!("{} {}",tr(en,("Slot","Slot")),index+1)); } });
    values(ui,"prey.slot",en,&[("Dostępność slotu","Slot availability"),("Wybrane stworzenie","Selected creature"),("Premia","Bonus"),("Pozostały czas","Time remaining"),("Koszt losowania","Reroll cost")]);
    text(ui,d,"prey.search",en,("Wyszukaj stworzenie","Search creature"));
    table(ui,"prey.creatures",en,&[("Stworzenie","Creature"),("Premia","Bonus"),("Ocena","Rating")]);
    flag(ui,d,"prey.automatic",en,("Automatyczne odnowienie premii","Automatically renew bonus"));
    ui.horizontal_wrapped(|ui| { action(ui,en,("Wybierz stworzenie","Choose creature")); action(ui,en,("Losuj stworzenia","Reroll creatures")); action(ui,en,("Losuj premię","Reroll bonus")); action(ui,en,("Odblokuj slot","Unlock slot")); });
}

#[rustfmt::skip]
fn forge(ui: &mut egui::Ui, d: &mut Draft, en: bool, state: &GameState) {
    tabs(ui,d,en,&[("Fuzja","Fusion"),("Przeniesienie","Transfer"),("Konwersja","Conversion"),("Historia","History")]);
    match d.tab {
        0 | 1 => {
            item_slot(ui,d,"forge.source",en,("Przedmiot źródłowy","Source item"),state);
            item_slot(ui,d,"forge.destination",en,("Przedmiot docelowy","Destination item"),state);
            if d.tab==0 { flag(ui,d,"forge.convergence",en,("Konwergencja","Convergence")); flag(ui,d,"forge.mitigation",en,("Ochrona przed utratą poziomu","Tier-loss mitigation")); }
            values(ui,"forge.chance",en,&[("Szansa powodzenia","Success probability"),("Poziom przedmiotu","Item tier")]);
            table(ui,"forge.resources",en,&[("Zasób","Resource"),("Wymagane","Required"),("Posiadane","Owned")]);
            details(ui,"forge.requirements",en,("Wymagania wybranych przedmiotów","Selected-item requirements"));
            action(ui,en,if d.tab==0 {("Wykonaj fuzję","Fuse items")} else {("Przenieś poziom","Transfer tier")});
        }
        2 => {
            values(ui,"forge.balances",en,&[("Pył","Dust"),("Odłamki","Slivers"),("Rdzenie","Cores"),("Limit pyłu","Dust limit")]);
            choice(ui,d,"forge.convert",en,("Operacja","Operation"),&[("Pył na odłamki","Dust to slivers"),("Odłamki na rdzenie","Slivers to cores"),("Zwiększenie limitu pyłu","Increase dust limit")]);
            table(ui,"forge.conversion.cost",en,&[("Zasób","Resource"),("Koszt","Cost"),("Wynik","Result")]); action(ui,en,("Konwertuj","Convert"));
        }
        _ => table(ui,"forge.history",en,&[("Data","Date"),("Operacja","Operation"),("Przedmiot","Item"),("Wynik","Result")]),
    }
}

#[rustfmt::skip]
fn tasks(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    tabs(ui,d,en,&[("Zlecenia łowieckie","Bounty Tasks"),("Zadania tygodniowe","Weekly Tasks"),("Sklep zleceń","Hunting Task Shop")]);
    if d.tab<2 { text(ui,d,"tasks.difficulty",en,("Trudność","Difficulty")); action(ui,en,("Zmień trudność","Change difficulty")); }
    match d.tab {
        0 => {
            ui.horizontal_wrapped(|ui| { for index in 0..3 { ui.selectable_value(&mut d.sector,index,format!("{} {}",tr(en,("Kandydat","Candidate")),index+1)); } });
            values(ui,"tasks.candidate",en,&[("Stworzenie","Creature"),("Wymagane zabójstwa","Required kills"),("Nagroda","Reward"),("Koszt ponownego losowania","Reroll cost")]); progress(ui,en,("Postęp zlecenia","Task progress"));
            table(ui,"tasks.preferred",en,&[("Preferowane zlecenie","Preferred task"),("Trudność","Difficulty")]);
            ui.horizontal_wrapped(|ui| { action(ui,en,("Wybierz zlecenie","Select task")); action(ui,en,("Losuj ponownie","Reroll")); action(ui,en,("Odbierz nagrodę","Claim reward")); action(ui,en,("Dodaj do preferowanych","Add to preferred tasks")); });
            details(ui,"tasks.talisman",en,("Talizman zleceń i ulepszenia","Bounty talisman and upgrades")); action(ui,en,("Ulepsz talizman","Upgrade talisman"));
        }
        1 => {
            progress(ui,en,("Zadania za zabójstwa","Kill tasks")); progress(ui,en,("Zadania za dostawy","Delivery tasks"));
            table(ui,"tasks.weekly",en,&[("Zadanie","Task"),("Postęp","Progress"),("Nagroda","Reward")]); action(ui,en,("Odbierz nagrody tygodniowe","Claim weekly rewards"));
        }
        _ => {
            choice(ui,d,"tasks.shop",en,("Kategoria","Category"),&[ALL,("Stroje","Outfits"),("Dodatki","Addons"),("Wierzchowce","Mounts")]);
            table(ui,"tasks.shop.offers",en,&[("Oferta","Offer"),("Koszt","Cost"),("Dostępność","Availability")]); action(ui,en,("Kup wybraną nagrodę","Buy selected reward"));
        }
    }
}

#[rustfmt::skip]
fn social(ui: &mut egui::Ui, d: &mut Draft, en: bool, party: bool) {
    if party {
        choice(ui,d,"party.sort",en,("Sortuj członków","Sort members"),&[("Nazwa","Name"),("Rola","Role")]);
        table(ui,"party.members",en,&[("Członek","Member"),("Rola","Role"),("Stan","Status")]);
        text(ui,d,"party.invite",en,("Postać do zaproszenia","Character to invite"));
        ui.horizontal_wrapped(|ui| { action(ui,en,("Zaproś","Invite")); action(ui,en,("Przekaż przywództwo","Transfer leadership")); action(ui,en,("Opuść drużynę","Leave party")); }); return;
    }
    tabs(ui,d,en,&[("Znajdź drużynę","Find team"),("Utwórz drużynę","Assemble team"),("Znajomi","Friends"),("Zaproszenia","Invitations"),("Wyszukaj konto","Account search")]);
    if d.tab<2 {
        text(ui,d,"social.activity",en,("Aktywność","Activity")); text(ui,d,"social.world",en,("Świat","World"));
        text(ui,d,"social.minimum",en,("Minimalny poziom","Minimum level")); text(ui,d,"social.maximum",en,("Maksymalny poziom","Maximum level"));
        choice(ui,d,"social.vocation",en,("Profesja","Vocation"),VOCATIONS);
        if d.tab==0 { table(ui,"social.teams",en,&[("Drużyna","Team"),("Aktywność","Activity"),("Członkowie","Members")]); action(ui,en,("Dołącz do drużyny","Join team")); }
        else { text(ui,d,"social.teamname",en,("Nazwa drużyny","Team name")); text(ui,d,"social.description",en,("Opis i wymagania","Description and requirements")); action(ui,en,("Opublikuj drużynę","Publish team")); }
    } else {
        text(ui,d,"social.account",en,("Nazwa konta lub postaci","Account or character name"));
        table(ui,"social.accounts",en,&[("Konto","Account"),("Stan","Status"),("Odznaki","Badges")]);
        ui.horizontal_wrapped(|ui| { action(ui,en,("Dodaj znajomego","Add friend")); action(ui,en,("Akceptuj zaproszenie","Accept invitation")); action(ui,en,("Odrzuć zaproszenie","Decline invitation")); });
    }
}

#[rustfmt::skip]
fn rewards(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    tabs(ui,d,en,&[("Cykl nagród","Reward cycle"),("Historia","History")]);
    if d.tab==1 { table(ui,"reward.history",en,&[("Data","Date"),("Dzień cyklu","Cycle day"),("Nagroda","Reward")]); return; }
    ui.horizontal_wrapped(|ui| { for day in 1..=7 { ui.group(|ui| { ui.label(format!("{} {day}",tr(en,("Dzień","Day")))); ui.label("—"); }); } });
    values(ui,"reward.state",en,&[("Seria odbiorów","Claim streak"),("Jokery","Jokers"),("Stan odbioru","Claim state")]);
    details(ui,"reward.rest",en,("Premie odpoczynku","Resting bonuses"));
    ui.horizontal_wrapped(|ui| { action(ui,en,("Odbierz nagrodę","Claim reward")); action(ui,en,("Użyj jokera","Use joker")); });
}

#[rustfmt::skip]
fn bestiary(ui: &mut egui::Ui, d: &mut Draft, en: bool, bosses: bool) {
    text(ui,d,"bestiary.search",en,if bosses {("Wyszukaj bossa","Search boss")} else {("Wyszukaj stworzenie","Search creature")});
    text(ui,d,"bestiary.category",en,("Kategoria","Category"));
    choice(ui,d,"bestiary.stage",en,("Postęp","Progress"),&[ALL,("Nieodkryte","Undiscovered"),("Odkryte","Discovered"),("Ukończone","Completed")]);
    table(ui,"bestiary.list",en,&[("Nazwa","Name"),("Zabójstwa","Kills"),("Etap","Stage")]); progress(ui,en,("Postęp wybranego wpisu","Selected-entry progress"));
    details(ui,"bestiary.details",en,("Statystyki i szczegóły","Statistics and details"));
    ui.horizontal_wrapped(|ui| { action(ui,en,("Śledź wpis","Track entry")); action(ui,en,("Usuń śledzenie","Stop tracking")); });
}

#[rustfmt::skip]
fn boss_slots(ui: &mut egui::Ui, en: bool) {
    values(ui,"boss.boosted",en,&[("Wzmocniony boss","Boosted boss")]);
    for slot in 1..=2 {
        ui.group(|ui| { ui.strong(format!("{} {slot}",tr(en,("Slot bossa","Boss slot"))));
            values(ui,&format!("boss.slot.{slot}"),en,&[("Przypisany boss","Assigned boss"),("Dostępność","Availability"),("Premia","Bonus")]);
            details(ui,&format!("boss.requirements.{slot}"),en,("Warunki odblokowania","Unlock requirements")); action(ui,en,("Wybierz bossa","Choose boss")); });
    }
}

#[rustfmt::skip]
fn tracker(ui: &mut egui::Ui, d: &mut Draft, en: bool, id: &str) -> Option<Route> {
    text(ui,d,"tracker.search",en,("Wyszukaj śledzony wpis","Search tracked entry"));
    ui.menu_button(tr(en,("Opcje śledzenia","Tracking options")),|ui| {
        flag(ui,d,"tracker.completed",en,("Pokaż ukończone","Show completed entries"));
        if id=="quest-tracker" { flag(ui,d,"tracker.add",en,("Automatycznie dodawaj nowe zadania","Automatically add new quests")); flag(ui,d,"tracker.remove",en,("Automatycznie usuwaj ukończone","Automatically remove completed quests")); }
        action(ui,en,("Usuń wybrany wpis","Remove selected entry")); action(ui,en,("Usuń wszystkie wpisy","Remove all entries")); action(ui,en,("Usuń ukończone","Remove completed entries"));
    });
    table(ui,"tracker.list",en,&[("Wpis","Entry"),("Postęp","Progress"),("Stan","Status")]);
    progress(ui,en,("Postęp wybranego wpisu","Selected-entry progress"));
    let (label,target,page) = match id {
        "quest-tracker" => (("Otwórz dziennik zadań","Open quest log"),"quest-log",None),
        "bestiary-tracker" => (("Otwórz bestiariusz","Open Bestiary"),"cyclopedia",Some(4)),
        "bosstiary-tracker" => (("Otwórz bestiariusz bossów","Open Bosstiary"),"cyclopedia",Some(6)),
        _ => (("Otwórz łowy","Open Prey"),"prey",None),
    };
    ui.button(tr(en,label)).clicked().then_some((target,page))
}

#[rustfmt::skip]
fn proficiency(ui: &mut egui::Ui, d: &mut Draft, en: bool, state: &GameState) {
    text(ui,d,"weapon.search",en,("Wyszukaj broń","Search weapon")); text(ui,d,"weapon.type",en,("Typ broni","Weapon type"));
    table(ui,"weapon.catalogue",en,&[("Broń","Weapon"),("Typ","Type"),("Wymagania","Requirements")]);
    item_slot(ui,d,"weapon.selected",en,("Wybrany przedmiot","Selected item"),state);
    progress(ui,en,("Doświadczenie broni","Weapon experience"));
    table(ui,"weapon.perks",en,&[("Korzyść","Perk"),("Ranga","Rank"),("Warunki","Requirements")]);
    details(ui,"weapon.requirements",en,("Dostępność korzyści i wymagania","Perk availability and requirements"));
    ui.horizontal_wrapped(|ui| { action(ui,en,("Przypisz korzyść","Assign perk")); action(ui,en,("Wyzeruj korzyści","Reset perks")); });
}

#[rustfmt::skip]
fn imbuements(ui: &mut egui::Ui, d: &mut Draft, en: bool, state: &GameState) {
    item_slot(ui,d,"imbuements.item",en,("Przedmiot ekwipunku","Equipment item"),state);
    table(ui,"imbuements.slots",en,&[("Slot nasycenia","Imbuement slot"),("Efekt","Effect"),("Pozostały czas","Time remaining")]);
    flag(ui,d,"imbuements.inactive",en,("Pokaż nieaktywne nasycenia","Show inactive imbuements"));
    details(ui,"imbuements.details",en,("Szczegóły wybranego efektu","Selected-effect details"));
}

#[rustfmt::skip]
fn unjustified(ui: &mut egui::Ui, en: bool) {
    values(ui,"unjustified.points",en,&[("Aktualne punkty","Current points")]);
    for label in [("Krótszy okres","Short period"),("Średni okres","Medium period"),("Dłuższy okres","Long period")] { progress(ui,en,label); }
    details(ui,"unjustified.thresholds",en,("Progi i czas odzyskania","Thresholds and recovery time"));
}

#[rustfmt::skip]
fn analytics(ui: &mut egui::Ui, d: &mut Draft, en: bool) -> bool {
    let mut open_items = false;
    let labels = [("Polowanie","Hunting"),("Łupy","Loot"),("Zużycie","Supply"),("Obrażenia i leczenie","Impact"),("Otrzymane obrażenia","Damage input"),("Doświadczenie","XP"),("Zdobycze","Drops"),("Polowanie drużynowe","Party hunt"),("Bossowie","Boss cooldowns")];
    choice(ui,d,"analytics.kind",en,("Analizator","Analyzer"),&labels);
    let kind=d.choices["analytics.kind"];
    ui.menu_button(tr(en,("Widok i dane","View and data")),|ui| {
        for (key,label) in [("analytics.session",("Wartości sesji","Session values")),("analytics.hour",("Wartości na godzinę","Per-hour values")),("analytics.graph",("Wykres","Graph")),("analytics.gauge",("Wskaźnik","Gauge"))] { flag(ui,d,key,en,label); }
        if kind==3 || kind==4 { flag(ui,d,"analytics.types",en,("Rodzaje obrażeń","Damage types")); flag(ui,d,"analytics.sources",en,("Źródła obrażeń","Damage sources")); }
        if kind==5 { flag(ui,d,"analytics.rawxp",en,("Surowe doświadczenie","Raw experience")); }
        action(ui,en,("Wyzeruj dane","Reset data")); action(ui,en,("Wyzeruj rekord","Reset all-time high")); action(ui,en,("Kopiuj wyniki","Copy results"));
    });
    match kind {
        1 | 2 => { values(ui,"analytics.value",en,&[("Czas sesji","Session duration"),("Łączna wartość","Total value"),("Wartość na godzinę","Value per hour")]); table(ui,"analytics.items",en,&[("Przedmiot","Item"),("Ilość","Quantity"),("Wartość","Value")]); }
        3 => { values(ui,"analytics.impact",en,&[("Obrażenia","Damage"),("DPS","DPS"),("Maksymalne DPS","Maximum DPS"),("Rekord DPS","All-time DPS"),("Leczenie","Healing"),("HPS","HPS"),("Maksymalne HPS","Maximum HPS"),("Rekord HPS","All-time HPS")]); table(ui,"analytics.impact.types",en,&[("Rodzaj","Type"),("Wartość","Value")]); }
        4 => { values(ui,"analytics.received",en,&[("Otrzymane obrażenia","Damage received"),("Maksymalne DPS","Maximum incoming DPS")]); table(ui,"analytics.sources",en,&[("Źródło","Source"),("Rodzaj","Type"),("Obrażenia","Damage")]); }
        5 => { values(ui,"analytics.xp",en,&[("Zdobyte doświadczenie","Experience gained"),("Doświadczenie na godzinę","XP per hour")]); progress(ui,en,("Postęp do poziomu","Next-level progress")); }
        6 => { table(ui,"analytics.drops",en,&[("Śledzony przedmiot","Tracked item"),("Zdobycze","Drops")]); open_items = ui.button(tr(en,("Dodaj śledzoną zdobycz","Add tracked drop"))).clicked(); }
        7 => { values(ui,"analytics.party",en,&[("Czas polowania","Hunt duration"),("Wycena łupów","Loot-price mode")]); table(ui,"analytics.party.members",en,&[("Członek","Member"),("Łupy","Loot"),("Zużycie","Supply"),("Bilans","Balance")]); }
        8 => { choice(ui,d,"analytics.boss.sort",en,("Sortuj","Sort"),&[("Odnowienie","Cooldown"),("Nazwa","Name")]); table(ui,"analytics.bosses",en,&[("Boss","Boss"),("Odnowienie","Cooldown")]); }
        _ => { values(ui,"analytics.hunt",en,&[("Czas polowania","Hunt duration"),("Doświadczenie","Experience"),("Łupy","Loot"),("Zużycie","Supply"),("Bilans","Balance")]); }
    }
    if *d.flags.get("analytics.graph").unwrap_or(&false) { ui.group(|ui| { ui.label(tr(en,("Wykres sesji","Session graph"))); ui.add_space(45.0); ui.label("—"); }); }
    if *d.flags.get("analytics.gauge").unwrap_or(&false) { progress(ui,en,("Wskaźnik analizatora","Analyzer gauge")); }
    open_items
}

#[rustfmt::skip]
fn cyclopedia(ui: &mut egui::Ui, d: &mut Draft, en: bool, state: &GameState) {
    let pages = [("Mapa","Map"),("Przedmioty","Items"),("Domy","Houses"),("Postać","Character"),("Bestiariusz","Bestiary"),("Talizmany","Charms"),("Bestiariusz bossów","Bosstiary"),("Sloty bossów","Boss slots"),("Archiwum magii","Magical archive"),("Statystyki postaci","Character stats"),("Atak","Offence"),("Obrona","Defence"),("Błogosławieństwa","Blessings"),("Ostatnie śmierci","Recent deaths"),("Zabójstwa PvP","PvP kills"),("Osiągnięcia","Achievements"),("Podsumowanie przedmiotów","Item summary"),("Wygląd","Appearances"),("Korzyści konta","Account benefits"),("Tytuły","Titles")];
    choice(ui,d,"cyclopedia.page",en,("Strona","Page"),&pages);
    match d.choices["cyclopedia.page"] {
        0 => { text(ui,d,"map.location",en,("Miejsce","Location")); flag(ui,d,"map.npcs",en,("Pokaż NPC","Show NPCs")); flag(ui,d,"map.houses",en,("Pokaż domy","Show houses")); values(ui,"map.position",en,&[("Piętro","Floor"),("Położenie","Position")]); ui.group(|ui| { ui.label(tr(en,("Mapa świata","World map"))); ui.add_space(60.0); ui.label("—"); }); action(ui,en,("Przejdź do własnej postaci","Locate own character")); }
        1 => { text(ui,d,"items.search",en,("Wyszukaj przedmiot","Search item")); text(ui,d,"items.category",en,("Kategoria","Category")); table(ui,"items.list",en,&[("Przedmiot","Item"),("Kategoria","Category")]); values(ui,"items.attributes",en,&[("Pancerz","Armor"),("Waga","Weight"),("Klasyfikacja","Classification"),("Wartość rynkowa","Market value")]); table(ui,"items.trade",en,&[("Kupiec","Trader"),("Kupno","Buy"),("Sprzedaż","Sell")]); text(ui,d,"items.valuation",en,("Własna wycena łupu","Own loot valuation")); flag(ui,d,"items.track",en,("Śledź zdobycze","Track drops")); flag(ui,d,"items.loot",en,("Akceptuj ten łup","Accept this loot")); action(ui,en,("Zapisz wycenę i łupy","Save valuation and loot preferences")); }
        2 => { choice(ui,d,"houses.type",en,("Rodzaj","Type"),&[ALL,("Domy","Houses"),("Siedziby gildii","Guildhalls")]); text(ui,d,"houses.city",en,("Miasto","Town")); table(ui,"houses.list",en,&[("Dom","House"),("Wielkość","Size"),("Stan","State")]); details(ui,"houses.details",en,("Wybrany dom","Selected house")); action(ui,en,("Licytuj dom","Bid on house")); }
        3 | 9 => { if let Some(v)=state.vitals { values(ui,"character.summary",en,&[("Poziom","Level"),("Doświadczenie","Experience")]); ui.label(format!("HP {} / {} · MP {} / {}",v.health,v.max_health,v.mana,v.max_mana)); ui.label(format!("Soul {} · {} {}",v.soul,tr(en,("Harmonia","Harmony")),v.harmony)); ui.label(format!("Serene: {}",tr(en,if v.serene {("aktywne","active")} else {("nieaktywne","inactive")}))); } else { values(ui,"character.summary",en,&[("Poziom","Level"),("Doświadczenie","Experience"),("Zdrowie","Health"),("Mana","Mana"),("Soul","Soul"),("Harmonia","Harmony"),("Serene","Serene")]); } table(ui,"character.skills",en,&[("Umiejętność","Skill"),("Poziom","Level"),("Postęp","Progress")]); values(ui,"character.resources",en,&[("Udźwig","Capacity"),("Szybkość","Speed"),("Wytrzymałość","Stamina"),("Trening offline","Offline training"),("Stan konta","Account status")]); }
        4 => bestiary(ui,d,en,false),
        5 => { choice(ui,d,"charms.kind",en,("Talizmany","Charms"),&[("Główne","Major"),("Mniejsze","Minor")]); table(ui,"charms.list",en,&[("Talizman","Charm"),("Koszt","Cost"),("Etap","Stage")]); details(ui,"charms.rules",en,("Efekt i wymagania","Effect and requirements")); action(ui,en,("Odblokuj talizman","Unlock charm")); action(ui,en,("Przypisz do stworzenia","Assign to creature")); action(ui,en,("Usuń przypisanie","Unassign")); }
        6 => bestiary(ui,d,en,true), 7 => boss_slots(ui,en), 8 => spells(ui,d,en),
        10 => offence(ui,en), 11 => defence(ui,en),
        12 => table(ui,"character.blessings",en,&[("Błogosławieństwo","Blessing"),("Stan","Status"),("Efekt","Effect")]),
        13 => recent_records(ui,d,en,false), 14 => recent_records(ui,d,en,true),
        15 => achievements(ui,d,en),
        16 => item_summary(ui,d,en,state),
        17 => { choice(ui,d,"appearance.kind",en,("Rodzaj","Type"),&[("Stroje","Outfits"),("Wierzchowce","Mounts"),("Chowańce","Familiars")]); table(ui,"appearance.list",en,&[("Wygląd","Appearance"),("Dostępność","Availability")]); action(ui,en,("Zastosuj wygląd","Apply appearance")); }
        18 => table(ui,"account.benefits",en,&[("Korzyść","Benefit"),("Dostępność","Availability"),("Pozostały czas","Time remaining")]),
        _ => titles(ui,d,en),
    }
}

// The page cursor is a local selection. It is never a claim about loaded records or totals.
fn pagination(ui: &mut egui::Ui, d: &mut Draft, key: &'static str, en: bool) {
    let page = d.choices.entry(key).or_default();
    *page = (*page).min(65_534);
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                *page > 0,
                egui::Button::new(tr(en, ("Poprzednia", "Previous"))),
            )
            .clicked()
        {
            *page -= 1;
        }
        ui.label(tr(en, ("Wybrana strona", "Selected page")));
        let mut selected = (*page).min(65_534) as u16 + 1;
        if ui
            .add(egui::DragValue::new(&mut selected).range(1..=u16::MAX))
            .changed()
        {
            *page = usize::from(selected.saturating_sub(1));
        }
        if ui
            .add_enabled(
                *page < 65_534,
                egui::Button::new(tr(en, ("Następna", "Next"))),
            )
            .clicked()
        {
            *page += 1;
        }
        ui.weak(tr(en, ("Liczba stron: —", "Total pages: —")));
    });
}

#[rustfmt::skip]
fn achievements(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    ui.strong(tr(en,("Liczba osiągnięć według stopnia","Achievement counts by grade")));
    table(ui,"achievements.counts",en,&[("Stopień","Grade"),("Łącznie","Total"),("Zdobyte","Accomplished")]);
    text(ui,d,"achievements.search",en,("Wyszukaj osiągnięcie","Search achievement"));
    text(ui,d,"achievements.grade",en,("Filtr stopnia","Grade filter"));
    choice(ui,d,"achievements.accomplished",en,("Stan","Accomplishment"),&[ALL,("Zdobyte","Accomplished"),("Nie zdobyte","Not accomplished")]);
    choice(ui,d,"achievements.sort",en,("Sortuj","Sort"),&[("Nazwa","Name"),("Stopień","Grade"),("Data","Date")]);
    flag(ui,d,"achievements.descending",en,("Malejąco","Descending"));
    egui::ScrollArea::vertical().id_salt("achievements.entries").max_height(150.0).show(ui,|ui| {
        table(ui,"achievements.list",en,&[("Osiągnięcie","Achievement"),("Stopień","Grade"),("Data","Date")]);
    });
    pagination(ui,d,"achievements.page",en);
    values(ui,"achievements.selected",en,&[("Wybrane osiągnięcie","Selected achievement"),("Data zdobycia","Date accomplished")]);
    details(ui,"achievements.details",en,("Opis osiągnięcia","Achievement description"));
}

#[rustfmt::skip]
fn recent_records(ui: &mut egui::Ui, d: &mut Draft, en: bool, pvp: bool) {
    ui.strong(tr(en,if pvp {("Ostatnie zabójstwa PvP","Recent PvP kills")} else {("Ostatnie śmierci","Recent deaths")}));
    table(ui,if pvp {"records.pvp"} else {"records.deaths"},en,&[("Data","Date"),("Postać","Character"),("Zdarzenie","Event")]);
    pagination(ui,d,if pvp {"records.pvp.page"} else {"records.deaths.page"},en);
    details(ui,if pvp {"records.pvp.detail"} else {"records.deaths.detail"},en,("Szczegóły wybranego wpisu","Selected-entry details"));
}

// Directly observed fields and navigation: REFERENCE-AUDIT.md, Character stat subpages.
fn offence(ui: &mut egui::Ui, en: bool) {
    values(
        ui,
        "offence.totals",
        en,
        &[
            ("Stałe obrażenia", "Flat damage"),
            ("Stałe leczenie", "Flat healing"),
            ("Atak", "Attack"),
            ("Szansa krytyczna", "Critical chance"),
            ("Dodatkowe obrażenia krytyczne", "Extra critical damage"),
        ],
    );
    ui.strong(tr(en, ("Składniki ataku", "Attack contributions")));
    table(
        ui,
        "offence.contributions",
        en,
        &[
            ("Składnik", "Contribution"),
            ("Wartość", "Value"),
            ("Źródło", "Source"),
        ],
    );
}

fn defence(ui: &mut egui::Ui, en: bool) {
    values(
        ui,
        "defence.totals",
        en,
        &[
            ("Pancerz", "Armor"),
            ("Redukcja obrażeń", "Mitigation"),
            ("Pojemność tarczy magicznej", "Magic-shield capacity"),
        ],
    );
    ui.strong(tr(en, ("Składniki obrony", "Defence contributions")));
    values(
        ui,
        "defence.contributions",
        en,
        &[
            ("Obrona z wyposażenia", "Equipment defence"),
            ("Obrona z umiejętności", "Skill defence"),
        ],
    );
    ui.strong(tr(
        en,
        (
            "Redukcja według rodzaju obrażeń",
            "Reduction by damage type",
        ),
    ));
    values(
        ui,
        "defence.reductions",
        en,
        &[
            ("Fizyczne", "Physical"),
            ("Ogień", "Fire"),
            ("Ziemia", "Earth"),
            ("Energia", "Energy"),
            ("Lód", "Ice"),
            ("Świętość", "Holy"),
            ("Śmierć", "Death"),
        ],
    );
}

fn titles(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    values(
        ui,
        "titles.current",
        en,
        &[("Aktualny tytuł", "Current title")],
    );
    text(
        ui,
        d,
        "titles.search",
        en,
        ("Wyszukaj tytuł", "Search title"),
    );
    choice(
        ui,
        d,
        "titles.duration",
        en,
        ("Czas dostępności", "Duration"),
        &[ALL, ("Stałe", "Permanent"), ("Tymczasowe", "Temporary")],
    );
    choice(
        ui,
        d,
        "titles.availability",
        en,
        ("Dostępność", "Availability"),
        &[ALL, ("Zablokowane", "Locked"), ("Odblokowane", "Unlocked")],
    );
    table(
        ui,
        "titles.list",
        en,
        &[
            ("Tytuł", "Title"),
            ("Dostępność", "Availability"),
            ("Czas", "Duration"),
        ],
    );
    action(ui, en, ("Ustaw tytuł", "Set title"));
}

fn item_summary(ui: &mut egui::Ui, d: &mut Draft, en: bool, state: &GameState) {
    choice(
        ui,
        d,
        "summary.source",
        en,
        ("Źródło", "Source"),
        &[
            ("Ekwipunek", "Inventory"),
            ("Depozyt", "Depot"),
            ("Skrzynka", "Inbox"),
            ("Magazyn", "Stash"),
            ("Skrzynka sklepu", "Store inbox"),
        ],
    );
    text(
        ui,
        d,
        "summary.search",
        en,
        ("Wyszukaj przedmiot", "Search item"),
    );
    choice(
        ui,
        d,
        "summary.view",
        en,
        ("Widok", "View"),
        &[("Lista", "List"), ("Siatka", "Grid")],
    );
    let columns = [
        ("Przedmiot", "Item"),
        ("Ilość", "Quantity"),
        ("Miejsce", "Location"),
    ];
    let grid = d.choices["summary.view"] == 1;
    let inventory = (d.choices["summary.source"] == 0)
        .then_some(state.inventory.as_ref())
        .flatten();
    let Some(inventory) = inventory else {
        if grid {
            ui.group(|ui| {
                ui.weak(tr(
                    en,
                    ("Nie otrzymano wpisów.", "No entries have been received."),
                ));
            });
        } else {
            table(ui, "summary.items", en, &columns);
        }
        return;
    };
    // These are the actual known backpack and equipment entries; no depot/store rows
    // or names are inferred from the selected source or an item definition number.
    let items: Vec<_> = inventory
        .entries
        .iter()
        .map(|item| (item, tr(en, ("Plecak", "Backpack"))))
        .chain(
            inventory
                .main_backpack
                .iter()
                .map(|item| (item, tr(en, ("Slot plecaka", "Backpack slot")))),
        )
        .chain(
            inventory
                .equipment
                .iter()
                .map(|entry| (&entry.item, tr(en, ("Wyposażone", "Equipped")))),
        )
        .collect();
    let selected = d.items.entry("summary.selected").or_default();
    if selected.is_some_and(|handle| !items.iter().any(|(item, _)| item.handle.get() == handle)) {
        *selected = None;
    }
    let query = d
        .texts
        .get("summary.search")
        .map_or("", String::as_str)
        .trim()
        .to_lowercase();
    let rows: Vec<_> = items
        .into_iter()
        .map(|(item, location)| {
            let label = format!(
                "{} #{}",
                tr(en, ("Przedmiot", "Item")),
                item.item_definition_ref
            );
            (item, location, label)
        })
        .filter(|(_, _, label)| query.is_empty() || label.to_lowercase().contains(&query))
        .collect();
    if rows.is_empty() {
        ui.weak(tr(
            en,
            if query.is_empty() {
                ("Brak przedmiotów.", "No items.")
            } else {
                ("Brak pasujących przedmiotów.", "No matching items.")
            },
        ));
        return;
    }
    if grid {
        ui.horizontal_wrapped(|ui| {
            for (item, location, label) in rows {
                ui.group(|ui| {
                    ui.set_max_width(ui.available_width().min(110.0));
                    if ui
                        .add(
                            egui::Button::selectable(*selected == Some(item.handle.get()), label)
                                .wrap(),
                        )
                        .clicked()
                    {
                        *selected = Some(item.handle.get());
                    }
                    ui.small(format!("× {}", item.count));
                    ui.small(location);
                });
            }
        });
    } else {
        let width = ((ui.available_width() - 16.0) / 3.0).max(24.0);
        egui::Grid::new("summary.items")
            .num_columns(3)
            .min_col_width(0.0)
            .max_col_width(width)
            .striped(true)
            .show(ui, |ui| {
                for column in columns {
                    ui.strong(tr(en, column));
                }
                ui.end_row();
                for (item, location, label) in rows {
                    if ui
                        .add(
                            egui::Button::selectable(*selected == Some(item.handle.get()), label)
                                .wrap(),
                        )
                        .clicked()
                    {
                        *selected = Some(item.handle.get());
                    }
                    ui.label(item.count.to_string());
                    ui.label(location);
                    ui.end_row();
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::LocalPresets;

    #[test]
    fn atelier_regions_fit_narrow_and_two_column_widths() {
        type Region = fn(&mut egui::Ui, &mut super::Draft, bool);
        for english in [false, true] {
            for width in [320.0_f32, 560.0] {
                for gems in [false, true] {
                    let context = egui::Context::default();
                    let mut draft = super::Draft::default();
                    let input = egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(900.0, 620.0),
                        )),
                        ..Default::default()
                    };
                    let mut output = context.run_ui(input, |root| {
                        root.allocate_ui_with_layout(
                            egui::vec2(width, 600.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                let right = ui.max_rect().right();
                                let (left, main): (Region, Region) = if gems {
                                    (super::gem_revelation, super::gem_collection)
                                } else {
                                    (super::fragment_grades, super::fragment_collection)
                                };
                                super::atelier_columns(
                                    ui,
                                    &mut draft,
                                    english,
                                    if gems { 145.0 } else { 190.0 },
                                    left,
                                    main,
                                );
                                assert!(
                                    ui.min_rect().right() <= right + 1.0,
                                    "gems={gems}, english={english}, width={width}, region={:?}",
                                    ui.min_rect()
                                );
                            },
                        );
                    });
                    // The headless check intentionally has no renderer to apply font textures.
                    output.textures_delta.clear();
                }
            }
        }
    }

    #[test]
    fn local_preset_names_and_count_are_bounded() {
        let mut presets = LocalPresets::default();
        assert!(!presets.create(" \u{2003}\t"));
        assert!(presets.names.is_empty());
        assert!(presets.create(&"ą".repeat(150)));
        assert_eq!(presets.names[0].chars().count(), 96);
        for index in 1..16 {
            assert!(presets.create(&format!("Draft {index}")));
        }
        let original = presets.names.clone();
        assert!(!presets.create("Overflow"));
        assert!(!presets.copy());
        assert_eq!(presets.names, original);
    }

    #[test]
    fn removing_selected_preset_keeps_selection_valid() {
        let mut presets = LocalPresets::default();
        for name in ["First", "Second", "Third"] {
            assert!(presets.create(name));
        }
        presets.remove();
        assert_eq!(presets.selected, Some(1));
        assert_eq!(presets.names, ["First", "Second"]);
        presets.remove();
        assert_eq!(presets.selected, Some(0));
        presets.remove();
        assert_eq!(presets.selected, None);
        assert!(presets.names.is_empty());
        presets.remove();
        assert!(presets.names.is_empty());
    }

    #[test]
    fn invalid_rename_cannot_replace_a_local_draft() {
        let mut presets = LocalPresets::default();
        assert!(presets.create("Original"));
        assert!(presets.copy());
        assert!(presets.rename("  Renamed  "));
        assert_eq!(presets.names, ["Original", "Renamed"]);
        assert!(!presets.rename(" \t"));
        presets.selected = Some(usize::MAX);
        assert!(!presets.rename("Replacement"));
        assert!(!presets.copy());
        assert_eq!(presets.names, ["Original", "Renamed"]);
    }
}
