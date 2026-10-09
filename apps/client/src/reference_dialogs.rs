//! Client-owned dialog layouts with local drafts. No protocol values or mutations are
//! inferred from reference symbols: data stays unknown and submission remains unavailable.
use egui::{Color32, RichText};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferenceDialogKind {
    BossDifficulty,
    DepotSearch,
    Stash,
    Market,
    Inspection,
    ContextControls,
}
impl ReferenceDialogKind {
    pub const ALL: [Self; 6] = [
        Self::BossDifficulty,
        Self::DepotSearch,
        Self::Stash,
        Self::Market,
        Self::Inspection,
        Self::ContextControls,
    ];
    pub const fn id(self) -> &'static str {
        match self {
            Self::BossDifficulty => "world-dialog-boss-difficulty",
            Self::DepotSearch => "world-dialog-depot-search",
            Self::Stash => "world-dialog-stash",
            Self::Market => "world-dialog-market",
            Self::Inspection => "world-dialog-inspection",
            Self::ContextControls => "world-dialog-context-controls",
        }
    }
    pub fn label(self, english: bool) -> &'static str {
        let (pl, en) = match self {
            Self::BossDifficulty => ("Trudność walki z bossem", "Boss fight difficulty"),
            Self::DepotSearch => ("Wyszukiwanie w depozycie", "Depot search"),
            Self::Stash => ("Magazyn zasobów", "Supply stash"),
            Self::Market => ("Rynek", "Market"),
            Self::Inspection => ("Podgląd postaci", "Character inspection"),
            Self::ContextControls => ("Kontenery i kanały czatu", "Containers and chat channels"),
        };
        tr(english, pl, en)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum OfferSide {
    Buy,
    Sell,
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ContainerSort {
    Manual,
    NameUp,
    NameDown,
    CountUp,
    CountDown,
    WeightUp,
    WeightDown,
    ExpiryUp,
    ExpiryDown,
}
impl ContainerSort {
    const ALL: [Self; 9] = [
        Self::Manual,
        Self::NameUp,
        Self::NameDown,
        Self::CountUp,
        Self::CountDown,
        Self::WeightUp,
        Self::WeightDown,
        Self::ExpiryUp,
        Self::ExpiryDown,
    ];
    fn label(self, en: bool) -> &'static str {
        let labels = match self {
            Self::Manual => ("Ręcznie", "Manual"),
            Self::NameUp => ("Nazwa rosnąco", "Name ascending"),
            Self::NameDown => ("Nazwa malejąco", "Name descending"),
            Self::CountUp => ("Ilość rosnąco", "Stack size ascending"),
            Self::CountDown => ("Ilość malejąco", "Stack size descending"),
            Self::WeightUp => ("Waga rosnąco", "Weight ascending"),
            Self::WeightDown => ("Waga malejąco", "Weight descending"),
            Self::ExpiryUp => ("Termin wygaśnięcia rosnąco", "Expiry ascending"),
            Self::ExpiryDown => ("Termin wygaśnięcia malejąco", "Expiry descending"),
        };
        tr(en, labels.0, labels.1)
    }
}

#[derive(Default)]
pub struct ReferenceDialogs {
    pub manage: bool,
    opened: [bool; 6],
    boss_search: String,
    depot_search: String,
    stash_search: String,
    stash_operation: Option<usize>,
    stash_amount: String,
    market_tab: usize,
    market_search: String,
    offer_side: Option<OfferSide>,
    offer_amount: String,
    offer_price: String,
    offer_anonymous: Option<bool>,
    inspected_name: String,
    inspection_tab: usize,
    context_tab: usize,
    container_filter: String,
    container_sort: Option<ContainerSort>,
    backpacks_first: Option<bool>,
    channel_search: String,
    channel_muted: Option<bool>,
    secondary_chat: Option<bool>,
}

impl ReferenceDialogs {
    pub fn open(&mut self, kind: ReferenceDialogKind) {
        self.opened[kind as usize] = true;
    }
    pub fn any_open(&self) -> bool {
        self.manage || self.opened.iter().any(|open| *open)
    }

    pub fn show(&mut self, ctx: &egui::Context, english: bool) {
        let rect = ctx.content_rect();
        let maximum = (rect.size() - egui::vec2(28.0, 36.0)).max(egui::vec2(250.0, 240.0));
        let mut manage = self.manage;
        if manage {
            egui::Window::new(tr(english, "Narzędzia świata", "World tools"))
                .id("world-dialog-chooser".into())
                .open(&mut manage)
                .default_size(egui::vec2(460.0, 280.0))
                .max_size(maximum)
                .constrain_to(rect)
                .show(ctx, |ui| {
                    ui.label(tr(english, "Otwórz okno", "Open a window"));
                    for kind in ReferenceDialogKind::ALL {
                        if ui.button(kind.label(english)).clicked() {
                            self.open(kind);
                        }
                    }
                });
        }
        self.manage = manage;
        for kind in ReferenceDialogKind::ALL {
            let index = kind as usize;
            if !self.opened[index] {
                continue;
            }
            let mut open = true;
            egui::Window::new(kind.label(english))
                .id(kind.id().into())
                .open(&mut open)
                .default_size(egui::vec2(650.0, 450.0))
                .max_size(maximum)
                .constrain_to(rect)
                .scroll([false, true])
                .show(ctx, |ui| {
                    match kind {
                        ReferenceDialogKind::BossDifficulty => self.boss(ui, english),
                        ReferenceDialogKind::DepotSearch => self.depot(ui, english),
                        ReferenceDialogKind::Stash => self.stash(ui, english),
                        ReferenceDialogKind::Market => self.market(ui, english),
                        ReferenceDialogKind::Inspection => self.inspection(ui, english),
                        ReferenceDialogKind::ContextControls => self.context(ui, english),
                    }
                    ui.separator();
                    waiting(ui, english);
                });
            self.opened[index] = open;
        }
    }

    fn boss(&mut self, ui: &mut egui::Ui, en: bool) {
        edit(
            ui,
            en,
            ("Wyszukaj bossa", "Search for a boss"),
            &mut self.boss_search,
            128,
        );
        unavailable_choice(ui, en, ("Wybrana trudność", "Selected difficulty"));
        unknown_fields(
            ui,
            en,
            &[
                ("Najwyższa trudność osobista", "Personal highest difficulty"),
                ("Najwyższa trudność drużyny", "Group highest difficulty"),
                ("Premia za niepowodzenia", "Bad-luck bonus"),
            ],
        );
        ui.heading(tr(en, "Drużyna", "Group"));
        table(
            ui,
            en,
            &[
                ("Postać", "Character"),
                ("Osiągnięta trudność", "Highest difficulty"),
            ],
        );
        ui.heading(tr(en, "Modyfikatory walki", "Fight modifiers"));
        ui.label("—");
        pending_button(ui, en, ("Rozpocznij walkę", "Start fight"));
    }

    fn depot(&mut self, ui: &mut egui::Ui, en: bool) {
        edit(
            ui,
            en,
            ("Nazwa przedmiotu", "Item name"),
            &mut self.depot_search,
            128,
        );
        unavailable_choice(ui, en, ("Miejsce przechowywania", "Storage location"));
        ui.horizontal(|ui| {
            pending_button(ui, en, ("Wyszukaj", "Search"));
            pending_button(ui, en, ("Odśwież", "Refresh"));
        });
        ui.separator();
        table(
            ui,
            en,
            &[
                ("Przedmiot", "Item"),
                ("Ilość", "Amount"),
                ("Lokalizacja", "Location"),
            ],
        );
        unknown_fields(
            ui,
            en,
            &[
                ("Liczba miejsc przechowywania", "Storage-location count"),
                ("Łączna ilość", "Total amount"),
            ],
        );
        pending_button(
            ui,
            en,
            ("Pobierz wyświetlone przedmioty", "Retrieve displayed items"),
        );
    }

    fn stash(&mut self, ui: &mut egui::Ui, en: bool) {
        let operations = [
            ("Pobierz z magazynu", "Retrieve from stash"),
            ("Odłóż stos", "Stow stack"),
            ("Odłóż przedmioty tego typu", "Stow item type"),
            ("Odłóż zawartość kontenera", "Stow container contents"),
        ];
        egui::ComboBox::from_id_salt("stash-operation")
            .selected_text(
                self.stash_operation
                    .and_then(|index| operations.get(index))
                    .map_or(tr(en, "Wybierz czynność", "Choose operation"), |label| {
                        tr(en, label.0, label.1)
                    }),
            )
            .show_ui(ui, |ui| {
                for (index, label) in operations.iter().enumerate() {
                    ui.selectable_value(
                        &mut self.stash_operation,
                        Some(index),
                        tr(en, label.0, label.1),
                    );
                }
            });
        edit(ui, en, ("Przedmiot", "Item"), &mut self.stash_search, 128);
        amount(ui, en, ("Ilość", "Amount"), &mut self.stash_amount);
        unavailable_choice(ui, en, ("Kontener źródłowy", "Source container"));
        unknown_fields(ui, en, &[("Dostępna ilość", "Available amount")]);
        table(
            ui,
            en,
            &[
                ("Przedmiot", "Item"),
                ("Ilość w magazynie", "Stashed amount"),
            ],
        );
        pending_button(ui, en, ("Wykonaj czynność", "Submit operation"));
    }

    fn market(&mut self, ui: &mut egui::Ui, en: bool) {
        tabs(
            ui,
            en,
            &mut self.market_tab,
            &[
                ("Oferty", "Browse"),
                ("Szczegóły", "Details"),
                ("Statystyki", "Statistics"),
                ("Nowa oferta", "New offer"),
            ],
        );
        edit(ui, en, ("Przedmiot", "Item"), &mut self.market_search, 128);
        match self.market_tab {
            1 => {
                unknown_fields(
                    ui,
                    en,
                    &[
                        ("Kategoria", "Category"),
                        ("Waga", "Weight"),
                        ("Wymagania", "Requirements"),
                        ("Wartość przedmiotu", "Item value"),
                    ],
                );
                ui.heading(tr(en, "Szczegóły przedmiotu", "Item details"));
                ui.label("—");
            }
            2 => {
                table(
                    ui,
                    en,
                    &[
                        ("Rodzaj", "Direction"),
                        ("Transakcje", "Transactions"),
                        ("Najniższa cena", "Lowest price"),
                        ("Najwyższa cena", "Highest price"),
                    ],
                );
                unknown_fields(
                    ui,
                    en,
                    &[
                        ("Średnia cena", "Average price"),
                        ("Okres statystyk", "Statistics period"),
                    ],
                );
            }
            3 => {
                ui.horizontal(|ui| {
                    ui.label(tr(en, "Rodzaj oferty", "Offer direction"));
                    ui.selectable_value(
                        &mut self.offer_side,
                        Some(OfferSide::Buy),
                        tr(en, "Kupno", "Buy"),
                    );
                    ui.selectable_value(
                        &mut self.offer_side,
                        Some(OfferSide::Sell),
                        tr(en, "Sprzedaż", "Sell"),
                    );
                });
                amount(ui, en, ("Ilość", "Amount"), &mut self.offer_amount);
                amount(
                    ui,
                    en,
                    ("Cena jednostkowa", "Unit price"),
                    &mut self.offer_price,
                );
                optional_bool(
                    ui,
                    en,
                    "market-anonymous",
                    ("Oferta anonimowa", "Anonymous offer"),
                    &mut self.offer_anonymous,
                );
                unknown_fields(
                    ui,
                    en,
                    &[
                        ("Waluta", "Currency"),
                        ("Opłata za ofertę", "Offer fee"),
                        ("Dostępne środki", "Available balance"),
                        ("Wygaśnięcie oferty", "Offer expiry"),
                    ],
                );
                pending_button(ui, en, ("Utwórz ofertę", "Create offer"));
            }
            _ => {
                ui.horizontal(|ui| {
                    ui.selectable_value(
                        &mut self.offer_side,
                        Some(OfferSide::Buy),
                        tr(en, "Oferty kupna", "Buy offers"),
                    );
                    ui.selectable_value(
                        &mut self.offer_side,
                        Some(OfferSide::Sell),
                        tr(en, "Oferty sprzedaży", "Sell offers"),
                    );
                });
                table(
                    ui,
                    en,
                    &[
                        ("Ilość", "Amount"),
                        ("Cena", "Price"),
                        ("Dostępność", "Availability"),
                    ],
                );
                pending_button(
                    ui,
                    en,
                    ("Zrealizuj wybraną ofertę", "Accept selected offer"),
                );
            }
        }
    }

    fn inspection(&mut self, ui: &mut egui::Ui, en: bool) {
        edit(
            ui,
            en,
            ("Imię postaci", "Character name"),
            &mut self.inspected_name,
            64,
        );
        pending_button(ui, en, ("Otwórz podgląd", "Request inspection"));
        unknown_fields(
            ui,
            en,
            &[("Uprawnienia do podglądu", "Inspection permission")],
        );
        tabs(
            ui,
            en,
            &mut self.inspection_tab,
            &[
                ("Postać", "Character"),
                ("Ekwipunek", "Equipment"),
                ("Statystyki", "Statistics"),
            ],
        );
        match self.inspection_tab {
            1 => table(
                ui,
                en,
                &[
                    ("Miejsce", "Slot"),
                    ("Przedmiot", "Item"),
                    ("Szczegóły", "Details"),
                ],
            ),
            2 => unknown_fields(
                ui,
                en,
                &[
                    ("Poziom", "Level"),
                    ("Umiejętności", "Skills"),
                    ("Atak", "Offence"),
                    ("Obrona", "Defence"),
                ],
            ),
            _ => unknown_fields(
                ui,
                en,
                &[
                    ("Postać", "Character"),
                    ("Profesja", "Vocation"),
                    ("Wygląd", "Appearance"),
                    ("Tytuł", "Title"),
                ],
            ),
        }
    }

    fn context(&mut self, ui: &mut egui::Ui, en: bool) {
        tabs(
            ui,
            en,
            &mut self.context_tab,
            &[
                ("Kontenery", "Containers"),
                ("Kanały czatu", "Chat channels"),
            ],
        );
        if self.context_tab == 0 {
            unavailable_choice(ui, en, ("Kontener", "Container"));
            edit(
                ui,
                en,
                ("Filtr przedmiotów", "Item filter"),
                &mut self.container_filter,
                128,
            );
            egui::ComboBox::from_id_salt("container-sort")
                .selected_text(
                    self.container_sort
                        .map_or(tr(en, "Wybierz sortowanie", "Choose sorting"), |sort| {
                            sort.label(en)
                        }),
                )
                .show_ui(ui, |ui| {
                    for sort in ContainerSort::ALL {
                        ui.selectable_value(&mut self.container_sort, Some(sort), sort.label(en));
                    }
                });
            optional_bool(
                ui,
                en,
                "backpacks-first",
                ("Plecaki na początku", "Backpacks first"),
                &mut self.backpacks_first,
            );
            pending_button(ui, en, ("Zastosuj sortowanie", "Apply sorting"));
            pending_button(ui, en, ("Sortuj rekurencyjnie…", "Sort recursively…"));
            ui.small(tr(
                en,
                "Sortowanie zawartości podkontenerów wymaga potwierdzenia.",
                "Sorting nested containers requires confirmation.",
            ));
        } else {
            edit(ui, en, ("Kanał", "Channel"), &mut self.channel_search, 128);
            unavailable_choice(ui, en, ("Wybrany kanał", "Selected channel"));
            optional_bool(
                ui,
                en,
                "chat-muted",
                ("Wycisz kanał", "Mute channel"),
                &mut self.channel_muted,
            );
            optional_bool(
                ui,
                en,
                "secondary-chat",
                ("Wyświetl w dodatkowej konsoli", "Show in secondary console"),
                &mut self.secondary_chat,
            );
            ui.heading(tr(en, "Uczestnicy kanału", "Channel participants"));
            table(ui, en, &[("Postać", "Character"), ("Stan", "Status")]);
            ui.horizontal_wrapped(|ui| {
                pending_button(ui, en, ("Otwórz kanał", "Open channel"));
                pending_button(ui, en, ("Zamknij kanał", "Close channel"));
                pending_button(ui, en, ("Kopiuj treść", "Copy contents"));
                pending_button(ui, en, ("Zapisz treść…", "Save contents…"));
            });
        }
    }
}

fn tr<'a>(en: bool, pl: &'a str, english: &'a str) -> &'a str {
    if en { english } else { pl }
}
fn waiting(ui: &mut egui::Ui, en: bool) {
    ui.label(
        RichText::new(tr(
            en,
            "Oczekiwanie na dane. Operacje są niedostępne.",
            "Waiting for data. Operations are unavailable.",
        ))
        .color(Color32::from_rgb(220, 183, 111)),
    );
}
fn pending_button(ui: &mut egui::Ui, en: bool, label: (&str, &str)) {
    ui.add_enabled(false, egui::Button::new(tr(en, label.0, label.1)))
        .on_hover_text(tr(
            en,
            "Ta czynność wymaga danych świata.",
            "This action requires world data.",
        ));
}
fn edit(ui: &mut egui::Ui, en: bool, label: (&str, &str), value: &mut String, limit: usize) {
    ui.label(tr(en, label.0, label.1));
    ui.add(
        egui::TextEdit::singleline(value)
            .char_limit(limit)
            .desired_width(f32::INFINITY),
    );
}
fn amount(ui: &mut egui::Ui, en: bool, label: (&str, &str), value: &mut String) {
    edit(ui, en, label, value, 20);
    if !value.is_empty() && !value.parse::<u64>().is_ok_and(|amount| amount > 0) {
        ui.small(
            RichText::new(tr(
                en,
                "Wpisz dodatnią liczbę całkowitą.",
                "Enter a positive whole number.",
            ))
            .color(Color32::from_rgb(230, 110, 105)),
        );
    }
}
fn unknown_fields(ui: &mut egui::Ui, en: bool, labels: &[(&str, &str)]) {
    for label in labels {
        ui.horizontal(|ui| {
            ui.label(tr(en, label.0, label.1));
            ui.strong("—");
        });
    }
}
fn table(ui: &mut egui::Ui, en: bool, columns: &[(&str, &str)]) {
    ui.horizontal_wrapped(|ui| {
        for column in columns {
            ui.strong(tr(en, column.0, column.1));
            ui.add_space(12.0);
        }
    });
    ui.separator();
    ui.weak(tr(
        en,
        "Dane nie zostały jeszcze pobrane.",
        "Data has not been received yet.",
    ));
    ui.add_space(16.0);
}
fn tabs(ui: &mut egui::Ui, en: bool, selected: &mut usize, labels: &[(&str, &str)]) {
    ui.horizontal_wrapped(|ui| {
        for (index, label) in labels.iter().enumerate() {
            ui.selectable_value(selected, index, tr(en, label.0, label.1));
        }
    });
    ui.separator();
}
fn unavailable_choice(ui: &mut egui::Ui, en: bool, label: (&str, &str)) {
    ui.horizontal(|ui| {
        ui.label(tr(en, label.0, label.1));
        ui.add_enabled(false, egui::Button::new("— ▾"));
    });
}
fn optional_bool(
    ui: &mut egui::Ui,
    en: bool,
    id: &str,
    label: (&str, &str),
    value: &mut Option<bool>,
) {
    ui.label(tr(en, label.0, label.1));
    egui::ComboBox::from_id_salt(id)
        .selected_text(match value {
            Some(true) => tr(en, "Tak", "Yes"),
            Some(false) => tr(en, "Nie", "No"),
            None => tr(en, "Nie wybrano", "Unselected"),
        })
        .show_ui(ui, |ui| {
            ui.selectable_value(value, None, tr(en, "Nie wybrano", "Unselected"));
            ui.selectable_value(value, Some(true), tr(en, "Tak", "Yes"));
            ui.selectable_value(value, Some(false), tr(en, "Nie", "No"));
        });
}
