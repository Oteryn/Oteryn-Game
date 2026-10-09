//! Oteryn-owned layouts informed by the 2026-10-09 reference audit.
//! This catalogue contains labels and structure, never sample gameplay values. Missing
//! projections stay unknown; selecting a capability alone does not populate a panel.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    ObservedPanel,
    ObservedShortcut,
    NamesOnly,
    ProductLayout,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelLayout {
    Statistics,
    Battle,
    SpellBook,
    Contacts,
    QuestTree,
    Documents,
    Cyclopedia,
    Ranking,
    Shortcuts,
    Party,
    Wheel,
    Tracker,
    Thresholds,
    PreySlots,
    RewardCalendar,
    Analytics,
    Bestiary,
    BossSlots,
    EquipmentEffects,
    PerkTree,
    Forge,
    TeamFinder,
    TaskBoard,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldKind {
    Value,
    Progress,
    Duration,
    Status,
    Table,
    Details,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelField {
    pub key: &'static str,
    pub polish: &'static str,
    pub english: &'static str,
    pub kind: FieldKind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelTab {
    pub id: &'static str,
    pub polish: &'static str,
    pub english: &'static str,
    pub evidence: Evidence,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelDefinition {
    pub id: &'static str,
    pub polish: &'static str,
    pub english: &'static str,
    pub layout: PanelLayout,
    pub default_visible: bool,
    pub evidence: Evidence,
    /// Accepted wire capability hint. None means no single accepted capability describes
    /// the complete projection; it must not mean data is available without a server.
    pub required_capability: Option<u32>,
    pub fields: &'static [PanelField],
    pub tabs: &'static [PanelTab],
}
impl PanelDefinition {
    #[must_use]
    pub const fn label(&self, english: bool) -> &'static str {
        if english { self.english } else { self.polish }
    }
}
impl PanelField {
    #[must_use]
    pub const fn label(&self, english: bool) -> &'static str {
        if english { self.english } else { self.polish }
    }
}
impl PanelTab {
    #[must_use]
    pub const fn label(&self, english: bool) -> &'static str {
        if english { self.english } else { self.polish }
    }
}
macro_rules! f {
    ($key:literal, $pl:literal, $en:literal, $kind:ident) => {
        PanelField {
            key: $key,
            polish: $pl,
            english: $en,
            kind: FieldKind::$kind,
        }
    };
}
macro_rules! t {
    ($id:literal, $pl:literal, $en:literal, $evidence:ident) => {
        PanelTab {
            id: $id,
            polish: $pl,
            english: $en,
            evidence: Evidence::$evidence,
        }
    };
}
macro_rules! p {
    ($id:literal, $pl:literal, $en:literal, $layout:ident, $visible:literal,
     $evidence:ident, $cap:expr, $fields:expr, $tabs:expr) => {
        PanelDefinition {
            id: $id,
            polish: $pl,
            english: $en,
            layout: PanelLayout::$layout,
            default_visible: $visible,
            evidence: Evidence::$evidence,
            required_capability: $cap,
            fields: $fields,
            tabs: $tabs,
        }
    };
}

// Capability identities: world_spatial_entities.rs=6, charm.rs=1, analyser.rs=10,
// quest_log.rs=16. These hints never negotiate capabilities or define new protocol routes.
// Label data is kept compact; no third-party artwork or translation corpus is included.
#[rustfmt::skip]
pub const PANELS: &[PanelDefinition] = &[
    p!("skills", "Umiejętności", "Skills", Statistics, true, ObservedPanel, None,
        &[f!("level","Poziom","Level",Value), f!("experience","Doświadczenie","Experience",Progress), f!("gain","Przyrost doświadczenia","Experience gain",Value), f!("health","Zdrowie","Health",Progress), f!("mana","Mana","Mana",Progress), f!("capacity","Udźwig","Capacity",Value), f!("speed","Szybkość","Speed",Value), f!("food","Najedzenie","Food",Duration), f!("stamina","Wytrzymałość","Stamina",Progress), f!("skills","Umiejętności postaci","Character skills",Table)], &[]),
    p!("battle", "Lista walki", "Battle List", Battle, true, ObservedPanel, Some(6),
        &[f!("name","Nazwa","Name",Value), f!("health","Zdrowie","Health",Progress), f!("kind","Rodzaj","Kind",Status)], &[]),
    p!("spells", "Lista czarów", "Spell List", SpellBook, true, ObservedPanel, None,
        &[f!("name","Nazwa","Name",Value), f!("formula","Formuła","Formula",Value), f!("vocation","Profesja","Vocation",Status), f!("group","Grupa","Group",Status), f!("level","Wymagany poziom","Required level",Value), f!("mana","Koszt many","Mana cost",Value), f!("cooldown","Odnowienie","Cooldown",Duration), f!("details","Opis czaru","Spell details",Details)], &[]),
    p!("vip", "Lista VIP", "VIP List", Contacts, true, ObservedPanel, None,
        &[f!("name","Imię","Name",Value), f!("status","Dostępność","Online status",Status), f!("group","Grupa","Group",Status)], &[]),
    p!("help", "Pomoc", "Help", Documents, true, ObservedShortcut, None,
        &[f!("controls","Sterowanie","Controls",Details), f!("connection","Połączenie","Connection",Details), f!("support","Wsparcie","Support",Details)], &[]),
    p!("quest-log", "Dziennik zadań", "Quest Log", QuestTree, true, ObservedPanel, Some(16),
        &[f!("quest","Zadanie","Quest",Value), f!("mission","Misja","Mission",Value), f!("state","Stan","State",Status), f!("description","Opis","Description",Details), f!("tracked","Śledzenie","Tracking",Status)], &[]),
    p!("compendium", "Kompendium", "Compendium", Documents, true, ObservedPanel, None,
        &[f!("content","Treść","Content",Details)], &[t!("guide","Poradnik gracza","Player guide",ObservedPanel), t!("features","Funkcje klienta","Client features",ObservedPanel), t!("information","Przydatne informacje","Useful information",ObservedPanel), t!("updates","Aktualizacje","Updates",ObservedPanel), t!("support","Wsparcie","Support",ObservedPanel)]),
    p!("cyclopedia", "Cyklopedia", "Cyclopedia", Cyclopedia, true, ObservedPanel, None,
        &[f!("category","Kategoria","Category",Value), f!("entry","Wpis","Entry",Value), f!("details","Szczegóły","Details",Details)],
        &[t!("map","Mapa","Map",ObservedPanel), t!("items","Przedmioty","Items",ObservedPanel), t!("houses","Domy","Houses",ObservedPanel), t!("character","Postać","Character",ObservedPanel), t!("bestiary","Bestiariusz","Bestiary",ObservedPanel), t!("charms","Talizmany","Charms",ObservedPanel), t!("bosses","Bestiariusz bossów","Bosstiary",ObservedPanel), t!("boss-slots","Sloty bossów","Boss Slots",ObservedPanel), t!("archive","Archiwum magii","Magical Archive",ObservedPanel), t!("character-stats","Statystyki postaci","Character Stats",ObservedPanel), t!("offence","Atak","Offence Stats",ObservedPanel), t!("defence","Obrona","Defence Stats",ObservedPanel), t!("blessings","Błogosławieństwa","Blessings",ObservedPanel), t!("deaths","Ostatnie śmierci","Recent Deaths",ObservedPanel), t!("pvp-kills","Ostatnie zabójstwa PvP","Recent PvP Kills",ObservedPanel), t!("achievements","Osiągnięcia","Achievements",ObservedPanel), t!("item-summary","Podsumowanie przedmiotów","Item Summary",ObservedPanel), t!("appearances","Wygląd","Appearances",ObservedPanel), t!("store-summary","Korzyści konta","Store Summary",ObservedPanel), t!("titles","Tytuły postaci","Character Titles",ObservedPanel)]),
    p!("highscores", "Rankingi", "Highscores", Ranking, true, ObservedPanel, None,
        &[f!("rank","Miejsce","Rank",Value), f!("name","Postać","Character",Value), f!("vocation","Profesja","Vocation",Status), f!("world","Świat","World",Status), f!("score","Wynik kategorii","Category score",Value)], &[]),
    p!("player-guide", "Poradnik gracza", "Player Guide", Documents, true, ObservedShortcut, None,
        &[f!("content","Poradnik","Guide",Details)], &[t!("start","Pierwsze kroki","Getting started",ProductLayout), t!("account","Konto","Account",ProductLayout), t!("controls","Sterowanie","Controls",ProductLayout), t!("world","Świat gry","Game world",ProductLayout)]),
    p!("shortcuts", "Zarządzaj skrótami", "Manage Shortcuts", Shortcuts, true, ObservedPanel, None,
        &[f!("displayed","Wyświetlane skróty","Displayed shortcuts",Table), f!("available","Dostępne skróty","Available shortcuts",Table), f!("order","Kolejność","Order",Value)], &[]),
    p!("party", "Drużyna", "Party List", Party, false, ObservedPanel, None,
        &[f!("member","Członek drużyny","Party member",Value), f!("role","Rola","Role",Status), f!("status","Stan drużyny","Party status",Status)], &[]),
    p!("wheel", "Koło przeznaczenia", "Wheel of Destiny", Wheel, false, ObservedPanel, None,
        &[f!("points","Punkty awansu","Promotion points",Value), f!("preset","Profil","Preset",Value), f!("perks","Korzyści","Perks",Table), f!("requirements","Wymagania","Requirements",Details)], &[t!("wheel","Koło","Wheel",ObservedPanel), t!("gems","Pracownia klejnotów","Gem Atelier",ObservedPanel), t!("fragments","Warsztat fragmentów","Fragment Workshop",ObservedPanel)]),
    p!("quest-tracker", "Śledzenie zadań", "Quest Tracker", Tracker, false, ObservedPanel, Some(16),
        &[f!("quest","Zadanie","Quest",Value), f!("mission","Misja","Mission",Value), f!("progress","Postęp","Progress",Progress)], &[t!("tracked","Śledzone zadania","Tracked quests",ObservedPanel), t!("automation","Automatyczne śledzenie","Automatic tracking",NamesOnly)]),
    p!("unjustified-points", "Punkty nieuzasadnionych zabójstw", "Unjustified Points", Thresholds, false, ObservedPanel, None,
        &[f!("points","Aktualne punkty","Current points",Value), f!("short-period","Próg w krótszym okresie","Short-period threshold",Progress), f!("medium-period","Próg w średnim okresie","Medium-period threshold",Progress), f!("long-period","Próg w dłuższym okresie","Long-period threshold",Progress)], &[]),
    p!("prey", "Polowanie Prey", "Prey", PreySlots, false, ObservedPanel, None,
        &[f!("creature","Stworzenie","Creature",Value), f!("bonus","Premia","Bonus",Value), f!("duration","Pozostały czas","Time remaining",Duration), f!("reroll","Koszt losowania","Reroll cost",Value), f!("availability","Dostępność slotu","Slot availability",Status)], &[]),
    p!("kill-tracker", "Licznik zabójstw", "Kill Tracker", Tracker, false, ObservedPanel, None,
        &[f!("creature","Stworzenie","Creature",Value), f!("kills","Zabójstwa","Kills",Value), f!("state","Stan śledzenia","Tracking state",Status)], &[]),
    p!("reward-wall", "Nagrody dzienne", "Reward Wall", RewardCalendar, false, ObservedPanel, None,
        &[f!("cycle","Cykl nagród","Reward cycle",Progress), f!("streak","Seria odbiorów","Claim streak",Value), f!("resting","Premie odpoczynku","Resting bonuses",Details), f!("jokers","Jokery","Jokers",Value), f!("claim-state","Stan odbioru","Claim state",Status)], &[t!("cycle","Cykl nagród","Reward cycle",ObservedPanel), t!("history","Historia","History",ObservedPanel)]),
    p!("analytics", "Analizatory", "Analytics", Analytics, false, ObservedPanel, Some(10),
        &[f!("duration","Czas sesji","Session duration",Duration), f!("total","Suma","Total",Value), f!("rate","Tempo","Rate",Value), f!("maximum","Wartość maksymalna","Maximum",Value), f!("breakdown","Podział","Breakdown",Table)],
        &[t!("hunt","Polowanie","Hunting",ObservedPanel), t!("loot","Łupy","Loot",ObservedPanel), t!("supply","Zużycie zasobów","Supply",ObservedPanel), t!("impact","Obrażenia i leczenie","Impact",ObservedPanel), t!("damage-input","Otrzymane obrażenia","Damage Input",ObservedPanel), t!("experience","Doświadczenie","XP",ObservedPanel), t!("drops","Śledzenie zdobyczy","Drop Tracker",ObservedPanel), t!("party-hunt","Polowanie drużynowe","Party Hunt",ObservedPanel), t!("boss-cooldowns","Odnowienia bossów","Boss Cooldowns",ObservedPanel)]),
    p!("bosstiary", "Bestiariusz bossów", "Bosstiary", Bestiary, false, ObservedPanel, None,
        &[f!("boss","Boss","Boss",Value), f!("category","Kategoria","Category",Status), f!("kills","Zabójstwa","Kills",Value), f!("stage","Etap postępu","Progression tier",Progress)], &[]),
    p!("boss-slots", "Sloty bossów", "Boss Slots", BossSlots, false, ObservedPanel, None,
        &[f!("boss","Przypisany boss","Assigned boss",Value), f!("boosted","Wzmocniony boss","Boosted boss",Value), f!("requirements","Warunki odblokowania","Unlock conditions",Details), f!("availability","Dostępność","Availability",Status)], &[]),
    p!("bosstiary-tracker", "Śledzenie bossów", "Bosstiary Tracker", Tracker, false, ObservedPanel, None,
        &[f!("boss","Boss","Boss",Value), f!("kills","Zabójstwa","Kills",Value), f!("progress","Postęp","Progress",Progress)], &[]),
    p!("bestiary-tracker", "Śledzenie bestiariusza", "Bestiary Tracker", Tracker, false, ObservedPanel, Some(1),
        &[f!("creature","Stworzenie","Creature",Value), f!("kills","Zabójstwa","Kills",Value), f!("progress","Postęp","Progress",Progress)], &[]),
    p!("imbuement-tracker", "Śledzenie nasyceń", "Imbuement Tracker", EquipmentEffects, false, ObservedPanel, None,
        &[f!("equipment","Ekwipunek","Equipment",Value), f!("slot","Slot nasycenia","Imbuement slot",Value), f!("effect","Efekt","Effect",Details), f!("duration","Pozostały czas","Time remaining",Duration)], &[]),
    p!("weapon-proficiency", "Biegłość w broni", "Weapon Proficiency", PerkTree, false, ObservedPanel, None,
        &[f!("weapon","Broń","Weapon",Value), f!("experience","Doświadczenie broni","Weapon experience",Progress), f!("perks","Drzewo korzyści","Perk tree",Table), f!("requirements","Wymagania","Requirements",Details), f!("state","Dostępność korzyści","Perk availability",Status)], &[]),
    p!("forge", "Kuźnia egzaltacji", "Exaltation Forge", Forge, false, ObservedPanel, None,
        &[f!("source","Przedmiot źródłowy","Source item",Value), f!("destination","Przedmiot docelowy","Destination item",Value), f!("resources","Wymagane zasoby","Required resources",Table), f!("chance","Szansa powodzenia","Success chance",Value), f!("tier-loss","Ochrona poziomu","Tier-loss mitigation",Details), f!("convergence","Konwergencja","Convergence",Status)], &[t!("fusion","Fuzja","Fusion",ObservedPanel), t!("transfer","Przeniesienie","Transfer",ObservedPanel), t!("conversion","Konwersja","Conversion",ObservedPanel), t!("history","Historia","History",ObservedPanel)]),
    p!("social", "Społeczność", "Social", TeamFinder, false, ObservedPanel, None,
        &[f!("team","Drużyna","Team",Value), f!("activity","Aktywność","Activity",Value), f!("members","Członkowie","Members",Table), f!("requirements","Wymagania udziału","Participation requirements",Details)], &[t!("finder","Wyszukaj drużynę","Find a team",ObservedPanel), t!("assemble","Utwórz drużynę","Assemble a team",ObservedPanel), t!("friends","Znajomi","Friends",NamesOnly), t!("invitations","Zaproszenia","Invitations",NamesOnly), t!("search","Wyszukaj konto","Account search",NamesOnly)]),
    p!("task-board", "Tablica zleceń", "Task Board", TaskBoard, false, ObservedPanel, None,
        &[f!("difficulty","Trudność","Difficulty",Status), f!("candidate","Zlecenie","Task",Value), f!("progress","Postęp","Progress",Progress), f!("reward","Nagroda","Reward",Details), f!("preferred","Preferowane zlecenia","Preferred tasks",Table), f!("talisman","Talizman zleceń","Bounty talisman",Details)], &[t!("bounty","Zlecenia łowieckie","Bounty Tasks",ObservedPanel), t!("weekly","Zadania tygodniowe","Weekly Tasks",ObservedPanel), t!("shop","Sklep zleceń","Hunting Task Shop",ObservedPanel)]),
];

#[must_use]
pub fn panel(id: &str) -> Option<&'static PanelDefinition> {
    PANELS.iter().find(|panel| panel.id == id)
}

/// Tab-specific fields replace the shared overview. This is layout metadata, not decoded
/// domain state; unknown values must remain unknown until the owning projection supplies them.
#[must_use]
#[rustfmt::skip]
pub fn fields_for_tab(panel_id: &str, tab_id: &str) -> &'static [PanelField] {
    match (panel_id, tab_id) {
        ("cyclopedia", "map") => &[f!("floor","Piętro","Floor",Value), f!("location","Położenie","Location",Value), f!("map","Mapa","Map",Details)],
        ("cyclopedia", "items") => &[f!("name","Przedmiot","Item",Value), f!("armor","Pancerz","Armor",Value), f!("weight","Waga","Weight",Value), f!("imbuements","Sloty nasyceń","Imbuement slots",Table), f!("offers","Oferty kupna i sprzedaży","Trade offers",Table), f!("market-value","Wartość rynkowa","Market value",Value), f!("valuation","Własna wycena łupu","Own loot value",Value), f!("tracking","Śledzenie zdobyczy","Drop tracking",Status)],
        ("cyclopedia", "houses") => &[f!("house","Dom lub siedziba gildii","House or guildhall",Value), f!("state","Stan","State",Status), f!("details","Szczegóły domu","House details",Details)],
        ("cyclopedia", "character") | ("cyclopedia", "character-stats") => &[f!("level","Poziom","Level",Progress), f!("experience","Doświadczenie","Experience",Progress), f!("skills","Umiejętności","Skills",Table), f!("health","Zdrowie","Health",Progress), f!("mana","Mana","Mana",Progress), f!("capacity","Udźwig","Capacity",Value), f!("speed","Szybkość","Speed",Value), f!("stamina","Wytrzymałość","Stamina",Duration), f!("offline-training","Trening offline","Offline training",Duration), f!("account-status","Stan konta","Account status",Status), f!("badges","Odznaki","Badges",Table)],
        ("cyclopedia", "offence") => &[f!("attack","Składniki ataku","Attack contributions",Table), f!("flat-damage","Stałe obrażenia","Flat damage",Value), f!("flat-healing","Stałe leczenie","Flat healing",Value), f!("critical-chance","Szansa krytyczna","Critical chance",Value), f!("critical-damage","Dodatkowe obrażenia krytyczne","Extra critical damage",Value)],
        ("cyclopedia", "defence") => &[f!("defence","Składniki obrony","Defence contributions",Table), f!("armor","Pancerz","Armor",Value), f!("mitigation","Redukcja obrażeń","Mitigation",Value), f!("magic-shield","Pojemność tarczy magicznej","Magic-shield capacity",Value), f!("reductions","Odporności według żywiołu","Damage-type reductions",Table)],
        ("cyclopedia", "blessings") => &[f!("blessings","Błogosławieństwa","Blessings",Table), f!("details","Szczegóły błogosławieństwa","Blessing details",Details)],
        ("cyclopedia", "deaths") | ("cyclopedia", "pvp-kills") => &[f!("records","Ostatnie zdarzenia","Recent records",Table), f!("details","Szczegóły zdarzenia","Record details",Details)],
        ("cyclopedia", "achievements") => &[f!("grades","Osiągnięcia według stopnia","Achievements by grade",Table), f!("achievement","Osiągnięcie","Achievement",Value), f!("date","Data zdobycia","Earned date",Value), f!("description","Opis","Description",Details)],
        ("cyclopedia", "item-summary") => &[f!("source","Ekwipunek, depozyt, skrzynka lub magazyn","Inventory, depot, inbox or stash",Status), f!("items","Przedmioty","Items",Table)],
        ("cyclopedia", "appearances") => &[f!("outfits","Stroje","Outfits",Table), f!("mounts","Wierzchowce","Mounts",Table), f!("familiars","Chowańce","Familiars",Table)],
        ("cyclopedia", "store-summary") => &[f!("benefits","Korzyści konta","Account benefits",Table), f!("details","Szczegóły korzyści","Benefit details",Details)],
        ("cyclopedia", "titles") => &[f!("current","Aktualny tytuł","Current title",Value), f!("titles","Tytuły","Titles",Table), f!("availability","Zablokowany lub odblokowany","Locked or unlocked",Status), f!("duration","Stały lub tymczasowy","Permanent or temporary",Status)],
        ("cyclopedia", "bestiary") | ("cyclopedia", "bosses") => &[f!("creature","Stworzenie lub boss","Creature or boss",Value), f!("category","Kategoria","Category",Status), f!("kills","Zabójstwa","Kills",Value), f!("progress","Postęp","Progress",Progress), f!("details","Szczegóły","Details",Details)],
        ("cyclopedia", "charms") => &[f!("major","Główne talizmany","Major charms",Table), f!("minor","Mniejsze talizmany","Minor charms",Table), f!("cost","Koszt","Cost",Value), f!("assignment","Przypisane stworzenie","Assigned creature",Value), f!("requirements","Wymagania i ograniczenia","Requirements and restrictions",Details)],
        ("cyclopedia", "boss-slots") => panel("boss-slots").map_or(&[], |panel| panel.fields),
        ("cyclopedia", "archive") => &[f!("spell","Czar lub runa","Spell or rune",Value), f!("combat","Szczegóły bojowe","Combat details",Details), f!("additional","Dodatkowe informacje","Additional details",Details)],
        ("wheel", "gems") => &[f!("gems","Kolekcja klejnotów","Gem collection",Table), f!("affinity","Powinowactwo","Affinity",Status), f!("quality","Jakość","Quality",Status), f!("cost","Koszt odkrycia","Revelation cost",Value)],
        ("wheel", "fragments") => &[f!("modifiers","Modyfikatory","Modifiers",Table), f!("grade","Klasa fragmentu","Fragment grade",Status), f!("cost","Koszt ulepszenia","Enhancement cost",Value)],
        ("analytics", "loot") | ("analytics", "supply") => &[f!("duration","Czas sesji","Session duration",Duration), f!("items","Przedmioty i ilości","Items and quantities",Table), f!("total","Łączna wartość","Total value",Value), f!("per-hour","Wartość na godzinę","Value per hour",Value)],
        ("analytics", "impact") => &[f!("damage","Zadane obrażenia","Damage dealt",Value), f!("dps","Obrażenia na sekundę","Damage per second",Value), f!("healing","Leczenie","Healing",Value), f!("hps","Leczenie na sekundę","Healing per second",Value), f!("maximum","Maksima sesji i historyczne","Session and all-time maxima",Table), f!("types","Podział według typu","Breakdown by type",Table)],
        ("analytics", "damage-input") => &[f!("damage","Otrzymane obrażenia","Damage received",Value), f!("maximum","Maksymalne obrażenia na sekundę","Maximum incoming DPS",Value), f!("sources","Źródła obrażeń","Damage sources",Table), f!("types","Typy obrażeń","Damage types",Table)],
        ("analytics", "experience") => &[f!("gain","Zdobyte doświadczenie","Experience gained",Value), f!("per-hour","Doświadczenie na godzinę","Experience per hour",Value), f!("next-level","Postęp do poziomu","Next-level progress",Progress)],
        ("analytics", "drops") => &[f!("items","Śledzone przedmioty","Tracked items",Table), f!("drops","Zdobycze","Drops",Value)],
        ("analytics", "party-hunt") => &[f!("duration","Czas polowania","Hunt duration",Duration), f!("pricing","Sposób wyceny łupów","Loot-price mode",Status), f!("members","Wyniki członków drużyny","Party-member results",Table)],
        ("analytics", "boss-cooldowns") => &[f!("boss","Boss","Boss",Value), f!("cooldown","Czas do następnego polowania","Time to next hunt",Duration)],
        ("forge", "conversion") => &[f!("dust","Pył","Dust",Value), f!("slivers","Odłamki","Slivers",Value), f!("cores","Rdzenie","Cores",Value), f!("dust-limit","Limit pyłu","Dust limit",Value), f!("requirements","Wymagane zasoby","Required resources",Table)],
        ("forge", "history") | ("reward-wall", "history") => &[f!("history","Historia operacji","Operation history",Table)],
        ("task-board", "weekly") => &[f!("difficulty","Trudność","Difficulty",Status), f!("kills","Zadania za zabójstwa","Kill tasks",Progress), f!("deliveries","Zadania za dostawy","Delivery tasks",Progress), f!("rewards","Nagrody","Rewards",Details)],
        ("task-board", "shop") => &[f!("offers","Stroje, dodatki i wierzchowce","Outfits, addons and mounts",Table), f!("cost","Koszt","Cost",Value), f!("availability","Dostępność oferty","Offer availability",Status)],
        ("social", "friends") | ("social", "invitations") | ("social", "search") => &[f!("account","Konto","Account",Value), f!("status","Stan","Status",Status), f!("details","Szczegóły","Details",Details)],
        _ => panel(panel_id).map_or(&[], |panel| panel.fields),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn audited_shortcuts_have_unique_bounded_stable_identifiers_and_original_defaults() {
        assert_eq!(PANELS.len(), 28);
        assert_eq!(
            PANELS.iter().filter(|panel| panel.default_visible).count(),
            11
        );
        let mut ids = BTreeSet::new();
        for definition in PANELS {
            assert!(ids.insert(definition.id));
            assert!(!definition.id.is_empty() && definition.id.len() <= 40);
            assert!(
                definition
                    .id
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte == b'-')
            );
            assert!(!definition.polish.is_empty() && !definition.english.is_empty());
            assert!(!definition.fields.is_empty() && definition.fields.len() <= 20);
            assert!(definition.tabs.len() <= 24);
            let mut fields = BTreeSet::new();
            assert!(
                definition
                    .fields
                    .iter()
                    .all(|field| fields.insert(field.key))
            );
            let mut tabs = BTreeSet::new();
            assert!(definition.tabs.iter().all(|tab| tabs.insert(tab.id)));
            assert_eq!(panel(definition.id), Some(definition));
        }
        assert!(panel("not-a-panel").is_none());
    }

    #[test]
    fn observed_analytics_routes_and_names_only_controls_keep_distinct_provenance() {
        let analytics = panel("analytics").map(|panel| panel.tabs);
        assert_eq!(analytics.map(<[PanelTab]>::len), Some(9));
        let names_only = panel("social").map(|panel| {
            panel
                .tabs
                .iter()
                .filter(|tab| tab.evidence == Evidence::NamesOnly)
                .count()
        });
        assert_eq!(names_only, Some(3));
        assert_eq!(
            panel("help").map(|panel| panel.evidence),
            Some(Evidence::ObservedShortcut)
        );
    }
}
