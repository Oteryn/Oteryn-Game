//! TUTORIAL-1: the authoritative Newhaven tutorial state, gated on the start island (CP D1426).
//!
//! The tutorial applies only to a Character whose start cell is on the Newhaven start island, as
//! in the global reference. The current start is the temporary test start StartThais (D1410), so
//! every Character of the current World gets [`TutorialGate::Inert`] and never enters the
//! tutorial: every [`TutorialEvent`] then leaves the progress unchanged and has no effect.
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
//!   once;
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

/// Bounding box of the main `oteryn:area.island.newhaven` land component, not its exact mask.
pub const NEWHAVEN_ISLAND: TutorialArea = TutorialArea {
    min_x: 32506,
    max_x: 32634,
    min_y: 32405,
    max_y: 32522,
    floor: 7,
};

/// Bounding box of the additional Newhaven component: the tutorial islet with the temple.
pub const NEWHAVEN_ISLET: TutorialArea = TutorialArea {
    min_x: 32532,
    max_x: 32539,
    min_y: 32511,
    max_y: 32518,
    floor: 7,
};

// Exact floor-7 land components of `oteryn:area.island.newhaven`, sorted by (y, x).
// Inclusive (y, min_x, max_x) row spans, derived from the committed WorldPlacement.Base B3
// map with convert_islands.py's land_component rule (4 neighbours, non-water/lava ground).
// Anchors: island (32560, 32488), 9,189 tiles; islet (32536, 32514), 50 tiles.
// tests/tutorial_newhaven.rs flood-fills both content-record anchors and checks this mask
// against the map, including other land components and water inside the bounding boxes.
const NEWHAVEN_LAND_SPANS: [(i32, i32, i32); 146] = [
    (32405, 32547, 32554),
    (32406, 32546, 32555),
    (32406, 32574, 32575),
    (32406, 32578, 32581),
    (32407, 32544, 32560),
    (32407, 32571, 32585),
    (32407, 32587, 32592),
    (32408, 32539, 32563),
    (32408, 32570, 32596),
    (32409, 32538, 32565),
    (32409, 32568, 32598),
    (32410, 32536, 32599),
    (32411, 32531, 32600),
    (32412, 32528, 32601),
    (32413, 32525, 32602),
    (32414, 32523, 32603),
    (32414, 32608, 32614),
    (32415, 32522, 32618),
    (32416, 32521, 32620),
    (32417, 32520, 32620),
    (32418, 32518, 32621),
    (32419, 32518, 32621),
    (32420, 32516, 32622),
    (32421, 32515, 32622),
    (32422, 32515, 32622),
    (32423, 32514, 32623),
    (32424, 32514, 32623),
    (32425, 32514, 32624),
    (32426, 32513, 32625),
    (32427, 32513, 32627),
    (32428, 32512, 32628),
    (32429, 32509, 32628),
    (32430, 32506, 32628),
    (32431, 32506, 32628),
    (32432, 32507, 32628),
    (32433, 32508, 32628),
    (32434, 32509, 32628),
    (32435, 32509, 32628),
    (32436, 32509, 32628),
    (32437, 32509, 32628),
    (32438, 32509, 32628),
    (32439, 32509, 32628),
    (32440, 32509, 32628),
    (32441, 32509, 32628),
    (32442, 32510, 32627),
    (32443, 32510, 32628),
    (32444, 32511, 32629),
    (32445, 32511, 32630),
    (32446, 32512, 32631),
    (32447, 32513, 32632),
    (32448, 32514, 32517),
    (32448, 32519, 32633),
    (32449, 32520, 32634),
    (32450, 32520, 32634),
    (32451, 32520, 32634),
    (32452, 32521, 32634),
    (32453, 32522, 32633),
    (32454, 32522, 32632),
    (32455, 32523, 32632),
    (32456, 32523, 32632),
    (32457, 32523, 32632),
    (32458, 32524, 32631),
    (32459, 32525, 32628),
    (32460, 32525, 32627),
    (32461, 32525, 32626),
    (32462, 32525, 32625),
    (32463, 32525, 32622),
    (32464, 32526, 32621),
    (32464, 32624, 32628),
    (32465, 32527, 32620),
    (32465, 32623, 32628),
    (32466, 32527, 32619),
    (32466, 32622, 32628),
    (32467, 32527, 32619),
    (32467, 32622, 32623),
    (32467, 32625, 32628),
    (32468, 32527, 32623),
    (32468, 32625, 32628),
    (32469, 32527, 32622),
    (32469, 32625, 32627),
    (32470, 32527, 32623),
    (32471, 32528, 32623),
    (32472, 32529, 32623),
    (32473, 32529, 32622),
    (32474, 32530, 32621),
    (32475, 32531, 32611),
    (32475, 32615, 32620),
    (32476, 32531, 32606),
    (32477, 32536, 32602),
    (32478, 32540, 32600),
    (32479, 32541, 32600),
    (32480, 32542, 32599),
    (32481, 32543, 32596),
    (32482, 32544, 32594),
    (32483, 32545, 32593),
    (32484, 32545, 32592),
    (32485, 32545, 32591),
    (32486, 32545, 32590),
    (32487, 32546, 32588),
    (32488, 32546, 32588),
    (32489, 32547, 32587),
    (32490, 32547, 32586),
    (32491, 32547, 32586),
    (32492, 32547, 32585),
    (32493, 32548, 32585),
    (32494, 32548, 32549),
    (32494, 32555, 32584),
    (32495, 32555, 32583),
    (32496, 32555, 32583),
    (32497, 32548, 32584),
    (32498, 32546, 32584),
    (32499, 32544, 32585),
    (32500, 32543, 32586),
    (32501, 32543, 32587),
    (32502, 32544, 32587),
    (32503, 32544, 32588),
    (32504, 32544, 32588),
    (32505, 32544, 32588),
    (32506, 32543, 32588),
    (32507, 32540, 32588),
    (32508, 32540, 32588),
    (32509, 32540, 32587),
    (32510, 32540, 32586),
    (32511, 32533, 32536),
    (32511, 32540, 32585),
    (32512, 32532, 32539),
    (32512, 32541, 32582),
    (32513, 32532, 32539),
    (32513, 32542, 32580),
    (32514, 32532, 32539),
    (32514, 32542, 32576),
    (32515, 32532, 32539),
    (32515, 32542, 32572),
    (32516, 32533, 32539),
    (32516, 32542, 32570),
    (32517, 32534, 32538),
    (32517, 32541, 32569),
    (32518, 32535, 32536),
    (32518, 32541, 32561),
    (32518, 32564, 32567),
    (32519, 32542, 32560),
    (32520, 32542, 32559),
    (32521, 32543, 32545),
    (32521, 32548, 32555),
    (32522, 32543, 32544),
    (32522, 32552, 32553),
];

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
/// The client tutorial id `select_vocation.lua` sends to a vocationless Character.
pub const ISLAND_HINT: u8 = 2;
/// The client tutorial id `update_tutorial_hunting.lua` sends.
pub const HUNTING_HINT: u8 = 3;

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
        // The islet's bounds are wholly inside the island's bounding box. The box alone also
        // includes water, void and unrelated land components; only the exact spans admit starts.
        if !NEWHAVEN_ISLAND.contains(start) {
            return Self::Inert;
        }
        let mut index = 0;
        while index < NEWHAVEN_LAND_SPANS.len() {
            let (y, min_x, max_x) = NEWHAVEN_LAND_SPANS[index];
            if start.y == y && start.x >= min_x && start.x <= max_x {
                return Self::Newhaven;
            }
            index += 1;
        }
        Self::Inert
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

/// What the reference scripts read of the Character.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TutorialActor {
    /// The Character has no vocation yet.
    pub vocationless: bool,
    /// The account has access (a gamemaster); the login and death scripts skip it.
    pub has_access: bool,
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
        TutorialEvent::Login(actor) if !citizen && actor.vocationless && !actor.has_access => {
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
            effect: Some(if actor.vocationless {
                TutorialEffect::PlaceWithHint {
                    cell: ISLAND_ARRIVAL,
                    hint: ISLAND_HINT,
                }
            } else {
                TutorialEffect::Place(ISLAND_ARRIVAL)
            }),
        },
        // update_tutorial_hunting.lua: an armed hunting track shows the hint once.
        TutorialEvent::StepIn { to, .. } if HUNTING_TILES.contains(&to) => match progress {
            Some(stored) if stored.hunting_hint_pending => TutorialTransition {
                progress: Some(TutorialProgress {
                    hunting_hint_pending: false,
                    ..stored
                }),
                effect: Some(TutorialEffect::Hint(HUNTING_HINT)),
            },
            _ => unchanged,
        },
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
        TutorialEvent::Death(actor) if citizen && actor.vocationless && !actor.has_access => {
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
