//! Monk Harmony and Serene in the runtime actor (SPELL-D8 H-2,
//! `docs/architecture/OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1.md` §8.2;
//! rules from `docs/architecture/OTERYN_SPELL_NATIVE_BEHAVIOURS_CANDIDATE_V1.md` §A.2).
//!
//! A [`MonkState`] is the live copy a runtime actor holds: Harmony 0..5 and the remaining forced
//! Serene time, both loaded from values the caller supplies (owner decision Q1=b, 2026-09-29: the
//! remaining forced time is durable, as in Canary), and the Serene flag, which is never stored and
//! is evaluated again at every initialization. The owner runs the
//! Serene evaluation when it makes the actor playable ([`MonkState::initialize`]) and then every
//! 1000 ms ([`MonkState::tick`]); no command is accepted before the first. Builders, spenders and
//! Focus spells change Harmony at the cast's PRIMARY COMMIT; a failed cast calls none of them.
//!
//! The party comes from a [`SereneWorld`]. No party service exists, so the live adapter is
//! [`SoloParty`] and a solo monk is always Serene (§A.2 step 6). No stance owner exists either, so
//! the owner passes no virtue (the interim rule of §4); the durable field, virtues, the death reset
//! and the Wheel's Ascetic points belong to other children.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

use oteryn_simulation_determinism::SemanticTimeMicros;

use super::Vocation;
use super::chain::ChainCreature;
use super::party::SoloParty;

/// Most Harmony a monk holds (§8.2: 0..5).
pub(crate) const MAX_HARMONY: u8 = 5;

/// Time between two Serene evaluations (Canary player think, §8.2).
pub(crate) const SERENE_EVALUATION_MICROS: u64 = 1_000_000;

/// How long Focus Serenity forces Serene (Canary `focus_serenity.lua`, §8.2).
pub(crate) const FOCUS_SERENITY_MICROS: u64 = 7_000_000;

/// Adjacent creatures that end Serene when a party member is visible (§A.2 step 6, news 8944).
pub(crate) const SERENE_CROWD: u32 = 8;

/// Denominator of a [`HarmonyMultiplier`]: percent (100) x the 0.005-per-level step (1000) x the
/// half steps of the virtue factor (2).
pub(crate) const MULTIPLIER_DENOMINATOR: u64 = 200_000;

/// World facts the Serene evaluation reads.
pub(crate) trait SereneWorld {
    fn monk(&self) -> &ChainCreature;
    /// The monk's party (the monk included); `None` without a party.
    fn party(&self) -> Option<&[ChainCreature]>;
    /// Creatures adjacent to the monk that count against Serene (which ones is §A.2 Q6).
    fn adjacent_creatures(&self) -> u32;
}

/// The live adapter while no party service exists: every monk is solo, so always Serene.
impl SereneWorld for SoloParty {
    fn monk(&self) -> &ChainCreature {
        &self.0
    }

    fn party(&self) -> Option<&[ChainCreature]> {
        None
    }

    fn adjacent_creatures(&self) -> u32 {
        0
    }
}

/// The §A.2 step 6 rule without a forced state: Serene unless another party member is visible
/// (same floor, dx in [-8, 9], dy in [-6, 7]) and [`SERENE_CROWD`] or more creatures are adjacent.
fn serene_by_rule(world: &dyn SereneWorld) -> bool {
    let Some(party) = world.party() else {
        return true;
    };
    let monk = world.monk();
    let visible = party.iter().any(|member| {
        let dx = i64::from(member.position.x) - i64::from(monk.position.x);
        let dy = i64::from(member.position.y) - i64::from(monk.position.y);
        member.id != monk.id
            && member.position.floor == monk.position.floor
            && (-8..=9).contains(&dx)
            && (-6..=7).contains(&dy)
    });
    !(visible && world.adjacent_creatures() >= SERENE_CROWD)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonkStateError {
    /// A supplied Harmony outside 0..5, or not 0 for another vocation: corrupt Character state,
    /// and the load fails closed (§8.2).
    CorruptHarmony {
        vocation: Vocation,
        value: u8,
    },
    /// A supplied remaining forced Serene above 7000 ms, or not 0 for another vocation.
    CorruptSereneForced {
        vocation: Vocation,
        micros: u64,
    },
    /// The owner has not run the initialization evaluation since the actor became playable.
    NotInitialized,
    TimeOverflow,
}

impl Display for MonkStateError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::CorruptHarmony { vocation, value } => {
                write!(formatter, "harmony {value} is not valid for {vocation:?}")
            }
            Self::CorruptSereneForced { vocation, micros } => {
                write!(
                    formatter,
                    "a forced serene of {micros} us is not valid for {vocation:?}"
                )
            }
            Self::NotInitialized => {
                formatter.write_str("the actor has not had its Serene initialization evaluation")
            }
            Self::TimeOverflow => formatter.write_str("serene time overflow"),
        }
    }
}

impl Error for MonkStateError {}

/// A monk runtime actor's Harmony and Serene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MonkState {
    harmony: u8,
    serene: bool,
    serene_forced_until: Option<SemanticTimeMicros>,
    /// Loaded remaining forced time; the first initialization turns it into `serene_forced_until`.
    loaded_forced_micros: u64,
    /// When the next periodic evaluation is due; `None` until the initialization evaluation, and
    /// again once the actor is detached, so no command is accepted meanwhile.
    next_evaluation: Option<SemanticTimeMicros>,
}

impl MonkState {
    /// The state of a new runtime actor from the supplied Harmony and remaining forced Serene
    /// (0 for none). `None` for a vocation other than monk, whose values must both be 0. The
    /// forced time runs from the first [`Self::initialize`]; no command is accepted before it.
    pub(crate) fn load(
        vocation: Vocation,
        harmony: u8,
        serene_forced_micros: u64,
    ) -> Result<Option<Self>, MonkStateError> {
        let monk = matches!(vocation, Vocation::Monk | Vocation::ExaltedMonk);
        if harmony > if monk { MAX_HARMONY } else { 0 } {
            return Err(MonkStateError::CorruptHarmony {
                vocation,
                value: harmony,
            });
        }
        if serene_forced_micros > if monk { FOCUS_SERENITY_MICROS } else { 0 } {
            return Err(MonkStateError::CorruptSereneForced {
                vocation,
                micros: serene_forced_micros,
            });
        }
        Ok(monk.then_some(Self {
            harmony,
            serene: false,
            serene_forced_until: None,
            loaded_forced_micros: serene_forced_micros,
            next_evaluation: None,
        }))
    }

    /// The initialization evaluation, in the owner step that makes the actor playable: a fresh
    /// admission, respawn, a same-GameSession reconnect or FND-04B §21 recovery. It evaluates
    /// Serene and schedules the next evaluation 1000 ms later. A kept forced time still holds; a
    /// loaded remaining time starts at the first initialization.
    pub(crate) fn initialize(
        &mut self,
        now: SemanticTimeMicros,
        world: &dyn SereneWorld,
    ) -> Result<(), MonkStateError> {
        let next = now
            .checked_add(SERENE_EVALUATION_MICROS)
            .map_err(|_| MonkStateError::TimeOverflow)?;
        if self.loaded_forced_micros > 0 {
            let until = now
                .checked_add(self.loaded_forced_micros)
                .map_err(|_| MonkStateError::TimeOverflow)?;
            self.serene_forced_until = Some(until);
            self.loaded_forced_micros = 0;
        }
        self.evaluate(now, world);
        self.next_evaluation = Some(next);
        Ok(())
    }

    /// The session left the actor (a reconnect or recovery is pending). Harmony and the forced
    /// time are kept; commands wait for the next [`Self::initialize`].
    pub(crate) fn detach(&mut self) {
        self.next_evaluation = None;
    }

    /// Whether the owner may accept a command from the actor.
    pub(crate) fn accept_command(&self) -> Result<(), MonkStateError> {
        self.next_evaluation
            .map(|_| ())
            .ok_or(MonkStateError::NotInitialized)
    }

    /// The periodic evaluation; nothing happens before it is due. Returns whether Serene changed,
    /// which the owner publishes.
    pub(crate) fn tick(
        &mut self,
        now: SemanticTimeMicros,
        world: &dyn SereneWorld,
    ) -> Result<bool, MonkStateError> {
        let due = self.next_evaluation.ok_or(MonkStateError::NotInitialized)?;
        if now < due {
            return Ok(false);
        }
        let next = now
            .checked_add(SERENE_EVALUATION_MICROS)
            .map_err(|_| MonkStateError::TimeOverflow)?;
        let before = self.serene;
        self.evaluate(now, world);
        self.next_evaluation = Some(next);
        Ok(self.serene != before)
    }

    /// While the owner time is before the forced time, Serene stays true; after it the forced time
    /// is gone and the rule decides.
    fn evaluate(&mut self, now: SemanticTimeMicros, world: &dyn SereneWorld) {
        if self.serene_forced_until.is_some_and(|until| now < until) {
            self.serene = true;
            return;
        }
        self.serene_forced_until = None;
        self.serene = serene_by_rule(world);
    }

    /// The live value, which the actor-end and death writes of H-1 read.
    pub(crate) fn harmony(&self) -> u8 {
        self.harmony
    }

    pub(crate) fn serene(&self) -> bool {
        self.serene
    }

    pub(crate) fn serene_forced_until(&self) -> Option<SemanticTimeMicros> {
        self.serene_forced_until
    }

    /// The forced Serene time left at `now`, which the durable writes of H-1 read (0 for none).
    pub(crate) fn serene_forced_remaining(&self, now: SemanticTimeMicros) -> u64 {
        match self.serene_forced_until {
            Some(until) => until.elapsed_since(now).unwrap_or(0),
            None => self.loaded_forced_micros,
        }
    }

    /// The multiplier of a spender cast from the current charges and Serene (§A.2 step 3b), read
    /// before the commit; reading it changes nothing.
    pub(crate) fn spender_multiplier(
        &self,
        level: u32,
        virtue_of_harmony: bool,
    ) -> Result<HarmonyMultiplier, MonkStateError> {
        self.accept_command()?;
        Ok(
            HarmonyMultiplier::new(level, self.harmony, virtue_of_harmony, self.serene)
                .unwrap_or(HarmonyMultiplier::ONE),
        )
    }

    /// A builder's PRIMARY COMMIT (§A.2 step 4): +1 Harmony up to 5. Returns the charges gained
    /// (0 at 5, when no Virtue Healing happens either).
    pub(crate) fn commit_builder(&mut self) -> Result<u8, MonkStateError> {
        self.accept_command()?;
        let gained = u8::from(self.harmony < MAX_HARMONY);
        self.harmony += gained;
        Ok(gained)
    }

    /// A spender's PRIMARY COMMIT (§A.2 step 3c): Harmony becomes 0. Returns the charges spent.
    pub(crate) fn commit_spender(&mut self) -> Result<u8, MonkStateError> {
        self.accept_command()?;
        Ok(std::mem::take(&mut self.harmony))
    }

    /// A Focus spell's fill (§A.2 step 9): Harmony becomes 5. Returns the charges gained.
    pub(crate) fn commit_fill(&mut self) -> Result<u8, MonkStateError> {
        self.accept_command()?;
        let gained = MAX_HARMONY - self.harmony;
        self.harmony = MAX_HARMONY;
        Ok(gained)
    }

    /// Focus Serenity's PRIMARY COMMIT (§8.2): Serene is true and forced until `now` + 7000 ms.
    /// Nothing changes on an error.
    pub(crate) fn commit_focus_serenity(
        &mut self,
        now: SemanticTimeMicros,
    ) -> Result<(), MonkStateError> {
        self.accept_command()?;
        let until = now
            .checked_add(FOCUS_SERENITY_MICROS)
            .map_err(|_| MonkStateError::TimeOverflow)?;
        self.serene = true;
        self.serene_forced_until = Some(until);
        Ok(())
    }
}

/// The Harmony multiplier `m` of §A.2 step 2 as the exact fraction
/// `numerator / MULTIPLIER_DENOMINATOR`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HarmonyMultiplier {
    numerator: u64,
}

impl HarmonyMultiplier {
    pub(crate) const ONE: Self = Self {
        numerator: MULTIPLIER_DENOMINATOR,
    };

    /// `m = 1 + B * 2^(c - 1) / 100` for c = `charges` > 0, else 1, with
    /// `B = (7 + 0.005 * level) * V` percent and `V` = 1 without Virtue of Harmony, 1.5 with it,
    /// 2 with it while Serene (news 8944). `None` above [`MAX_HARMONY`] charges.
    pub(crate) fn new(
        level: u32,
        charges: u8,
        virtue_of_harmony: bool,
        serene: bool,
    ) -> Option<Self> {
        if charges > MAX_HARMONY {
            return None;
        }
        if charges == 0 {
            return Some(Self::ONE);
        }
        let half_steps = match (virtue_of_harmony, serene) {
            (false, _) => 2,
            (true, false) => 3,
            (true, true) => 4,
        };
        // B in thousandths of a percent, doubled: (7000 + 5 * level) * 2V.
        let bonus = (7_000 + 5 * u64::from(level)) * half_steps;
        Some(Self {
            numerator: MULTIPLIER_DENOMINATOR + bonus * (1 << (charges - 1)),
        })
    }

    pub(crate) fn numerator(self) -> u64 {
        self.numerator
    }

    /// `bound * m`, truncated toward zero (§A.2 step 3b), in exact integer arithmetic.
    pub(crate) fn apply(self, bound: i64) -> i64 {
        let scaled =
            i128::from(bound) * i128::from(self.numerator) / i128::from(MULTIPLIER_DENOMINATOR);
        i64::try_from(scaled).unwrap_or(if scaled < 0 { i64::MIN } else { i64::MAX })
    }
}
