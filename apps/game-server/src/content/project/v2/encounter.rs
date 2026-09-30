//! Declarative encounter authoring profiles (OTERYN_WORLD_PROJECT_V2_ENCOUNTER_ADMISSION_V1 E1).
//!
//! The profile mirrors the encounter authoring format v1 (triggers, conditions, actions,
//! positions and anchors). It is candidate-only: no runtime path interprets it. Rust checks the
//! structure: ranges, exact references, and that every role, anchor, counter, flag, timer, phase,
//! outcome and encounter ability a rule names is declared. `validate_encounter.py` stays the
//! semantic source. Percentages are exact parts per million (`ppm = percent * 10_000`).

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::creature::PROJECT_V2_PPM_SCALE;
use super::{
    ProjectError, ProjectEvidenceLimits, ProjectV2DefinitionRef, ProjectV2Family,
    validate_v2_source_text,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterDetails {
    pub display_name: String,
    /// Authored order; roles are unique.
    pub participants: Vec<ProjectV2EncounterParticipant>,
    /// E3: the participant creatures this encounter covers, sorted and unique. Each lists the
    /// encounter in its Creature profile, and is never activated without it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub covers: Vec<ProjectV2DefinitionRef>,
    /// Authored order; names are unique.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phases: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub anchors: Vec<ProjectV2EncounterAnchor>,
    pub state: ProjectV2EncounterState,
    /// Rules run in their authored order.
    pub rules: Vec<ProjectV2EncounterRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub outcomes: Vec<String>,
    /// Area effects authored by the encounter itself (D34).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub abilities: Vec<ProjectV2EncounterAbility>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_after_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterParticipant {
    pub role: String,
    /// Sorted and unique.
    pub creatures: Vec<ProjectV2DefinitionRef>,
}

/// E2: an anchor's location in the project coordinate frame; the Canary description stays as
/// source text. Binding to a map revision is required before activation, not admission.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterAnchor {
    pub key: String,
    pub description: String,
    pub location: ProjectV2AnchorLocation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2AnchorLocation {
    Point { x: u16, y: u16, floor: u8 },
    Area { boxes: Vec<ProjectV2AnchorBox> },
}

/// Whole tiles from `[0]` to `[1]` inclusive on each axis, on one floor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2AnchorBox {
    pub x: [u16; 2],
    pub y: [u16; 2],
    pub floor: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterState {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub counters: Vec<ProjectV2EncounterCounter>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<ProjectV2EncounterFlag>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub timers: Vec<ProjectV2EncounterTimer>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterCounter {
    pub name: String,
    pub initial: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterFlag {
    pub name: String,
    pub initial: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterTimer {
    pub name: String,
    pub duration_ms: ProjectV2Amount,
    pub repeat: bool,
}

/// A value drawn uniformly from `min..=max` by the encounter instance; a fixed value has
/// `min == max`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2Amount {
    pub min: u64,
    pub max: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterRule {
    pub key: String,
    pub trigger: ProjectV2EncounterTrigger,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delay_ms: Option<ProjectV2Amount>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conditions: Vec<ProjectV2EncounterCondition>,
    pub actions: Vec<ProjectV2EncounterAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2HitSource {
    Player,
    NonPlayer,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2AreaWho {
    Player,
    Role,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2BaseVocation {
    Knight,
    Paladin,
    Sorcerer,
    Druid,
    Monk,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2EncounterTrigger {
    CreatureDied {
        role: String,
    },
    LethalDamage {
        role: String,
    },
    /// Exactly one of a percent of maximum health and an absolute health.
    HealthCrossed {
        role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        percent_ppm: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        health: Option<u64>,
    },
    CreatureSpawned {
        role: String,
    },
    AbilityCast {
        role: String,
        ability: ProjectV2DefinitionRef,
    },
    DamageTaken {
        role: String,
        source: ProjectV2HitSource,
    },
    HealReceived {
        role: String,
        source: ProjectV2HitSource,
    },
    /// Exactly one of an amount and a percent of maximum health.
    DamageAccumulated {
        role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        amount: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        percent_ppm: Option<u32>,
    },
    TimerElapsed {
        timer: String,
    },
    CounterReached {
        counter: String,
        value: i64,
    },
    /// A role is present exactly when `who` is `role`.
    AreaEntered {
        anchor: String,
        who: ProjectV2AreaWho,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<String>,
    },
    AreaLeft {
        anchor: String,
        who: ProjectV2AreaWho,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<String>,
    },
    PhaseEntered {
        phase: String,
    },
    ItemUsed {
        role: String,
        item: ProjectV2DefinitionRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_vocation: Option<ProjectV2BaseVocation>,
    },
    /// Exactly one of an item and `corpse_of`, the role whose corpse the creature steps on.
    SteppedOn {
        role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item: Option<ProjectV2DefinitionRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        corpse_of: Option<String>,
    },
    EncounterStarted,
    EncounterReset,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectV2CompareOp {
    #[serde(rename = "==")]
    Equal,
    #[serde(rename = "!=")]
    NotEqual,
    #[serde(rename = "<")]
    Less,
    #[serde(rename = "<=")]
    LessOrEqual,
    #[serde(rename = ">")]
    Greater,
    #[serde(rename = ">=")]
    GreaterOrEqual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2CreatureConditionType {
    Poison,
    Fire,
    Energy,
    Bleeding,
    Drown,
    Freezing,
    Dazzled,
    Cursed,
}

/// A read-only value of another domain's state (a quest progress or world state).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProjectV2StateValue {
    Integer(i64),
    Boolean(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2NearShape {
    Square,
    Circle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterNear {
    pub role: String,
    pub radius: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shape: Option<ProjectV2NearShape>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2EquipmentSlot {
    Head,
    Necklace,
    Armor,
    RightHand,
    LeftHand,
    Legs,
    Feet,
    Ring,
    Ammo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2EncounterCondition {
    /// Above 0 and at most 100%.
    ChancePercent {
        value_ppm: u32,
    },
    ChanceFromAmount {
        per: u64,
    },
    CounterCompare {
        counter: String,
        op: ProjectV2CompareOp,
        value: i64,
    },
    /// Sorted and unique conditions.
    HasCondition {
        role: String,
        conditions: Vec<ProjectV2CreatureConditionType>,
        present: bool,
    },
    Flag {
        flag: String,
        value: bool,
    },
    /// Exactly one of an area anchor and `near`.
    CreaturePresent {
        role: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        near: Option<ProjectV2EncounterNear>,
        present: bool,
    },
    WorldState {
        state: String,
        op: ProjectV2CompareOp,
        value: ProjectV2StateValue,
    },
    InAnchor {
        subject: ProjectV2InAnchorSubject,
        anchor: String,
    },
    KillerIsPlayer,
    HasMaster {
        role: String,
        value: bool,
    },
    SummonCount {
        role: String,
        op: ProjectV2CompareOp,
        value: u64,
    },
    HealthPercent {
        role: String,
        op: ProjectV2CompareOp,
        value_ppm: u32,
    },
    AttackerWears {
        item: ProjectV2DefinitionRef,
        wears: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slot: Option<ProjectV2EquipmentSlot>,
    },
    KillerProgress {
        progress: String,
        op: ProjectV2CompareOp,
        value: ProjectV2StateValue,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2EncounterSubject {
    Role { role: String },
    Killer,
    Spawned,
}

/// The `in_anchor` subject: a shared subject, or the creature that fired the rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2InAnchorSubject {
    Role { role: String },
    Killer,
    Spawned,
    Triggering,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2EncounterPosition {
    DeathPosition,
    SubjectPosition,
    ClosestFreeTile,
    Anchor {
        anchor: String,
    },
    RandomIn {
        anchor: String,
        /// Draw only among tiles a creature can be placed on now.
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        free: bool,
    },
    RolePosition {
        role: String,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        otherwise_death_position: bool,
    },
    OffsetTiles {
        tiles: u32,
    },
    Relative {
        x: i32,
        y: i32,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2EncounterHealth {
    Full,
    KeepPercent,
    KeepAbsolute,
    Remembered,
    Percent { percent: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2SpawnOwner {
    None,
    Subject,
    DeathMaster,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2SpawnByVocation {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub knight: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paladin: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sorcerer: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub druid: Option<ProjectV2DefinitionRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monk: Option<ProjectV2DefinitionRef>,
}

impl ProjectV2SpawnByVocation {
    fn entries(&self) -> impl Iterator<Item = &ProjectV2DefinitionRef> {
        [
            &self.knight,
            &self.paladin,
            &self.sorcerer,
            &self.druid,
            &self.monk,
        ]
        .into_iter()
        .flatten()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2TransformInto {
    Creature {
        creature: ProjectV2DefinitionRef,
    },
    NextStage,
    /// At least two, sorted and unique.
    RandomOf {
        creatures: Vec<ProjectV2DefinitionRef>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2HealAmount {
    Full,
    /// `min` may be 0 (D31).
    Range {
        min: u64,
        max: u64,
    },
}

/// D34: a fixed percent, or the percent of a timer's duration still remaining (never below
/// `floor_percent`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2DamageMultiplier {
    Fixed { percent: u32 },
    TimerRemaining { timer: String, floor_percent: u8 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2DamageComponent {
    All,
    Primary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2ModifierSources {
    Player,
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2ModifierUntil {
    ThisHit,
    Reset,
    Timer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2TeleportWho {
    Role {
        role: String,
    },
    PlayersIn {
        anchor: String,
    },
    /// The creature that fired the rule.
    Triggering,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2MapItemOperation {
    Create,
    Transform,
    Remove,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2CounterOperation {
    Set,
    Add,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2TimerOperation {
    Start,
    Stop,
    Add,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2EncounterAttribute {
    OutgoingDamagePercent,
    Defense,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2AttributeOperation {
    Add,
    Reset,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2AttributeValue {
    Fixed { value: u64 },
    Counter { counter: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2EncounterSpeech {
    Say,
    Yell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectV2OutcomeCredit {
    DamageContributors,
    Killer,
    PlayersInAnchor,
    Party,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2WeightedBranch {
    pub weight: u32,
    pub actions: Vec<ProjectV2EncounterAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectV2EncounterAction {
    Spawn {
        creature: ProjectV2DefinitionRef,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<String>,
        count: ProjectV2Amount,
        at: ProjectV2EncounterPosition,
        owner: ProjectV2SpawnOwner,
        health: ProjectV2EncounterHealth,
    },
    SpawnPerPlayer {
        players_in: String,
        by_base_vocation: ProjectV2SpawnByVocation,
        at: ProjectV2EncounterPosition,
        owner: ProjectV2SpawnOwner,
        health: ProjectV2EncounterHealth,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        counter: Option<String>,
    },
    /// Exactly one of a role, an area anchor and the triggering creature.
    Remove {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        role: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        all_in: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        triggering: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        keep_summons: Option<bool>,
    },
    Transform {
        role: String,
        into: ProjectV2TransformInto,
        health: ProjectV2EncounterHealth,
    },
    Heal {
        subject: ProjectV2EncounterSubject,
        amount: ProjectV2HealAmount,
    },
    Damage {
        subject: ProjectV2EncounterSubject,
        amount: ProjectV2Amount,
        damage_type: String,
    },
    PreventDeath {
        role: String,
    },
    DamageModifier {
        role: String,
        multiplier: ProjectV2DamageMultiplier,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        damage_types: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        component: Option<ProjectV2DamageComponent>,
        sources: ProjectV2ModifierSources,
        until: ProjectV2ModifierUntil,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        timer: Option<String>,
    },
    ReflectDamage {
        role: String,
        percent: u8,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        damage_types: Vec<String>,
    },
    ConvertDamageToHeal {
        role: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        damage_types: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        component: Option<ProjectV2DamageComponent>,
    },
    Teleport {
        who: ProjectV2TeleportWho,
        to: String,
    },
    /// Exactly one of an anchor and the death position, or `triggering` alone: a remove of the
    /// item that fired a `stepped_on` rule.
    MapItem {
        operation: ProjectV2MapItemOperation,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        item: Option<ProjectV2DefinitionRef>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        triggering: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        into: Option<ProjectV2DefinitionRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor: Option<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        at_death_position: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        revert_after_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        destination: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        revert_destination: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        effect: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        interaction: Option<String>,
    },
    Counter {
        counter: String,
        operation: ProjectV2CounterOperation,
        value: i64,
    },
    Flag {
        flag: String,
        value: bool,
    },
    Timer {
        timer: String,
        operation: ProjectV2TimerOperation,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ms: Option<u64>,
    },
    SetPhase {
        phase: String,
    },
    MoveLock {
        role: String,
        locked: bool,
    },
    Attribute {
        role: String,
        attribute: ProjectV2EncounterAttribute,
        operation: ProjectV2AttributeOperation,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        value: Option<ProjectV2AttributeValue>,
    },
    SharedLife {
        role: String,
    },
    /// Exactly one of an Ability and an encounter ability.
    Cast {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ability: Option<ProjectV2DefinitionRef>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        encounter_ability: Option<String>,
        at: ProjectV2EncounterPosition,
    },
    Say {
        subject: ProjectV2EncounterSubject,
        text: String,
        mode: ProjectV2EncounterSpeech,
    },
    DropItem {
        item: ProjectV2DefinitionRef,
        at: ProjectV2EncounterPosition,
    },
    Message {
        players_in: String,
        text: String,
    },
    OneOf {
        branches: Vec<ProjectV2WeightedBranch>,
    },
    EmitOutcome {
        outcome: String,
        credited: ProjectV2OutcomeCredit,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        anchor: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterAbility {
    pub key: String,
    pub area: ProjectV2EncounterAbilityArea,
    pub damage: ProjectV2EncounterAbilityDamage,
    pub affects: ProjectV2EncounterAbilityAffects,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effect: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterAbilityArea {
    pub shape: ProjectV2NearShape,
    pub radius: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterAbilityDamage {
    pub damage_type: String,
    pub min: u64,
    pub max: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectV2EncounterAbilityAffects {
    pub players: bool,
    /// Sorted and unique.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub creatures: Vec<ProjectV2DefinitionRef>,
}

impl ProjectV2EncounterDetails {
    /// Only sets are sorted; every authored sequence keeps its order.
    pub(super) fn canonicalize(&mut self) {
        for participant in &mut self.participants {
            participant.creatures.sort();
        }
        self.covers.sort();
        for ability in &mut self.abilities {
            ability.affects.creatures.sort();
        }
        for rule in &mut self.rules {
            for condition in &mut rule.conditions {
                if let ProjectV2EncounterCondition::HasCondition { conditions, .. } = condition {
                    conditions.sort();
                }
            }
            canonicalize_actions(&mut rule.actions);
        }
    }
}

fn canonicalize_actions(actions: &mut [ProjectV2EncounterAction]) {
    for action in actions {
        match action {
            ProjectV2EncounterAction::Transform {
                into: ProjectV2TransformInto::RandomOf { creatures },
                ..
            } => creatures.sort(),
            ProjectV2EncounterAction::DamageModifier { damage_types, .. }
            | ProjectV2EncounterAction::ReflectDamage { damage_types, .. }
            | ProjectV2EncounterAction::ConvertDamageToHeal { damage_types, .. } => {
                damage_types.sort();
            }
            ProjectV2EncounterAction::OneOf { branches } => {
                for branch in branches {
                    canonicalize_actions(&mut branch.actions);
                }
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Validation

fn invalid<T>(error: &'static str) -> Result<T, ProjectError> {
    Err(ProjectError::InvalidProject(error))
}

/// Encounter names (`^[a-z][a-z0-9_]*$`): roles, anchors, state, phases, outcomes, rules.
fn name(value: &str) -> Result<(), ProjectError> {
    let mut bytes = value.bytes();
    if value.len() > 128
        || !bytes.next().is_some_and(|byte| byte.is_ascii_lowercase())
        || !bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return invalid("invalid v2 encounter name");
    }
    Ok(())
}

/// Declared names of one kind; the declaration order is kept, duplicates fail.
fn declare<'a>(
    names: impl Iterator<Item = &'a str>,
    error: &'static str,
) -> Result<BTreeSet<&'a str>, ProjectError> {
    let mut declared = BTreeSet::new();
    for value in names {
        name(value)?;
        if !declared.insert(value) {
            return invalid(error);
        }
    }
    Ok(declared)
}

fn known(pool: &BTreeSet<&str>, value: &str, error: &'static str) -> Result<(), ProjectError> {
    if !pool.contains(value) {
        return invalid(error);
    }
    Ok(())
}

/// A namespaced token of another domain (`namespace:local`): quest progress, world state and
/// interactions stay tokens until their owning domain admits them.
fn token(value: &str, limits: ProjectEvidenceLimits) -> Result<(), ProjectError> {
    validate_v2_source_text("v2 encounter domain token", value, limits)?;
    // The encounter authoring KEY grammar: `^[a-z][a-z0-9_.-]*:[a-z0-9_./-]+$`.
    let key_byte =
        |byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_.-".contains(&byte);
    let valid = value.split_once(':').is_some_and(|(namespace, local)| {
        namespace
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_lowercase)
            && namespace.bytes().all(key_byte)
            && !local.is_empty()
            && local.bytes().all(|byte| key_byte(byte) || byte == b'/')
    });
    if !valid {
        return invalid("invalid v2 encounter domain token");
    }
    Ok(())
}

fn percent_ppm(value: u32) -> Result<(), ProjectError> {
    if value == 0 || value >= PROJECT_V2_PPM_SCALE {
        return invalid("v2 encounter percent is outside (0, 100)");
    }
    Ok(())
}

fn amount(value: ProjectV2Amount, minimum: u64) -> Result<(), ProjectError> {
    if value.min < minimum || value.min > value.max {
        return invalid("v2 encounter amount is out of range");
    }
    Ok(())
}

fn reference(
    value: &ProjectV2DefinitionRef,
    expected: ProjectV2Family,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
) -> Result<(), ProjectError> {
    if value.family != expected {
        return invalid("v2 encounter reference family mismatch");
    }
    require_ref(value)
}

fn sorted_refs(
    values: &[ProjectV2DefinitionRef],
    expected: ProjectV2Family,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    limits.check(
        "v2 encounter references",
        values.len(),
        limits.max_reference_records,
    )?;
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return invalid("v2 encounter references are not sorted and unique");
    }
    values
        .iter()
        .try_for_each(|value| reference(value, expected, require_ref))
}

fn damage_types(values: &[String], limits: ProjectEvidenceLimits) -> Result<(), ProjectError> {
    limits.check(
        "v2 encounter damage types",
        values.len(),
        limits.max_reference_records,
    )?;
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return invalid("v2 encounter damage types are not sorted and unique");
    }
    values.iter().try_for_each(|value| name(value))
}

struct Names<'a> {
    roles: BTreeSet<&'a str>,
    anchors: BTreeSet<&'a str>,
    areas: BTreeSet<&'a str>,
    counters: BTreeSet<&'a str>,
    flags: BTreeSet<&'a str>,
    timers: BTreeSet<&'a str>,
    phases: BTreeSet<&'a str>,
    outcomes: BTreeSet<&'a str>,
    abilities: BTreeSet<&'a str>,
}

impl Names<'_> {
    fn role(&self, value: &str) -> Result<(), ProjectError> {
        known(&self.roles, value, "v2 encounter names an unknown role")
    }

    fn anchor(&self, value: &str) -> Result<(), ProjectError> {
        known(&self.anchors, value, "v2 encounter names an unknown anchor")
    }

    fn area(&self, value: &str) -> Result<(), ProjectError> {
        self.anchor(value)?;
        known(&self.areas, value, "v2 encounter needs an area anchor")
    }

    fn counter(&self, value: &str) -> Result<(), ProjectError> {
        known(
            &self.counters,
            value,
            "v2 encounter names an unknown counter",
        )
    }

    fn timer(&self, value: &str) -> Result<(), ProjectError> {
        known(&self.timers, value, "v2 encounter names an unknown timer")
    }
}

fn spawned_roles<'a>(
    actions: &'a [ProjectV2EncounterAction],
    roles: &mut Vec<(&'a str, &'a ProjectV2DefinitionRef)>,
) {
    for action in actions {
        match action {
            ProjectV2EncounterAction::Spawn {
                role: Some(role),
                creature,
                ..
            } => roles.push((role, creature)),
            ProjectV2EncounterAction::OneOf { branches } => {
                for branch in branches {
                    spawned_roles(&branch.actions, roles);
                }
            }
            _ => {}
        }
    }
}

impl ProjectV2EncounterDetails {
    /// The creatures a role can be: its participant creatures and every creature spawned into it.
    pub(super) fn role_creatures(&self, role: &str) -> BTreeSet<&ProjectV2DefinitionRef> {
        let mut spawned = Vec::new();
        for rule in &self.rules {
            spawned_roles(&rule.actions, &mut spawned);
        }
        self.participants
            .iter()
            .filter(|value| value.role == role)
            .flat_map(|value| value.creatures.iter())
            .chain(
                spawned
                    .into_iter()
                    .filter(|(name, _)| *name == role)
                    .map(|(_, creature)| creature),
            )
            .collect()
    }
}

pub(super) fn validate_encounter_details(
    details: &ProjectV2EncounterDetails,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    validate_v2_source_text("v2 encounter display name", &details.display_name, limits)?;
    for (label, count) in [
        ("v2 encounter participants", details.participants.len()),
        ("v2 encounter anchors", details.anchors.len()),
        ("v2 encounter rules", details.rules.len()),
        ("v2 encounter counters", details.state.counters.len()),
        ("v2 encounter flags", details.state.flags.len()),
        ("v2 encounter timers", details.state.timers.len()),
        ("v2 encounter phases", details.phases.len()),
        ("v2 encounter outcomes", details.outcomes.len()),
        ("v2 encounter abilities", details.abilities.len()),
    ] {
        limits.check(label, count, limits.max_reference_records)?;
    }
    if details.participants.is_empty() || details.rules.is_empty() {
        return invalid("v2 encounter needs a participant and a rule");
    }
    for participant in &details.participants {
        if participant.creatures.is_empty() {
            return invalid("v2 encounter participant needs a creature");
        }
        sorted_refs(
            &participant.creatures,
            ProjectV2Family::Creature,
            require_ref,
            limits,
        )?;
    }
    sorted_refs(
        &details.covers,
        ProjectV2Family::Creature,
        require_ref,
        limits,
    )?;
    if details.covers.iter().any(|covered| {
        !details
            .participants
            .iter()
            .any(|participant| participant.creatures.contains(covered))
    }) {
        return invalid("v2 encounter covers a creature that is not a participant");
    }
    for anchor in &details.anchors {
        validate_v2_source_text(
            "v2 encounter anchor description",
            &anchor.description,
            limits,
        )?;
        match &anchor.location {
            ProjectV2AnchorLocation::Point { floor, .. } if *floor > 15 => {
                return invalid("v2 encounter anchor floor is outside 0..=15");
            }
            ProjectV2AnchorLocation::Point { .. } => {}
            ProjectV2AnchorLocation::Area { boxes } => {
                if boxes.is_empty() {
                    return invalid("v2 encounter area needs a box");
                }
                limits.check(
                    "v2 encounter boxes",
                    boxes.len(),
                    limits.max_reference_records,
                )?;
                for area in boxes {
                    if area.x[0] > area.x[1] || area.y[0] > area.y[1] {
                        return invalid("v2 encounter box starts after it ends");
                    }
                    if area.floor > 15 {
                        return invalid("v2 encounter anchor floor is outside 0..=15");
                    }
                }
            }
        }
    }
    let state = &details.state;
    for timer in &state.timers {
        amount(timer.duration_ms, 1)?;
    }
    if details.reset_after_ms == Some(0) {
        return invalid("v2 encounter reset must be positive");
    }
    for ability in &details.abilities {
        name(&ability.damage.damage_type)?;
        if ability.damage.min == 0 || ability.damage.min > ability.damage.max {
            return invalid("v2 encounter ability damage is out of range");
        }
        sorted_refs(
            &ability.affects.creatures,
            ProjectV2Family::Creature,
            require_ref,
            limits,
        )?;
        if let Some(effect) = &ability.effect {
            validate_v2_source_text("v2 encounter ability effect", effect, limits)?;
        }
    }
    let mut roles = declare(
        details.participants.iter().map(|value| value.role.as_str()),
        "v2 encounter roles are not unique",
    )?;
    let mut spawned = Vec::new();
    for rule in &details.rules {
        spawned_roles(&rule.actions, &mut spawned);
    }
    roles.extend(spawned.into_iter().map(|(role, _)| role));
    for role in &roles {
        name(role)?;
    }
    let names = Names {
        roles,
        anchors: declare(
            details.anchors.iter().map(|value| value.key.as_str()),
            "v2 encounter anchors are not unique",
        )?,
        areas: details
            .anchors
            .iter()
            .filter(|value| matches!(value.location, ProjectV2AnchorLocation::Area { .. }))
            .map(|value| value.key.as_str())
            .collect(),
        counters: declare(
            state.counters.iter().map(|value| value.name.as_str()),
            "v2 encounter counters are not unique",
        )?,
        flags: declare(
            state.flags.iter().map(|value| value.name.as_str()),
            "v2 encounter flags are not unique",
        )?,
        timers: declare(
            state.timers.iter().map(|value| value.name.as_str()),
            "v2 encounter timers are not unique",
        )?,
        phases: declare(
            details.phases.iter().map(String::as_str),
            "v2 encounter phases are not unique",
        )?,
        outcomes: declare(
            details.outcomes.iter().map(String::as_str),
            "v2 encounter outcomes are not unique",
        )?,
        abilities: declare(
            details.abilities.iter().map(|value| value.key.as_str()),
            "v2 encounter abilities are not unique",
        )?,
    };
    declare(
        details.rules.iter().map(|value| value.key.as_str()),
        "v2 encounter rules are not unique",
    )?;
    for rule in &details.rules {
        if let Some(delay) = rule.delay_ms {
            amount(delay, 1)?;
        }
        trigger(&rule.trigger, &names, require_ref)?;
        limits.check(
            "v2 encounter conditions",
            rule.conditions.len(),
            limits.max_reference_records,
        )?;
        for value in &rule.conditions {
            condition(value, &rule.trigger, &names, require_ref, limits)?;
        }
        actions(&rule.actions, &rule.trigger, &names, require_ref, limits)?;
    }
    Ok(())
}

fn trigger(
    value: &ProjectV2EncounterTrigger,
    names: &Names<'_>,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
) -> Result<(), ProjectError> {
    use ProjectV2EncounterTrigger as T;
    match value {
        T::CreatureDied { role }
        | T::LethalDamage { role }
        | T::CreatureSpawned { role }
        | T::DamageTaken { role, .. }
        | T::HealReceived { role, .. } => names.role(role),
        T::HealthCrossed {
            role,
            percent_ppm: percent,
            health,
        } => {
            names.role(role)?;
            match (percent, health) {
                (Some(value), None) => percent_ppm(*value),
                (None, Some(value)) if *value > 0 => Ok(()),
                _ => invalid("v2 health_crossed takes exactly one positive threshold"),
            }
        }
        T::DamageAccumulated {
            role,
            amount: total,
            percent_ppm: percent,
        } => {
            names.role(role)?;
            match (total, percent) {
                (Some(value), None) if *value > 0 => Ok(()),
                (None, Some(value)) => percent_ppm(*value),
                _ => invalid("v2 damage_accumulated takes exactly one positive threshold"),
            }
        }
        T::AbilityCast { role, ability } => {
            names.role(role)?;
            reference(ability, ProjectV2Family::Ability, require_ref)
        }
        T::ItemUsed { role, item, .. } => {
            names.role(role)?;
            reference(item, ProjectV2Family::Item, require_ref)
        }
        T::SteppedOn {
            role,
            item,
            corpse_of,
        } => {
            names.role(role)?;
            match (item, corpse_of) {
                (Some(item), None) => reference(item, ProjectV2Family::Item, require_ref),
                (None, Some(corpse)) => names.role(corpse),
                _ => invalid("v2 stepped_on takes exactly one of item and corpse_of"),
            }
        }
        T::TimerElapsed { timer } => names.timer(timer),
        T::CounterReached { counter, .. } => names.counter(counter),
        T::AreaEntered { anchor, who, role } | T::AreaLeft { anchor, who, role } => {
            names.area(anchor)?;
            match (who, role) {
                (ProjectV2AreaWho::Role, Some(role)) => names.role(role),
                (ProjectV2AreaWho::Player, None) => Ok(()),
                _ => invalid("v2 area trigger names a role exactly when who is role"),
            }
        }
        T::PhaseEntered { phase } => {
            known(&names.phases, phase, "v2 encounter names an unknown phase")
        }
        T::EncounterStarted | T::EncounterReset => Ok(()),
    }
}

fn subject(value: &ProjectV2EncounterSubject, names: &Names<'_>) -> Result<(), ProjectError> {
    match value {
        ProjectV2EncounterSubject::Role { role } => names.role(role),
        ProjectV2EncounterSubject::Killer | ProjectV2EncounterSubject::Spawned => Ok(()),
    }
}

/// Triggers fired by one creature.
fn one_creature(value: &ProjectV2EncounterTrigger) -> bool {
    use ProjectV2EncounterTrigger as T;
    matches!(
        value,
        T::CreatureDied { .. }
            | T::LethalDamage { .. }
            | T::HealthCrossed { .. }
            | T::CreatureSpawned { .. }
            | T::AbilityCast { .. }
            | T::DamageTaken { .. }
            | T::HealReceived { .. }
            | T::DamageAccumulated { .. }
            | T::ItemUsed { .. }
            | T::SteppedOn { .. }
    )
}

/// Triggers that name a triggering creature: one creature, or an area entered or left.
fn triggering_creature(value: &ProjectV2EncounterTrigger) -> bool {
    one_creature(value)
        || matches!(
            value,
            ProjectV2EncounterTrigger::AreaEntered { .. }
                | ProjectV2EncounterTrigger::AreaLeft { .. }
        )
}

fn condition(
    value: &ProjectV2EncounterCondition,
    fired: &ProjectV2EncounterTrigger,
    names: &Names<'_>,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    use ProjectV2EncounterCondition as C;
    match value {
        C::ChancePercent { value_ppm } => {
            if *value_ppm == 0 || *value_ppm > PROJECT_V2_PPM_SCALE {
                return invalid("v2 encounter chance is outside (0, 100]");
            }
            Ok(())
        }
        C::ChanceFromAmount { per } => {
            if *per == 0 {
                return invalid("v2 chance_from_amount needs a positive divisor");
            }
            Ok(())
        }
        C::CounterCompare { counter, .. } => names.counter(counter),
        C::HasCondition {
            role, conditions, ..
        } => {
            names.role(role)?;
            limits.check(
                "v2 encounter creature conditions",
                conditions.len(),
                limits.max_reference_records,
            )?;
            if conditions.is_empty() || conditions.windows(2).any(|pair| pair[0] >= pair[1]) {
                return invalid("v2 has_condition conditions are empty, unsorted or duplicated");
            }
            Ok(())
        }
        C::Flag { flag, .. } => known(&names.flags, flag, "v2 encounter names an unknown flag"),
        C::CreaturePresent {
            role, anchor, near, ..
        } => {
            names.role(role)?;
            match (anchor, near) {
                (Some(anchor), None) => names.area(anchor),
                (None, Some(near)) => names.role(&near.role),
                _ => invalid("v2 creature_present takes exactly one of anchor and near"),
            }
        }
        C::WorldState { state, .. } => token(state, limits),
        C::KillerProgress { progress, .. } => token(progress, limits),
        C::InAnchor {
            subject: who,
            anchor,
        } => {
            match who {
                ProjectV2InAnchorSubject::Role { role } => names.role(role)?,
                ProjectV2InAnchorSubject::Killer | ProjectV2InAnchorSubject::Spawned => {}
                ProjectV2InAnchorSubject::Triggering if triggering_creature(fired) => {}
                ProjectV2InAnchorSubject::Triggering => {
                    return invalid("v2 in_anchor of triggering needs a creature or area trigger");
                }
            }
            names.area(anchor)
        }
        C::KillerIsPlayer => Ok(()),
        C::HasMaster { role, .. } | C::SummonCount { role, .. } => names.role(role),
        C::HealthPercent {
            role, value_ppm, ..
        } => {
            names.role(role)?;
            if *value_ppm > PROJECT_V2_PPM_SCALE {
                return invalid("v2 health_percent exceeds 100%");
            }
            Ok(())
        }
        C::AttackerWears { item, .. } => reference(item, ProjectV2Family::Item, require_ref),
    }
}

fn position(value: &ProjectV2EncounterPosition, names: &Names<'_>) -> Result<(), ProjectError> {
    use ProjectV2EncounterPosition as P;
    match value {
        P::Anchor { anchor } => names.anchor(anchor),
        P::RandomIn { anchor, .. } => names.area(anchor),
        P::RolePosition { role, .. } => names.role(role),
        P::DeathPosition
        | P::SubjectPosition
        | P::ClosestFreeTile
        | P::OffsetTiles { .. }
        | P::Relative { .. } => Ok(()),
    }
}

fn health(value: ProjectV2EncounterHealth) -> Result<(), ProjectError> {
    if let ProjectV2EncounterHealth::Percent { percent } = value
        && (percent == 0 || percent > 100)
    {
        return invalid("v2 encounter health percent is outside 1..=100");
    }
    Ok(())
}

fn text(value: &str, limits: ProjectEvidenceLimits) -> Result<(), ProjectError> {
    validate_v2_source_text("v2 encounter text", value, limits)
}

fn actions(
    values: &[ProjectV2EncounterAction],
    fired: &ProjectV2EncounterTrigger,
    names: &Names<'_>,
    require_ref: &impl Fn(&ProjectV2DefinitionRef) -> Result<(), ProjectError>,
    limits: ProjectEvidenceLimits,
) -> Result<(), ProjectError> {
    use ProjectV2EncounterAction as A;
    if values.is_empty() {
        return invalid("v2 encounter rule needs an action");
    }
    limits.check(
        "v2 encounter actions",
        values.len(),
        limits.max_reference_records,
    )?;
    for value in values {
        match value {
            A::Spawn {
                creature,
                role,
                count,
                at,
                health: kept,
                ..
            } => {
                reference(creature, ProjectV2Family::Creature, require_ref)?;
                amount(*count, 1)?;
                position(at, names)?;
                health(*kept)?;
                if *kept == ProjectV2EncounterHealth::Remembered && role.is_none() {
                    return invalid("v2 remembered health needs a spawn into a named role");
                }
            }
            A::SpawnPerPlayer {
                players_in,
                by_base_vocation,
                at,
                owner,
                health: kept,
                counter,
            } => {
                names.area(players_in)?;
                if *owner == ProjectV2SpawnOwner::DeathMaster {
                    return invalid("v2 spawn_per_player owner is none or subject");
                }
                if by_base_vocation.entries().next().is_none() {
                    return invalid("v2 spawn_per_player needs a vocation entry");
                }
                for creature in by_base_vocation.entries() {
                    reference(creature, ProjectV2Family::Creature, require_ref)?;
                }
                position(at, names)?;
                health(*kept)?;
                if *kept == ProjectV2EncounterHealth::Remembered {
                    return invalid("v2 remembered health needs a spawn into a named role");
                }
                if let Some(counter) = counter {
                    names.counter(counter)?;
                }
            }
            A::Remove {
                role,
                all_in,
                triggering,
                keep_summons,
            } => {
                match (role, all_in, triggering) {
                    (Some(role), None, false) => names.role(role)?,
                    (None, Some(area), false) => names.area(area)?,
                    (None, None, true) => {
                        if !one_creature(fired)
                            || matches!(
                                fired,
                                ProjectV2EncounterTrigger::AreaEntered {
                                    who: ProjectV2AreaWho::Player,
                                    ..
                                }
                            )
                        {
                            return invalid(
                                "v2 remove triggering needs a trigger fired by a creature",
                            );
                        }
                    }
                    _ => {
                        return invalid(
                            "v2 remove takes exactly one of role, all_in and triggering",
                        );
                    }
                }
                if keep_summons.is_some() && all_in.is_none() {
                    return invalid("v2 keep_summons applies only to remove all_in");
                }
            }
            A::Transform {
                role,
                into,
                health: kept,
            } => {
                names.role(role)?;
                health(*kept)?;
                if *kept == ProjectV2EncounterHealth::Remembered {
                    return invalid("v2 remembered health needs a spawn into a named role");
                }
                match into {
                    ProjectV2TransformInto::Creature { creature } => {
                        reference(creature, ProjectV2Family::Creature, require_ref)?;
                    }
                    ProjectV2TransformInto::NextStage => {}
                    ProjectV2TransformInto::RandomOf { creatures } => {
                        if creatures.len() < 2 {
                            return invalid("v2 transform random_of needs two creatures");
                        }
                        sorted_refs(creatures, ProjectV2Family::Creature, require_ref, limits)?;
                    }
                }
            }
            A::Heal {
                subject: who,
                amount: healed,
            } => {
                subject(who, names)?;
                if let ProjectV2HealAmount::Range { min, max } = healed
                    && (min > max || *max == 0)
                {
                    return invalid("v2 encounter amount is out of range");
                }
            }
            A::Damage {
                subject: who,
                amount: dealt,
                damage_type,
            } => {
                subject(who, names)?;
                amount(*dealt, 1)?;
                name(damage_type)?;
            }
            A::PreventDeath { role } | A::SharedLife { role } | A::MoveLock { role, .. } => {
                names.role(role)?;
            }
            A::DamageModifier {
                role,
                multiplier,
                damage_types: types,
                until,
                timer,
                ..
            } => {
                names.role(role)?;
                damage_types(types, limits)?;
                if let ProjectV2DamageMultiplier::TimerRemaining {
                    timer,
                    floor_percent,
                } = multiplier
                {
                    names.timer(timer)?;
                    if *floor_percent > 100 {
                        return invalid("v2 timer_remaining floor exceeds 100%");
                    }
                }
                match (until, timer) {
                    (ProjectV2ModifierUntil::Timer, Some(timer)) => names.timer(timer)?,
                    (ProjectV2ModifierUntil::Timer, None) | (_, Some(_)) => {
                        return invalid("v2 damage_modifier until timer needs exactly its timer");
                    }
                    _ => {}
                }
            }
            A::ReflectDamage {
                role,
                percent,
                damage_types: types,
            } => {
                names.role(role)?;
                damage_types(types, limits)?;
                if *percent == 0 || *percent > 100 {
                    return invalid("v2 reflect percent is outside 1..=100");
                }
            }
            A::ConvertDamageToHeal {
                role,
                damage_types: types,
                ..
            } => {
                names.role(role)?;
                damage_types(types, limits)?;
            }
            A::Teleport { who, to } => {
                match who {
                    ProjectV2TeleportWho::Role { role } => names.role(role)?,
                    ProjectV2TeleportWho::PlayersIn { anchor } => names.area(anchor)?,
                    ProjectV2TeleportWho::Triggering if triggering_creature(fired) => {}
                    ProjectV2TeleportWho::Triggering => {
                        return invalid(
                            "v2 teleport of triggering needs a creature or area trigger",
                        );
                    }
                }
                names.anchor(to)?;
            }
            A::MapItem {
                operation,
                item,
                triggering,
                into,
                anchor,
                at_death_position,
                revert_after_ms,
                destination,
                revert_destination,
                effect,
                interaction,
            } => {
                if *triggering {
                    if *operation != ProjectV2MapItemOperation::Remove
                        || !matches!(fired, ProjectV2EncounterTrigger::SteppedOn { .. })
                    {
                        return invalid("v2 map_item triggering is a remove in a stepped_on rule");
                    }
                    if item.is_some()
                        || into.is_some()
                        || anchor.is_some()
                        || *at_death_position
                        || destination.is_some()
                        || revert_after_ms.is_some()
                        || revert_destination.is_some()
                        || effect.is_some()
                        || interaction.is_some()
                    {
                        return invalid("v2 map_item triggering takes no other field");
                    }
                    continue;
                }
                let Some(item) = item else {
                    return invalid("v2 map_item needs an item");
                };
                reference(item, ProjectV2Family::Item, require_ref)?;
                match (operation, into) {
                    (ProjectV2MapItemOperation::Transform, Some(into)) => {
                        reference(into, ProjectV2Family::Item, require_ref)?;
                    }
                    (ProjectV2MapItemOperation::Transform, None) | (_, Some(_)) => {
                        return invalid(
                            "v2 map_item transform needs into, create and remove forbid it",
                        );
                    }
                    _ => {}
                }
                match (anchor, at_death_position) {
                    (Some(anchor), false) => names.anchor(anchor)?,
                    (None, true) => {}
                    _ => {
                        return invalid(
                            "v2 map_item takes exactly one of anchor and the death position",
                        );
                    }
                }
                if *revert_after_ms == Some(0) {
                    return invalid("v2 map_item revert must be positive");
                }
                for target in [destination, revert_destination].into_iter().flatten() {
                    names.anchor(target)?;
                }
                if revert_destination.is_some() && revert_after_ms.is_none() {
                    return invalid("v2 map_item revert_destination needs revert_after_ms");
                }
                if let Some(effect) = effect {
                    text(effect, limits)?;
                }
                if let Some(interaction) = interaction {
                    token(interaction, limits)?;
                }
            }
            A::Counter { counter, .. } => names.counter(counter)?,
            A::Flag { flag, .. } => {
                known(&names.flags, flag, "v2 encounter names an unknown flag")?;
            }
            A::Timer {
                timer,
                operation,
                ms,
            } => {
                names.timer(timer)?;
                match (operation, ms) {
                    (ProjectV2TimerOperation::Add, Some(ms)) if *ms > 0 => {}
                    (ProjectV2TimerOperation::Start | ProjectV2TimerOperation::Stop, None) => {}
                    _ => return invalid("v2 timer add needs exactly a positive ms"),
                }
            }
            A::SetPhase { phase } => {
                known(&names.phases, phase, "v2 encounter names an unknown phase")?;
            }
            A::Attribute {
                role,
                operation,
                value,
                ..
            } => {
                names.role(role)?;
                match (operation, value) {
                    (
                        ProjectV2AttributeOperation::Add,
                        Some(ProjectV2AttributeValue::Fixed { value }),
                    ) if *value > 0 => {}
                    (
                        ProjectV2AttributeOperation::Add,
                        Some(ProjectV2AttributeValue::Counter { counter }),
                    ) => names.counter(counter)?,
                    (ProjectV2AttributeOperation::Reset, None) => {}
                    _ => {
                        return invalid(
                            "v2 attribute add needs a positive value, a reset takes none",
                        );
                    }
                }
            }
            A::Cast {
                ability,
                encounter_ability,
                at,
            } => {
                match (ability, encounter_ability) {
                    (Some(ability), None) => {
                        reference(ability, ProjectV2Family::Ability, require_ref)?;
                    }
                    (None, Some(local)) => known(
                        &names.abilities,
                        local,
                        "v2 encounter names an unknown encounter ability",
                    )?,
                    _ => {
                        return invalid(
                            "v2 cast takes exactly one of ability and encounter_ability",
                        );
                    }
                }
                position(at, names)?;
            }
            A::Say {
                subject: who,
                text: said,
                ..
            } => {
                subject(who, names)?;
                text(said, limits)?;
            }
            A::DropItem { item, at } => {
                reference(item, ProjectV2Family::Item, require_ref)?;
                position(at, names)?;
            }
            A::Message {
                players_in,
                text: said,
            } => {
                names.area(players_in)?;
                text(said, limits)?;
            }
            A::OneOf { branches } => {
                if branches.len() < 2 {
                    return invalid("v2 one_of needs two branches");
                }
                limits.check(
                    "v2 encounter branches",
                    branches.len(),
                    limits.max_reference_records,
                )?;
                for branch in branches {
                    if branch.weight == 0 {
                        return invalid("v2 one_of weight must be positive");
                    }
                    actions(&branch.actions, fired, names, require_ref, limits)?;
                }
            }
            A::EmitOutcome {
                outcome,
                credited,
                anchor,
            } => {
                known(
                    &names.outcomes,
                    outcome,
                    "v2 encounter names an unknown outcome",
                )?;
                match (credited, anchor) {
                    (ProjectV2OutcomeCredit::PlayersInAnchor, Some(anchor)) => {
                        names.area(anchor)?
                    }
                    (ProjectV2OutcomeCredit::PlayersInAnchor, None) | (_, Some(_)) => {
                        return invalid("v2 players_in_anchor credit needs exactly its anchor");
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}
