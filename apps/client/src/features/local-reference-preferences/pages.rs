//! Settings navigation and built-in window commands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsPage {
    #[default]
    Gameplay,
    Hotkeys,
    BarHotkeys,
    CustomHotkeys,
    Arcs,
    ActorHud,
    Status,
    Console,
    Interface,
    Graphics,
    Sound,
    Help,
}

impl SettingsPage {
    pub const ALL: [Self; 12] = [
        Self::Gameplay,
        Self::Hotkeys,
        Self::BarHotkeys,
        Self::CustomHotkeys,
        Self::Arcs,
        Self::ActorHud,
        Self::Status,
        Self::Console,
        Self::Interface,
        Self::Graphics,
        Self::Sound,
        Self::Help,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Gameplay => "Gameplay",
            Self::Hotkeys => "Controls / General Hotkeys",
            Self::BarHotkeys => "Controls / Action Bar Hotkeys",
            Self::Console => "Interface / Console",
            Self::Status => "Interface / Status Panel",
            Self::ActorHud => "Interface / HUD",
            Self::Arcs => "Interface / HUD Arcs",
            Self::CustomHotkeys => "Controls / Custom Hotkey Profiles",
            Self::Interface => "Interface",
            Self::Graphics => "Graphics",
            Self::Sound => "Sound",
            Self::Help => "Help / settings transfer",
        }
    }
}

pub const WINDOW_ACTIONS: &[(&str, &str)] = &[
    ("0199-cyclopedia-character-open", "Cyclopedia"),
    ("0287-actor-spell-catalog-open", "Spellbook"),
    ("local-spell-action-bar-open", "Spell action bar"),
    ("0279-skill-progress-open", "Skills"),
    ("0276-condition-status-open", "Conditions"),
    ("0277-rest-status-open", "Rest and stamina"),
    ("0016-quest-log-open", "Quest log"),
    ("0007-chat-keybindings", "Chat"),
];
