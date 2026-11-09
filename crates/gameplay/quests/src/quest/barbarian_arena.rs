//! Source rules for credited Barbarian Arena deaths and pit geometry.
//! Crystal 201189f, data-global/scripts/quests/svargrond_arena/
//! creaturescripts_arena_kill.lua:2-38; data-global/lib/quests/svargrond_arena.lua:
//! 7-22,25-115,118-172,193-256,285-313.
//!
//! These are proposals, not write authority. The production owner must separately
//! retain the current exclusive pit lease and native most-damage attribution.

pub const KICK_AFTER_SECONDS: u64 = 600;
pub const KICK_POSITION: [i32; 3] = [32251, 31098, -6];
pub const REWARD_POSITION: [i32; 3] = [32222, 31080, -6];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Greenhorn,
    Scrapper,
    Warlord,
}

impl Tier {
    pub fn from_storage(value: i64) -> Option<Self> {
        match value {
            1 => Some(Self::Greenhorn),
            2 => Some(Self::Scrapper),
            3 => Some(Self::Warlord),
            _ => None,
        }
    }

    pub fn creatures(self) -> &'static [&'static str; 10] {
        match self {
            Self::Greenhorn => &GREENHORN,
            Self::Scrapper => &SCRAPPER,
            Self::Warlord => &WARLORD,
        }
    }

    pub fn creature_for(self, pit: Pit) -> &'static str {
        self.creatures()[usize::from(pit.0 - 1)]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pit(u8);

impl Pit {
    pub fn new(number: u8) -> Option<Self> {
        (1..=10).contains(&number).then_some(Self(number))
    }

    pub fn number(self) -> u8 {
        self.0
    }

    pub fn layout(self) -> PitLayout {
        // The last pit deliberately differs by one Y tile between its reset
        // rectangle and its center/pillar/summon (donor lib:108-114).
        let (dx, reset_dy, object_dy) = [
            (0, 0, 0),
            (-14, 0, 0),
            (-28, 0, 0),
            (-42, 0, 0),
            (-35, -14, -14),
            (-21, -14, -14),
            (-7, -14, -14),
            (-14, -28, -28),
            (-28, -28, -28),
            (-21, -42, -41),
        ][usize::from(self.0 - 1)];
        PitLayout {
            reset_from: [32205 + dx, 31094 + reset_dy, -7],
            reset_to: [32214 + dx, 31103 + reset_dy, -7],
            center: [32210 + dx, 31098 + object_dy, -7],
            pillar: [32204 + dx, 31098 + object_dy, -7],
            summon: [32210 + dx, 31100 + object_dy, -7],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PitLayout {
    pub reset_from: [i32; 3],
    pub reset_to: [i32; 3],
    pub center: [i32; 3],
    pub pillar: [i32; 3],
    pub summon: [i32; 3],
}

/// Original native stack-top observation, captured before removing Monsters.
/// Crystal201189f lib/quests/svargrond_arena.lua:217-232 checks only this Thing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResetTopThing<T> {
    Empty,
    Creature,
    Item {
        identity: T,
        appearance: u16,
        movable: bool,
    },
}

impl PitLayout {
    /// Read proposal only: the physical Item owner must revalidate this exact
    /// original top Thing and retain its reservation, receipt and audit.
    pub fn item_to_reset<T: Copy>(self, at: [i32; 3], top: ResetTopThing<T>) -> Option<T> {
        if !self.contains_reset(at) {
            return None;
        }
        // Source lib:22,217-225: never search beneath a Creature, an immovable
        // Item or one of the five preserved timer/pillar/basin appearances.
        match top {
            ResetTopThing::Item {
                identity,
                appearance,
                movable: true,
            } if ![21464, 1841, 2181, 2182, 2183].contains(&appearance) => Some(identity),
            _ => None,
        }
    }

    /// Crystal svargrond_arena.lua:213-216: inclusive source reset rectangle,
    /// deliberately distinct from spectator range and the last pit's center.
    pub fn reset_cells(self) -> impl Iterator<Item = [i32; 3]> {
        (self.reset_from[0]..=self.reset_to[0]).flat_map(move |x| {
            (self.reset_from[1]..=self.reset_to[1]).flat_map(move |y| {
                (self.reset_from[2]..=self.reset_to[2]).map(move |floor| [x, y, floor])
            })
        })
    }

    pub fn contains_reset(self, at: [i32; 3]) -> bool {
        (0..3).all(|axis| (self.reset_from[axis]..=self.reset_to[axis]).contains(&at[axis]))
    }

    /// Source lib:229-232 removes the top Monster, never an NPC or Player.
    /// The native caller supplies only real live Monster observations and keeps
    /// its runtime guard. Duplicate cells/identities are ambiguous, so no plan
    /// is returned; the current native placement owner forbids overlapping mobs.
    /// This read plan grants no removal or durable Item authority.
    pub fn monsters_to_reset<T: Copy + PartialEq>(
        self,
        monsters: impl IntoIterator<Item = (T, [i32; 3])>,
    ) -> Option<Vec<T>> {
        let mut cells = std::collections::BTreeSet::new();
        let mut selected = Vec::new();
        for (actor, at) in monsters {
            if !self.contains_reset(at) {
                continue;
            }
            if selected.len() == 100 || !cells.insert(at) || selected.contains(&actor) {
                return None;
            }
            selected.push(actor);
        }
        Some(selected)
    }

    /// Crystal svargrond_arena.lua:193-205,249-256: every other player in
    /// the spectator rectangle blocks admission, including unregistered players.
    pub fn occupied_by_other<T: PartialEq>(
        self,
        entrant: &T,
        occupants: impl IntoIterator<Item = (T, [i32; 3])>,
    ) -> bool {
        occupants
            .into_iter()
            .any(|(player, at)| &player != entrant && self.contains_occupant(at))
    }

    /// Occupancy uses the source's 5-tile spectator radius, not its smaller reset box.
    pub fn contains_occupant(self, at: [i32; 3]) -> bool {
        at[2] == self.center[2]
            && at[0].abs_diff(self.center[0]) <= 5
            && at[1].abs_diff(self.center[1]) <= 5
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PitOpening {
    pub position: [i32; 3],
    pub remove_item: u16,
    pub portal_item: u16,
    pub portal_action: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeathPlan {
    pub next_pit: u8,
    pub opening: Option<PitOpening>,
    pub speech: &'static str,
}

/// Preserve donor tier membership: it does not require the creature's array
/// index to equal PitDoor. Last-hit or party membership alone grants no credit.
/// A missing pillar suppresses portal creation but not progress/speech (kill:22-38).
pub fn death_plan(
    tier: Tier,
    pit: Pit,
    creature: &str,
    most_damage_credit: bool,
    pillar_present: bool,
) -> Option<DeathPlan> {
    if !most_damage_credit || !tier.creatures().contains(&creature) {
        return None;
    }
    Some(DeathPlan {
        next_pit: pit.0 + 1,
        opening: pillar_present.then_some(PitOpening {
            position: pit.layout().pillar,
            remove_item: 1841,
            portal_item: 5022,
            portal_action: 25200,
        }),
        speech: "Victory! Head through the new teleporter into the next room.",
    })
}

// Crystal data-global/lib/quests/svargrond_arena.lua:124-135.
const GREENHORN: [&str; 10] = [
    "oteryn:creature.frostfur",
    "oteryn:creature.bloodpaw",
    "oteryn:creature.bovinus",
    "oteryn:creature.achad",
    "oteryn:creature.colerian_the_barbarian",
    "oteryn:creature.the_hairy_one",
    "oteryn:creature.axeitus_headbanger",
    "oteryn:creature.rocky",
    "oteryn:creature.cursed_gladiator",
    "oteryn:creature.orcus_the_cruel",
];

// Crystal data-global/lib/quests/svargrond_arena.lua:142-153.
const SCRAPPER: [&str; 10] = [
    "oteryn:creature.avalanche",
    "oteryn:creature.kreebosh_the_exile",
    "oteryn:creature.the_dark_dancer",
    "oteryn:creature.the_hag",
    "oteryn:creature.slim",
    "oteryn:creature.grimgor_guteater",
    "oteryn:creature.drasilla",
    "oteryn:creature.spirit_of_earth",
    "oteryn:creature.spirit_of_water",
    "oteryn:creature.spirit_of_fire",
];

// Crystal data-global/lib/quests/svargrond_arena.lua:160-171.
const WARLORD: [&str; 10] = [
    "oteryn:creature.webster",
    "oteryn:creature.darakan_the_executioner",
    "oteryn:creature.norgle_glacierbeard",
    "oteryn:creature.the_pit_lord",
    "oteryn:creature.svoren_the_mad",
    "oteryn:creature.the_masked_marauder",
    "oteryn:creature.gnorre_chyllson",
    "oteryn:creature.fallen_mooh_tah_master_ghar",
    "oteryn:creature.deathbringer",
    "oteryn:creature.the_obliverator",
];
