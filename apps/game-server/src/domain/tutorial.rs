//! TUTORIAL-1: the authoritative Newhaven tutorial state, gated on the start island (CP D1426).
//!
//! The tutorial applies only to a Character whose start cell is on the Newhaven start island, as
//! in the global reference. CP Q2 on #2186 moves the World start (`entry_start` of
//! `content/world/pins/oteryn.json`) from the StartThais test start (D1410, D1463) to the islet
//! temple, so a Character created on this World gets [`TutorialGate::Newhaven`]. A Character
//! placed before that switch keeps its stored position outside Newhaven and gets
//! [`TutorialGate::Inert`]: every [`TutorialEvent`] then leaves the progress unchanged and has no
//! effect.
//!
//! The island is the content record `oteryn:area.island.newhaven` (TibiaWiki page 109339, rev
//! 1197143), in its coordinate frame `global-target-2026-09-27`: the island footprint and its
//! additional component, the tutorial islet. The rules are those of the reference scripts of
//! `zimbadev/crystalserver` at `9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d`, under
//! `data-global/scripts/quests/newhaven/` (git blob ids in parentheses):
//!
//! - `on_login_newhaven.lua` (`b101507c`): a vocationless Character without account access whose
//!   citizen track is not set is placed on the islet temple and becomes a citizen;
//! - `movement_init_newhaven.lua` (`9d4adb9b`): the islet hole pushes the Character back;
//! - `movement_select_vocation.lua` (`8366ff4e`): the islet forcefield moves the Character to the
//!   island and arms the hunting hint and The Corruptor's vocation loot;
//! - `movement_update_tutorial_hunting.lua` (`4c3cb41f`): the hunting tiles show the hunting hint
//!   once (the reference sends client tutorial 3 to every vocation; this module sends each base
//!   vocation its own basic-combat tutorial of the client tutorial table, see
//!   [`TutorialVocation::hunting_hint`]);
//! - `#ondroploot_the_corruptor.lua` (`a6984946`): The Corruptor's vocation loot is added once;
//! - `on_death_newhaven.lua` (`680ec85a`, registered at login for a citizen): a vocationless
//!   Character without access goes back to the islet temple;
//! - the NPCs `a_seagull` and `anna` (`content/quests/missions/quest-state.json`, quest
//!   `newhaven_quest`): leaving Newhaven clears every track.
//!
//! The reference keeps three storage tracks (citizen, tutorial hunting, The Corruptor's loot).
//! [`TutorialProgress`] keeps the same facts: the citizen track becomes the [`TutorialStep`]
//! (islet, island, departed; the islet/island split is where the forcefield last moved the
//! Character), and the two other tracks are the two pending flags. Starter items, map marks, the
//! vocation loot itself, the Dawnport branch of the death script and the level-8 door
//! (`on_use_newhaven.lua`) are not part of this module. The death rule reads the current step,
//! where the reference reads the citizen track at the last login.

/// A cell in the content coordinate frame `global-target-2026-09-27`, with the legacy floor (`7`
/// is the ground floor). A native World-bundle floor is the negated legacy floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TutorialCell {
    pub x: i32,
    pub y: i32,
    pub floor: i16,
}

impl TutorialCell {
    #[must_use]
    pub const fn new(x: i32, y: i32, floor: i16) -> Self {
        Self { x, y, floor }
    }

    /// The cell of a native World-bundle position (native floor `-7` is legacy floor `7`).
    #[must_use]
    pub const fn from_native(x: i32, y: i32, native_floor: i16) -> Self {
        Self::new(x, y, -native_floor)
    }
}

/// An inclusive rectangle of one floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TutorialArea {
    pub min_x: i32,
    pub max_x: i32,
    pub min_y: i32,
    pub max_y: i32,
    pub floor: i16,
}

impl TutorialArea {
    #[must_use]
    pub const fn contains(self, cell: TutorialCell) -> bool {
        cell.floor == self.floor
            && cell.x >= self.min_x
            && cell.x <= self.max_x
            && cell.y >= self.min_y
            && cell.y <= self.max_y
    }
}

/// `oteryn:area.island.newhaven` footprint.
pub const NEWHAVEN_ISLAND: TutorialArea = TutorialArea {
    min_x: 32506,
    max_x: 32634,
    min_y: 32405,
    max_y: 32522,
    floor: 7,
};

/// `oteryn:area.island.newhaven` additional component: the tutorial islet with the temple.
pub const NEWHAVEN_ISLET: TutorialArea = TutorialArea {
    min_x: 32532,
    max_x: 32539,
    min_y: 32511,
    max_y: 32518,
    floor: 7,
};

/// The islet temple: the login and death placement of `on_login_newhaven.lua` and
/// `on_death_newhaven.lua`.
pub const ISLET_TEMPLE: TutorialCell = TutorialCell::new(32534, 32513, 7);
/// The islet hole of `init_newhaven.lua`.
pub const ISLET_HOLE: TutorialCell = TutorialCell::new(32533, 32513, 7);
/// The message `init_newhaven.lua` shows when it pushes the Character back.
pub const ISLET_HOLE_MESSAGE: &str = "I should not climb down again but rather venture forth.";
/// The islet forcefield of `select_vocation.lua`.
pub const ISLET_FORCEFIELD: TutorialCell = TutorialCell::new(32538, 32514, 7);
/// The island arrival of `select_vocation.lua`.
pub const ISLAND_ARRIVAL: TutorialCell = TutorialCell::new(32544, 32515, 7);
/// The two hunting tiles of `update_tutorial_hunting.lua`.
pub const HUNTING_TILES: [TutorialCell; 2] = [
    TutorialCell::new(32561, 32485, 7),
    TutorialCell::new(32562, 32485, 7),
];
/// The client tutorial id `select_vocation.lua` sends to a vocationless Character
/// (`chooseVocation` of the client tutorial table).
pub const ISLAND_HINT: u8 = 2;
/// The client tutorial id `update_tutorial_hunting.lua` sends to every vocation. The client
/// tutorial table (`opentibiabr/otclient` `53d3a8ac`, `modules/game_tutorial/const/vocations.lua`)
/// names 3 `combatBasicKnight`, 4 `combatBasicPaladin`, 5 `combatBasicSorcerer`, 6
/// `combatBasicDruid` and 11 `combatBasicMonk`, so 3 is the Knight's; the tiles send the hint of
/// the Character's base vocation instead ([`TutorialVocation::hunting_hint`]).
pub const HUNTING_HINT: u8 = 3;
/// The basic-combat tutorial ids of the client tutorial table, per base vocation.
pub const HUNTING_HINT_KNIGHT: u8 = HUNTING_HINT;
pub const HUNTING_HINT_PALADIN: u8 = 4;
pub const HUNTING_HINT_SORCERER: u8 = 5;
pub const HUNTING_HINT_DRUID: u8 = 6;
pub const HUNTING_HINT_MONK: u8 = 11;

/// Whether the tutorial applies to a Character, decided once from its start cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TutorialGate {
    /// The start cell is on the Newhaven start island: the tutorial applies.
    Newhaven,
    /// Any other start (the StartThais test start): the tutorial never starts.
    Inert,
}

impl TutorialGate {
    #[must_use]
    pub const fn for_start(start: TutorialCell) -> Self {
        if NEWHAVEN_ISLAND.contains(start) || NEWHAVEN_ISLET.contains(start) {
            Self::Newhaven
        } else {
            Self::Inert
        }
    }
}

/// The citizen track as a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TutorialStep {
    /// A citizen on the tutorial islet (placed at login or after a death, not yet through the
    /// forcefield).
    Islet,
    /// A citizen moved to the island by the forcefield.
    Island,
    /// Left Newhaven through `a_seagull` or `anna`: every track cleared.
    Departed,
}

impl TutorialStep {
    #[must_use]
    pub const fn code(self) -> i16 {
        match self {
            Self::Islet => 1,
            Self::Island => 2,
            Self::Departed => 3,
        }
    }

    #[must_use]
    pub const fn from_code(code: i16) -> Option<Self> {
        match code {
            1 => Some(Self::Islet),
            2 => Some(Self::Island),
            3 => Some(Self::Departed),
            _ => None,
        }
    }
}

/// The durable tutorial progress of one Character. `None` (no row) is a Character that never
/// entered the tutorial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TutorialProgress {
    pub step: TutorialStep,
    /// The tutorial hunting track is armed: the hunting tiles still show [`HUNTING_HINT`].
    pub hunting_hint_pending: bool,
    /// The Corruptor's loot track is armed: its next loot still adds the vocation loot.
    pub corruptor_loot_pending: bool,
}

impl TutorialProgress {
    #[must_use]
    pub const fn new(step: TutorialStep) -> Self {
        Self {
            step,
            hunting_hint_pending: false,
            corruptor_loot_pending: false,
        }
    }

    /// A departed Character has every track cleared; no transition produces anything else.
    #[must_use]
    pub const fn is_valid(self) -> bool {
        !matches!(self.step, TutorialStep::Departed)
            || (!self.hunting_hint_pending && !self.corruptor_loot_pending)
    }
}

/// The Character's base vocation as the tutorial reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TutorialVocation {
    /// No vocation yet (the build vocation `none`).
    None,
    Knight,
    Paladin,
    Sorcerer,
    Druid,
    Monk,
    /// A vocation key the tutorial does not know: it is not vocationless and gets no hint.
    Other,
}

impl TutorialVocation {
    /// The vocation of a build vocation key: `none`, a base vocation or its promotion.
    #[must_use]
    pub fn from_key(key: &str) -> Self {
        match key {
            "none" => Self::None,
            "knight" | "elite_knight" => Self::Knight,
            "paladin" | "royal_paladin" => Self::Paladin,
            "sorcerer" | "master_sorcerer" => Self::Sorcerer,
            "druid" | "elder_druid" => Self::Druid,
            "monk" | "exalted_monk" => Self::Monk,
            _ => Self::Other,
        }
    }

    /// The basic-combat tutorial the hunting tiles show; `None` without a base vocation, which
    /// leaves the hunting track armed until the Character has one.
    #[must_use]
    pub const fn hunting_hint(self) -> Option<u8> {
        match self {
            Self::Knight => Some(HUNTING_HINT_KNIGHT),
            Self::Paladin => Some(HUNTING_HINT_PALADIN),
            Self::Sorcerer => Some(HUNTING_HINT_SORCERER),
            Self::Druid => Some(HUNTING_HINT_DRUID),
            Self::Monk => Some(HUNTING_HINT_MONK),
            Self::None | Self::Other => None,
        }
    }
}

/// What the reference scripts read of the Character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TutorialActor {
    pub vocation: TutorialVocation,
    /// The account has access (a gamemaster); the login and death scripts skip it.
    pub has_access: bool,
}

impl TutorialActor {
    /// The Character has no vocation yet.
    #[must_use]
    pub const fn vocationless(self) -> bool {
        matches!(self.vocation, TutorialVocation::None)
    }
}

/// One authoritative input of the tutorial.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TutorialEvent {
    Login(TutorialActor),
    /// The Character stepped onto `to`, coming from `from`.
    StepIn {
        actor: TutorialActor,
        from: TutorialCell,
        to: TutorialCell,
    },
    /// The Corruptor dropped loot for this Character.
    CorruptorLoot,
    /// `a_seagull` or `anna` took the Character away from Newhaven.
    Depart,
    Death(TutorialActor),
}

/// The visible effect of a transition, for the caller to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TutorialEffect {
    /// Move the Character to the cell.
    Place(TutorialCell),
    /// Refuse the step and keep the Character on `to`, with [`ISLET_HOLE_MESSAGE`].
    PushBack { to: TutorialCell },
    /// Move the Character to the cell and show a client tutorial hint.
    PlaceWithHint { cell: TutorialCell, hint: u8 },
    /// Show a client tutorial hint.
    Hint(u8),
    /// Add the vocation loot of The Corruptor (its items are not part of this module).
    CorruptorVocationLoot,
}

/// The result of one event: the progress to store and the effect to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TutorialTransition {
    pub progress: Option<TutorialProgress>,
    pub effect: Option<TutorialEffect>,
}

impl TutorialTransition {
    /// Whether the progress must be written.
    #[must_use]
    pub fn changes(self, before: Option<TutorialProgress>) -> bool {
        self.progress != before
    }
}

/// Applies one event to the stored progress. An [`TutorialGate::Inert`] Character never changes.
#[must_use]
pub fn apply(
    gate: TutorialGate,
    progress: Option<TutorialProgress>,
    event: TutorialEvent,
) -> TutorialTransition {
    let unchanged = TutorialTransition {
        progress,
        effect: None,
    };
    if gate == TutorialGate::Inert {
        return unchanged;
    }
    let citizen = matches!(
        progress,
        Some(TutorialProgress {
            step: TutorialStep::Islet | TutorialStep::Island,
            ..
        })
    );
    match event {
        // on_login_newhaven.lua: `newhavenCitizen <= 0` for a vocationless Character without
        // access places it on the islet temple and sets the citizen track; a citizen resumes.
        TutorialEvent::Login(actor) if !citizen && actor.vocationless() && !actor.has_access => {
            TutorialTransition {
                progress: Some(TutorialProgress {
                    step: TutorialStep::Islet,
                    ..progress.unwrap_or(TutorialProgress::new(TutorialStep::Islet))
                }),
                effect: Some(TutorialEffect::Place(ISLET_TEMPLE)),
            }
        }
        TutorialEvent::Login(_) => unchanged,
        // init_newhaven.lua: the hole always pushes back.
        TutorialEvent::StepIn { to, from, .. } if to == ISLET_HOLE => TutorialTransition {
            progress,
            effect: Some(TutorialEffect::PushBack { to: from }),
        },
        // select_vocation.lua: the forcefield moves to the island and sets all three tracks;
        // only a vocationless Character gets the hint.
        TutorialEvent::StepIn { actor, to, .. } if to == ISLET_FORCEFIELD => TutorialTransition {
            progress: Some(TutorialProgress {
                step: TutorialStep::Island,
                hunting_hint_pending: true,
                corruptor_loot_pending: true,
            }),
            effect: Some(if actor.vocationless() {
                TutorialEffect::PlaceWithHint {
                    cell: ISLAND_ARRIVAL,
                    hint: ISLAND_HINT,
                }
            } else {
                TutorialEffect::Place(ISLAND_ARRIVAL)
            }),
        },
        // update_tutorial_hunting.lua: an armed hunting track shows the hint once, here the
        // basic-combat hint of the Character's base vocation.
        TutorialEvent::StepIn { actor, to, .. } if HUNTING_TILES.contains(&to) => {
            match (progress, actor.vocation.hunting_hint()) {
                (Some(stored), Some(hint)) if stored.hunting_hint_pending => TutorialTransition {
                    progress: Some(TutorialProgress {
                        hunting_hint_pending: false,
                        ..stored
                    }),
                    effect: Some(TutorialEffect::Hint(hint)),
                },
                _ => unchanged,
            }
        }
        TutorialEvent::StepIn { .. } => unchanged,
        // ondroploot_the_corruptor.lua: an armed track adds the vocation loot once.
        TutorialEvent::CorruptorLoot => match progress {
            Some(stored) if stored.corruptor_loot_pending => TutorialTransition {
                progress: Some(TutorialProgress {
                    corruptor_loot_pending: false,
                    ..stored
                }),
                effect: Some(TutorialEffect::CorruptorVocationLoot),
            },
            _ => unchanged,
        },
        // a_seagull / anna: every track becomes -1.
        TutorialEvent::Depart => match progress {
            Some(_) => TutorialTransition {
                progress: Some(TutorialProgress::new(TutorialStep::Departed)),
                effect: None,
            },
            None => unchanged,
        },
        // on_death_newhaven.lua (registered for a citizen at login): a vocationless Character
        // without access goes back to the islet temple, still a citizen.
        TutorialEvent::Death(actor) if citizen && actor.vocationless() && !actor.has_access => {
            TutorialTransition {
                progress: progress.map(|stored| TutorialProgress {
                    step: TutorialStep::Islet,
                    ..stored
                }),
                effect: Some(TutorialEffect::Place(ISLET_TEMPLE)),
            }
        }
        TutorialEvent::Death(_) => unchanged,
    }
}
