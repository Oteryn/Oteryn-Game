//! Dedicated native panel compositions. Form inputs are session-local drafts, never game facts.
//! Recreate PanelViews on admission; selected item handles must not cross session generations.
use egui::{Color32, RichText, Stroke, Vec2};
use oteryn_client::play::GameState;
use std::collections::BTreeMap;

type Label = (&'static str, &'static str);

#[derive(Default)]
struct Draft {
    tab: usize,
    sector: usize,
    texts: BTreeMap<&'static str, String>,
    choices: BTreeMap<&'static str, usize>,
    flags: BTreeMap<&'static str, bool>,
    items: BTreeMap<&'static str, Option<u64>>,
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
        let mut open_items = false;
        ui.push_id(("dedicated-panel", id), |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(5.0, 4.0);
            match id {
                "spells" => spells(ui, draft, english),
                "vip" => vip(ui, draft, english),
                "quest-log" => quests(ui, draft, english),
                "highscores" => highscores(ui, draft, english),
                "wheel" => wheel(ui, draft, english),
                "prey" => prey(ui, draft, english),
                "forge" => forge(ui, draft, english, state),
                "task-board" => tasks(ui, draft, english),
                "party" | "social" => social(ui, draft, english, id == "party"),
                "reward-wall" => rewards(ui, draft, english),
                "analytics" => open_items = analytics(ui, draft, english),
                "bosstiary" => bestiary(ui, draft, english, true),
                "boss-slots" => boss_slots(ui, english),
                "weapon-proficiency" => proficiency(ui, draft, english, state),
                "imbuement-tracker" => imbuements(ui, draft, english, state),
                "unjustified-points" => unjustified(ui, english),
                "cyclopedia" => cyclopedia(ui, draft, english, state),
                _ => tracker(ui, draft, english, id),
            }
        });
        if open_items {
            // Directly observed route: Add Tracked Drop opens the item catalogue;
            // opening it must not assign a tracked item or alter a server preference.
            self.drafts
                .entry("cyclopedia".to_owned())
                .or_default()
                .choices
                .insert("cyclopedia.page", 1);
            self.navigation = Some("cyclopedia");
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
fn quests(ui: &mut egui::Ui, d: &mut Draft, en: bool) {
    text(ui,d,"quest.search",en,("Wyszukaj zadanie lub misję","Search quest or mission"));
    choice(ui,d,"quest.state",en,("Stan","State"),&[ALL,("Aktywne","Active"),("Ukończone","Completed")]);
    choice(ui,d,"quest.sort",en,("Kolejność","Order"),&[("Nazwa","Name"),("Stan","State")]);
    ui.columns(2,|columns| {
        table(&mut columns[0],"quest.tree",en,&[("Zadanie","Quest"),("Stan","State")]);
        table(&mut columns[1],"quest.missions",en,&[("Misja","Mission"),("Postęp","Progress")]);
    });
    details(ui,"quest.description",en,("Opis wybranej misji","Selected mission description"));
    ui.horizontal_wrapped(|ui| { action(ui,en,("Poprzednia misja","Previous mission")); action(ui,en,("Następna misja","Next mission")); action(ui,en,("Śledź misję","Track mission")); action(ui,en,("Usuń śledzenie","Stop tracking")); });
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
            text(ui,d,"wheel.preset",en,("Nazwa profilu","Preset name"));
            values(ui,"wheel.points",en,&[("Dostępne punkty awansu","Available promotion points"),("Przydzielone punkty","Allocated points")]);
            ui.horizontal_wrapped(|ui| { for (index,label) in [("Lewy górny","Upper left"),("Prawy górny","Upper right"),("Lewy dolny","Lower left"),("Prawy dolny","Lower right")].iter().enumerate() { ui.selectable_value(&mut d.sector,index,tr(en,*label)); } });
            wheel_partition(ui,d.sector);
            table(ui,"wheel.perks",en,&[("Korzyść","Perk"),("Ranga","Rank"),("Punkty","Points")]);
            details(ui,"wheel.requirements",en,("Wymagania i naczynia","Requirements and vessels"));
            ui.horizontal_wrapped(|ui| { action(ui,en,("Przydziel punkty","Allocate points")); action(ui,en,("Zastosuj profil","Apply preset")); action(ui,en,("Wyzeruj koło","Reset wheel")); });
        }
        1 => {
            text(ui,d,"gems.affinity",en,("Powinowactwo","Affinity")); text(ui,d,"gems.quality",en,("Jakość","Quality"));
            table(ui,"gems.collection",en,&[("Klejnot","Gem"),("Powinowactwo","Affinity"),("Jakość","Quality")]);
            values(ui,"gems.cost",en,&[("Koszt odkrycia","Revelation cost")]); details(ui,"gems.modifiers",en,("Modyfikatory klejnotu","Gem modifiers"));
            action(ui,en,("Odkryj klejnot","Reveal gem"));
        }
        _ => {
            text(ui,d,"fragments.search",en,("Wyszukaj modyfikator","Search modifier")); text(ui,d,"fragments.grade",en,("Klasa fragmentu","Fragment grade"));
            table(ui,"fragments.modifiers",en,&[("Modyfikator","Modifier"),("Klasa","Grade"),("Efekt","Effect")]);
            values(ui,"fragments.cost",en,&[("Koszt ulepszenia","Enhancement cost")]); action(ui,en,("Ulepsz fragment","Enhance fragment"));
        }
    }
}

fn wheel_partition(ui: &mut egui::Ui, selected: usize) {
    let side = ui.available_width().min(170.0);
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(side), egui::Sense::hover());
    let center = rect.center();
    let radius = side * 0.43;
    for index in 0..4 {
        let start = [-2.0, -1.0, 1.0, 0.0][index] * std::f32::consts::FRAC_PI_2;
        let points: Vec<_> = std::iter::once(center)
            .chain((0..=20).map(|step| {
                let angle = start + step as f32 / 20.0 * std::f32::consts::FRAC_PI_2;
                center + Vec2::angled(angle) * radius
            }))
            .collect();
        ui.painter().add(egui::Shape::convex_polygon(
            points,
            if index == selected {
                Color32::from_rgb(37, 47, 57)
            } else {
                Color32::from_rgb(20, 25, 31)
            },
            Stroke::new(1.0, Color32::from_rgb(156, 135, 89)),
        ));
    }
    ui.painter().text(
        center,
        egui::Align2::CENTER_CENTER,
        "—",
        egui::FontId::proportional(18.0),
        Color32::GRAY,
    );
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
fn tracker(ui: &mut egui::Ui, d: &mut Draft, en: bool, id: &str) {
    text(ui,d,"tracker.search",en,("Wyszukaj śledzony wpis","Search tracked entry"));
    ui.menu_button(tr(en,("Opcje śledzenia","Tracking options")),|ui| {
        flag(ui,d,"tracker.completed",en,("Pokaż ukończone","Show completed entries"));
        if id=="quest-tracker" { flag(ui,d,"tracker.add",en,("Automatycznie dodawaj nowe zadania","Automatically add new quests")); flag(ui,d,"tracker.remove",en,("Automatycznie usuwaj ukończone","Automatically remove completed quests")); }
        action(ui,en,("Usuń wybrany wpis","Remove selected entry")); action(ui,en,("Usuń wszystkie wpisy","Remove all entries")); action(ui,en,("Usuń ukończone","Remove completed entries"));
    });
    table(ui,"tracker.list",en,&[("Wpis","Entry"),("Postęp","Progress"),("Stan","Status")]);
    progress(ui,en,("Postęp wybranego wpisu","Selected-entry progress"));
    action(ui,en,if id=="quest-tracker" {("Otwórz dziennik zadań","Open quest log")} else {("Wybierz wpis do śledzenia","Choose entry to track")});
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
        13 | 14 => table(ui,"character.records",en,&[("Data","Date"),("Postać","Character"),("Zdarzenie","Event")]),
        15 => { text(ui,d,"achievements.search",en,("Wyszukaj osiągnięcie","Search achievement")); table(ui,"achievements.list",en,&[("Osiągnięcie","Achievement"),("Stopień","Grade"),("Data","Date")]); details(ui,"achievements.details",en,("Opis osiągnięcia","Achievement description")); }
        16 => item_summary(ui,d,en,state),
        17 => { choice(ui,d,"appearance.kind",en,("Rodzaj","Type"),&[("Stroje","Outfits"),("Wierzchowce","Mounts"),("Chowańce","Familiars")]); table(ui,"appearance.list",en,&[("Wygląd","Appearance"),("Dostępność","Availability")]); action(ui,en,("Zastosuj wygląd","Apply appearance")); }
        18 => table(ui,"account.benefits",en,&[("Korzyść","Benefit"),("Dostępność","Availability"),("Pozostały czas","Time remaining")]),
        _ => titles(ui,d,en),
    }
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
