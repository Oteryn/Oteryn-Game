//! Typed adapters over the retained preferences; no serialized-field editing.
use crate::play::ReferencePreferences as Preferences;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Page {
    #[default]
    Basic,
    Controls,
    GeneralHotkeys,
    Interface,
    Hud,
    Console,
    GameWindow,
    ActionBars,
    Graphics,
    Effects,
    Sound,
    BattleSounds,
    UiSounds,
    Misc,
    Gameplay,
    Screenshots,
    Diagnostics,
}
impl Page {
    pub const BASIC_NAV: [Self; 1] = [Self::Basic];
    pub const ADVANCED: [Self; 16] = [
        Self::Controls,
        Self::GeneralHotkeys,
        Self::Interface,
        Self::Hud,
        Self::Console,
        Self::GameWindow,
        Self::ActionBars,
        Self::Graphics,
        Self::Effects,
        Self::Sound,
        Self::BattleSounds,
        Self::UiSounds,
        Self::Misc,
        Self::Gameplay,
        Self::Screenshots,
        Self::Diagnostics,
    ];
    pub const fn label(self) -> &'static str {
        match self {
            Self::Basic => "Basic Options",
            Self::Controls => "Controls",
            Self::GeneralHotkeys => "General Hotkeys",
            Self::Interface => "Interface",
            Self::Hud => "HUD",
            Self::Console => "Console",
            Self::GameWindow => "Game Window",
            Self::ActionBars => "Action Bars",
            Self::Graphics => "Graphics",
            Self::Effects => "Effects",
            Self::Sound => "Sound",
            Self::BattleSounds => "Battle Sounds",
            Self::UiSounds => "UI Sounds",
            Self::Misc => "Misc.",
            Self::Gameplay => "Gameplay",
            Self::Screenshots => "Screenshots",
            Self::Diagnostics => "Diagnostics",
        }
    }
    pub const fn child(self) -> bool {
        matches!(
            self,
            Self::GeneralHotkeys
                | Self::Hud
                | Self::Console
                | Self::GameWindow
                | Self::ActionBars
                | Self::Effects
                | Self::BattleSounds
                | Self::UiSounds
                | Self::Gameplay
        )
    }
}
#[derive(Clone, Copy)]
pub(super) enum Editor {
    Check {
        read: fn(&Preferences) -> bool,
        write: fn(&mut Preferences, bool),
    },
    Number {
        read: fn(&Preferences) -> u16,
        write: fn(&mut Preferences, u16),
        min: u16,
        max: u16,
    },
    Choice {
        read: fn(&Preferences) -> usize,
        write: fn(&mut Preferences, usize),
        values: &'static [&'static str],
    },
}
#[derive(Clone, Copy)]
pub(super) struct Row {
    pub id: &'static str,
    pub label: &'static str,
    pub editor: Editor,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Value {
    Check(bool),
    Number(u16),
    Choice(usize),
}
impl Row {
    pub fn enabled(self, p: &Preferences) -> bool {
        match self.id {
            "battle_sound.own_attacks"
            | "battle_sound.own_healing"
            | "battle_sound.own_support" => p.audio.own_spells,
            "battle_sound.other_attacks"
            | "battle_sound.other_healing"
            | "battle_sound.other_support" => p.audio.other_spells,
            "ui_sound.global"
            | "ui_sound.guild"
            | "ui_sound.npc"
            | "ui_sound.party_messages"
            | "ui_sound.private"
            | "ui_sound.private_without_tab"
            | "ui_sound.raid"
            | "ui_sound.system" => p.audio.ui_sounds[3],
            _ => true,
        }
    }
    pub fn value(self, p: &Preferences) -> Value {
        match self.editor {
            Editor::Check { read, .. } => Value::Check(self.enabled(p) && read(p)),
            Editor::Number { read, .. } => Value::Number(read(p)),
            Editor::Choice { read, .. } => Value::Choice(read(p)),
        }
    }
    pub fn set(self, p: &mut Preferences, value: Value) -> bool {
        match (self.editor, value) {
            (Editor::Check { write, .. }, Value::Check(v)) => write(p, v),
            (
                Editor::Number {
                    write, min, max, ..
                },
                Value::Number(v),
            ) if (min..=max).contains(&v) => write(p, v),
            (Editor::Choice { write, values, .. }, Value::Choice(v)) if v < values.len() => {
                write(p, v)
            }
            _ => return false,
        }
        true
    }
    pub fn reset(self, p: &mut Preferences, defaults: &Preferences) {
        self.set(p, self.value(defaults));
    }
}
macro_rules! check {
    ($id:literal,$label:literal,$($field:tt)+) => {
        Row { id:$id,label:$label,editor:Editor::Check {read:|p|p.$($field)+,write:|p,v|{p.$($field)+ = v;}} }
    };
}
macro_rules! number {
    ($id:literal,$label:literal,$min:literal,$max:literal,u16,$($field:tt)+) => {
        Row { id:$id,label:$label,editor:Editor::Number {read:|p|p.$($field)+,
            write:|p,v|{p.$($field)+ = v;},min:$min,max:$max} }
    };
    ($id:literal,$label:literal,$min:literal,$max:literal,u8,$($field:tt)+) => {
        Row { id:$id,label:$label,editor:Editor::Number {read:|p|u16::from(p.$($field)+),
            // Row::set checks the declared 0..=100 range before this bounded conversion.
            write:|p,v|{p.$($field)+ = v as u8;},min:$min,max:$max} }
    };
}

mod controls;
mod diagnostics;
mod graphics;
mod hud;
mod interface;
mod miscellaneous;
mod sound;
pub(super) fn rows(page: Page) -> &'static [Row] {
    match page {
        Page::Basic => &BASIC,
        Page::Controls => &controls::ROWS,
        // Hotkey sets and key bindings are list editors (`profiles.rs`, `keys.rs`).
        Page::GeneralHotkeys => &[],
        Page::Interface => &interface::MAIN,
        Page::Hud => &hud::HUD,
        Page::Console => &interface::CONSOLE,
        Page::GameWindow => &interface::GAME_WINDOW,
        Page::ActionBars => &interface::ACTION_BARS,
        Page::Graphics => &graphics::MAIN,
        Page::Effects => &graphics::EFFECTS,
        Page::Sound => &sound::MAIN,
        Page::BattleSounds => &sound::BATTLE,
        Page::UiSounds => &sound::UI,
        Page::Misc => &miscellaneous::MAIN,
        Page::Gameplay => &miscellaneous::GAMEPLAY,
        Page::Diagnostics => &diagnostics::ROWS,
        Page::Screenshots => &[
            check!(
                "screenshots.reward_chest",
                "Capture reward-chest treasure",
                screenshots.reward_chest
            ),
            check!(
                "screenshots.achievement",
                "Capture on achievement",
                screenshots.on_achievement
            ),
            Row {
                id: "screenshots.automatic",
                label: "Enable automatic screenshots",
                editor: Editor::Check {
                    read: |p| !p.screenshots.automatic_disabled,
                    write: |p, v| p.screenshots.automatic_disabled = !v,
                },
            },
            Row {
                id: "screenshots.backlog",
                label: "Screenshot backlog (seconds, 0 disables)",
                editor: Editor::Number {
                    read: |p| u16::from(p.screenshots.effective().backlog_frames),
                    write: |p, value| {
                        p.screenshots.backlog_frames = 0;
                        p.screenshots.backlog_seconds = value as u8;
                    },
                    min: 0,
                    max: 10,
                },
            },
            check!(
                "screenshots.bestiary-complete",
                "Capture on bestiary entry completed",
                screenshots.on_bestiary_complete
            ),
            check!(
                "screenshots.bestiary-unlock",
                "Capture on bestiary entry unlocked",
                screenshots.on_bestiary_unlock
            ),
            check!(
                "screenshots.boss",
                "Capture on boss defeated",
                screenshots.on_boss_defeat
            ),
            check!(
                "screenshots.death",
                "Capture on death",
                screenshots.on_death
            ),
            check!(
                "screenshots.game-only",
                "Capture game area only",
                screenshots.game_only
            ),
            check!(
                "screenshots.level",
                "Capture on level up",
                screenshots.on_level
            ),
            check!(
                "screenshots.low-health",
                "Capture on low health",
                screenshots.on_low_health
            ),
            check!(
                "screenshots.player-attacking",
                "Capture when a player attacks you",
                screenshots.on_player_attacking
            ),
            check!(
                "screenshots.record-damage",
                "Capture on highest damage dealt",
                screenshots.on_record_damage
            ),
            check!(
                "screenshots.record-healing",
                "Capture on highest healing done",
                screenshots.on_record_healing
            ),
            check!(
                "screenshots.skill",
                "Capture on skill up",
                screenshots.on_skill
            ),
            check!(
                "screenshots.valuable-loot",
                "Capture on tracked loot",
                screenshots.on_valuable_loot
            ),
        ],
    }
}
const BASIC: [Row; 7] = [
    controls::ROWS[0],
    controls::ROWS[1],
    graphics::MAIN[0],
    graphics::MAIN[1],
    graphics::MAIN[2],
    sound::MAIN[0],
    sound::MAIN[1],
];
pub(super) use miscellaneous::LOGIN_CHOICES;
pub(super) fn find(page: Page, id: &str) -> Option<Row> {
    rows(page).iter().find(|row| row.id == id).copied()
}
