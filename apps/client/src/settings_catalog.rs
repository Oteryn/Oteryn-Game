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
    Implemented {
        field: &'static str,
    },
    /// A validated entry in `future_preferences` with a real runtime consumer.
    /// This keeps unset reference defaults representable without confusing
    /// persistence with implementation.
    Stored {
        consumer: &'static str,
    },
    Pending {
        consumer: &'static str,
    },
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
    // Bound the map by the finite catalogue plus the explicitly retained hidden
    // compatibility values. Unknown keys and the serialized byte limit remain
    // rejected below.
    let catalogue_capacity: usize = SETTINGS_SECTIONS
        .iter()
        .map(|section| section.options.len())
        .sum::<usize>()
        + 3;
    if values.len() > catalogue_capacity {
        return Err(invalid());
    }
    for (key, value) in values {
        let kind = SETTINGS_SECTIONS
            .iter()
            .find_map(|section| {
                section.options.iter().find(|option| {
                    key == &format!("{}.{}", section.id, option.id)
                        && matches!(
                            option.implementation,
                            Implementation::Stored { .. } | Implementation::Pending { .. }
                        )
                })
            })
            .map(|option| option.kind)
            .or(match key.as_str() {
                // Older Oteryn drafts exposed indoor and cloud attenuation separately.
                // The reference has one combined row, so keep the old indoor value loadable
                // without advertising a second control in navigation or search.
                "effects.effects.indoor_light" => Some(PERCENT),
                // Food and item movement were previously stored under UI Sounds. They belong
                // to the observed Sound / Item group; retain the old keys without exposing
                // duplicate controls or search results.
                "ui_sounds.sound.eating" | "ui_sounds.sound.item_movement" => Some(Toggle),
                _ => None,
            })
            .ok_or_else(invalid)?;
        let valid = match (kind, value) {
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

/// Returns the runtime consumer declared for a value stored in
/// [`crate::settings::ClientSettings::future_preferences`]. Pending catalogue
/// rows deliberately return `None`: persistence by itself is not a feature.
#[must_use]
pub fn runtime_consumer(key: &str) -> Option<&'static str> {
    SETTINGS_SECTIONS
        .iter()
        .find_map(|section| {
            section
                .options
                .iter()
                .find(|option| key == format!("{}.{}", section.id, option.id))
        })
        .and_then(|option| match option.implementation {
            Implementation::Stored { consumer } => Some(consumer),
            Implementation::Implemented { .. } | Implementation::Pending { .. } => None,
        })
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
macro_rules! s {
    ($id:literal, $pl:literal, $en:literal, $kind:expr, $consumer:literal) => {
        SettingOption {
            implementation: Implementation::Stored {
                consumer: $consumer,
            },
            ..p!($id, $pl, $en, $kind, $consumer)
        }
    };
}
macro_rules! sn {
    ($id:literal, $pl:literal, $en:literal, $kind:expr, $consumer:literal, $key:literal) => {
        SettingOption {
            implementation: Implementation::Stored {
                consumer: $consumer,
            },
            ..n!($id, $pl, $en, $kind, $consumer, $key)
        }
    };
}
macro_rules! qs {
    ($id:literal, $pl:literal, $en:literal, $kind:expr, $consumer:literal) => {
        SettingOption {
            implementation: Implementation::Stored {
                consumer: $consumer,
            },
            ..q!($id, $pl, $en, $kind, $consumer)
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
// Visible only while Classic Controls is selected. Membership was inspected in
// the live reference draft and cancelled without applying the account change.
const CLASSIC_LOOT_BUTTONS: &[LocalizedChoice] = &[
    LocalizedChoice {
        id: "right",
        pl: "Prawy",
        en: "Right",
    },
    LocalizedChoice {
        id: "shift_right",
        pl: "SHIFT+Prawy",
        en: "SHIFT+Right",
    },
    LocalizedChoice {
        id: "left",
        pl: "Lewy",
        en: "Left",
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
const TARGET_MARKING: &[LocalizedChoice] = &[
    LocalizedChoice {
        id: "frame_and_highlight",
        pl: "Ramka i wyróżnienie",
        en: "Frame & Highlight",
    },
    LocalizedChoice {
        id: "frame_only",
        pl: "Tylko ramka",
        en: "Frame Only",
    },
    LocalizedChoice {
        id: "highlight_only",
        pl: "Tylko wyróżnienie",
        en: "Highlight Only",
    },
    LocalizedChoice {
        id: "none",
        pl: "Brak",
        en: "None",
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

/// The complete Tibia Global settings navigation established by the private
/// reference audit. Additional catalogue sections are compatibility/storage
/// scope and must not leak into this 20-page surface.
pub const REFERENCE_SETTINGS_SECTION_IDS: [&str; 20] = [
    "basic",
    "controls",
    "general_hotkeys",
    "action_hotkeys",
    "custom_hotkeys",
    "interface",
    "hud",
    "console",
    "game_window",
    "action_bars",
    "shortcuts",
    "graphics",
    "effects",
    "sound",
    "battle_sounds",
    "ui_sounds",
    "miscellaneous",
    "gameplay",
    "screenshots",
    "help",
];

#[rustfmt::skip]
pub const SETTINGS_SECTIONS: &[SettingsSection] = &[
    section!("basic", "Podstawowe", "Basic Options", [
        s!("basic.advanced", "Pokaż opcje zaawansowane", "Show Advanced Options", Toggle, "settings discovery"),
        r!("basic.click_walk", "Chodzenie kliknięciem", "Click to walk", Toggle, "click_to_walk"),
        r!("basic.ui_scale", "Skala interfejsu", "Interface scale", Decimal { min: 0.8, max: 1.8 }, "ui_scale"),
    ]),
    section!("controls", "Sterowanie", "Controls", [
        p!("controls.turn_ctrl", "Obracanie: Ctrl", "Turn: Ctrl", Toggle, "movement actions"),
        p!("controls.turn_shift", "Obracanie: Shift", "Turn: Shift", Toggle, "movement actions"),
        p!("controls.turn_alt", "Obracanie: Alt", "Turn: Alt", Toggle, "movement actions"),
        p!("controls.complete_stack_ctrl", "Ctrl do przenoszenia całych stosów", "Press CTRL to Drag Complete Stacks", Toggle, "item-move amount intent"),
        p!("controls.mouse_preset", "Schemat myszy", "Mouse Preset", Choice(MOUSE_PRESETS), "input router"),
        p!("controls.loot_button", "Łup", "Loot", Choice(CLASSIC_LOOT_BUTTONS), "input router"),
        r!("controls.movement", "Klawisze kierunków", "Movement keys", Binding, "movement_keys"),
        q!("controls.rotation", "Modyfikator obracania postaci", "Turn modifier", Binding, "movement actions"),
        q!("controls.held_keys", "Działanie przytrzymanych klawiszy", "Held-key behavior", UNKNOWN_CHOICES, "input router"),
        q!("controls.drag_modifier", "Modyfikator przeciągania", "Drag modifier", Binding, "inventory commands"),
        n!("controls.face_movement", "Zawsze obracaj w kierunku ruchu", "Always turn towards the direction of movement", Toggle, "movement presentation", "alwaysTurnTowardsMoveDirection"),
        sn!("controls.default_key_delay", "Używaj domyślnego opóźnienia klawiatury", "Use Default Keyboard Delay", Toggle, "movement repeat scheduler", "keyboardDelayUseDefault"),
        // Preserve legacy nonnegative values. The reference slider range is
        // 0–1000 ms; displaying a page must not rewrite a retained wider value.
        sn!("controls.key_delay_ms", "Opóźnienie klawiatury", "Keyboard Delay", Integer { min: 0, max: i32::MAX }, "movement repeat scheduler", "keyboardDelayMs"),
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
        s!("interface.highlight_mouse", "Wyróżnij cel myszy", "Highlight Mouse Target", Toggle, "scene hover projection"),
        s!("interface.big_cursor", "Duży kursor myszy", "Show Big Mouse Cursor", Toggle, "native/custom pointer renderer"),
        p!("interface.link_copy_warning", "Ostrzeżenie przy kopiowaniu linku", "Show Link Copy Warning", Toggle, "safe clipboard UI"),
        p!("interface.expiry_inventory", "Wygaśnięcie w ekwipunku", "Show Expiry in Inventory", Toggle, "item expiry projection"),
        p!("interface.expiry_containers", "Wygaśnięcie w pojemnikach", "Show Expiry in Containers", Toggle, "item expiry projection"),
        p!("interface.expiry_unused", "Wygaśnięcie nieużywanych przedmiotów", "Show Expiry On Unused Items", Toggle, "item expiry projection"),
        p!("interface.colourise_loot_value", "Koloruj wartość łupu", "Colourise Loot Value", Choice(LOOT_COLOUR), "item value presentation"),
        r!("interface.english", "Interfejs po angielsku", "English interface", Toggle, "english"),
        r!("interface.contrast", "Wyższy kontrast paneli", "Higher panel contrast", Toggle, "high_contrast"),
        r!("interface.reduced_motion", "Ograniczone animacje", "Reduced animation", Toggle, "reduced_motion"),
        q!("interface.cursor_size", "Rozmiar kursora", "Cursor size", UNKNOWN_CHOICES, "cursor integration"),
        p!("interface.cursor_animation", "Animowany kursor", "Animated cursor", Toggle, "contextual interaction cursor"),
        q!("interface.link_confirmation", "Potwierdzaj otwieranie linków", "Confirm external links", Toggle, "safe link routing"),
        sn!("interface.system_cursor", "Używaj kursora systemowego", "Use system cursor", Toggle, "native/custom pointer renderer", "mouseSystemCursor"),
    ]),
    section!("hud", "HUD i wskaźniki", "HUD", [
        s!("arc_size_preset", "Rozmiar łuków", "Arc Size", Choice(&[LocalizedChoice {id:"default",pl:"Rozmiar domyślny",en:"Default Size"}, LocalizedChoice {id:"small",pl:"Mały rozmiar",en:"Small Size"}, LocalizedChoice {id:"large",pl:"Duży rozmiar",en:"Large Size"}]), "own-resource HUD"),
        s!("harmony_position", "Położenie harmonii", "Harmony Position", Choice(&[LocalizedChoice {id:"health",pl:"Obok łuku zdrowia",en:"next to Health Arc"}, LocalizedChoice {id:"mana",pl:"Obok łuku many",en:"next to Mana Arc"}]), "own-resource HUD"),
        s!("show_harmony", "Pokaż harmonię", "Show Harmony", Toggle, "own-resource HUD"),
        p!("other_marks", "Znaczniki innych postaci", "Other Creature Marks", Toggle, "actor/condition projection"),
        p!("status_bars", "Pokaż paski stanu", "Show Status Bars", Toggle, "actor/condition projection"),
        p!("custom_status_bars", "Konfigurowalne paski stanu", "Show Customisable Status Bars", Toggle, "actor/condition projection"),
        p!("condition_poison_hud", "Zatrucie: hud", "Poisoned: hud", Toggle, "actor/condition projection"),
        p!("condition_poison_bar", "Zatrucie: bar", "Poisoned: bar", Toggle, "actor/condition projection"),
        p!("condition_burning_hud", "Płonięcie: hud", "Burning: hud", Toggle, "actor/condition projection"),
        p!("condition_burning_bar", "Płonięcie: bar", "Burning: bar", Toggle, "actor/condition projection"),
        p!("condition_electric_hud", "Porażenie: hud", "Electrified: hud", Toggle, "actor/condition projection"),
        p!("condition_electric_bar", "Porażenie: bar", "Electrified: bar", Toggle, "actor/condition projection"),
        p!("condition_bleeding_hud", "Krwawienie: hud", "Bleeding: hud", Toggle, "actor/condition projection"),
        p!("condition_bleeding_bar", "Krwawienie: bar", "Bleeding: bar", Toggle, "actor/condition projection"),
        p!("condition_agony_hud", "Agonia: hud", "Agony: hud", Toggle, "actor/condition projection"),
        p!("condition_agony_bar", "Agonia: bar", "Agony: bar", Toggle, "actor/condition projection"),
        p!("condition_powerless_hud", "Bezsilność: hud", "Powerless: hud", Toggle, "actor/condition projection"),
        p!("condition_powerless_bar", "Bezsilność: bar", "Powerless: bar", Toggle, "actor/condition projection"),
        p!("condition_rooted_hud", "Unieruchomienie: hud", "Rooted: hud", Toggle, "actor/condition projection"),
        p!("condition_rooted_bar", "Unieruchomienie: bar", "Rooted: bar", Toggle, "actor/condition projection"),
        p!("condition_feared_hud", "Strach: hud", "Feared: hud", Toggle, "actor/condition projection"),
        p!("condition_feared_bar", "Strach: bar", "Feared: bar", Toggle, "actor/condition projection"),
        p!("condition_drunk_hud", "Upicie: hud", "Drunk: hud", Toggle, "actor/condition projection"),
        p!("condition_drunk_bar", "Upicie: bar", "Drunk: bar", Toggle, "actor/condition projection"),
        p!("condition_magic_shield_hud", "Tarcza magiczna: hud", "Magic Shield: hud", Toggle, "actor/condition projection"),
        p!("condition_magic_shield_bar", "Tarcza magiczna: bar", "Magic Shield: bar", Toggle, "actor/condition projection"),
        p!("condition_monk_virtue_hud", "Bonus cnót mnicha: hud", "Monk’s Virtue bonus: hud", Toggle, "actor/condition projection"),
        p!("condition_monk_virtue_bar", "Bonus cnót mnicha: bar", "Monk’s Virtue bonus: bar", Toggle, "actor/condition projection"),
        p!("condition_slowed_hud", "Spowolnienie: hud", "Slowed: hud", Toggle, "actor/condition projection"),
        p!("condition_slowed_bar", "Spowolnienie: bar", "Slowed: bar", Toggle, "actor/condition projection"),
        p!("condition_haste_hud", "Przyspieszenie: hud", "Haste: hud", Toggle, "actor/condition projection"),
        p!("condition_haste_bar", "Przyspieszenie: bar", "Haste: bar", Toggle, "actor/condition projection"),
        p!("condition_logout_block_hud", "Blokada wylogowania: hud", "Logout Block: hud", Toggle, "actor/condition projection"),
        p!("condition_logout_block_bar", "Blokada wylogowania: bar", "Logout Block: bar", Toggle, "actor/condition projection"),
        p!("condition_drowning_hud", "Tonięcie: hud", "Drowning: hud", Toggle, "actor/condition projection"),
        p!("condition_drowning_bar", "Tonięcie: bar", "Drowning: bar", Toggle, "actor/condition projection"),
        p!("condition_freezing_hud", "Zamarzanie: hud", "Freezing: hud", Toggle, "actor/condition projection"),
        p!("condition_freezing_bar", "Zamarzanie: bar", "Freezing: bar", Toggle, "actor/condition projection"),
        p!("condition_dazzled_hud", "Oślepienie: hud", "Dazzled: hud", Toggle, "actor/condition projection"),
        p!("condition_dazzled_bar", "Oślepienie: bar", "Dazzled: bar", Toggle, "actor/condition projection"),
        p!("condition_cursed_hud", "Klątwa: hud", "Cursed: hud", Toggle, "actor/condition projection"),
        p!("condition_cursed_bar", "Klątwa: bar", "Cursed: bar", Toggle, "actor/condition projection"),
        p!("condition_strengthened_hud", "Wzmocnienie: hud", "Strengthened: hud", Toggle, "actor/condition projection"),
        p!("condition_strengthened_bar", "Wzmocnienie: bar", "Strengthened: bar", Toggle, "actor/condition projection"),
        p!("condition_pz_block_hud", "Blokada strefy ochronnej: hud", "Protection Zone Block: hud", Toggle, "actor/condition projection"),
        p!("condition_pz_block_bar", "Blokada strefy ochronnej: bar", "Protection Zone Block: bar", Toggle, "actor/condition projection"),
        p!("condition_in_pz_hud", "W strefie ochronnej: hud", "In Protection Zone: hud", Toggle, "actor/condition projection"),
        p!("condition_in_pz_bar", "W strefie ochronnej: bar", "In Protection Zone: bar", Toggle, "actor/condition projection"),
        p!("condition_resting_hud", "Strefa odpoczynku: hud", "Resting Area: hud", Toggle, "actor/condition projection"),
        p!("condition_resting_bar", "Strefa odpoczynku: bar", "Resting Area: bar", Toggle, "actor/condition projection"),
        p!("condition_lesser_hex_hud", "Słabszy urok: hud", "Lesser Hex: hud", Toggle, "actor/condition projection"),
        p!("condition_lesser_hex_bar", "Słabszy urok: bar", "Lesser Hex: bar", Toggle, "actor/condition projection"),
        p!("condition_intense_hex_hud", "Silny urok: hud", "Intense Hex: hud", Toggle, "actor/condition projection"),
        p!("condition_intense_hex_bar", "Silny urok: bar", "Intense Hex: bar", Toggle, "actor/condition projection"),
        p!("condition_greater_hex_hud", "Potężny urok: hud", "Greater Hex: hud", Toggle, "actor/condition projection"),
        p!("condition_greater_hex_bar", "Potężny urok: bar", "Greater Hex: bar", Toggle, "actor/condition projection"),
        p!("condition_goshnar_taint_hud", "Skaza Goshnara: hud", "Goshnar’s Taint: hud", Toggle, "actor/condition projection"),
        p!("condition_goshnar_taint_bar", "Skaza Goshnara: bar", "Goshnar’s Taint: bar", Toggle, "actor/condition projection"),
        p!("condition_bakragore_taint_hud", "Skaza Bakragore: hud", "Bakragore’s Taint: hud", Toggle, "actor/condition projection"),
        p!("condition_bakragore_taint_bar", "Skaza Bakragore: bar", "Bakragore’s Taint: bar", Toggle, "actor/condition projection"),
        p!("condition_yellow_skull_hud", "Żółta czaszka: hud", "Yellow Skull: hud", Toggle, "actor/condition projection"),
        p!("condition_yellow_skull_bar", "Żółta czaszka: bar", "Yellow Skull: bar", Toggle, "actor/condition projection"),
        p!("condition_party_mode_hud", "Tryb drużyny: hud", "Party Mode: hud", Toggle, "actor/condition projection"),
        p!("condition_party_mode_bar", "Tryb drużyny: bar", "Party Mode: bar", Toggle, "actor/condition projection"),
        p!("condition_white_skull_hud", "Biała czaszka: hud", "White Skull: hud", Toggle, "actor/condition projection"),
        p!("condition_white_skull_bar", "Biała czaszka: bar", "White Skull: bar", Toggle, "actor/condition projection"),
        p!("condition_red_skull_hud", "Czerwona czaszka: hud", "Red Skull: hud", Toggle, "actor/condition projection"),
        p!("condition_red_skull_bar", "Czerwona czaszka: bar", "Red Skull: bar", Toggle, "actor/condition projection"),
        p!("condition_black_skull_hud", "Czarna czaszka: hud", "Black Skull: hud", Toggle, "actor/condition projection"),
        p!("condition_black_skull_bar", "Czarna czaszka: bar", "Black Skull: bar", Toggle, "actor/condition projection"),
        p!("condition_orange_skull_hud", "Pomarańczowa czaszka: hud", "Orange Skull: hud", Toggle, "actor/condition projection"),
        p!("condition_orange_skull_bar", "Pomarańczowa czaszka: bar", "Orange Skull: bar", Toggle, "actor/condition projection"),
        p!("condition_guild_war_hud", "Wojna gildii: hud", "In Guild War: hud", Toggle, "actor/condition projection"),
        p!("condition_guild_war_bar", "Wojna gildii: bar", "In Guild War: bar", Toggle, "actor/condition projection"),
        p!("condition_hungry_hud", "Głód: hud", "Hungry: hud", Toggle, "actor/condition projection"),
        p!("condition_hungry_bar", "Głód: bar", "Hungry: bar", Toggle, "actor/condition projection"),
        p!("conditions_hud_enabled", "Stany na HUD", "Conditions in HUD", Toggle, "actor/condition projection"),
        p!("conditions_bar_enabled", "Stany na pasku", "Conditions in bar", Toggle, "actor/condition projection"),
        p!("condition_order", "Kolejność stanów", "Condition order", Text { max_bytes: 1024 }, "actor/condition projection"),
        s!("hud.owner_name", "Nazwa własnej postaci", "Own character name", Toggle, "own-character HUD"),
        p!("hud.other_names", "Nazwy innych postaci i stworzeń", "Other actor names", Toggle, "actor HUD"),
        s!("hud.owner_health", "Własny pasek zdrowia", "Own health bar", Toggle, "own-resource HUD"),
        s!("hud.other_health", "Paski zdrowia innych", "Other health bars", Toggle, "visible-actor HUD"),
        s!("hud.owner_mana", "Własny pasek many", "Own mana bar", Toggle, "own-resource HUD"),
        p!("hud.other_mana", "Paski many innych", "Other mana bars", Toggle, "supported resource projection"),
        p!("hud.marks", "Znaczniki postaci", "Actor marks", Toggle, "actor status projection"),
        p!("hud.npc_icons", "Ikony NPC", "NPC icons", Toggle, "NPC presentation"),
        s!("hud.arcs", "Łuki zdrowia i many", "Health and mana arcs", Toggle, "own-resource HUD"),
        p!("hud.arc_size", "Rozmiar łuków", "Arc size", PERCENT, "actor HUD"),
        s!("hud.arc_distance", "Odległość łuków od postaci", "Arc distance", PERCENT, "own-resource HUD"),
        s!("hud.arc_opacity", "Przezroczystość łuków", "Arc opacity", PERCENT, "own-resource HUD"),
        p!("hud.conditions_bar", "Stany postaci na pasku", "Conditions in status bar", Toggle, "condition projection"),
        p!("hud.conditions_world", "Stany postaci przy postaci", "Conditions beside character", Toggle, "condition projection"),
        p!("hud.conditions_order", "Kolejność stanów postaci", "Condition order", Action, "condition layout"),
        p!("hud.status_placement", "Położenie pasków stanu", "Status-bar placement", UNKNOWN_CHOICES, "HUD layout"),
        p!("hud.status_customization", "Zawartość pasków stanu", "Customize status bars", Action, "HUD layout"),
        sn!("hud.player_enabled", "Pokaż HUD własnej postaci", "Show HUD for Own Character", Toggle, "own-resource HUD", "playerHudEnabled"),
        sn!("hud.creatures_enabled", "Pokaż HUD innych stworzeń", "Show HUD for Other Creatures", Toggle, "visible-actor HUD", "creatureHudEnabled"),
        sn!("hud.resource_bars", "Paski zasobów", "Resource bars", Toggle, "own-resource HUD", "playerHudShowBars"),
        n!("hud.cooldown_bar", "Osobny pasek czasów odnowienia", "Standalone cooldown bar", Toggle, "cooldown projection", "cooldownBarEnabled"),
        n!("hud.harmony_left", "Harmonia po lewej stronie HUD", "Harmony on the left of HUD", Toggle, "vocation HUD", "playerHudShowHarmonyLeft"),
        n!("hud.serene_harmony", "Pokazuj spokojną harmonię", "Show serene Harmony", Toggle, "vocation condition projection", "playerShowHarmonySerene"),
    ]),
    section!("console", "Czat i konsola", "Console", [
        r!("console.visible", "Pokaż panel czatu", "Show chat panel", Toggle, "show_chat"),
        s!("console.font_size", "Rozmiar tekstu", "Text size", Integer { min: 10, max: 32 }, "chat text renderer"),
        s!("console.timestamps", "Znaczniki czasu", "Timestamps", Toggle, "received chat clock"),
        s!("console.seconds", "Sekundy w znacznikach czasu", "Timestamp seconds", Toggle, "received chat clock"),
        p!("console.levels", "Poziomy przy nazwach", "Levels beside names", Toggle, "chat projection"),
        p!("console.own_status", "Własne komunikaty stanu", "Own status messages", Toggle, "session events"),
        p!("console.other_status", "Komunikaty stanu innych", "Other status messages", Toggle, "session events"),
        p!("console.events", "Komunikaty o zdarzeniach", "Event messages", Toggle, "session events"),
        p!("console.information", "Komunikaty informacyjne", "Information messages", Toggle, "session events"),
        p!("console.private_tab", "Otwieraj nowe karty po otrzymaniu prywatnych wiadomości", "Open New Tabs When Receiving Private Messages", Toggle, "chat presentation"),
        n!("console.join_leave", "Komunikaty wejścia i wyjścia z kanału", "Channel join/leave messages", Toggle, "chat membership projection", "consoleShowJoinLeaveMessages"),
    ]),
    section!("game_window", "Okno gry", "Game Window", [
        s!("window.target_marking", "Oznaczaj cel wizualnie", "Mark Target Visually", Choice(TARGET_MARKING), "scene target projection"),
        p!("window.textual_effects", "Efekty tekstowe", "Show Textual Effects", Toggle, "effect projection"),
        p!("window.potion_sounds", "Efekty dźwiękowe mikstur", "Show Potion Sound Effects", Toggle, "effect projection"),
        q!("window.fit", "Dopasowanie obszaru gry", "Game viewport fit", UNKNOWN_CHOICES, "scene layout"),
        q!("window.top_pane", "Panel nad obszarem gry", "Top game pane", Toggle, "scene layout"),
        q!("window.bottom_pane", "Panel pod obszarem gry", "Bottom game pane", Toggle, "scene layout"),
        p!("window.private_messages", "Wiadomości prywatne", "Show Private Messages", Toggle, "chat overlays"),
        p!("window.general_messages", "Wiadomości", "Show Messages", Toggle, "session overlays"),
        p!("window.own_spells", "Zaklęcia", "Show Spells", Toggle, "effect projection"),
        p!("window.other_spells", "Zaklęcia innych", "Show Spells of Others", Toggle, "effect projection"),
        p!("window.hotkey_notices", "Komunikaty użycia skrótów", "Show Hotkey Usage Notifications", Toggle, "action feedback"),
        p!("window.loot_notices", "Komunikaty łupów", "Show Loot Messages", Toggle, "loot events"),
        p!("window.loot_highlight", "Wyróżnianie łupów", "Show Loot Highlighting", Toggle, "loot projection"),
        p!("window.training", "Postęp treningu offline", "Show Offline Training Progress", Toggle, "training events"),
        p!("window.combat_frame", "Ramki walki", "Show Combat Frames", Toggle, "combat projection"),
        p!("window.pvp_frame", "Ramki PvP", "Show PvP Frames", Toggle, "PvP projection"),
        p!("window.attack_animation", "Animacja ataku wręcz", "Show Melee Attack Animation", Toggle, "combat presentation"),
        p!("window.banner", "Baner informacyjny", "Show Info Banner", Toggle, "scene presentation"),
        p!("window.target_frame", "Ramka celu", "Target frame", Toggle, "target projection"),
        p!("window.target_highlight", "Wyróżnianie celu", "Target highlight", Toggle, "target projection"),
        n!("window.potion_notices", "Komunikaty mikstur", "Potion messages", Toggle, "item-use notices", "gameWindowShowPotionMessages"),
        n!("window.store_notices", "Powiadomienia sklepu w walce", "Show Store Notifications in Combat", Toggle, "commercial notices", "gameWindowShowStoreMessages"),
        n!("window.boosted_creatures", "Wzmocnione stworzenie", "Show Boosted Creature", Toggle, "world notices", "gameWindowShowBoostedCreatureMessages"),
    ]),
    section!("action_bars", "Paski akcji", "Action Bars", [
        i!("bars.bottom_rows", "Dolne paski akcji", "Bottom action rows", Integer { min: 0, max: 3 }, "action_bottom_rows"),
        i!("bars.left_rows", "Lewe paski akcji", "Left action rows", Integer { min: 0, max: 3 }, "action_left_rows"),
        i!("bars.right_rows", "Prawe paski akcji", "Right action rows", Integer { min: 0, max: 3 }, "action_right_rows"),
        q!("bars.locked", "Zablokuj układ pasków", "Lock action bars", Toggle, "action bar layout"),
        s!("bars.labels", "Etykiety skrótów", "Hotkey labels", Toggle, "action-bar renderer"),
        p!("bars.item_amounts", "Liczba przedmiotów", "Item amounts", Toggle, "action-bar assignment source"),
        p!("bars.spell_parameters", "Parametry zaklęć", "Spell parameters", Toggle, "spell actions"),
        p!("bars.graphic_cooldown", "Graficzny czas odnowienia", "Graphical cooldown", Toggle, "cooldown projection"),
        p!("bars.numeric_cooldown", "Liczbowy czas odnowienia", "Numeric cooldown", Toggle, "cooldown projection"),
        p!("bars.tooltips", "Podpowiedzi", "Tooltips", Toggle, "action-bar assignment source"),
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
        r!("graphics.fps", "Limit FPS (0: bez limitu)", "FPS limit (0: unlimited)", Integer { min: 0, max: 200 }, "fps"),
        r!("graphics.background_fps", "Limit FPS w tle", "Background FPS limit", Integer { min: 5, max: 60 }, "background_fps"),
        p!("graphics.engine", "Silnik graficzny", "Graphics Engine", UNKNOWN_CHOICES, "renderer capabilities"),
        p!("graphics.antialiasing", "Wygładzanie obrazu", "Antialiasing Mode", Choice(ANTIALIASING), "renderer capabilities"),
        s!("graphics.integer_scale", "Skalowanie całkowite", "Scale Using Only Integral Multiples", Toggle, "scene viewport"),
        q!("graphics.monitor", "Monitor", "Monitor", UNKNOWN_CHOICES, "window integration"),
        qs!("graphics.fps_indicator", "Wskaźnik FPS", "FPS indicator", Toggle, "frame diagnostics overlay"),
        q!("graphics.latency_indicator", "Wskaźnik opóźnienia", "Latency indicator", Toggle, "session diagnostics"),
    ]),
    section!("effects", "Efekty graficzne", "Effects", [
        s!("effects.ambient_light", "Światło otoczenia", "Ambient light", PERCENT, "scene lighting overlay"),
        p!("effects.level_separator", "Oddzielenie pięter", "Level separator", PERCENT, "floor projection"),
        p!("effects.cloud_light", "Efekt chmur i wnętrz", "Clouds & Indoor Effect", PERCENT, "lighting renderer"),
        p!("effects.own_opacity", "Efekty własnych zaklęć", "Own Spell Effects", PERCENT, "effect renderer"),
        p!("effects.other_opacity", "Efekty innych graczy", "Other Players' Effects", PERCENT, "effect renderer"),
        p!("effects.creature_opacity", "Efekty zaklęć stworzeń", "Creature Spell Effects", PERCENT, "effect renderer"),
        p!("effects.boss_opacity", "Efekty zaklęć stworzeń w obszarach bossów", "Boss Area Creature Spell Effects", PERCENT, "effect renderer"),
        sn!("effects.lighting", "Efekty oświetlenia", "Lighting effects", Toggle, "scene lighting overlay", "lightEffectsEnabled"),
    ]),
    section!("sound", "Dźwięk", "Sound", [
        p!("sound.device", "Urządzenie wyjściowe", "Output device", UNKNOWN_CHOICES, "audio backend"),
        p!("sound.master", "Głośność główna", "Master volume", PERCENT, "audio mixer"),
        p!("sound.music", "Muzyka", "Music volume", PERCENT, "audio mixer"),
        p!("sound.ambience", "Dźwięki otoczenia", "Ambience volume", PERCENT, "audio mixer"),
        p!("sound.items", "Dźwięki przedmiotów", "Item volume", PERCENT, "audio mixer"),
        p!("sound.eating", "Jedzenie i napoje", "Food and Beverages", Toggle, "item-use audio"),
        p!("sound.item_movement", "Przenoszenie przedmiotów", "Move Item", Toggle, "inventory audio"),
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
        p!("buy_confirmation", "Pytaj przed zakupem produktów", "Ask Before Buying Products", Toggle, "accepted command/session policy"),
        p!("stow_confirmation", "Pytaj przed schowaniem zawartości", "Ask Before Stowing Container Content", Toggle, "accepted command/session policy"),
        p!("sort_confirmation", "Pytaj przed sortowaniem zagnieżdżonych pojemników", "Ask Before Sorting Nested Containers", Toggle, "accepted command/session policy"),
        p!("move_confirmation", "Pytaj przed przenoszeniem zawartości pojemników", "Ask Before Moving Contents of Nested Containers", Toggle, "accepted command/session policy"),
        p!("stay_logged_in", "Pozostań zalogowany w tej sesji", "Stay Logged In for Session", Toggle, "accepted command/session policy"),
        p!("optimise_connection", "Optymalizuj stabilność połączenia", "Optimise Connection Stability", Toggle, "accepted command/session policy"),
        p!("misc.confirmations", "Potwierdzenia czynności", "Action confirmations", Action, "safe command UI"),
        p!("misc.session_retention", "Zachowanie sesji", "Session retention", UNKNOWN_CHOICES, "accepted session policy"),
        p!("misc.connection_stability", "Stabilność połączenia", "Connection stability", UNKNOWN_CHOICES, "accepted transport policy"),
        p!("misc.quick_login", "Szybkie logowanie", "Quick login", Toggle, "approved device-session contract"),
    ]),
    section!("gameplay", "Rozgrywka", "Gameplay", [
        p!("gameplay.inspect_permission", "Pozwól wszystkim oglądać postać", "Allow All to Inspect Me", Toggle, "inspection authority"),
        p!("gameplay.cancel_chase", "Automatycznie przerywaj pościg", "Auto Chase Off", Toggle, "combat commands"),
        p!("gameplay.nearby_corpses", "Łupienie pobliskich ciał", "Quick Loot Nearby Corpses", Toggle, "accepted loot commands"),
    ]),
    section!("screenshots", "Zrzuty ekranu", "Screenshots", [
        p!("auto", "Automatyczne zrzuty", "Enable Auto Screenshots", Toggle, "capture event pipeline"),
        p!("level", "Awans poziomu", "Level Up", Toggle, "capture event pipeline"),
        p!("skill", "Awans umiejętności", "Skill Up", Toggle, "capture event pipeline"),
        p!("bestiary_unlocked", "Odblokowany wpis bestiariusza", "Bestiary Entry Unlocked", Toggle, "capture event pipeline"),
        p!("bestiary_completed", "Ukończony wpis bestiariusza", "Bestiary Entry Completed", Toggle, "capture event pipeline"),
        p!("death_pve", "Śmierć PvE", "Death PvE", Toggle, "capture event pipeline"),
        p!("death_pvp", "Śmierć PvP", "Death PvP", Toggle, "capture event pipeline"),
        p!("player_kill", "Zabójstwo gracza", "Player Kill", Toggle, "capture event pipeline"),
        p!("player_assist", "Asysta przy zabójstwie", "Player Kill Assist", Toggle, "capture event pipeline"),
        p!("player_attack", "Atak gracza", "Player Attacking", Toggle, "capture event pipeline"),
        p!("gift_of_life", "Aktywacja daru życia", "Gift of Life Triggered", Toggle, "capture event pipeline"),
        q!("capture.manual", "Zrób zrzut ekranu", "Take screenshot", Action, "renderer capture"),
        q!("capture.folder", "Folder zrzutów", "Screenshot folder", Text { max_bytes: 1024 }, "capture storage"),
        s!("capture.game_only", "Tylko obszar gry", "Game-only capture", Toggle, "native renderer viewport crop"),
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
        p!("help.client_help", "Pomoc klienta", "Client Help", Action, "safe link routing"),
        p!("help.compendium", "Kompendium", "Compendium", Action, "safe link routing"),
        p!("help.rule_violations", "Naruszenia zasad", "Rule Violations", Action, "safe link routing"),
        p!("help.manual", "Instrukcja", "Manual", Action, "safe link routing"),
        p!("help.faq", "Najczęstsze pytania", "FAQ", Action, "safe link routing"),
        p!("help.info", "Informacje o kliencie", "Info", Action, "local client information"),
        p!("help.export_all_options", "Eksportuj wszystkie ustawienia", "Export All Options", Action, "credential-free preference export"),
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

/// Ordered settings sections rendered by the reference-compatible Options UI.
#[must_use]
pub fn reference_settings_sections() -> &'static [SettingsSection] {
    &SETTINGS_SECTIONS[..REFERENCE_SETTINGS_SECTION_IDS.len()]
}

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
        assert_eq!(
            reference_settings_sections()
                .iter()
                .map(|section| section.id)
                .collect::<Vec<_>>(),
            REFERENCE_SETTINGS_SECTION_IDS
        );
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
        let retained_legacy = BTreeMap::from([
            (
                "effects.effects.indoor_light".to_owned(),
                FutureValue::Int(65),
            ),
            (
                "ui_sounds.sound.eating".to_owned(),
                FutureValue::Bool(true),
            ),
            (
                "ui_sounds.sound.item_movement".to_owned(),
                FutureValue::Bool(false),
            ),
        ]);
        validate_future_preferences(&retained_legacy)?;
        assert!(SETTINGS_SECTIONS.iter().all(|section| section
            .options
            .iter()
            .all(|option| option.id != "effects.indoor_light")));
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
    fn observed_item_sound_children_belong_only_to_the_sound_page() -> Result<(), &'static str> {
        let sound = SETTINGS_SECTIONS.iter().find(|section| section.id == "sound").ok_or("sound")?;
        let ui = SETTINGS_SECTIONS.iter().find(|section| section.id == "ui_sounds").ok_or("ui sounds")?;
        for (id, label) in [
            ("sound.eating", "Food and Beverages"),
            ("sound.item_movement", "Move Item"),
        ] {
            assert_eq!(sound.options.iter().find(|option| option.id == id).map(|option| option.en), Some(label));
            assert!(ui.options.iter().all(|option| option.id != id));
        }
        let gameplay = SETTINGS_SECTIONS.iter().find(|section| section.id == "gameplay").ok_or("gameplay")?;
        assert_eq!(
            gameplay.options.iter().find(|option| option.id == "gameplay.inspect_permission").map(|option| option.en),
            Some("Allow All to Inspect Me")
        );
        Ok(())
    }
    #[test]
    fn reference_display_catalog_copy_matches_the_observed_labels() -> Result<(), &'static str> {
        for (section_id, option_id, exact_label) in [
            ("controls", "controls.mouse_preset", "Mouse Preset"),
            ("controls", "controls.loot_button", "Loot"),
            ("controls", "controls.face_movement", "Always turn towards the direction of movement"),
            ("controls", "controls.default_key_delay", "Use Default Keyboard Delay"),
            ("controls", "controls.key_delay_ms", "Keyboard Delay"),
            ("interface", "interface.expiry_unused", "Show Expiry On Unused Items"),
            ("game_window", "window.private_messages", "Show Private Messages"),
            ("game_window", "window.general_messages", "Show Messages"),
            ("game_window", "window.own_spells", "Show Spells"),
            ("game_window", "window.other_spells", "Show Spells of Others"),
            ("game_window", "window.hotkey_notices", "Show Hotkey Usage Notifications"),
            ("game_window", "window.loot_notices", "Show Loot Messages"),
            ("game_window", "window.loot_highlight", "Show Loot Highlighting"),
            ("game_window", "window.boosted_creatures", "Show Boosted Creature"),
            ("game_window", "window.training", "Show Offline Training Progress"),
            ("game_window", "window.store_notices", "Show Store Notifications in Combat"),
            ("game_window", "window.combat_frame", "Show Combat Frames"),
            ("game_window", "window.pvp_frame", "Show PvP Frames"),
            ("game_window", "window.attack_animation", "Show Melee Attack Animation"),
            ("game_window", "window.banner", "Show Info Banner"),
            ("graphics", "graphics.engine", "Graphics Engine"),
            ("graphics", "graphics.antialiasing", "Antialiasing Mode"),
            ("graphics", "graphics.integer_scale", "Scale Using Only Integral Multiples"),
            ("gameplay", "gameplay.cancel_chase", "Auto Chase Off"),
            ("gameplay", "gameplay.nearby_corpses", "Quick Loot Nearby Corpses"),
        ] {
            let label = SETTINGS_SECTIONS
                .iter()
                .find(|section| section.id == section_id)
                .and_then(|section| section.options.iter().find(|option| option.id == option_id))
                .map(|option| option.en)
                .ok_or("reference display option")?;
            assert_eq!(label, exact_label, "stale catalog alias for {option_id}");
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
    fn screenshot_catalog_enables_only_the_consumed_game_viewport_choice() {
        assert_eq!(
            runtime_consumer("screenshots.capture.game_only"),
            Some("native renderer viewport crop")
        );
        for pending in [
            "screenshots.capture.backlog",
            "screenshots.auto",
            "screenshots.level",
            "screenshots.skill",
        ] {
            assert_eq!(runtime_consumer(pending), None, "{pending}");
        }
    }

    #[test]
    fn assignment_dependent_action_bar_controls_wait_for_a_production_source()
    -> Result<(), &'static str> {
        let section = SETTINGS_SECTIONS
            .iter()
            .find(|section| section.id == "action_bars")
            .ok_or("action bars")?;
        for id in ["bars.item_amounts", "bars.tooltips"] {
            let key = format!("action_bars.{id}");
            assert_eq!(runtime_consumer(&key), None, "{key}");
            let option = section
                .options
                .iter()
                .find(|option| option.id == id)
                .ok_or("assignment-dependent action-bar control")?;
            assert!(matches!(
                option.implementation,
                Implementation::Pending {
                    consumer: "action-bar assignment source"
                }
            ));
        }
        Ok(())
    }

    #[test]
    fn runtime_enabled_catalogue_is_an_explicit_audited_allowlist() {
        use std::collections::BTreeMap;

        let stored: BTreeMap<_, _> = SETTINGS_SECTIONS
            .iter()
            .flat_map(|section| section.options.iter().map(move |option| (section, option)))
            .filter_map(|(section, option)| match option.implementation {
                Implementation::Stored { consumer } => {
                    Some((format!("{}.{}", section.id, option.id), consumer))
                }
                Implementation::Implemented { .. } | Implementation::Pending { .. } => None,
            })
            .collect();
        let expected_stored: BTreeMap<_, _> = [
            ("action_bars.bars.labels", "action-bar renderer"),
            ("basic.basic.advanced", "settings discovery"),
            ("console.console.font_size", "chat text renderer"),
            ("console.console.timestamps", "received chat clock"),
            ("console.console.seconds", "received chat clock"),
            ("controls.controls.default_key_delay", "movement repeat scheduler"),
            ("controls.controls.key_delay_ms", "movement repeat scheduler"),
            ("effects.effects.ambient_light", "scene lighting overlay"),
            ("effects.effects.lighting", "scene lighting overlay"),
            ("game_window.window.target_marking", "scene target projection"),
            ("graphics.graphics.fps_indicator", "frame diagnostics overlay"),
            ("graphics.graphics.integer_scale", "scene viewport"),
            ("hud.arc_size_preset", "own-resource HUD"),
            ("hud.harmony_position", "own-resource HUD"),
            ("hud.hud.arc_distance", "own-resource HUD"),
            ("hud.hud.arc_opacity", "own-resource HUD"),
            ("hud.hud.arcs", "own-resource HUD"),
            ("hud.hud.creatures_enabled", "visible-actor HUD"),
            ("hud.hud.other_health", "visible-actor HUD"),
            ("hud.hud.owner_health", "own-resource HUD"),
            ("hud.hud.owner_mana", "own-resource HUD"),
            ("hud.hud.owner_name", "own-character HUD"),
            ("hud.hud.player_enabled", "own-resource HUD"),
            ("hud.hud.resource_bars", "own-resource HUD"),
            ("hud.show_harmony", "own-resource HUD"),
            ("interface.interface.highlight_mouse", "scene hover projection"),
            ("interface.interface.big_cursor", "native/custom pointer renderer"),
            ("interface.interface.system_cursor", "native/custom pointer renderer"),
            ("screenshots.capture.game_only", "native renderer viewport crop"),
        ]
        .into_iter()
        .map(|(key, consumer)| (key.to_owned(), consumer))
        .collect();
        assert_eq!(stored, expected_stored, "a Stored row requires a reviewed runtime consumer");

        let implemented: BTreeMap<_, _> = SETTINGS_SECTIONS
            .iter()
            .flat_map(|section| section.options.iter().map(move |option| (section, option)))
            .filter_map(|(section, option)| match option.implementation {
                Implementation::Implemented { field } => {
                    Some((format!("{}.{}", section.id, option.id), field))
                }
                Implementation::Stored { .. } | Implementation::Pending { .. } => None,
            })
            .collect();
        let expected_implemented: BTreeMap<_, _> = [
            ("action_bars.bars.bottom_rows", "action_bottom_rows"),
            ("action_bars.bars.bottom_visible", "action_bottom_enabled"),
            ("action_bars.bars.left_rows", "action_left_rows"),
            ("action_bars.bars.left_visible", "action_left_enabled"),
            ("action_bars.bars.right_rows", "action_right_rows"),
            ("action_bars.bars.right_visible", "action_right_enabled"),
            ("basic.basic.click_walk", "click_to_walk"),
            ("basic.basic.ui_scale", "ui_scale"),
            ("console.console.visible", "show_chat"),
            ("controls.controls.movement", "movement_keys"),
            ("graphics.graphics.background_fps", "background_fps"),
            ("graphics.graphics.fps", "fps"),
            ("graphics.graphics.fullscreen", "fullscreen"),
            ("graphics.graphics.height", "window_height"),
            ("graphics.graphics.vsync", "vsync"),
            ("graphics.graphics.width", "window_width"),
            ("interface.interface.contrast", "high_contrast"),
            ("interface.interface.english", "english"),
            ("interface.interface.reduced_motion", "reduced_motion"),
            ("panels.panels.battle", "show_battle"),
            ("panels.panels.inventory", "show_inventory"),
            ("panels.panels.minimap", "show_minimap"),
        ]
        .into_iter()
        .map(|(key, field)| (key.to_owned(), field))
        .collect();
        assert_eq!(
            implemented, expected_implemented,
            "an Implemented row requires a reviewed live field"
        );
    }

    #[test]
    fn observed_reference_dropdowns_contain_only_captured_choices() -> Result<(), &'static str> {
        let observed: &[(&str, &str, &[&str])] = &[
            (
                "controls",
                "controls.mouse_preset",
                &["Classic Controls", "Regular Controls", "Left Smart-Click"],
            ),
            (
                "controls",
                "controls.loot_button",
                &["Right", "SHIFT+Right", "Left"],
            ),
            (
                "interface",
                "interface.colourise_loot_value",
                &["None", "Frames", "Corners"],
            ),
            (
                "game_window",
                "window.target_marking",
                &["Frame & Highlight", "Frame Only", "Highlight Only", "None"],
            ),
            (
                "graphics",
                "graphics.antialiasing",
                &["None", "Antialiasing", "Smooth Retro"],
            ),
        ];
        for (section_id, option_id, expected) in observed {
            let option = SETTINGS_SECTIONS
                .iter()
                .find(|section| section.id == *section_id)
                .and_then(|section| section.options.iter().find(|option| option.id == *option_id))
                .ok_or("observed dropdown")?;
            let OptionKind::Choice(choices) = option.kind else {
                return Err("observed dropdown has the wrong control type");
            };
            assert_eq!(
                choices.iter().map(|choice| choice.en).collect::<Vec<_>>(),
                *expected,
                "unexpected choices for {option_id}"
            );
            assert!(choices.iter().all(|choice| !choice.id.is_empty()));
        }
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
