//! Declarative creature authoring profiles (OTERYN_WORLD_PROJECT_V2_CREATURE_ADMISSION_V1 §5).
//!
//! These profiles mirror the monster authoring format for the admitted Canary monsters. They are
//! candidate-only: no runtime path interprets them, and they never override a Reference record.
//! Percentages are exact parts per million (monster authoring D1: `ppm = percent * 10_000`).

use serde::{Deserialize, Serialize};

use super::{
    ProjectError, ProjectEvidenceLimits, ProjectV2DefinitionRef, ProjectV2ExactRatio,
    ProjectV2Family, validate_v2_ratio, validate_v2_source_text,
};

pub const PROJECT_V2_PPM_SCALE: u32 = 1_000_000;

// ---------------------------------------------------------------------------------------------
// Creature details

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2CreatureDetails {
    pub display_name: String,
    /// `Some("")` is an explicit empty article.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub article: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plural: Option<String>,
    pub inspection: String,
    pub defense: u32,
    pub critical_chance_ppm: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub condition_immunities: Vec<String>,
    pub flags: ProjectV2CreatureFlags,
    pub summoning: ProjectV2CreatureSummoning,
    pub system_eligibility: ProjectV2SystemEligibility,
    pub spawn_eligibility: ProjectV2SpawnEligibility,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bestiary: Option<ProjectV2BestiaryDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bosstiary: Option<ProjectV2BosstiaryDetails>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub corpse_item: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub soul_core_item: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death_residue: Option<ProjectV2DeathResidue>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encyclopedia_document: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_reflection: Vec<ProjectV2DamageResponse>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub healing_from_damage: Vec<ProjectV2DamageResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reward_encounter: Option<ProjectV2DefinitionRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2CreatureFlags {
    pub attackable: bool,
    pub illusionable: bool,
    pub health_hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2CreatureSummoning {
    pub summonable: bool,
    pub convinceable: bool,
    /// Present exactly when the creature is summonable or convinceable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mana_cost: Option<u32>,
    pub is_familiar: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2SystemEligibility {
    pub prey: bool,
    pub exclusive_prey: bool,
    pub forge: bool,
    pub reward_boss: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2SpawnPeriod {
    All,
    Day,
    Night,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2SpawnEligibility {
    pub period: ProjectV2SpawnPeriod,
    pub ignore_period_underground: bool,
    pub blocked_by_nearby_players: bool,
}

/// Bestiary fields beyond the base `ProjectV2BestiaryProfile`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2BestiaryDetails {
    pub class: String,
    pub taxonomy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stars: Option<u8>,
    /// Original editorial location text, not world spawn coordinates.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locations: Option<String>,
}

/// Bosstiary with the points of each stage (the base profile carries a single value).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2BosstiaryDetails {
    pub category: String,
    pub prowess_kills: u32,
    pub expertise_kills: u32,
    pub mastery_kills: u32,
    pub prowess_points: u16,
    pub expertise_points: u16,
    pub mastery_points: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2DeathResidue {
    pub item: ProjectV2DefinitionRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fluid_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2DamageResponse {
    pub damage_type: String,
    pub percent: ProjectV2ExactRatio,
}

// ---------------------------------------------------------------------------------------------
// Behavior

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2BehaviorAuthoring {
    pub movement: ProjectV2Movement,
    pub targeting: ProjectV2Targeting,
    /// Authored order is the evaluation order; never sorted.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attacks: Vec<ProjectV2AbilitySchedule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub defenses: Vec<ProjectV2AbilitySchedule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voices: Option<ProjectV2Voices>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summons: Option<ProjectV2Summons>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub periodic_audio: Option<ProjectV2PeriodicAudio>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub faction: Option<ProjectV2Faction>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub event_bindings: Vec<ProjectV2BehaviorEventBinding>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Movement {
    pub can_walk: bool,
    pub pass_through: bool,
    pub pushable: bool,
    pub push_items: bool,
    pub push_creatures: bool,
    pub walks_on_energy: bool,
    pub walks_on_fire: bool,
    pub walks_on_poison: bool,
    /// Idle wandering around the spawn point, as NPCs do; requires `can_walk`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wander: Option<ProjectV2Wander>,
}

/// NPC admission §4: the source walk interval and radius of an idle, non-hostile walker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Wander {
    /// Milliseconds between steps; positive.
    pub interval_ms: u64,
    /// Maximum distance from the spawn point in tiles.
    pub radius_tiles: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Targeting {
    pub hostile: bool,
    pub can_target: bool,
    pub sense_invisible: bool,
    pub target_distance_tiles: u16,
    pub static_attack_chance_ppm: u32,
    pub flee_health: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub change_target: Option<ProjectV2ChangeTarget>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy_weights: Option<ProjectV2StrategyWeights>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2ChangeTarget {
    /// 0 disables timed target changes.
    pub interval_ms: u64,
    pub chance_ppm: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2StrategyWeights {
    pub nearest: u32,
    pub damage: u32,
    pub health: u32,
    pub random: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AbilitySchedule {
    pub ability: ProjectV2DefinitionRef,
    pub interval_ms: u64,
    pub chance_ppm: u32,
    /// This creature's magnitude for `caster_magnitude` formulas (monster authoring D11).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub magnitude: Option<ProjectV2Magnitude>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range_tiles: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Magnitude {
    pub minimum: u64,
    pub maximum: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Voices {
    pub interval_ms: u64,
    pub chance_ppm: u32,
    pub entries: Vec<ProjectV2Voice>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2SpeechMode {
    Say,
    Yell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Voice {
    pub text: String,
    pub mode: ProjectV2SpeechMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Summons {
    pub max_summons: u32,
    pub entries: Vec<ProjectV2SummonEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2SummonEntry {
    pub creature: ProjectV2DefinitionRef,
    pub interval_ms: u64,
    pub chance_ppm: u32,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2PeriodicAudio {
    pub interval_ms: u64,
    pub chance_ppm: u32,
    pub cue_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Faction {
    pub faction: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub enemy_factions: Vec<String>,
    pub prefer_player: bool,
    pub prefer_master: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProjectV2BehaviorEvent {
    Think,
    Appear,
    Disappear,
    Move,
    Say,
    AttackedByPlayer,
    Spawn,
    Death,
    DropLoot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2BehaviorEventBinding {
    pub event: ProjectV2BehaviorEvent,
    pub interaction: ProjectV2DefinitionRef,
}

// ---------------------------------------------------------------------------------------------
// Presentation

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2PresentationAuthoring {
    /// Source asset binding token; unbound until an asset slice admits it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub asset_binding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection: Option<ProjectV2AppearanceSelection>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub palette_bindings: Vec<ProjectV2SlotBinding<ProjectV2PaletteSlot>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attachment_bindings: Vec<ProjectV2SlotBinding<ProjectV2AttachmentSlot>>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visual_effect_bindings: Vec<ProjectV2SlotBinding<ProjectV2VisualEffectSlot>>,
    pub light_level: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub light_color_binding: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub audio_bindings: Vec<ProjectV2AudioBinding>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant_label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status_marker: Option<ProjectV2StatusMarker>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2AppearanceSelection {
    OwnerFamiliarLook,
    Invisible,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2SlotBinding<Slot> {
    pub slot: Slot,
    pub asset_binding: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProjectV2PaletteSlot {
    Head,
    Body,
    Legs,
    Feet,
    MountHead,
    MountBody,
    MountLegs,
    MountFeet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProjectV2AttachmentSlot {
    Addon,
    Mount,
    Familiar,
    Wing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProjectV2VisualEffectSlot {
    Aura,
    Effect,
    Shader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProjectV2AudioEvent {
    Cast,
    Impact,
    Death,
    Periodic,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AudioBinding {
    pub event: ProjectV2AudioEvent,
    pub cue_id: String,
    pub asset_binding: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2StatusMarker {
    None,
    Yellow,
    Green,
    White,
    Red,
    Black,
    Orange,
}

// ---------------------------------------------------------------------------------------------
// Ability details, inline effects, Effect and Formula profiles

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2AbilityKind {
    Melee,
    Spell,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AbilityDetails {
    pub kind: ProjectV2AbilityKind,
    pub range_tiles: u16,
    pub needs_target: bool,
    pub needs_direction: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub area: Option<ProjectV2AbilityArea>,
    /// Authored effect order, executable Reference effects and inline candidate effects together.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub effects: Vec<ProjectV2AbilityEffect>,
    /// Each cast runs one variant picked uniformly; exclusive with effects, area and chain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_requirement: Option<ProjectV2PathRequirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain: Option<ProjectV2Chain>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cast_cue: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact_cue: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "shape", deny_unknown_fields)]
pub enum ProjectV2AbilityArea {
    Beam {
        length_tiles: u16,
        spread_tiles: u16,
    },
    Circle {
        radius_tiles: u16,
    },
    /// Rows of `.` (not hit), `x` (hit), `c` (centre not hit), `C` (centre hit), authored facing
    /// north; `diagonal` faces north-west.
    Matrix {
        north: Vec<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        diagonal: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2PathRequirement {
    pub max_search_tiles: u16,
    pub clear_sight: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Chain {
    pub max_targets: u16,
    pub range_tiles: u16,
    pub backtracking: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chain_asset_binding: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "effect", deny_unknown_fields)]
pub enum ProjectV2AbilityEffect {
    /// A damage or heal Reference Effect of this Ability.
    Executable(ProjectV2DefinitionRef),
    /// An effect without executable semantics yet (admission §4); candidate-only.
    Inline(Box<ProjectV2InlineEffect>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2InlineEffect {
    /// Stable key of the authored effect, kept for provenance and reimport.
    pub key: String,
    pub operation: ProjectV2InlineEffectOperation,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<ProjectV2EffectPresentation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", deny_unknown_fields)]
pub enum ProjectV2InlineEffectOperation {
    Condition {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        duration_ms: Option<u64>,
        condition: ProjectV2Condition,
    },
    AppearanceTransform {
        duration_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        creature: Option<ProjectV2DefinitionRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item: Option<ProjectV2DefinitionRef>,
    },
    CreateItem {
        item: ProjectV2DefinitionRef,
    },
    PresentationOnly,
    RemoveCondition {
        condition: String,
    },
    RemoveItems {
        items: Vec<ProjectV2DefinitionRef>,
        selection: ProjectV2RemoveItemsSelection,
    },
    SummonCreature {
        creatures: Vec<ProjectV2DefinitionRef>,
        count_mode: ProjectV2SummonCountMode,
        count: u32,
        only_below_summons: u32,
        owned: bool,
        max_offset_tiles: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2RemoveItemsSelection {
    FirstListedPerTile,
    TopItemFirstTile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2SummonCountMode {
    FillToLimit,
    Fixed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Condition {
    pub condition_type: String,
    pub lifetime: ProjectV2ConditionLifetime,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub damage_over_time: Option<ProjectV2DamageOverTime>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speed_formula: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attribute_modifiers: Vec<ProjectV2AttributeModifier>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2ConditionLifetime {
    FixedDuration,
    DamageSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tick_profile", deny_unknown_fields)]
pub enum ProjectV2DamageOverTime {
    Fixed {
        first_tick: ProjectV2FirstTick,
        ticks: Vec<ProjectV2FixedTicks>,
    },
    Decreasing {
        first_tick: ProjectV2FirstTick,
        total_minimum: u64,
        total_maximum: u64,
        tick_interval_ms: u64,
        /// `None` is the automatic first tick; `Some(n)` a fixed first tick of `n`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        initial_tick_amount: Option<u64>,
    },
    Geometric {
        first_tick: ProjectV2FirstTick,
        base_minimum: u64,
        base_maximum: u64,
        factor: ProjectV2ExactRatio,
        tick_counts: Vec<u32>,
        tick_interval_ms: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2FirstTick {
    AfterInterval,
    Immediate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2FixedTicks {
    pub count: u32,
    pub interval_ms: u64,
    pub amount: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2AttributeModifierMode {
    PercentOfBase,
    Add,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AttributeModifier {
    pub attribute: String,
    pub mode: ProjectV2AttributeModifierMode,
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EffectPresentation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub impact_asset_binding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projectile_asset_binding: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_asset_binding: Option<String>,
}

/// Details of a damage or heal Reference Effect; its family and formula stay on the record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EffectAuthoring {
    pub damage_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mitigated_by: Vec<ProjectV2Mitigation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub affects: Option<ProjectV2EffectAffects>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presentation: Option<ProjectV2EffectPresentation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProjectV2Mitigation {
    Armor,
    Shield,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2AffectsKind {
    MasterlessMonsters,
    NonPlayerSide,
    PlayerSide,
    NamedCreatures,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EffectAffects {
    pub kind: ProjectV2AffectsKind,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creatures: Vec<ProjectV2DefinitionRef>,
    pub top_creature_only: bool,
    pub excludes_caster_name: bool,
    pub includes_caster: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "formula", deny_unknown_fields)]
pub enum ProjectV2FormulaAuthoring {
    Range {
        minimum: u64,
        maximum: u64,
    },
    MeleeAttackSkill {
        attack: u32,
        skill: u32,
    },
    SpeedModifier {
        minimum_multiplier: ProjectV2ExactRatio,
        minimum_offset: i64,
        maximum_multiplier: ProjectV2ExactRatio,
        maximum_offset: i64,
    },
    /// The magnitude comes from the casting creature's schedule entry.
    CasterMagnitude,
}

/// Loot entry fields the Reference Loot entry lacks, aligned with its entries by index.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2LootAuthoring {
    pub entries: Vec<ProjectV2LootEntryDetails>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2LootEntryDetails {
    pub skip_later_same_item_after_success: bool,
}

// ---------------------------------------------------------------------------------------------
// Validation

fn ppm(value: u32, error: &'static str) -> Result<(), ProjectError> {
    if value > PROJECT_V2_PPM_SCALE {
        return Err(ProjectError::InvalidProject(error));
    }
    Ok(())
}

fn positive(value: u64, error: &'static str) -> Result<(), ProjectError> {
    if value == 0 {
        return Err(ProjectError::InvalidProject(error));
    }
    Ok(())
}

fn family(
    reference: &ProjectV2DefinitionRef,
    expected: ProjectV2Family,
    error: &'static str,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
) -> Result<(), ProjectError> {
    if reference.family != expected {
        return Err(ProjectError::InvalidProject(error));
    }
    require_ref(reference)
}

/// Canonical lowercase snake key (`^[a-z][a-z0-9_]*$`): damage, condition and fluid types.
fn snake(value: &str, error: &'static str) -> Result<(), ProjectError> {
    let mut bytes = value.bytes();
    if !bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return Err(ProjectError::InvalidProject(error));
    }
    Ok(())
}

/// Asset binding tokens are namespaced keys (`namespace:local`) of printable ASCII.
fn asset_binding(value: &str, limits: ProjectEvidenceLimits) -> Result<(), ProjectError> {
    validate_v2_source_text("v2 asset binding", value, limits)?;
    let namespaced = value
        .split_once(':')
        .is_some_and(|(namespace, local)| !namespace.is_empty() && !local.is_empty());
    if !namespaced || !value.bytes().all(|byte| byte.is_ascii_graphic()) {
        return Err(ProjectError::InvalidProject("invalid v2 asset binding"));
    }
    Ok(())
}

fn damage_responses(
    values: &[ProjectV2DamageResponse],
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    limits.check(
        "v2 damage responses",
        values.len(),
        limits.max_reference_records,
    )?;
    if values
        .windows(2)
        .any(|pair| pair[0].damage_type >= pair[1].damage_type)
    {
        return Err(ProjectError::InvalidProject(
            "v2 damage responses are not sorted and unique",
        ));
    }
    for value in values {
        snake(&value.damage_type, "invalid v2 damage type")?;
        validate_v2_ratio(value.percent, "v2 damage response percent is not canonical")?;
        if value.percent.numerator < 0 {
            return Err(ProjectError::InvalidProject(
                "v2 damage response percent is negative",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_creature_details(
    details: &ProjectV2CreatureDetails,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    validate_v2_source_text("v2 Creature display name", &details.display_name, limits)?;
    if let Some(article) = &details.article {
        limits.check(
            "v2 Creature article",
            article.len(),
            limits.max_string_bytes,
        )?;
        if !article.is_empty() {
            validate_v2_source_text("v2 Creature article", article, limits)?;
        }
    }
    if let Some(plural) = &details.plural {
        validate_v2_source_text("v2 Creature plural", plural, limits)?;
    }
    validate_v2_source_text("v2 Creature inspection", &details.inspection, limits)?;
    ppm(
        details.critical_chance_ppm,
        "v2 Creature critical chance exceeds 100%",
    )?;
    limits.check(
        "v2 Creature condition immunities",
        details.condition_immunities.len(),
        limits.max_reference_records,
    )?;
    if details
        .condition_immunities
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(ProjectError::InvalidProject(
            "v2 Creature condition immunities are not sorted and unique",
        ));
    }
    for condition in &details.condition_immunities {
        snake(condition, "invalid v2 condition type")?;
    }
    let summoning = details.summoning;
    if (summoning.summonable || summoning.convinceable) != summoning.mana_cost.is_some() {
        return Err(ProjectError::InvalidProject(
            "v2 Creature summon mana cost must be present exactly when summonable or convinceable",
        ));
    }
    if let Some(bestiary) = &details.bestiary {
        validate_v2_source_text("v2 Bestiary class", &bestiary.class, limits)?;
        validate_v2_source_text("v2 Bestiary taxonomy", &bestiary.taxonomy, limits)?;
        if bestiary.stars.is_some_and(|stars| stars > 5) {
            return Err(ProjectError::InvalidProject("v2 Bestiary stars exceed 5"));
        }
        if let Some(locations) = &bestiary.locations {
            validate_v2_source_text("v2 Bestiary locations", locations, limits)?;
        }
    }
    if let Some(bosstiary) = &details.bosstiary {
        validate_v2_source_text("v2 Bosstiary category", &bosstiary.category, limits)?;
        if bosstiary.prowess_kills == 0
            || bosstiary.prowess_kills >= bosstiary.expertise_kills
            || bosstiary.expertise_kills >= bosstiary.mastery_kills
        {
            return Err(ProjectError::InvalidProject(
                "v2 Bosstiary thresholds are invalid",
            ));
        }
    }
    for item in [&details.corpse_item, &details.soul_core_item]
        .into_iter()
        .flatten()
    {
        family(
            item,
            ProjectV2Family::Item,
            "v2 Creature item family mismatch",
            require_ref,
        )?;
    }
    if let Some(residue) = &details.death_residue {
        family(
            &residue.item,
            ProjectV2Family::Item,
            "v2 death residue family mismatch",
            require_ref,
        )?;
        if let Some(fluid) = &residue.fluid_type {
            snake(fluid, "invalid v2 fluid type")?;
        }
    }
    if let Some(document) = &details.encyclopedia_document {
        family(
            document,
            ProjectV2Family::Document,
            "v2 Creature encyclopedia family mismatch",
            require_ref,
        )?;
    }
    if let Some(encounter) = &details.reward_encounter {
        family(
            encounter,
            ProjectV2Family::Encounter,
            "v2 Creature reward encounter family mismatch",
            require_ref,
        )?;
    }
    damage_responses(&details.damage_reflection, limits)?;
    damage_responses(&details.healing_from_damage, limits)
}

fn schedules(
    values: &[ProjectV2AbilitySchedule],
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    limits.check(
        "v2 ability schedule",
        values.len(),
        limits.max_reference_records,
    )?;
    for value in values {
        family(
            &value.ability,
            ProjectV2Family::Ability,
            "v2 schedule Ability family mismatch",
            require_ref,
        )?;
        positive(value.interval_ms, "v2 schedule interval must be positive")?;
        ppm(value.chance_ppm, "v2 schedule chance exceeds 100%")?;
        if value
            .magnitude
            .is_some_and(|magnitude| magnitude.minimum > magnitude.maximum)
        {
            return Err(ProjectError::InvalidProject(
                "v2 schedule magnitude range is inverted",
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_behavior(
    behavior: &ProjectV2BehaviorAuthoring,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    if let Some(wander) = behavior.movement.wander {
        if !behavior.movement.can_walk || wander.interval_ms == 0 {
            return Err(ProjectError::InvalidProject(
                "v2 wander requires a walking creature and a positive interval",
            ));
        }
    }
    let targeting = behavior.targeting;
    ppm(
        targeting.static_attack_chance_ppm,
        "v2 static attack chance exceeds 100%",
    )?;
    if let Some(change) = targeting.change_target {
        ppm(change.chance_ppm, "v2 change target chance exceeds 100%")?;
    }
    schedules(&behavior.attacks, require_ref, limits)?;
    schedules(&behavior.defenses, require_ref, limits)?;
    if let Some(voices) = &behavior.voices {
        positive(voices.interval_ms, "v2 voice interval must be positive")?;
        ppm(voices.chance_ppm, "v2 voice chance exceeds 100%")?;
        limits.check(
            "v2 voices",
            voices.entries.len(),
            limits.max_reference_records,
        )?;
        if voices.entries.is_empty() {
            return Err(ProjectError::InvalidProject("v2 voices require an entry"));
        }
        for voice in &voices.entries {
            validate_v2_source_text("v2 voice text", &voice.text, limits)?;
        }
    }
    if let Some(summons) = &behavior.summons {
        if summons.max_summons == 0 || summons.entries.is_empty() {
            return Err(ProjectError::InvalidProject(
                "v2 summons require a positive limit and an entry",
            ));
        }
        limits.check(
            "v2 summons",
            summons.entries.len(),
            limits.max_reference_records,
        )?;
        for entry in &summons.entries {
            family(
                &entry.creature,
                ProjectV2Family::Creature,
                "v2 summon Creature family mismatch",
                require_ref,
            )?;
            positive(entry.interval_ms, "v2 summon interval must be positive")?;
            ppm(entry.chance_ppm, "v2 summon chance exceeds 100%")?;
            positive(u64::from(entry.count), "v2 summon count must be positive")?;
        }
    }
    if let Some(audio) = &behavior.periodic_audio {
        positive(
            audio.interval_ms,
            "v2 periodic audio interval must be positive",
        )?;
        ppm(audio.chance_ppm, "v2 periodic audio chance exceeds 100%")?;
        if audio.cue_ids.is_empty() || audio.cue_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(ProjectError::InvalidProject(
                "v2 periodic audio cues are empty, unsorted or duplicated",
            ));
        }
        for cue in &audio.cue_ids {
            validate_v2_source_text("v2 audio cue", cue, limits)?;
        }
    }
    if let Some(faction) = &behavior.faction {
        validate_v2_source_text("v2 faction", &faction.faction, limits)?;
        if faction
            .enemy_factions
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(ProjectError::InvalidProject(
                "v2 enemy factions are not sorted and unique",
            ));
        }
        for enemy in &faction.enemy_factions {
            validate_v2_source_text("v2 enemy faction", enemy, limits)?;
        }
    }
    if behavior
        .event_bindings
        .windows(2)
        .any(|pair| (pair[0].event, &pair[0].interaction) >= (pair[1].event, &pair[1].interaction))
    {
        return Err(ProjectError::InvalidProject(
            "v2 behavior event bindings are not sorted and unique",
        ));
    }
    for binding in &behavior.event_bindings {
        family(
            &binding.interaction,
            ProjectV2Family::Interaction,
            "v2 behavior event binding family mismatch",
            require_ref,
        )?;
    }
    Ok(())
}

fn slot_bindings<Slot: Ord + Copy>(
    values: &[ProjectV2SlotBinding<Slot>],
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    if values.windows(2).any(|pair| pair[0].slot >= pair[1].slot) {
        return Err(ProjectError::InvalidProject(
            "v2 presentation slots are not sorted and unique",
        ));
    }
    for value in values {
        asset_binding(&value.asset_binding, limits)?;
    }
    Ok(())
}

pub(super) fn validate_presentation(
    presentation: &ProjectV2PresentationAuthoring,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    match presentation.selection {
        Some(ProjectV2AppearanceSelection::Invisible) => {
            if presentation.asset_binding.is_some()
                || !presentation.palette_bindings.is_empty()
                || !presentation.attachment_bindings.is_empty()
            {
                return Err(ProjectError::InvalidProject(
                    "v2 invisible appearance forbids appearance bindings",
                ));
            }
        }
        _ => {
            if presentation.asset_binding.is_none() {
                return Err(ProjectError::InvalidProject(
                    "v2 visible appearance requires an asset binding",
                ));
            }
        }
    }
    if let Some(binding) = &presentation.asset_binding {
        asset_binding(binding, limits)?;
    }
    slot_bindings(&presentation.palette_bindings, limits)?;
    slot_bindings(&presentation.attachment_bindings, limits)?;
    slot_bindings(&presentation.visual_effect_bindings, limits)?;
    if let Some(color) = &presentation.light_color_binding {
        asset_binding(color, limits)?;
    }
    if presentation
        .audio_bindings
        .windows(2)
        .any(|pair| (pair[0].event, &pair[0].cue_id) >= (pair[1].event, &pair[1].cue_id))
    {
        return Err(ProjectError::InvalidProject(
            "v2 audio bindings are not sorted and unique",
        ));
    }
    for binding in &presentation.audio_bindings {
        validate_v2_source_text("v2 audio cue", &binding.cue_id, limits)?;
        asset_binding(&binding.asset_binding, limits)?;
    }
    if let Some(label) = &presentation.variant_label {
        validate_v2_source_text("v2 presentation variant label", label, limits)?;
    }
    Ok(())
}

fn area_matrix(rows: &[String]) -> Result<(), ProjectError> {
    let width = rows.first().map(String::len).unwrap_or(0);
    let centres = rows
        .iter()
        .flat_map(|row| row.bytes())
        .filter(|byte| matches!(byte, b'c' | b'C'))
        .count();
    if width == 0
        || centres != 1
        || rows
            .iter()
            .any(|row| row.len() != width || !row.bytes().all(|byte| b".xcC".contains(&byte)))
    {
        return Err(ProjectError::InvalidProject(
            "v2 area matrix must be rectangular with exactly one centre",
        ));
    }
    Ok(())
}

fn effect_presentation(
    presentation: &ProjectV2EffectPresentation,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    for binding in [
        &presentation.impact_asset_binding,
        &presentation.projectile_asset_binding,
        &presentation.path_asset_binding,
    ]
    .into_iter()
    .flatten()
    {
        asset_binding(binding, limits)?;
    }
    Ok(())
}

fn condition(
    value: &ProjectV2Condition,
    duration_ms: Option<u64>,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    snake(&value.condition_type, "invalid v2 condition type")?;
    match value.lifetime {
        ProjectV2ConditionLifetime::DamageSchedule => {
            if value.damage_over_time.is_none()
                || value.speed_formula.is_some()
                || duration_ms.is_some()
            {
                return Err(ProjectError::InvalidProject(
                    "v2 damage-schedule condition needs a schedule and no duration or speed formula",
                ));
            }
        }
        ProjectV2ConditionLifetime::FixedDuration => {
            if value.damage_over_time.is_some() || duration_ms.is_none_or(|duration| duration == 0)
            {
                return Err(ProjectError::InvalidProject(
                    "v2 fixed-duration condition needs a positive duration and no damage schedule",
                ));
            }
        }
    }
    if let Some(formula) = &value.speed_formula {
        family(
            formula,
            ProjectV2Family::Formula,
            "v2 condition speed formula family mismatch",
            require_ref,
        )?;
    }
    match &value.damage_over_time {
        Some(ProjectV2DamageOverTime::Fixed { ticks, .. }) => {
            if ticks.is_empty()
                || ticks
                    .iter()
                    .any(|tick| tick.count == 0 || tick.interval_ms == 0 || tick.amount == 0)
            {
                return Err(ProjectError::InvalidProject(
                    "v2 fixed damage ticks must be positive",
                ));
            }
        }
        Some(ProjectV2DamageOverTime::Decreasing {
            total_minimum,
            total_maximum,
            tick_interval_ms,
            initial_tick_amount,
            ..
        }) => {
            if *total_maximum == 0
                || total_minimum > total_maximum
                || *tick_interval_ms == 0
                || *initial_tick_amount == Some(0)
            {
                return Err(ProjectError::InvalidProject(
                    "v2 decreasing damage schedule is invalid",
                ));
            }
        }
        Some(ProjectV2DamageOverTime::Geometric {
            base_minimum,
            base_maximum,
            factor,
            tick_counts,
            tick_interval_ms,
            ..
        }) => {
            validate_v2_ratio(*factor, "v2 geometric factor is not canonical")?;
            if *base_minimum == 0
                || base_minimum > base_maximum
                || *tick_interval_ms == 0
                || tick_counts.is_empty()
                || tick_counts.contains(&0)
                || tick_counts.windows(2).any(|pair| pair[0] >= pair[1])
            {
                return Err(ProjectError::InvalidProject(
                    "v2 geometric damage schedule is invalid",
                ));
            }
        }
        None => {}
    }
    limits.check(
        "v2 attribute modifiers",
        value.attribute_modifiers.len(),
        limits.max_reference_records,
    )?;
    for modifier in &value.attribute_modifiers {
        snake(&modifier.attribute, "invalid v2 attribute")?;
    }
    Ok(())
}

fn inline_effect(
    effect: &ProjectV2InlineEffect,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    super::ProductionKey::new(&effect.key)?;
    if let Some(presentation) = &effect.presentation {
        effect_presentation(presentation, limits)?;
    }
    match &effect.operation {
        ProjectV2InlineEffectOperation::Condition {
            duration_ms,
            condition: value,
        } => condition(value, *duration_ms, require_ref, limits),
        ProjectV2InlineEffectOperation::AppearanceTransform {
            duration_ms,
            creature,
            item,
        } => {
            positive(
                *duration_ms,
                "v2 appearance transform duration must be positive",
            )?;
            match (creature, item) {
                (Some(creature), None) => family(
                    creature,
                    ProjectV2Family::Creature,
                    "v2 appearance transform family mismatch",
                    require_ref,
                ),
                (None, Some(item)) => family(
                    item,
                    ProjectV2Family::Item,
                    "v2 appearance transform family mismatch",
                    require_ref,
                ),
                _ => Err(ProjectError::InvalidProject(
                    "v2 appearance transform needs exactly one Creature or Item",
                )),
            }
        }
        ProjectV2InlineEffectOperation::CreateItem { item } => family(
            item,
            ProjectV2Family::Item,
            "v2 created item family mismatch",
            require_ref,
        ),
        ProjectV2InlineEffectOperation::PresentationOnly => {
            if effect.presentation.as_ref().is_none_or(|presentation| {
                presentation.impact_asset_binding.is_none()
                    && presentation.projectile_asset_binding.is_none()
                    && presentation.path_asset_binding.is_none()
            }) {
                return Err(ProjectError::InvalidProject(
                    "v2 presentation-only effect needs a presentation",
                ));
            }
            Ok(())
        }
        ProjectV2InlineEffectOperation::RemoveCondition { condition } => {
            snake(condition, "invalid v2 condition type")
        }
        ProjectV2InlineEffectOperation::RemoveItems { items, .. } => {
            if items.is_empty() || items.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(ProjectError::InvalidProject(
                    "v2 removed items are empty or repeated",
                ));
            }
            for item in items {
                family(
                    item,
                    ProjectV2Family::Item,
                    "v2 removed item family mismatch",
                    require_ref,
                )?;
            }
            Ok(())
        }
        ProjectV2InlineEffectOperation::SummonCreature {
            creatures,
            count,
            only_below_summons,
            ..
        } => {
            if creatures.is_empty() || *count == 0 || *only_below_summons == 0 {
                return Err(ProjectError::InvalidProject(
                    "v2 summon effect needs creatures and positive counts",
                ));
            }
            if creatures.windows(2).any(|pair| pair[0] >= pair[1]) {
                return Err(ProjectError::InvalidProject(
                    "v2 summon effect creatures are not sorted and unique",
                ));
            }
            for creature in creatures {
                family(
                    creature,
                    ProjectV2Family::Creature,
                    "v2 summon effect family mismatch",
                    require_ref,
                )?;
            }
            Ok(())
        }
    }
}

pub(super) fn validate_ability_details(
    details: &ProjectV2AbilityDetails,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    if details.variants.is_empty() == details.effects.is_empty() {
        return Err(ProjectError::InvalidProject(
            "v2 Ability details need exactly one of effects and variants",
        ));
    }
    if !details.variants.is_empty() {
        if details.variants.len() < 2 || details.area.is_some() || details.chain.is_some() {
            return Err(ProjectError::InvalidProject(
                "v2 Ability variants need two or more and no area or chain",
            ));
        }
        for variant in &details.variants {
            family(
                variant,
                ProjectV2Family::Ability,
                "v2 Ability variant family mismatch",
                require_ref,
            )?;
        }
    }
    limits.check(
        "v2 Ability effects",
        details.effects.len(),
        limits.max_reference_records,
    )?;
    for effect in &details.effects {
        match effect {
            ProjectV2AbilityEffect::Executable(reference) => family(
                reference,
                ProjectV2Family::Effect,
                "v2 Ability executable effect family mismatch",
                require_ref,
            )?,
            ProjectV2AbilityEffect::Inline(inline) => inline_effect(inline, require_ref, limits)?,
        }
    }
    match &details.area {
        Some(ProjectV2AbilityArea::Beam { length_tiles, .. }) if *length_tiles == 0 => {
            return Err(ProjectError::InvalidProject(
                "v2 beam length must be positive",
            ));
        }
        Some(ProjectV2AbilityArea::Matrix { north, diagonal }) => {
            area_matrix(north)?;
            if !diagonal.is_empty() {
                area_matrix(diagonal)?;
            }
        }
        _ => {}
    }
    if details
        .path_requirement
        .is_some_and(|path| path.max_search_tiles == 0)
    {
        return Err(ProjectError::InvalidProject(
            "v2 path requirement search must be positive",
        ));
    }
    if let Some(chain) = &details.chain {
        if chain.max_targets == 0 || chain.range_tiles == 0 {
            return Err(ProjectError::InvalidProject(
                "v2 chain needs positive targets and range",
            ));
        }
        if let Some(binding) = &chain.chain_asset_binding {
            asset_binding(binding, limits)?;
        }
    }
    for cue in [&details.cast_cue, &details.impact_cue]
        .into_iter()
        .flatten()
    {
        validate_v2_source_text("v2 Ability audio cue", cue, limits)?;
    }
    Ok(())
}

pub(super) fn validate_effect(
    effect: &ProjectV2EffectAuthoring,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    snake(&effect.damage_type, "invalid v2 damage type")?;
    if effect
        .mitigated_by
        .windows(2)
        .any(|pair| pair[0] >= pair[1])
    {
        return Err(ProjectError::InvalidProject(
            "v2 effect mitigation is not sorted and unique",
        ));
    }
    if let Some(affects) = &effect.affects {
        let named = affects.kind == ProjectV2AffectsKind::NamedCreatures;
        if named == affects.creatures.is_empty()
            || affects.creatures.windows(2).any(|pair| pair[0] >= pair[1])
        {
            return Err(ProjectError::InvalidProject(
                "v2 affected creatures are required exactly for named creatures, sorted and unique",
            ));
        }
        for creature in &affects.creatures {
            family(
                creature,
                ProjectV2Family::Creature,
                "v2 affected creature family mismatch",
                require_ref,
            )?;
        }
    }
    if let Some(presentation) = &effect.presentation {
        effect_presentation(presentation, limits)?;
    }
    Ok(())
}

pub(super) fn validate_formula(formula: &ProjectV2FormulaAuthoring) -> Result<(), ProjectError> {
    match formula {
        ProjectV2FormulaAuthoring::Range { minimum, maximum } if minimum > maximum => {
            Err(ProjectError::InvalidProject("v2 formula range is inverted"))
        }
        ProjectV2FormulaAuthoring::SpeedModifier {
            minimum_multiplier,
            maximum_multiplier,
            ..
        } => {
            for ratio in [*minimum_multiplier, *maximum_multiplier] {
                validate_v2_ratio(ratio, "v2 speed multiplier is not canonical")?;
                if ratio.numerator < 0 {
                    return Err(ProjectError::InvalidProject(
                        "v2 speed multiplier is negative",
                    ));
                }
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

pub(super) fn validate_loot(loot: &ProjectV2LootAuthoring) -> Result<(), ProjectError> {
    if loot.entries.is_empty() {
        return Err(ProjectError::InvalidProject(
            "v2 Loot details need the entries of their table",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------
// Canonical order of unordered sets; authored sequences (schedules, effects, voices, removed
// items, loot entries) keep their order.

impl ProjectV2CreatureDetails {
    pub(super) fn canonicalize(&mut self) {
        self.condition_immunities.sort();
        self.damage_reflection
            .sort_by(|left, right| left.damage_type.cmp(&right.damage_type));
        self.healing_from_damage
            .sort_by(|left, right| left.damage_type.cmp(&right.damage_type));
    }
}

impl ProjectV2BehaviorAuthoring {
    pub(super) fn canonicalize(&mut self) {
        if let Some(audio) = &mut self.periodic_audio {
            audio.cue_ids.sort();
        }
        if let Some(faction) = &mut self.faction {
            faction.enemy_factions.sort();
        }
        self.event_bindings.sort_by(|left, right| {
            (left.event, &left.interaction).cmp(&(right.event, &right.interaction))
        });
    }
}

impl ProjectV2PresentationAuthoring {
    pub(super) fn canonicalize(&mut self) {
        self.palette_bindings.sort_by_key(|binding| binding.slot);
        self.attachment_bindings.sort_by_key(|binding| binding.slot);
        self.visual_effect_bindings
            .sort_by_key(|binding| binding.slot);
        self.audio_bindings
            .sort_by(|left, right| (left.event, &left.cue_id).cmp(&(right.event, &right.cue_id)));
    }
}

impl ProjectV2AbilityDetails {
    pub(super) fn canonicalize(&mut self) {
        for effect in &mut self.effects {
            if let ProjectV2AbilityEffect::Inline(inline) = effect
                && let ProjectV2InlineEffectOperation::SummonCreature { creatures, .. } =
                    &mut inline.operation
            {
                creatures.sort();
            }
        }
    }
}

impl ProjectV2EffectAuthoring {
    pub(super) fn canonicalize(&mut self) {
        self.mitigated_by.sort();
        if let Some(affects) = &mut self.affects {
            affects.creatures.sort();
        }
    }
}
