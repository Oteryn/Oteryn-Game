//! Intended preferences catalogue. Observed reference UI is evidence of scope, not
//! implemented Oteryn behavior. Pending preferences have no assumed default or consumer.
//! The names-only inventory is deliberately separate: a Qt/config key is not a control.

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OptionKind {
    Toggle,
    Integer {
        min: i32,
        max: i32,
    },
    Decimal {
        min: f32,
        max: f32,
    },
    /// Empty choices mean the precise supported choices are still unspecified.
    Choice(&'static [LocalizedChoice]),
    Text {
        max_bytes: usize,
    },
    Binding,
    /// An operation, never a persisted preference or implied executed command.
    Action,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LocalizedChoice {
    pub id: &'static str,
    pub pl: &'static str,
    pub en: &'static str,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    ObservedReferenceUi,
    NamesOnly,
    OterynRequested,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Implementation {
    Implemented { field: &'static str },
    Pending { consumer: &'static str },
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettingOption {
    pub id: &'static str,
    pub pl: &'static str,
    pub en: &'static str,
    pub kind: OptionKind,
    pub evidence: Evidence,
    /// Names-only provenance, never a config value or a statement of tested behavior.
    pub reference_key: Option<&'static str>,
    pub implementation: Implementation,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettingsSection {
    pub id: &'static str,
    pub pl: &'static str,
    pub en: &'static str,
    pub options: &'static [SettingOption],
}
impl SettingOption {
    pub const fn text(&self, english: bool) -> &'static str {
        if english { self.en } else { self.pl }
    }
}
impl SettingsSection {
    pub const fn text(&self, english: bool) -> &'static str {
        if english { self.en } else { self.pl }
    }
}
impl LocalizedChoice {
    pub const fn text(&self, english: bool) -> &'static str {
        if english { self.en } else { self.pl }
    }
}
pub const NAMES_ONLY_INVENTORY_JSON: &str = include_str!("../REFERENCE-PREFERENCE-KEYS.json");
pub const NAMES_ONLY_INVENTORY_COUNT: usize = 213;
/// Saved player intent only. These values do not enable or authorize game behavior.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(
    tag = "type",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum FutureValue {
    Bool(bool),
    Int(i32),
    Decimal(f32),
    Text(String),
    Choice(String),
    Binding(ShortcutBinding),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShortcutBinding {
    pub key: u16,
    pub ctrl: bool,
    pub alt: bool,
    pub shift: bool,
    pub meta: bool,
}

/// Missing entries mean unknown/unselected; no invented defaults or secret-bearing fields.
pub fn validate_future_preferences(
    values: &std::collections::BTreeMap<String, FutureValue>,
) -> std::io::Result<()> {
    let invalid = || {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid future preferences",
        )
    };
    if values.len() > 256 {
        return Err(invalid());
    }
    for (key, value) in values {
        let option = SETTINGS_SECTIONS
            .iter()
            .find_map(|section| {
                section.options.iter().find(|option| {
                    key == &format!("{}.{}", section.id, option.id)
                        && matches!(option.implementation, Implementation::Pending { .. })
                })
            })
            .ok_or_else(invalid)?;
        let valid = match (option.kind, value) {
            (Toggle, FutureValue::Bool(_)) => true,
            (Integer { min, max }, FutureValue::Int(value)) => (min..=max).contains(value),
            (Decimal { min, max }, FutureValue::Decimal(value)) => {
                value.is_finite() && (min..=max).contains(value)
            }
            (Text { max_bytes }, FutureValue::Text(value)) => {
                value.len() <= max_bytes && !value.chars().any(char::is_control)
            }
            (Choice(choices), FutureValue::Choice(value)) => {
                choices.iter().any(|choice| choice.id == value)
            }
            (Binding, FutureValue::Binding(value)) => {
                oteryn_input_actions::KeyCode::new(value.key).is_ok()
            }
            _ => false,
        };
        if !valid {
            return Err(invalid());
        }
    }
    if serde_json::to_vec(values)?.len() > 24 * 1024 {
        return Err(invalid());
    }
    Ok(())
}
macro_rules! p {
    ($id:literal, $pl:literal, $en:literal, $kind:expr, $consumer:literal) => {
        SettingOption {
            id: $id,
            pl: $pl,
            en: $en,
            kind: $kind,
            evidence: Evidence::ObservedReferenceUi,
            reference_key: None,
            implementation: Implementation::Pending {
                consumer: $consumer,
            },
        }
    };
}
macro_rules! r {
    ($id:literal, $pl:literal, $en:literal, $kind:expr, $field:literal) => {
        SettingOption {
            id: $id,
            pl: $pl,
            en: $en,
            kind: $kind,
            evidence: Evidence::OterynRequested,
            reference_key: None,
            implementation: Implementation::Implemented { field: $field },
        }
    };
}
macro_rules! i {
    ($id:literal, $pl:literal, $en:literal, $kind:expr, $field:literal) => {
        SettingOption {
            implementation: Implementation::Implemented { field: $field },
            ..p!($id, $pl, $en, $kind, "local action bar layout")
        }
    };
}
macro_rules! q {
    ($id:literal, $pl:literal, $en:literal, $kind:expr, $consumer:literal) => {
        SettingOption {
            evidence: Evidence::OterynRequested,
            ..p!($id, $pl, $en, $kind, $consumer)
        }
    };
}
macro_rules! n {
    ($id:literal, $pl:literal, $en:literal, $kind:expr, $consumer:literal, $key:literal) => {
        SettingOption {
            evidence: Evidence::NamesOnly,
            reference_key: Some($key),
            ..p!($id, $pl, $en, $kind, $consumer)
        }
    };
}
macro_rules! section {
    ($id:literal, $pl:literal, $en:literal, [$($option:expr),* $(,)?]) => {
        SettingsSection { id: $id, pl: $pl, en: $en, options: &[$($option),*] }
    };
}
use OptionKind::{Action, Binding, Choice, Decimal, Integer, Text, Toggle};
const UNKNOWN_CHOICES: OptionKind = Choice(&[]);
// Mouse dropdown membership verified in the reference Options draft (Cancel only).
const MOUSE_PRESETS: &[LocalizedChoice] = &[
    LocalizedChoice {
        id: "classic_controls",
        pl: "Klasyczne sterowanie",
        en: "Classic Controls",
    },
    LocalizedChoice {
        id: "regular_controls",
        pl: "Zwykłe sterowanie",
        en: "Regular Controls",
    },
    LocalizedChoice {
        id: "left_smart_click",
        pl: "Lewy Smart-Click",
        en: "Left Smart-Click",
    },
];
const ANTIALIASING: &[LocalizedChoice] = &[
    LocalizedChoice {
        id: "none",
        pl: "Brak",
        en: "None",
    },
    LocalizedChoice {
        id: "antialiasing",
        pl: "Wygładzanie",
        en: "Antialiasing",
    },
    LocalizedChoice {
        id: "smooth_retro",
        pl: "Smooth Retro",
        en: "Smooth Retro",
    },
];
const LOOT_COLOUR: &[LocalizedChoice] = &[
    LocalizedChoice {
        id: "none",
        pl: "Brak",
        en: "None",
    },
    LocalizedChoice {
        id: "frames",
        pl: "Ramki",
        en: "Frames",
    },
    LocalizedChoice {
        id: "corners",
        pl: "Narożniki",
        en: "Corners",
    },
];
const PERCENT: OptionKind = Integer { min: 0, max: 100 };
#[rustfmt::skip]
pub const CONTAINER_SORT_CHOICES: &[LocalizedChoice] = &[
    LocalizedChoice { id: "manual", pl: "Ręcznie", en: "Manual" },
    LocalizedChoice { id: "name_ascending", pl: "Nazwa rosnąco", en: "Name ascending" },
    LocalizedChoice { id: "name_descending", pl: "Nazwa malejąco", en: "Name descending" },
    LocalizedChoice { id: "amount_ascending", pl: "Ilość rosnąco", en: "Stack size ascending" },
    LocalizedChoice { id: "amount_descending", pl: "Ilość malejąco", en: "Stack size descending" },
    LocalizedChoice { id: "weight_ascending", pl: "Waga rosnąco", en: "Weight ascending" },
    LocalizedChoice { id: "weight_descending", pl: "Waga malejąco", en: "Weight descending" },
    LocalizedChoice { id: "expiry_ascending", pl: "Termin wygaśnięcia rosnąco", en: "Expiry ascending" },
    LocalizedChoice { id: "expiry_descending", pl: "Termin wygaśnięcia malejąco", en: "Expiry descending" },
];

#[rustfmt::skip]
pub const SETTINGS_SECTIONS: &[SettingsSection] = &[
    section!("basic", "Podstawowe", "Basic Options", [
        p!("basic.advanced", "Pokaż opcje zaawansowane", "Show advanced options", Toggle, "settings discovery"),
        r!("basic.click_walk", "Chodzenie kliknięciem", "Click to walk", Toggle, "click_to_walk"),
        r!("basic.ui_scale", "Skala interfejsu", "Interface scale", Decimal { min: 0.8, max: 1.8 }, "ui_scale"),
    ]),
    section!("controls", "Sterowanie", "Controls", [
        p!("controls.turn_ctrl", "Obracanie: Ctrl", "Turn: Ctrl", Toggle, "movement actions"),
        p!("controls.turn_shift", "Obracanie: Shift", "Turn: Shift", Toggle, "movement actions"),
        p!("controls.turn_alt", "Obracanie: Alt", "Turn: Alt", Toggle, "movement actions"),
        p!("controls.complete_stack_ctrl", "Ctrl do przenoszenia całych stosów", "Press CTRL to Drag Complete Stacks", Toggle, "item-move amount intent"),
        p!("controls.mouse_preset", "Schemat sterowania myszą", "Mouse control preset", Choice(MOUSE_PRESETS), "input router"),
        r!("controls.movement", "Klawisze kierunków", "Movement keys", Binding, "movement_keys"),
        q!("controls.rotation", "Modyfikator obracania postaci", "Turn modifier", Binding, "movement actions"),
        q!("controls.held_keys", "Działanie przytrzymanych klawiszy", "Held-key behavior", UNKNOWN_CHOICES, "input router"),
        q!("controls.drag_modifier", "Modyfikator przeciągania", "Drag modifier", Binding, "inventory commands"),
        n!("controls.face_movement", "Zwracaj postać w kierunku ruchu", "Face movement direction", Toggle, "movement presentation", "alwaysTurnTowardsMoveDirection"),
        n!("controls.default_key_delay", "Używaj domyślnego opóźnienia klawiatury", "Use default keyboard delay", Toggle, "input repeat policy", "keyboardDelayUseDefault"),
        // Nonnegative i32 is the local draft bound; the reference's supported delay range is unknown.
        n!("controls.key_delay_ms", "Własne opóźnienie klawiatury (ms)", "Custom keyboard delay (ms)", Integer { min: 0, max: i32::MAX }, "input repeat policy", "keyboardDelayMs"),
        n!("controls.drag_all", "Domyślnie przenoś cały stos", "Move whole stack by default", Toggle, "item-move amount intent", "dragAndDropDefaultActionIsMoveAll"),
    ]),
    section!("general_hotkeys", "Skróty ogólne", "General Hotkeys", [
        q!("hotkeys.profile", "Profil skrótów", "Hotkey profile", UNKNOWN_CHOICES, "binding profiles"),
        q!("hotkeys.add_profile", "Dodaj profil", "Add profile", Action, "binding profiles"),
        q!("hotkeys.copy_profile", "Kopiuj profil", "Copy profile", Action, "binding profiles"),
        q!("hotkeys.rename_profile", "Zmień nazwę profilu", "Rename profile", Action, "binding profiles"),
        q!("hotkeys.remove_profile", "Usuń profil", "Remove profile", Action, "binding profiles"),
        q!("hotkeys.search", "Wyszukaj czynność", "Search actions", Text { max_bytes: 128 }, "action catalogue"),
        q!("hotkeys.primary", "Skrót główny", "Primary shortcut", Binding, "binding profiles"),
        q!("hotkeys.secondary", "Skrót dodatkowy", "Secondary shortcut", Binding, "binding profiles"),
        q!("hotkeys.chat_on", "Skrót przy aktywnym czacie", "Chat-on shortcut", Binding, "text/input routing"),
        q!("hotkeys.chat_off", "Skrót przy wyłączonym czacie", "Chat-off shortcut", Binding, "text/input routing"),
    ]),
    section!("action_hotkeys", "Skróty pasków akcji", "Action Bar Hotkeys", [
        p!("hotkeys.bottom_slots", "Skróty dolnych pasków", "Bottom-bar shortcuts", Binding, "action bar"),
        p!("hotkeys.left_slots", "Skróty lewych pasków", "Left-bar shortcuts", Binding, "action bar"),
        p!("hotkeys.right_slots", "Skróty prawych pasków", "Right-bar shortcuts", Binding, "action bar"),
    ]),
    section!("custom_hotkeys", "Własne skróty", "Custom Hotkeys", [
        q!("hotkeys.custom_action", "Dodaj własną czynność", "Add custom action", Action, "action catalogue"),
        q!("hotkeys.action_text", "Tekst czynności", "Action text", Text { max_bytes: 256 }, "supported action commands"),
        q!("hotkeys.custom_binding", "Skrót własnej czynności", "Custom-action shortcut", Binding, "supported action commands"),
    ]),
    section!("interface", "Interfejs", "Interface", [
        p!("interface.highlight_mouse", "Wyróżnij cel myszy", "Highlight Mouse Target", Toggle, "target presentation"),
        p!("interface.big_cursor", "Duży kursor myszy", "Show Big Mouse Cursor", Toggle, "cursor integration"),
        p!("interface.link_copy_warning", "Ostrzeżenie przy kopiowaniu linku", "Show Link Copy Warning", Toggle, "safe clipboard UI"),
        p!("interface.expiry_inventory", "Wygaśnięcie w ekwipunku", "Show Expiry in Inventory", Toggle, "item expiry projection"),
        p!("interface.expiry_containers", "Wygaśnięcie w pojemnikach", "Show Expiry in Containers", Toggle, "item expiry projection"),
        p!("interface.expiry_unused", "Wygaśnięcie nieużywanych przedmiotów", "Show Expiry on Unused Items", Toggle, "item expiry projection"),
        p!("interface.colourise_loot_value", "Koloruj wartość łupu", "Colourise Loot Value", Choice(LOOT_COLOUR), "item value presentation"),
        r!("interface.english", "Interfejs po angielsku", "English interface", Toggle, "english"),
        r!("interface.contrast", "Wyższy kontrast paneli", "Higher panel contrast", Toggle, "high_contrast"),
        r!("interface.reduced_motion", "Ograniczone animacje", "Reduced animation", Toggle, "reduced_motion"),
        q!("interface.cursor_size", "Rozmiar kursora", "Cursor size", UNKNOWN_CHOICES, "cursor integration"),
        q!("interface.cursor_animation", "Animowany kursor", "Animated cursor", Toggle, "cursor integration"),
        q!("interface.link_confirmation", "Potwierdzaj otwieranie linków", "Confirm external links", Toggle, "safe link routing"),
        n!("interface.system_cursor", "Używaj kursora systemowego", "Use system cursor", Toggle, "cursor integration", "mouseSystemCursor"),
    ]),
    section!("hud", "HUD i wskaźniki", "HUD", [
        p!("hud.owner_name", "Nazwa własnej postaci", "Own character name", Toggle, "actor HUD"),
        p!("hud.other_names", "Nazwy innych postaci i stworzeń", "Other actor names", Toggle, "actor HUD"),
        p!("hud.owner_health", "Własny pasek zdrowia", "Own health bar", Toggle, "actor HUD"),
        p!("hud.other_health", "Paski zdrowia innych", "Other health bars", Toggle, "actor HUD"),
        p!("hud.owner_mana", "Własny pasek many", "Own mana bar", Toggle, "actor HUD"),
        p!("hud.other_mana", "Paski many innych", "Other mana bars", Toggle, "supported resource projection"),
        p!("hud.marks", "Znaczniki postaci", "Actor marks", Toggle, "actor status projection"),
        p!("hud.npc_icons", "Ikony NPC", "NPC icons", Toggle, "NPC presentation"),
        p!("hud.arcs", "Łuki zdrowia i many", "Health and mana arcs", Toggle, "actor HUD"),
        p!("hud.arc_size", "Rozmiar łuków", "Arc size", PERCENT, "actor HUD"),
        p!("hud.arc_distance", "Odległość łuków od postaci", "Arc distance", PERCENT, "actor HUD"),
        p!("hud.arc_opacity", "Przezroczystość łuków", "Arc opacity", PERCENT, "actor HUD"),
        p!("hud.conditions_bar", "Stany postaci na pasku", "Conditions in status bar", Toggle, "condition projection"),
        p!("hud.conditions_world", "Stany postaci przy postaci", "Conditions beside character", Toggle, "condition projection"),
        p!("hud.conditions_order", "Kolejność stanów postaci", "Condition order", Action, "condition layout"),
        p!("hud.status_placement", "Położenie pasków stanu", "Status-bar placement", UNKNOWN_CHOICES, "HUD layout"),
        p!("hud.status_customization", "Zawartość pasków stanu", "Customize status bars", Action, "HUD layout"),
        n!("hud.player_enabled", "HUD własnej postaci", "Own character HUD", Toggle, "actor HUD", "playerHudEnabled"),
        n!("hud.creatures_enabled", "HUD pozostałych postaci i stworzeń", "Other actor HUD", Toggle, "actor HUD", "creatureHudEnabled"),
        n!("hud.resource_bars", "Paski zasobów", "Resource bars", Toggle, "actor HUD", "playerHudShowBars"),
        n!("hud.cooldown_bar", "Osobny pasek czasów odnowienia", "Standalone cooldown bar", Toggle, "cooldown projection", "cooldownBarEnabled"),
        n!("hud.harmony_left", "Harmonia po lewej stronie HUD", "Harmony on the left of HUD", Toggle, "vocation HUD", "playerHudShowHarmonyLeft"),
        n!("hud.serene_harmony", "Pokazuj spokojną harmonię", "Show serene Harmony", Toggle, "vocation condition projection", "playerShowHarmonySerene"),
    ]),
    section!("console", "Czat i konsola", "Console", [
        r!("console.visible", "Pokaż panel czatu", "Show chat panel", Toggle, "show_chat"),
        p!("console.font_size", "Rozmiar tekstu", "Text size", Integer { min: 10, max: 32 }, "chat presentation"),
        p!("console.timestamps", "Znaczniki czasu", "Timestamps", Toggle, "chat presentation"),
        p!("console.seconds", "Sekundy w znacznikach czasu", "Timestamp seconds", Toggle, "chat presentation"),
        p!("console.levels", "Poziomy przy nazwach", "Levels beside names", Toggle, "chat projection"),
        p!("console.own_status", "Własne komunikaty stanu", "Own status messages", Toggle, "session events"),
        p!("console.other_status", "Komunikaty stanu innych", "Other status messages", Toggle, "session events"),
        p!("console.events", "Komunikaty o zdarzeniach", "Event messages", Toggle, "session events"),
        p!("console.information", "Komunikaty informacyjne", "Information messages", Toggle, "session events"),
        p!("console.private_tab", "Osobna karta prywatnej rozmowy", "Private message tab", Toggle, "chat presentation"),
        n!("console.join_leave", "Komunikaty wejścia i wyjścia z kanału", "Channel join/leave messages", Toggle, "chat membership projection", "consoleShowJoinLeaveMessages"),
    ]),
    section!("game_window", "Okno gry", "Game Window", [
        p!("window.textual_effects", "Efekty tekstowe", "Show Textual Effects", Toggle, "effect projection"),
        p!("window.potion_sounds", "Efekty dźwiękowe mikstur", "Show Potion Sound Effects", Toggle, "effect projection"),
        q!("window.fit", "Dopasowanie obszaru gry", "Game viewport fit", UNKNOWN_CHOICES, "scene layout"),
        q!("window.top_pane", "Panel nad obszarem gry", "Top game pane", Toggle, "scene layout"),
        q!("window.bottom_pane", "Panel pod obszarem gry", "Bottom game pane", Toggle, "scene layout"),
        p!("window.private_messages", "Prywatne wiadomości nad grą", "Private messages over game", Toggle, "chat overlays"),
        p!("window.general_messages", "Komunikaty nad grą", "General messages over game", Toggle, "session overlays"),
        p!("window.own_spells", "Własne efekty zaklęć", "Own spell effects", Toggle, "effect projection"),
        p!("window.other_spells", "Efekty zaklęć innych", "Other spell effects", Toggle, "effect projection"),
        p!("window.hotkey_notices", "Komunikaty skrótów", "Hotkey messages", Toggle, "action feedback"),
        p!("window.loot_notices", "Komunikaty o łupach", "Loot messages", Toggle, "loot events"),
        p!("window.loot_highlight", "Wyróżnianie łupów", "Loot highlighting", Toggle, "loot projection"),
        p!("window.training", "Postęp treningu", "Training progress", Toggle, "training events"),
        p!("window.combat_frame", "Ramka walki", "Combat frame", Toggle, "combat projection"),
        p!("window.pvp_frame", "Ramka PvP", "PvP frame", Toggle, "PvP projection"),
        p!("window.attack_animation", "Animacja ataku", "Attack animation", Toggle, "combat presentation"),
        p!("window.banner", "Baner gry", "Game banner", Toggle, "scene presentation"),
        p!("window.target_frame", "Ramka celu", "Target frame", Toggle, "target projection"),
        p!("window.target_highlight", "Wyróżnianie celu", "Target highlight", Toggle, "target projection"),
        n!("window.potion_notices", "Komunikaty mikstur", "Potion messages", Toggle, "item-use notices", "gameWindowShowPotionMessages"),
        n!("window.store_notices", "Komunikaty sklepu", "Store messages", Toggle, "commercial notices", "gameWindowShowStoreMessages"),
        n!("window.boosted_creatures", "Komunikaty wzmocnionych stworzeń", "Boosted-creature messages", Toggle, "world notices", "gameWindowShowBoostedCreatureMessages"),
    ]),
    section!("action_bars", "Paski akcji", "Action Bars", [
        i!("bars.bottom_rows", "Dolne paski akcji", "Bottom action rows", Integer { min: 0, max: 3 }, "action_bottom_rows"),
        i!("bars.left_rows", "Lewe paski akcji", "Left action rows", Integer { min: 0, max: 3 }, "action_left_rows"),
        i!("bars.right_rows", "Prawe paski akcji", "Right action rows", Integer { min: 0, max: 3 }, "action_right_rows"),
        q!("bars.locked", "Zablokuj układ pasków", "Lock action bars", Toggle, "action bar layout"),
        p!("bars.labels", "Etykiety skrótów", "Hotkey labels", Toggle, "action bar"),
        p!("bars.item_amounts", "Liczba przedmiotów", "Item amounts", Toggle, "inventory projection"),
        p!("bars.spell_parameters", "Parametry zaklęć", "Spell parameters", Toggle, "spell actions"),
        p!("bars.graphic_cooldown", "Graficzny czas odnowienia", "Graphical cooldown", Toggle, "cooldown projection"),
        p!("bars.numeric_cooldown", "Liczbowy czas odnowienia", "Numeric cooldown", Toggle, "cooldown projection"),
        p!("bars.tooltips", "Podpowiedzi", "Tooltips", Toggle, "action bar"),
        p!("bars.auto_spells", "Dodawaj nowe zaklęcia", "Automatically add new spells", Toggle, "spell catalogue"),
        p!("bars.clear_row", "Wyczyść wybrany pasek", "Clear selected row", Action, "action bar layout"),
        i!("bars.bottom_visible", "Pokaż dolne paski", "Show bottom bars", Toggle, "action_bottom_enabled"),
        i!("bars.left_visible", "Pokaż lewe paski", "Show left bars", Toggle, "action_left_enabled"),
        i!("bars.right_visible", "Pokaż prawe paski", "Show right bars", Toggle, "action_right_enabled"),
        n!("bars.bottom_locked", "Zablokuj dolne paski", "Lock bottom bars", Toggle, "action bar layout", "actionBarBottomLocked"),
        n!("bars.left_locked", "Zablokuj lewe paski", "Lock left bars", Toggle, "action bar layout", "actionBarLeftLocked"),
        n!("bars.right_locked", "Zablokuj prawe paski", "Lock right bars", Toggle, "action bar layout", "actionBarRightLocked"),
        n!("bars.bottom_first", "Dolny pasek 1", "Bottom row 1", Toggle, "action bar layout", "actionBarShowBottom1"),
        n!("bars.bottom_second", "Dolny pasek 2", "Bottom row 2", Toggle, "action bar layout", "actionBarShowBottom2"),
        n!("bars.bottom_third", "Dolny pasek 3", "Bottom row 3", Toggle, "action bar layout", "actionBarShowBottom3"),
        n!("bars.left_first", "Lewy pasek 1", "Left row 1", Toggle, "action bar layout", "actionBarShowLeft1"),
        n!("bars.left_second", "Lewy pasek 2", "Left row 2", Toggle, "action bar layout", "actionBarShowLeft2"),
        n!("bars.left_third", "Lewy pasek 3", "Left row 3", Toggle, "action bar layout", "actionBarShowLeft3"),
        n!("bars.right_first", "Prawy pasek 1", "Right row 1", Toggle, "action bar layout", "actionBarShowRight1"),
        n!("bars.right_second", "Prawy pasek 2", "Right row 2", Toggle, "action bar layout", "actionBarShowRight2"),
        n!("bars.right_third", "Prawy pasek 3", "Right row 3", Toggle, "action bar layout", "actionBarShowRight3"),
    ]),
    section!("shortcuts", "Skróty paneli", "Shortcuts", [
        p!("shortcuts.visible", "Widoczne skróty paneli", "Displayed panel shortcuts", Action, "panel registry"),
        p!("shortcuts.available", "Dostępne skróty paneli", "Available panel shortcuts", Action, "panel registry"),
        p!("shortcuts.add", "Dodaj skrót panelu", "Add panel shortcut", Action, "panel registry"),
        p!("shortcuts.remove", "Usuń skrót panelu", "Remove panel shortcut", Action, "panel registry"),
        p!("shortcuts.order", "Zmień kolejność skrótów", "Reorder shortcuts", Action, "panel layout"),
    ]),
    section!("graphics", "Grafika", "Graphics", [
        r!("graphics.fullscreen", "Pełny ekran bez ramki", "Borderless fullscreen", Toggle, "fullscreen"),
        r!("graphics.width", "Szerokość okna", "Window width", Integer { min: 800, max: 3840 }, "window_width"),
        r!("graphics.height", "Wysokość okna", "Window height", Integer { min: 600, max: 2160 }, "window_height"),
        r!("graphics.vsync", "Synchronizacja pionowa", "VSync", Toggle, "vsync"),
        r!("graphics.fps", "Limit FPS (0: bez limitu)", "FPS limit (0: unlimited)", Integer { min: 0, max: 360 }, "fps"),
        r!("graphics.background_fps", "Limit FPS w tle", "Background FPS limit", Integer { min: 5, max: 60 }, "background_fps"),
        p!("graphics.engine", "Obsługiwany silnik graficzny", "Supported graphics engine", UNKNOWN_CHOICES, "renderer capabilities"),
        p!("graphics.antialiasing", "Wygładzanie obrazu", "Antialiasing", Choice(ANTIALIASING), "renderer capabilities"),
        p!("graphics.integer_scale", "Skalowanie całkowite", "Integer scaling", Toggle, "scene viewport"),
        q!("graphics.monitor", "Monitor", "Monitor", UNKNOWN_CHOICES, "window integration"),
        q!("graphics.fps_indicator", "Wskaźnik FPS", "FPS indicator", Toggle, "frame diagnostics"),
        q!("graphics.latency_indicator", "Wskaźnik opóźnienia", "Latency indicator", Toggle, "session diagnostics"),
    ]),
    section!("effects", "Efekty graficzne", "Effects", [
        p!("effects.ambient_light", "Światło otoczenia", "Ambient light", PERCENT, "lighting renderer"),
        p!("effects.level_separator", "Oddzielenie pięter", "Level separator", PERCENT, "floor projection"),
        p!("effects.indoor_light", "Tłumienie światła we wnętrzach", "Indoor light attenuation", PERCENT, "lighting renderer"),
        p!("effects.cloud_light", "Tłumienie światła przez chmury", "Cloud light attenuation", PERCENT, "lighting renderer"),
        p!("effects.own_opacity", "Widoczność własnych zaklęć", "Own spell opacity", PERCENT, "effect renderer"),
        p!("effects.other_opacity", "Widoczność zaklęć innych", "Other-player spell opacity", PERCENT, "effect renderer"),
        p!("effects.creature_opacity", "Widoczność zaklęć stworzeń", "Creature spell opacity", PERCENT, "effect renderer"),
        p!("effects.boss_opacity", "Widoczność obszarów ataków bossów", "Boss-area spell opacity", PERCENT, "effect renderer"),
        n!("effects.lighting", "Efekty oświetlenia", "Lighting effects", Toggle, "lighting renderer", "lightEffectsEnabled"),
    ]),
    section!("sound", "Dźwięk", "Sound", [
        p!("sound.device", "Urządzenie wyjściowe", "Output device", UNKNOWN_CHOICES, "audio backend"),
        p!("sound.master", "Głośność główna", "Master volume", PERCENT, "audio mixer"),
        p!("sound.music", "Muzyka", "Music volume", PERCENT, "audio mixer"),
        p!("sound.ambience", "Dźwięki otoczenia", "Ambience volume", PERCENT, "audio mixer"),
        p!("sound.items", "Dźwięki przedmiotów", "Item volume", PERCENT, "audio mixer"),
        p!("sound.events", "Dźwięki zdarzeń", "Event volume", PERCENT, "audio mixer"),
        q!("sound.background_mute", "Wycisz w tle", "Mute in background", Toggle, "audio mixer"),
        n!("sound.anthem", "Odtwarzaj hymn", "Play anthem", Toggle, "audio event policy", "soundAnthemEnabled"),
    ]),
    section!("battle_sounds", "Dźwięki walki", "Battle Sounds", [
        p!("sound.own_combat", "Dźwięki własnej walki", "Own combat volume", PERCENT, "combat audio"),
        p!("sound.other_combat", "Dźwięki walki innych graczy", "Other-player combat volume", PERCENT, "combat audio"),
        p!("sound.creature_combat", "Dźwięki walki stworzeń", "Creature combat volume", PERCENT, "combat audio"),
        p!("sound.attack_spells", "Własne zaklęcia ofensywne", "Own attack spells", Toggle, "spell audio"),
        p!("sound.healing_spells", "Własne zaklęcia lecznicze", "Own healing spells", Toggle, "spell audio"),
        p!("sound.support_spells", "Własne zaklęcia wspierające", "Own support spells", Toggle, "spell audio"),
        p!("sound.weapons", "Własna broń", "Own weapons", Toggle, "combat audio"),
        p!("sound.other_attack_spells", "Zaklęcia ofensywne innych", "Other attack spells", Toggle, "spell audio"),
        p!("sound.other_healing_spells", "Zaklęcia lecznicze innych", "Other healing spells", Toggle, "spell audio"),
        p!("sound.other_support_spells", "Zaklęcia wspierające innych", "Other support spells", Toggle, "spell audio"),
        p!("sound.other_weapons", "Broń innych", "Other weapons", Toggle, "combat audio"),
        p!("sound.creature_attacks", "Ataki stworzeń", "Creature attacks", Toggle, "creature audio"),
        p!("sound.creature_noise", "Odgłosy stworzeń", "Creature noises", Toggle, "creature audio"),
        p!("sound.creature_death", "Śmierć stworzeń", "Creature deaths", Toggle, "creature audio"),
        n!("sound.own_spells", "Własne zaklęcia — cała grupa", "Own spells — entire group", Toggle, "combat audio", "soundBattleOwnSpellsEnabled"),
        n!("sound.other_spells", "Zaklęcia innych — cała grupa", "Others' spells — entire group", Toggle, "combat audio", "soundBattleOthersSpellsEnabled"),
    ]),
    section!("ui_sounds", "Dźwięki interfejsu", "UI Sounds", [
        p!("sound.ui", "Głośność interfejsu", "UI volume", PERCENT, "UI audio"),
        p!("sound.interactions", "Kliknięcia i interakcje", "UI interactions", Toggle, "UI audio"),
        p!("sound.item_movement", "Przenoszenie przedmiotów", "Item movement", Toggle, "inventory audio"),
        p!("sound.eating", "Jedzenie", "Eating", Toggle, "item-use audio"),
        p!("sound.party_notice", "Powiadomienia drużyny", "Party notifications", Toggle, "social audio"),
        p!("sound.vip_notice", "Powiadomienia znajomych", "VIP notifications", Toggle, "social audio"),
        p!("sound.chat_channels", "Dźwięki wybranych kanałów", "Chat-channel sounds", Action, "chat audio filters"),
        n!("sound.chat_enabled", "Powiadomienia czatu", "Chat notifications", Toggle, "chat audio", "ChatEnabled"),
        n!("sound.global", "Kanały ogólne", "Global channels", Toggle, "chat audio", "GlobalMessagesEnabled"),
        n!("sound.guild", "Wiadomości gildii", "Guild messages", Toggle, "chat audio", "GuildMessagesEnabled"),
        n!("sound.npc", "Rozmowy z NPC", "NPC conversations", Toggle, "chat audio", "NpcMessageseEnabled"),
        n!("sound.party_chat", "Czat drużyny", "Party chat", Toggle, "chat audio", "PartyMessagesEnabled"),
        n!("sound.private", "Wiadomości prywatne", "Private messages", Toggle, "chat audio", "PrivateMessagesEnabled"),
        n!("sound.private_without_tab", "Prywatne wiadomości bez osobnej karty", "Private messages without a tab", Toggle, "chat audio", "PrivateMessagesWithoutTabEnabled"),
        n!("sound.raids", "Komunikaty najazdów", "Raid messages", Toggle, "world-event audio", "RaidMessagesEnabled"),
        n!("sound.system", "Komunikaty systemowe", "System messages", Toggle, "session audio", "SystemMessagesEnabled"),
        n!("sound.team_finder", "Wyszukiwarka drużyn", "Team finder", Toggle, "social audio", "TeamFinderMessagesEnabled"),
    ]),
    section!("miscellaneous", "Różne", "Miscellaneous", [
        p!("misc.confirmations", "Potwierdzenia czynności", "Action confirmations", Action, "safe command UI"),
        p!("misc.session_retention", "Zachowanie sesji", "Session retention", UNKNOWN_CHOICES, "accepted session policy"),
        p!("misc.connection_stability", "Stabilność połączenia", "Connection stability", UNKNOWN_CHOICES, "accepted transport policy"),
        p!("misc.quick_login", "Szybkie logowanie", "Quick login", Toggle, "approved device-session contract"),
    ]),
    section!("gameplay", "Rozgrywka", "Gameplay", [
        p!("gameplay.inspect_permission", "Zezwalaj na oglądanie postaci", "Allow character inspection", Toggle, "inspection authority"),
        p!("gameplay.cancel_chase", "Przerywaj pościg przy ruchu", "Cancel chase on movement", Toggle, "combat commands"),
        p!("gameplay.nearby_corpses", "Łupienie pobliskich ciał", "Loot nearby corpses", Toggle, "accepted loot commands"),
    ]),
    section!("screenshots", "Zrzuty ekranu", "Screenshots", [
        q!("capture.manual", "Zrób zrzut ekranu", "Take screenshot", Action, "renderer capture"),
        q!("capture.folder", "Folder zrzutów", "Screenshot folder", Text { max_bytes: 1024 }, "capture storage"),
        p!("capture.game_only", "Tylko obszar gry", "Game-only capture", Toggle, "renderer capture"),
        p!("capture.backlog", "Pięciosekundowy bufor zrzutów", "Five-second backlog", Toggle, "bounded capture buffer"),
        p!("capture.progression", "Przy awansie i postępie", "Progression events", Toggle, "progression events"),
        p!("capture.achievements", "Przy osiągnięciu", "Achievement events", Toggle, "achievement events"),
        p!("capture.bestiary", "Przy postępie bestiariusza", "Bestiary events", Toggle, "bestiary events"),
        p!("capture.treasure", "Przy znalezieniu skarbu", "Treasure events", Toggle, "treasure events"),
        p!("capture.loot", "Przy zdobyciu łupu", "Loot events", Toggle, "loot events"),
        p!("capture.boss", "Przy pokonaniu bossa", "Boss events", Toggle, "boss events"),
        p!("capture.death", "Przy śmierci", "Death events", Toggle, "death events"),
        p!("capture.pvp", "Przy zdarzeniu PvP", "PvP events", Toggle, "PvP events"),
        p!("capture.damage", "Przy wysokich obrażeniach", "Damage events", Toggle, "combat events"),
        p!("capture.healing", "Przy wysokim leczeniu", "Healing events", Toggle, "combat events"),
        p!("capture.low_health", "Przy niskim zdrowiu", "Low-health events", Toggle, "vitals projection"),
        p!("capture.gift_of_life", "Przy efekcie Gift of Life", "Gift of Life events", Toggle, "accepted resurrection mechanic"),
        n!("capture.auto_enabled", "Automatyczne zrzuty", "Automatic screenshots", Toggle, "capture events", "screenshotsAutoScreenshotsEnabled"),
        n!("capture.skill_up", "Przy awansie umiejętności", "Skill-up events", Toggle, "progression events", "screenshotsSkillUpEnabled"),
        n!("capture.bestiary_partial", "Przy częściowym odkryciu bestiariusza", "Partial bestiary completion", Toggle, "bestiary events", "screenshotsBestiaryPartlyEnabled"),
        n!("capture.bestiary_full", "Przy pełnym odkryciu bestiariusza", "Full bestiary completion", Toggle, "bestiary events", "screenshotsBestiaryFullEnabled"),
        n!("capture.death_pve", "Przy śmierci PvE", "PvE death", Toggle, "death events", "screenshotsDeathPvEEnabled"),
        n!("capture.death_pvp", "Przy śmierci PvP", "PvP death", Toggle, "death events", "screenshotsDeathPvPEnabled"),
        n!("capture.pvp_attack", "Przy ataku PvP", "PvP attack", Toggle, "PvP events", "screenshotsPvPAttackEnabled"),
        n!("capture.player_kill", "Przy pokonaniu gracza", "Player kill", Toggle, "PvP events", "screenshotsPlayerKillFullEnabled"),
        n!("capture.player_kill_assist", "Przy pomocy w pokonaniu gracza", "Player-kill assist", Toggle, "PvP events", "screenshotsPlayerKillAssistEnabled"),
        n!("capture.reward_chest", "Przy skrzyni nagród", "Reward chest", Toggle, "reward events", "screenshotsRewardChestEnabled"),
    ]),
    section!("help", "Pomoc i dane lokalne", "Help", [
        p!("help.documentation", "Otwórz dokumentację", "Open documentation", Action, "safe link routing"),
        p!("help.import_options", "Importuj ustawienia", "Import options", Action, "validated preference import"),
        p!("help.export_options", "Eksportuj ustawienia", "Export options", Action, "credential-free preference export"),
        p!("help.import_minimap", "Importuj minimapę", "Import minimap", Action, "validated minimap import"),
        p!("help.export_minimap", "Eksportuj minimapę", "Export minimap", Action, "minimap export"),
        p!("help.reset_options", "Przywróć ustawienia", "Reset options", Action, "explicit local confirmation"),
        p!("help.reset_minimap", "Wyczyść minimapę", "Reset minimap", Action, "explicit local confirmation"),
    ]),
    section!("panels", "Panele i minimapa", "Panels and Minimap", [
        r!("panels.inventory", "Pokaż ekwipunek", "Show inventory", Toggle, "show_inventory"),
        r!("panels.battle", "Pokaż listę walki", "Show battle list", Toggle, "show_battle"),
        r!("panels.minimap", "Pokaż minimapę", "Show minimap", Toggle, "show_minimap"),
        p!("panels.skills", "Pokaż umiejętności", "Show skills", Toggle, "skills projection"),
        p!("panels.quest_tracker", "Pokaż śledzenie zadań", "Show quest tracker", Toggle, "quest projection"),
        p!("panels.layout", "Rozmieszczenie paneli", "Panel placement", Action, "panel layout"),
        p!("panels.battle_sort", "Sortowanie listy walki", "Battle-list sorting", UNKNOWN_CHOICES, "battle presentation"),
        p!("panels.battle_filters", "Filtry listy walki", "Battle-list filters", Action, "battle presentation"),
        p!("panels.minimap_layers", "Warstwy minimapy", "Minimap layers", Action, "minimap projection"),
        p!("panels.minimap_filters", "Filtry minimapy", "Minimap filters", Action, "minimap projection"),
    ]),
    section!("containers", "Pojemniki i łupy", "Containers and Loot", [
        p!("containers.loot_assignment", "Pojemniki łupów według kategorii", "Loot containers by category", Action, "accepted inventory/loot commands"),
        p!("containers.obtain_assignment", "Pojemniki zdobywanych przedmiotów", "Obtain containers by category", Action, "accepted inventory commands"),
        p!("containers.main_fallback", "Używaj głównego pojemnika awaryjnie", "Use main-container fallback", Toggle, "accepted inventory commands"),
        p!("containers.accepted_loot", "Lista akceptowanych łupów", "Accepted loot list", Action, "accepted loot preferences"),
        p!("containers.skipped_loot", "Lista pomijanych łupów", "Skipped loot list", Action, "accepted loot preferences"),
        p!("containers.loot_search", "Wyszukaj na liście łupów", "Search loot lists", Text { max_bytes: 128 }, "item catalogue"),
        p!("containers.loot_valuation", "Wycena łupów", "Loot valuation", UNKNOWN_CHOICES, "accepted item valuation"),
        p!("containers.track_drop", "Śledź łupy wybranego przedmiotu", "Track selected item drops", Action, "drop tracker"),
        p!("containers.skip_item", "Pomijaj wybrany przedmiot", "Skip selected item when looting", Action, "accepted loot preferences"),
        n!("containers.backpacks_first", "Plecaki przed innymi przedmiotami", "Backpacks first", Toggle, "container presentation", "containerSortBackpacksFirst"),
        n!("containers.sort_recursive", "Sortuj podkontenery", "Sort nested containers", Toggle, "container presentation", "containerSortRecursive"),
        n!("containers.confirm_recursive_sort", "Potwierdzaj sortowanie podkontenerów", "Confirm nested-container sorting", Toggle, "container confirmation", "containerSortRecursiveShowWarningAgain"),
        n!("containers.managed_recursive", "Przenoś zawartość podkontenerów do przypisanych pojemników", "Move nested contents to assigned containers", Toggle, "accepted managed-container intents", "containerMoveToManagedContainerRecursive"),
        n!("containers.confirm_managed_move", "Potwierdzaj przenoszenie podkontenerów", "Confirm nested-container moves", Toggle, "container confirmation", "containerMoveToManagedContainerRecursiveShowWarningAgain"),
        n!("containers.expiry", "Wygaśnięcie w kontenerach", "Expiry in containers", Toggle, "item expiry projection", "showExpireInContainers"),
        n!("containers.inventory_expiry", "Wygaśnięcie w ekwipunku", "Expiry in inventory", Toggle, "item expiry projection", "showExpireInInventory"),
        n!("containers.unused_expiry", "Wygaśnięcie nieużywanych przedmiotów", "Expiry of unused items", Toggle, "item expiry projection", "showExpireWhenUnused"),
        n!("containers.confirm_stash", "Potwierdzaj odłożenie zawartości do magazynu", "Confirm stowing container contents", Toggle, "stash confirmation", "stashAskBeforeStowContainerContent"),
        q!("containers.sort_order", "Kolejność sortowania", "Sort order", Choice(CONTAINER_SORT_CHOICES), "container presentation"),
    ]),
    section!("character_selection", "Wybór postaci", "Character Selection", [
        n!("characters.hidden", "Pokaż ukryte postacie", "Show hidden characters", Toggle, "character-list presentation", "characterSelectionShowHidden"),
        n!("characters.outfits", "Pokaż stroje postaci", "Show character outfits", Toggle, "character appearance projection", "characterSelectionShowOutfits"),
        n!("characters.sort_column", "Sortuj listę według kolumny", "Sort by column", UNKNOWN_CHOICES, "character-list presentation", "characterSelectionSortColumn"),
        n!("characters.sort_ascending", "Rosnąca kolejność postaci", "Ascending character order", Toggle, "character-list presentation", "characterSelectionSortAscendingOrder"),
    ]),
    section!("market", "Rynek", "Market", [
        n!("market.locker_only", "Tylko przedmioty z depozytu", "Locker items only", Toggle, "market item eligibility", "marketShowLockerOnly"),
    ]),
    section!("floating_messages", "Teksty nad grą", "Floating Messages", [
        p!("floating.enabled", "Tekstowe efekty w grze", "Textual game effects", Toggle, "scene text renderer"),
        q!("floating.own_messages", "Własne teksty nad postacią", "Own floating messages", Toggle, "message projection"),
        q!("floating.other_messages", "Teksty innych nad postaciami", "Other floating messages", Toggle, "message projection"),
    ]),
    section!("analyzers", "Analizatory", "Analyzers", [
        p!("analyzers.session_values", "Wartości bieżącej sesji", "Current-session values", Toggle, "session analyzers"),
        p!("analyzers.per_hour", "Wartości na godzinę", "Per-hour values", Toggle, "session analyzers"),
        p!("analyzers.damage_types", "Podział według rodzaju obrażeń", "Damage-type breakdown", Toggle, "combat analyzers"),
        p!("analyzers.damage_sources", "Podział według źródła obrażeń", "Damage-source breakdown", Toggle, "combat analyzers"),
        p!("analyzers.damage_graph", "Wykres otrzymanych obrażeń", "Damage-input graph", Toggle, "combat analyzers"),
        p!("analyzers.gauges", "Wskaźniki analizatorów", "Analyzer gauges", Toggle, "analyzer presentation"),
        p!("analyzers.graphs", "Wykresy analizatorów", "Analyzer graphs", Toggle, "analyzer presentation"),
        p!("analyzers.raw_xp", "Surowe doświadczenie", "Raw experience", Toggle, "XP analyzer"),
        p!("analyzers.party_valuation", "Wycena łupów drużyny", "Party-hunt loot pricing", UNKNOWN_CHOICES, "party analyzer"),
        p!("analyzers.boss_sort", "Sortowanie czasów bossów", "Boss-cooldown sorting", UNKNOWN_CHOICES, "boss cooldown projection"),
        p!("analyzers.reset", "Wyzeruj wybrany analizator", "Reset selected analyzer", Action, "explicit local confirmation"),
        p!("analyzers.reset_high", "Wyzeruj rekord analizatora", "Reset analyzer all-time high", Action, "explicit local confirmation"),
        p!("analyzers.copy", "Kopiuj wyniki analizatora", "Copy analyzer results", Action, "clipboard privacy"),
    ]),
    section!("notifications", "Powiadomienia", "Notifications", [
        q!("notifications.private", "Wiadomości prywatne", "Private messages", Toggle, "chat notification events"),
        q!("notifications.party", "Zdarzenia drużyny", "Party events", Toggle, "party notification events"),
        q!("notifications.vip", "Zdarzenia znajomych", "VIP events", Toggle, "social notification events"),
    ]),
];
#[cfg(test)]
#[rustfmt::skip]
mod tests {
    use super::*;
    use std::collections::HashSet;
    #[test]
    fn catalogue_ids_are_unique_and_all_audited_pages_exist() {
        let mut ids = HashSet::new();
        for section in SETTINGS_SECTIONS {
            assert!(ids.insert(section.id));
            assert!(!section.pl.is_empty() && !section.en.is_empty());
            for option in section.options {
                assert!(ids.insert(option.id), "duplicate option {}", option.id);
                assert!(!option.pl.is_empty() && !option.en.is_empty());
            }
        }
        for page in ["basic", "controls", "general_hotkeys", "action_hotkeys", "custom_hotkeys",
            "interface", "hud", "console", "game_window", "action_bars", "shortcuts", "graphics",
            "effects", "sound", "battle_sounds", "ui_sounds", "miscellaneous", "gameplay", "screenshots", "help"] {
            assert!(ids.contains(page), "missing audited page {page}");
        }
    }
    #[test]
    fn future_preferences_preserve_unknown_defaults_and_reject_invalid_intent() -> std::io::Result<()> {
        use std::collections::BTreeMap;
        assert!(validate_future_preferences(&BTreeMap::new()).is_ok());
        let accepted = BTreeMap::from([("sound.sound.master".to_owned(), FutureValue::Int(50))]);
        validate_future_preferences(&accepted)?;
        let encoded = serde_json::to_vec(&accepted)?;
        assert_eq!(accepted, serde_json::from_slice(&encoded)?);
        for (key, value) in [
            ("sound.sound.master", FutureValue::Int(101)),
            ("sound.sound.master", FutureValue::Bool(false)),
            ("sound.sound.device", FutureValue::Choice("invented-device".to_owned())),
            ("graphics.graphics.vsync", FutureValue::Bool(false)),
            ("account.password", FutureValue::Text("never-store-this".to_owned())),
            ("screenshots.capture.folder", FutureValue::Text("bad\0folder".to_owned())),
            ("general_hotkeys.hotkeys.primary", FutureValue::Binding(ShortcutBinding {
                key: 0, ctrl: false, alt: false, shift: false, meta: false,
            })),
        ] {
            assert!(validate_future_preferences(&BTreeMap::from([(key.to_owned(), value)])).is_err());
        }
        Ok(())
    }
    #[test]
    fn names_only_controls_have_inventory_provenance_and_exclude_credentials() -> Result<(), Box<dyn std::error::Error>> {
        #[derive(serde::Deserialize)]
        struct Inventory { options: Vec<String> }
        let inventory: Inventory = serde_json::from_str(NAMES_ONLY_INVENTORY_JSON)?;
        let keys: HashSet<_> = inventory.options.iter().map(String::as_str).collect();
        let mut references = HashSet::new();
        for option in SETTINGS_SECTIONS.iter().flat_map(|section| section.options) {
            if option.evidence == Evidence::NamesOnly {
                let key = option.reference_key.ok_or("names-only control requires its source key")?;
                assert!(keys.contains(key), "missing source key {key}");
                assert!(references.insert(key), "duplicated source key {key}");
                assert_ne!(key, "loginEmailAddress");
            } else {
                assert!(option.reference_key.is_none());
            }
        }
        assert_eq!(references.len(), 65);
        let choices: HashSet<_> = CONTAINER_SORT_CHOICES.iter().map(|choice| choice.id).collect();
        assert_eq!(choices.len(), 9);
        let value = std::collections::BTreeMap::from([("containers.containers.sort_order".into(),
            FutureValue::Choice("amount_descending".into()))]);
        assert!(validate_future_preferences(&value).is_ok());
        Ok(())
    }
    #[test]
    fn observed_action_row_counts_have_real_local_fields() {
        let counts: Vec<_> = SETTINGS_SECTIONS.iter().flat_map(|section| section.options)
            .filter(|option| matches!(option.id, "bars.bottom_rows" | "bars.left_rows" | "bars.right_rows")).collect();
        assert_eq!(counts.len(), 3);
        for option in counts {
            assert_eq!(option.evidence, Evidence::ObservedReferenceUi);
            assert_eq!(option.kind, Integer { min: 0, max: 3 });
            assert!(matches!(option.implementation, Implementation::Implemented { .. }));
        }
    }
}
