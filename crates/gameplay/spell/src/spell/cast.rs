//! Player cast composition (`OTERYN_PLAYER_SPELL_CAST_WIRE_AND_VITALS_CONTRACT_CANDIDATE_V1` §9
//! step 2): one cast intent through the core checks, the Ability plan and the owner commit, with
//! the §5 anchor payment in the same mutation.
//!
//! [`PlayerSpellState`] is the runtime actor's own vitals and cooldowns (§4, SPELL-D2). The
//! Channel owner keeps it beside the actor; it is never durable and never belongs to a
//! GameSession. [`cast`] is pure: it returns the next state, and the owner stores that value in one
//! write, so a rejected cast changes nothing and an accepted one pays mana, soul and cooldowns
//! exactly with its heal (SPELL-D3).

use oteryn_gameplay_content::content::spell_data::authoring::spell_from_bundle;
use oteryn_protocol_oteryn::actor_spell::{MAX_SOUL, MAX_VITAL_POOL};
use serde_json::Value;

use super::chain::{ChainCreature, TilePosition};
use super::harmony::MonkState;
use super::party::SoloParty;
use super::{Cooldowns, SpellBook, Vocation};

macro_rules! starter {
    ($dir:literal) => {
        (
            include_str!(concat!(
                "../../../../../tools/content-schema/spell-authoring/samples/starter-bundles/",
                $dir,
                "/spell.json"
            )),
            include_str!(concat!(
                "../../../../../tools/content-schema/spell-authoring/samples/starter-bundles/",
                $dir,
                "/dependencies.json"
            )),
        )
    };
}

/// The V1 castable set of §6 among the candidate starter bundles: self heal and condition removal
/// (`exana pox`, `exura gran`, `exura`, `exura infir`). Every member is a plain self-target instant
/// on the `healing` cooldown group. The stock client bar casts index 1-4 only (`SPELL_SLOTS`), so
/// `ultimate_healing` and `wound_cleansing` (indices 5-6) and `bruise_bane` (sorts first, shifts
/// indices 1-3) wait for the client bar and the index-pinned tests.
const V1_BUNDLES: [(&str, &str); 4] = [
    starter!("instant-cure_poison"),
    starter!("instant-intense_healing"),
    starter!("instant-light_healing"),
    starter!("instant-magic_patch"),
];

/// The V1 spell book, in the canonical SPELL-D1 order. Any bundle that does not load fails the
/// whole book closed; nothing is skipped, so an index never shifts silently.
pub fn v1_spell_book() -> Result<SpellBook, String> {
    book_from_bundles(&V1_BUNDLES)
}

/// The loader behind [`v1_spell_book`], parameterized only so a test can inject a bundle that
/// does not load (behaviour-neutral for production, which passes `V1_BUNDLES`).
fn book_from_bundles(bundles: &[(&str, &str)]) -> Result<SpellBook, String> {
    let spells = bundles
        .iter()
        .map(|(spell, dependencies)| {
            let spell: Value = serde_json::from_str(spell).map_err(|error| error.to_string())?;
            let dependencies: Value =
                serde_json::from_str(dependencies).map_err(|error| error.to_string())?;
            spell_from_bundle(&spell, &dependencies).map_err(|error| error.to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;
    SpellBook::canonical(spells).map_err(|error| error.to_string())
}

/// The Character-owned inputs of a cast and of the vitals maxima (§4). The Character owner
/// supplies them; the client never does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterCastFacts {
    pub vocation: Vocation,
    pub level: u32,
    pub magic_level: u32,
    pub max_health: u32,
    pub max_mana: u32,
    pub max_soul: u32,
}

/// The runtime actor's vitals and cooldowns (§4). `revision` is the `ACTOR_VITALS` revision: 1
/// when the actor's state is created, one more for every committed cast and for every periodic
/// evaluation that changes Serene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSpellState {
    pub facts: CharacterCastFacts,
    pub(super) premium_current: bool,
    pub(super) premium_valid_until_micros: Option<u64>,
    pub health: u32,
    pub(super) field_attack_history: super::field_history::FieldAttackHistory,
    pub(super) mana: u32,
    /// FOOD-REGEN-1: milliseconds of fed time accumulated toward the next health and mana credit.
    /// Runtime-only; a fresh actor starts at zero.
    pub(super) regen_health_ms: u32,
    pub(super) regen_mana_ms: u32,
    pub(super) soul: u32,
    pub cooldowns: Cooldowns,
    pub(super) revision: u64,
    /// SPELL-D8 §8.2: a monk's live Harmony and Serene; `None` for every other vocation.
    pub monk: Option<MonkState>,
    pub conditions: oteryn_gameplay_foundation::foundation::condition::ConditionStore<String>,
    pub(super) base_speed: u32,
    pub(super) stance: Option<oteryn_gameplay_spell_rules::native_actor_states::StandardStance>,
    pub(super) stance_key: Option<String>,
    pub(super) pending_stance: Option<Box<super::stance_execution::PreparedStance>>,
    pub(super) owned_skills: Option<[u16; 7]>,
    /// ARMOR-MITIGATION-1: the worn armor the last committed equipment read installed.
    pub(super) equipped_armor: u32,
    pub(super) training: Option<super::mana_training::LiveManaTraining>,
    pub(super) training_checkpoint: Option<Box<super::mana_training::RetainedTrainingCheckpoint>>,
    pub(super) training_payment: Option<super::mana_training::TrainingPaymentWitness>,
}

/// The Serene world of a runtime actor. No party service exists, so every monk is solo (§4
/// interim rule) and always Serene by the rule; a solo evaluation never reads the creature.
fn solo() -> SoloParty {
    SoloParty(ChainCreature {
        id: 0,
        actor: String::new(),
        position: TilePosition {
            x: 0,
            y: 0,
            floor: 0,
        },
    })
}

impl PlayerSpellState {
    /// A new runtime actor starts at its maxima (SPELL-D2), with the durable Harmony and remaining
    /// forced Serene time the Character owner loaded (§8.2; both 0 for any other vocation).
    /// Maxima above the SPELL-D8 wire bounds, and corrupt Harmony values, are refused.
    pub fn new(facts: CharacterCastFacts, harmony: u8, serene_forced_micros: u64) -> Option<Self> {
        let monk = MonkState::load(facts.vocation, harmony, serene_forced_micros).ok()?;
        // Canary's pinned vocations.xml assigns base speed 110 to every class.
        // The accepted SPEED-1 curve is class base plus (level - 1).
        let base_speed = crate::movement::speed::player_base_speed(facts.level)?;
        (facts.max_health <= MAX_VITAL_POOL
            && facts.max_mana <= MAX_VITAL_POOL
            && facts.max_soul <= MAX_SOUL)
            .then(|| Self {
                facts,
                health: facts.max_health,
                mana: facts.max_mana,
                regen_health_ms: 0,
                regen_mana_ms: 0,
                premium_current: false,
                premium_valid_until_micros: None,
                soul: facts.max_soul,
                cooldowns: Cooldowns::default(),
                revision: 1,
                monk,
                conditions: oteryn_gameplay_foundation::foundation::condition::ConditionStore::new(
                ),
                base_speed,
                stance: None,
                stance_key: None,
                pending_stance: None,
                owned_skills: None,
                equipped_armor: 0,
                training: None,
                training_payment: None,
                field_attack_history: super::field_history::FieldAttackHistory::default(),
                training_checkpoint: None,
            })
    }
}

mod equipped_armor;
mod gate;
mod level_up_vitals;
mod live_skill;
mod prepare;
mod state;
mod vocationless_vitals;

pub use gate::{
    CastContext, admit_condition_gates, cast, condition_gate, disposition, store_condition_gate,
};
pub use live_skill::SwingSkills;
pub use prepare::{
    PaidFocusOwnerCast, PaidNativeCallbackCast, PaidNativeCast, PaidOrdinaryCast,
    PaidSourceSelfCast, prepare_conjure_owner_cast_with_caster,
    prepare_direct_acquisition_owner_cast_with_caster, prepare_focus_owner_cast_with_caster,
    prepare_locate_name_failure_owner_cast_with_caster,
    prepare_named_player_failure_owner_cast_with_caster,
    prepare_native_callback_owner_cast_with_carried_item,
    prepare_native_callback_owner_cast_with_caster, prepare_native_owner_cast,
    prepare_native_owner_cast_with_caster, prepare_ordinary_owner_cast_with_caster,
    prepare_source_self_cast_with_caster, resolve_ordinary_due_effects,
};

#[cfg(test)]
mod gate_tests;
#[cfg(test)]
mod tests;
