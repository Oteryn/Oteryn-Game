//! Bounded world effects. The Quest writer owns durable outfit grants; Channel owns projections.
use super::*;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Facing {
    North,
    East,
    South,
    West,
}

/// A once-only earned mount through the existing account appearance owner.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MountReward {
    pub mount: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub achievement: Option<String>,
}

impl MountReward {
    pub fn grant_key(&self) -> String {
        format!("{}/quest-grant", self.mount)
    }
}

/// A source-authored, nonportable object distinct from the interaction target.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TargetTransform {
    pub position: Position,
    pub from: u16,
    pub to: u16,
    pub reset_after_ms: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemDisguiseRequirement {
    pub item: String,
    pub condition: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CallbackEffects {
    /// Individual admission to an encounter already opened by its source owner.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boss_admission_cooldown: Option<BossAdmissionCooldown>,
    /// Match the exact creature entering this source tile and remove it from the room.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub monster_step: Option<MonsterStepEffect>,
    /// Presence of Channel-created scene items at every authored layout cell.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub generated_item_requirements: Vec<ItemPresence>,
    /// Distinct carried definitions consumed atomically by the Quest receipt owner.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carried_inputs: Vec<super::super::reward_callbacks::ItemAmount>,
    /// Opt in to source-script displacement onto a present blocked pad.
    #[serde(default, skip_serializing_if = "is_false")]
    pub scripted_teleport: bool,
    /// Current native occupants must be absent before a previously started stage commits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear_area: Option<ClearArea>,
    /// Finish this quest only through its bound, irreversible native trial.
    #[serde(default, skip_serializing_if = "is_false")]
    pub completes_quest: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requires_item_disguise: Option<ItemDisguiseRequirement>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub restore_disguise: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mount_reward: Option<MountReward>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_reward: Option<super::super::reward_callbacks::ItemReward>,
    /// An existing, preserved local quest epoch; commands never create reward cycles.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_reward_cycle: Option<String>,
    /// Source-authored renewable pickup: one mint for each admitted USE command.
    #[serde(default, skip_serializing_if = "is_false")]
    pub item_reward_per_use: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quest_completion: Option<QuestCompletion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party_transfer: Option<PartyTransfer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reward_door: Option<RewardDoor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outfit_reward: Option<super::super::reward_callbacks::OutfitReward>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<super::SceneEdits>,
    /// Source-authored public player MONSTER_SAY, emitted only after commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_speech: Option<String>,
    pub transform: Option<u16>,
    /// Restore the exact pre-transform source revision after this delay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_after_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_transform: Option<TargetTransform>,
    /// Restore remote passage objects independently of the source object.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passages_reset_after_ms: Option<u32>,
    pub facing: Option<Facing>,
    pub magic: Option<MagicEffect>,
    /// One cue at a committed landing, or at the restored STEP origin on refusal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teleport_effect: Option<u16>,
    pub portals: Option<PortalPair>,
    pub item_cycle: Option<ItemCycle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_presence: Option<ItemPresence>,
    pub encounter: Option<Encounter>,
}

/// The original wire shapes retain distinct owners: a child quest or the current quest.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(untagged, deny_unknown_fields)]
pub enum QuestCompletion {
    Child(String),
    Current {
        #[serde(default)]
        claimed_rewards: Vec<String>,
    },
}

impl QuestCompletion {
    pub fn child_quest(&self) -> Option<&str> {
        match self {
            Self::Child(quest) => Some(quest),
            Self::Current { .. } => None,
        }
    }

    pub fn claimed_rewards(&self) -> Option<&[String]> {
        match self {
            Self::Current { claimed_rewards } => Some(claimed_rewards),
            Self::Child(_) => None,
        }
    }
}

impl From<String> for QuestCompletion {
    fn from(quest: String) -> Self {
        Self::Child(quest)
    }
}

impl From<&str> for QuestCompletion {
    fn from(quest: &str) -> Self {
        Self::Child(quest.to_owned())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RewardDoor {
    pub claim: String,
}

impl CallbackEffects {
    pub(super) fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Preserve the existing positional magic array alongside the shared engine's
/// structured effects. Each wire form keeps its original receipt serialization.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CallbackEffectSet {
    shared: CallbackEffects,
    legacy: Vec<CallbackEffect>,
}

impl CallbackEffectSet {
    pub(super) fn from_shared(shared: CallbackEffects) -> Self {
        Self {
            shared,
            legacy: Vec::new(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.shared.is_empty() && self.legacy.is_empty()
    }

    pub fn has_shared_effects(&self) -> bool {
        !self.shared.is_empty()
    }

    pub fn len(&self) -> usize {
        self.legacy.len()
    }

    pub fn iter(&self) -> std::slice::Iter<'_, CallbackEffect> {
        self.legacy.iter()
    }
}

impl std::ops::Deref for CallbackEffectSet {
    type Target = CallbackEffects;
    fn deref(&self) -> &Self::Target {
        &self.shared
    }
}

impl std::ops::DerefMut for CallbackEffectSet {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.shared
    }
}

impl<'a> IntoIterator for &'a CallbackEffectSet {
    type Item = &'a CallbackEffect;
    type IntoIter = std::slice::Iter<'a, CallbackEffect>;
    fn into_iter(self) -> Self::IntoIter {
        self.legacy.iter()
    }
}

impl<'de> Deserialize<'de> for CallbackEffectSet {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Wire {
            Legacy(Vec<CallbackEffect>),
            Shared(Box<CallbackEffects>),
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Legacy(legacy) => Self {
                legacy,
                ..Self::default()
            },
            Wire::Shared(shared) => Self {
                shared: *shared,
                ..Self::default()
            },
        })
    }
}

impl Serialize for CallbackEffectSet {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.legacy.is_empty() {
            self.shared.serialize(serializer)
        } else if self.shared.is_empty() {
            self.legacy.serialize(serializer)
        } else {
            Err(serde::ser::Error::custom(
                "mixed legacy and structured callback effects",
            ))
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MagicEffect {
    pub position: Position,
    pub effect: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Portal {
    pub position: Position,
    pub destination: Position,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PortalPair {
    pub appearance: u16,
    pub portals: [Portal; 2],
    /// None means until the matching lever removes the pair or the Channel ends.
    pub lifetime_us: Option<u64>,
}

/// An immovable walkable projection owned by its source callback, never an Item instance.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemPresence {
    pub position: Position,
    pub appearance: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemCycle {
    pub position: Position,
    pub remove: u16,
    pub create: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EncounterPlayer {
    pub from: Position,
    pub to: Position,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EncounterMonster {
    pub position: Position,
    pub creature: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BossAdmissionCooldown {
    pub creature: String,
    pub seconds: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MonsterStepEffect {
    pub creature: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub replace_with: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub consume_source: bool,
}

impl MonsterStepEffect {
    fn valid(&self, callback: &Callback) -> bool {
        let creature =
            |key: &str| key.starts_with("oteryn:creature.") && super::super::valid_quest_key(key);
        callback.actor_kind == ActorKind::Monster
            && callback.trigger == Trigger::StepIn
            && !callback.reversible
            && (callback.selector.action.is_some() || callback.selector.unique.is_some())
            && creature(&self.creature)
            && (!self.consume_source || self.replace_with.is_some())
            && self
                .replace_with
                .as_ref()
                .is_none_or(|key| key != &self.creature && creature(key))
            && *callback.effects
                == CallbackEffects {
                    monster_step: Some(self.clone()),
                    ..CallbackEffects::default()
                }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Encounter {
    pub minimum_level: u32,
    pub players: Vec<EncounterPlayer>,
    pub monsters: Vec<EncounterMonster>,
    pub room_min: Position,
    pub room_max: Position,
    /// A value-free source switch schedules native monsters without moving a party.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<EncounterTiming>,
    /// Source phases run on the shared encounter owner after atomic party admission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<EncounterPhase>,
    /// Exact encounter-owned death transitions and their terminal creature.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub death_phases: Option<DeathPhases>,
    /// Use every occupied staging tile and permit a shared party landing.
    #[serde(default, skip_serializing_if = "is_false")]
    pub shared_staging: bool,
    /// Cooldown of the sole boss admitted by a fenced personal StepIn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal_cooldown_seconds: Option<u32>,
    /// Expire the visit and return the admitted participant to the original step tile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personal_timeout_ms: Option<u32>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum EncounterPhase {
    #[serde(rename = "cults_boss_phase_closures")]
    Cults,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeathPhases {
    pub terminal: String,
    pub replacements: Vec<DeathReplacement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_portal: Option<TerminalPortal>,
    /// Room movement on the sealed terminal death. Quest credit uses its own writer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_transfer: Option<TerminalTransfer>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TerminalPortal {
    pub position: Position,
    pub original_appearance: u16,
    pub open_appearance: u16,
    pub destination: Position,
    pub return_destination: Position,
    pub duration_ms: u32,
}

impl TerminalPortal {
    fn valid(self) -> bool {
        [self.position, self.destination, self.return_destination]
            .iter()
            .all(|p| (-15..=0).contains(&p.floor))
            && self.original_appearance != 0
            && self.open_appearance != 0
            && self.original_appearance != self.open_appearance
            && self.position != self.destination
            && self.position != self.return_destination
            && self.destination != self.return_destination
            && (1..=3_600_000).contains(&self.duration_ms)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TerminalTransfer {
    pub area_min: Position,
    pub area_max: Position,
    pub destination: Position,
}

impl TerminalTransfer {
    pub fn contains(self, position: Position) -> bool {
        position.floor == self.area_min.floor
            && (self.area_min.x..=self.area_max.x).contains(&position.x)
            && (self.area_min.y..=self.area_max.y).contains(&position.y)
    }

    fn valid(self) -> bool {
        let pos = |position: Position| (-15..=0).contains(&position.floor);
        pos(self.area_min)
            && pos(self.area_max)
            && pos(self.destination)
            && self.area_min.floor == self.area_max.floor
            && self
                .area_max
                .x
                .checked_sub(self.area_min.x)
                .is_some_and(|span| span <= 64)
            && self
                .area_max
                .y
                .checked_sub(self.area_min.y)
                .is_some_and(|span| span <= 64)
            && !self.contains(self.destination)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeathReplacement {
    pub defeated: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub retire_siblings: bool,
    pub spawn: Vec<ReplacementActor>,
    /// Move an existing encounter member only if it still occupies the source tile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relocate: Option<DeathRelocation>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeathRelocation {
    pub creature: String,
    pub from: Position,
    pub to: Position,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReplacementActor {
    pub creature: String,
    /// Omit the position to use the exact defeated actor's tile.
    pub position: Option<Position>,
}

impl Encounter {
    pub fn has_phases(&self) -> bool {
        self.phase.is_some() || self.death_phases.is_some()
    }

    pub fn contains(&self, position: Position) -> bool {
        (self.room_min.x..=self.room_max.x).contains(&position.x)
            && (self.room_min.y..=self.room_max.y).contains(&position.y)
            && (self.room_max.floor..=self.room_min.floor).contains(&position.floor)
    }
}

fn valid_death_phases(
    encounter: &Encounter,
    plan: &DeathPhases,
    in_room: impl Fn(Position) -> bool,
) -> bool {
    use super::super::valid_quest_key;
    if encounter.phase.is_some()
        || encounter.timing.is_some()
        || encounter.personal_cooldown_seconds.is_some()
        || !valid_quest_key(&plan.terminal)
        || plan.replacements.len() > 8
        || (plan.replacements.is_empty()
            && plan.terminal_portal.is_none()
            && plan.terminal_transfer.is_none())
        || plan
            .terminal_portal
            .is_some_and(|portal| !portal.valid() || !in_room(portal.position))
        || plan
            .terminal_transfer
            .is_some_and(|transfer| !transfer.valid())
    {
        return false;
    }
    let mut known: BTreeSet<_> = encounter
        .monsters
        .iter()
        .map(|actor| actor.creature.as_str())
        .collect();
    let mut defeated = BTreeSet::new();
    for rule in &plan.replacements {
        if rule.defeated == plan.terminal
            || !known.contains(rule.defeated.as_str())
            || !defeated.insert(rule.defeated.as_str())
            || if let Some(relocate) = &rule.relocate {
                !rule.spawn.is_empty()
                    || rule.retire_siblings
                    || !known.contains(relocate.creature.as_str())
                    || relocate.creature == rule.defeated
                    || !in_room(relocate.from)
                    || !in_room(relocate.to)
                    || relocate.from == relocate.to
            } else {
                !(1..=8).contains(&rule.spawn.len())
            }
        {
            return false;
        }
        for actor in &rule.spawn {
            if !valid_quest_key(&actor.creature)
                || defeated.contains(actor.creature.as_str())
                || actor.position.is_some_and(|position| !in_room(position))
            {
                return false;
            }
            known.insert(&actor.creature);
        }
    }
    known.contains(plan.terminal.as_str())
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EncounterTiming {
    pub spawn_after_ms: u32,
    pub reset_after_ms: u32,
}

/// A source rite transports the exact players on bounded stations. The initiator
/// may stand elsewhere; no social party or monster room is inferred.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PartyTransfer {
    /// Run the rite only after the lever changes to this source appearance.
    pub activation_appearance: u16,
    pub stations: Vec<PartyStation>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PartyStation {
    pub from: Position,
    pub to: Position,
    /// Base vocation; promoted members retain their base eligibility.
    pub vocation: String,
    pub offering: Position,
}

fn valid_party_transfer(callback: &Callback, party: &PartyTransfer) -> bool {
    let pos = |p: Position| (-15..=0).contains(&p.floor);
    let unique = |positions: Vec<Position>| {
        positions.iter().copied().collect::<BTreeSet<_>>().len() == positions.len()
    };
    callback.trigger == Trigger::Use
        && callback
            .lever
            .is_some_and(|pair| pair.contains(&party.activation_appearance))
        && callback.reversible
        && !callback.storage.is_empty()
        && callback.offerings.len() == party.stations.len()
        && callback.item_on_target.is_none()
        && callback.minimum_level.is_none()
        && callback.experience.is_none()
        && callback.monk_reward.is_none()
        && callback.native_rule.is_none()
        && callback.passages.is_empty()
        && callback.teleport.is_none()
        && callback.effects.outfit_reward.is_none()
        && callback.effects.transform.is_none()
        && callback.effects.facing.is_none()
        && callback.effects.magic.is_none()
        && (1..=8).contains(&party.stations.len())
        && unique(party.stations.iter().map(|s| s.from).collect())
        && unique(party.stations.iter().map(|s| s.to).collect())
        && unique(party.stations.iter().map(|s| s.offering).collect())
        && party.stations.iter().all(|station| {
            pos(station.from)
                && pos(station.to)
                && pos(station.offering)
                && station.from != station.to
                && matches!(
                    station.vocation.as_str(),
                    "druid" | "sorcerer" | "paladin" | "knight" | "monk"
                )
                && callback
                    .offerings
                    .iter()
                    .filter(|o| o.position == station.offering && o.count == 1)
                    .count()
                    == 1
        })
}

pub(super) fn valid(callback: &Callback) -> bool {
    let effects = &callback.effects;
    if effects
        .boss_admission_cooldown
        .as_ref()
        .is_some_and(|cooldown| {
            callback.trigger != Trigger::StepIn
                || callback.actor_kind != ActorKind::Player
                || !callback.requires_world_admission
                || !callback.has_state_writes()
                || !callback.return_on_failure
                || callback.teleport.is_none()
                || effects.encounter.is_some()
                || !cooldown.creature.starts_with("oteryn:creature.")
                || !super::super::valid_quest_key(&cooldown.creature)
                || !(1..=30 * 24 * 60 * 60).contains(&cooldown.seconds)
        })
    {
        return false;
    }
    let inputs = &effects.carried_inputs;
    let unique: BTreeSet<_> = inputs.iter().map(|input| &input.item).collect();
    if inputs.len() > 8
        || unique.len() != inputs.len()
        || inputs.iter().any(|input| {
            input.count == 0
                || input.count > 100
                || !super::super::valid_quest_key(&input.item)
                || callback
                    .item_on_target
                    .as_ref()
                    .is_some_and(|target| target.item == input.item)
        })
        || (!inputs.is_empty()
            && (callback.trigger != Trigger::Use
                || !callback.has_state()
                || callback.is_read_only()
                || effects.scene.is_some()
                || effects.portals.is_some()
                || effects.encounter.is_some()
                || effects.item_cycle.is_some()))
    {
        return false;
    }
    if effects
        .scene
        .as_ref()
        .is_some_and(|scene| !super::scene::valid_scene(callback, scene))
    {
        return false;
    }
    let paid_craft = effects
        .item_reward_cycle
        .as_deref()
        .is_some_and(|track| callback.craft_reward_epoch(track));
    if effects.item_reward.as_ref().is_some_and(|reward| {
        (callback.trigger != Trigger::Use && !callback.has_fenced_step_reward())
            || callback.is_read_only()
            || (!callback.offerings.is_empty() && !paid_craft)
            || callback.item_on_target.as_ref().is_some_and(|input| {
                input.reusable || input.replacement.is_some() || input.target_replacement.is_some()
            })
            || !callback.has_state()
            || (!callback.has_repeating_clock()
                && !super::per_use_reward::admitted(callback)
                && !callback.has_fenced_regrowth_reward()
                && !paid_craft
                && callback
                    .storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set)))
            || !super::super::reward_callbacks::valid_item_reward(reward)
    }) || (callback.completes && (!callback.has_state() || callback.is_read_only()))
    {
        return false;
    }

    if effects.scripted_teleport
        && (callback.teleport.is_none()
            || callback.selector.native.is_none()
            || callback.actor_kind != ActorKind::Player)
    {
        return false;
    }
    if effects.item_reward_per_use && !super::per_use_reward::admitted(callback) {
        return false;
    }
    if effects.player_speech.as_ref().is_some_and(|text| {
        callback.actor_kind != ActorKind::Player
            || text.is_empty()
            // CHAT0-RL-01: 255 Unicode scalars, each at most four UTF-8 bytes.
            || text.len() > 1_020
            || text.chars().count() > 255
            || text.trim() != text.as_str()
            || text.chars().any(char::is_control)
    }) {
        return false;
    }
    if effects
        .clear_area
        .as_ref()
        .is_some_and(|area| !area.valid(callback))
    {
        return false;
    }
    if effects
        .requires_item_disguise
        .as_ref()
        .is_some_and(|required| {
            !required.item.starts_with("oteryn:item.tibia.i")
                || required.item.len() > 128
                || !required.condition.starts_with("quest.")
                || required.condition.len() > 128
                || callback.trigger != Trigger::StepIn
                || callback.actor_kind != ActorKind::Player
                || callback.teleport.is_none()
                || !callback.has_personal_state()
        })
        || (effects.restore_disguise
            && (callback.trigger != Trigger::StepIn
                || callback.actor_kind != ActorKind::Player
                || callback.teleport.is_none()))
    {
        return false;
    }
    if effects.completes_quest
        && (!callback.requires_world_admission
            || callback.actor_kind != ActorKind::Player
            || callback.reversible
            || callback.opening_only
            || callback.storage.iter().all(|state| state.preserve)
            || effects.quest_completion.is_some()
            || effects.item_reward_cycle.is_some())
    {
        return false;
    }
    if effects.item_reward_cycle.as_ref().is_some_and(|track| {
        effects.item_reward.is_none()
            || !callback.storage.iter().any(|state| {
                state.track == *track
                    && state.preserve
                    && !state.increment
                    && !state.at_least
                    && state.minimum >= 0
            }) && !callback.clock_reward_epoch(track)
                && !callback.craft_reward_epoch(track)
    }) {
        return false;
    }
    if effects
        .quest_completion
        .as_ref()
        .and_then(QuestCompletion::child_quest)
        .is_some_and(|quest| {
            !super::super::valid_quest_key(quest)
                || !quest.starts_with("oteryn:quest.")
                || quest == callback.quest
                || callback.trigger != Trigger::Use
                || callback.reversible
                || callback.opening_only
                || callback.actor_kind != ActorKind::Player
                || effects.item_reward.is_none()
                || effects.item_reward_cycle.is_some()
                || callback.storage.iter().all(|s| s.preserve)
        })
    {
        return false;
    }
    let pos = |p: Position| (-15..=0).contains(&p.floor);
    let lever = callback.trigger == Trigger::Use && callback.lever.is_some();
    // Crystal 201189f8, ferumbras_ascension/actions_flower_puzzle_lever.lua:1-11,30-37.
    let generated = &effects.generated_item_requirements;
    if !generated.is_empty()
        && (callback.trigger != Trigger::Use
            || generated.len() > 9
            || generated
                .iter()
                .map(|item| item.position)
                .collect::<BTreeSet<_>>()
                .len()
                != generated.len()
            || generated
                .iter()
                .any(|item| item.appearance == 0 || !pos(item.position)))
    {
        return false;
    }
    if effects
        .monster_step
        .as_ref()
        .is_some_and(|step| !step.valid(callback))
    {
        return false;
    }
    if callback.item_reward().as_ref().is_some_and(|reward| {
        let item = |key: &str, count: u16| {
            key.len() <= 256
                && key.strip_prefix("oteryn:item.").is_some_and(|tail| {
                    !tail.is_empty()
                        && tail.bytes().all(|b| {
                            b.is_ascii_lowercase() || b.is_ascii_digit() || b"._/:-".contains(&b)
                        })
                })
                && (1..=100).contains(&count)
        };
        (callback.trigger != Trigger::Use && !callback.has_fenced_step_reward())
            || !callback.has_personal_state()
            || (!effects.item_reward_per_use
                && !callback.has_fenced_regrowth_reward()
                && (callback
                    .storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set))
                    && !callback.operations().any(|operation| {
                        matches!(
                            operation.effect,
                            crate::quest::reward_callbacks::StateMutation::SetNow
                                | crate::quest::reward_callbacks::StateMutation::Add { value: 1.. }
                                | crate::quest::reward_callbacks::StateMutation::Add {
                                    value: ..=-1
                                }
                        )
                    })
                    && !effects
                        .item_reward_cycle
                        .as_deref()
                        .is_some_and(|track| callback.craft_reward_epoch(track))))
            || !item(&reward.item, reward.count)
            || !reward.valid_literal_text()
            || reward.additional.len() > 19
            || reward
                .additional
                .iter()
                .any(|c| !item(&c.item, c.count) || c.fluid_subtype.is_some())
            || reward.contents.len() > 20
            || reward.contents.iter().any(|c| {
                !item(&c.item, c.count) || c.fluid_subtype.is_some_and(|f| f == 0 || c.count != 1)
            })
    }) || (callback.facing.is_some() && effects.facing.is_some())
        || effects.outfit_reward.as_ref().is_some_and(|r| {
            callback.trigger != Trigger::Use
                || !callback.has_state()
                || callback
                    .storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set))
                || r.addons > 3
                || r.outfit.len() > 160
                || !r.outfit.strip_prefix("oteryn:outfit.").is_some_and(|tail| {
                    !tail.is_empty()
                        && tail
                            .bytes()
                            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                })
        })
        || effects.mount_reward.as_ref().is_some_and(|r| {
            !callback.requires_world_admission
                || callback.trigger != Trigger::Use
                || callback.storage.is_empty()
                || callback
                    .storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set))
                || r.achievement.as_ref().is_some_and(|key| {
                    key.len() > 160
                        || !key.strip_prefix("oteryn:achievement/").is_some_and(|tail| {
                            !tail.is_empty()
                                && tail.bytes().all(|b| {
                                    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'
                                })
                        })
                })
                || r.mount.len() > 160
                || !r
                    .mount
                    .strip_prefix("oteryn:content.mount.")
                    .is_some_and(|tail| {
                        !tail.is_empty()
                            && tail
                                .bytes()
                                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                    })
        })
        || effects.reset_after_ms.is_some_and(|delay| {
            !(1..=3_600_000).contains(&delay)
                || callback.actor_kind != ActorKind::Player
                || (effects.transform.is_none() && callback.lever.is_none())
                || effects.portals.is_some()
                || effects.item_presence.is_some()
                || effects.item_cycle.is_some()
                || effects.encounter.is_some()
                || (!callback.passages.is_empty() && effects.passages_reset_after_ms.is_none())
        })
        || effects.passages_reset_after_ms.is_some_and(|delay| {
            !(1..=3_600_000).contains(&delay)
                || callback.actor_kind != ActorKind::Player
                || callback.trigger != Trigger::Use
                || callback.reversible
                || callback.passages.is_empty()
                || callback
                    .passages
                    .iter()
                    .any(|p| p.create_on_close.is_some())
                || effects.portals.is_some()
                || effects.item_presence.is_some()
                || effects.item_cycle.is_some()
                || effects.encounter.is_some()
        })
        || effects.mount_reward.as_ref().is_some_and(|r| {
            !callback.requires_world_admission
                || callback.trigger != Trigger::Use
                || callback.storage.is_empty()
                || callback
                    .storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set))
                || r.achievement.as_ref().is_some_and(|key| {
                    key.len() > 160
                        || !key.strip_prefix("oteryn:achievement/").is_some_and(|tail| {
                            !tail.is_empty()
                                && tail.bytes().all(|b| {
                                    b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'
                                })
                        })
                })
                || r.mount.len() > 160
                || !r
                    .mount
                    .strip_prefix("oteryn:content.mount.")
                    .is_some_and(|tail| {
                        !tail.is_empty()
                            && tail
                                .bytes()
                                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
                    })
        })
        || effects.target_transform.as_ref().is_some_and(|target| {
            !pos(target.position)
                || target.from == 0
                || target.to == 0
                || target.from == target.to
                || !(1..=3_600_000).contains(&target.reset_after_ms)
                || callback.actor_kind != ActorKind::Player
                || (callback.trigger != Trigger::Use && !callback.has_fenced_step_reward())
                || !callback.has_personal_state()
                || (callback.item_on_target.is_none() && !callback.has_fenced_step_reward())
                || callback
                    .selector
                    .native
                    .as_ref()
                    .is_none_or(|source| source.position == target.position)
                || callback.lever.is_some()
                || callback.reversible
                || !callback.passages.is_empty()
                || effects.transform.is_some()
                || effects.reset_after_ms.is_some()
                || effects.portals.is_some()
                || effects.item_presence.is_some()
                || effects.item_cycle.is_some()
                || effects.encounter.is_some()
        })
        || effects.transform == Some(0)
        || (effects.facing.is_some() && callback.teleport.is_none())
        || effects
            .magic
            .as_ref()
            .is_some_and(|m| !pos(m.position) || m.effect == 0)
        || (callback.trigger == Trigger::StepOut && callback.return_on_failure)
    {
        return false;
    }
    if effects.item_presence.as_ref().is_some_and(|presence| {
        !pos(presence.position)
            || presence.appearance == 0
            || (callback.has_state()
                && !callback.requires_world_admission
                && !callback.opening_only)
            || effects.portals.is_some()
            || effects.item_cycle.is_some()
            || effects.encounter.is_some()
    }) {
        return false;
    }
    let resets_scene = effects
        .scene
        .as_ref()
        .is_some_and(|scene| super::scene::is_encounter_reset(callback, scene));
    let operations = usize::from(effects.portals.is_some())
        + usize::from(effects.item_cycle.is_some())
        + usize::from(effects.encounter.is_some())
        + usize::from(effects.scene.is_some() && !resets_scene)
        + usize::from(effects.reward_door.is_some())
        + usize::from(effects.party_transfer.is_some());
    if let Some(claimed_rewards) = effects
        .quest_completion
        .as_ref()
        .and_then(QuestCompletion::claimed_rewards)
    {
        let unique: std::collections::BTreeSet<_> = claimed_rewards.iter().collect();
        if claimed_rewards.len() > 8
            || unique.len() != claimed_rewards.len()
            || claimed_rewards.iter().any(|key| {
                !super::super::valid_quest_key(key) || !key.starts_with("oteryn:reward-claim.")
            })
            || callback.storage.is_empty()
            || callback.storage.iter().all(|storage| {
                storage.preserve || (storage.minimum..=storage.maximum).contains(&storage.set)
            })
            || callback.reversible
            || callback.native_rule.is_some()
            || operations > 0
            || effects.outfit_reward.is_some()
            || callback.monk_reward.is_some()
        {
            return false;
        }
    }
    let personal_encounter = effects.encounter.as_ref().is_some_and(|encounter| {
        callback.has_personal_state()
            && callback.requires_world_admission
            && callback.actor_kind == ActorKind::Player
            && matches!(callback.trigger, Trigger::Use | Trigger::StepIn)
            && callback.lever.is_none()
            && !callback.opening_only
            && !callback.reversible
            && callback.offerings.is_empty()
            && callback.item_on_target.is_none()
            && encounter.players.len() == 1
            && encounter.timing.is_none()
            && effects.item_reward.is_none()
            && effects.outfit_reward.is_none()
    });
    if operations > 1
        || (operations > 0
            && ((!personal_encounter
                && !super::personal_trial::admitted(callback)
                && (!lever
                    || (effects.scene.is_none()
                        && effects.party_transfer.is_none()
                        && callback.has_state())))
                || !callback.passages.is_empty()
                || callback.teleport.is_some())
            && !resets_scene
            && !(effects.encounter.is_some() && super::asura_authored::admitted(callback)))
    {
        return false;
    }
    if effects
        .party_transfer
        .as_ref()
        .is_some_and(|party| !valid_party_transfer(callback, party))
    {
        return false;
    }
    if let Some(door) = &effects.reward_door
        && (!super::super::valid_quest_key(&door.claim)
            || !door.claim.starts_with("oteryn:reward-claim.")
            || !callback.reversible
            || callback.selector.action.is_none()
            || !callback.offerings.is_empty()
            || callback.item_on_target.is_some()
            || effects.outfit_reward.is_some()
            || effects.transform.is_some()
            || effects.facing.is_some()
            || effects.magic.is_some())
    {
        return false;
    }
    if let Some(pair) = &effects.portals
        && (pair.appearance == 0
            || pair.lifetime_us == Some(0)
            || pair.portals[0].position == pair.portals[1].position
            || pair
                .portals
                .iter()
                .any(|p| !pos(p.position) || !pos(p.destination)))
    {
        return false;
    }
    if let Some(cycle) = &effects.item_cycle
        && (!pos(cycle.position)
            || cycle.remove == 0
            || cycle.create == 0
            || cycle.remove == cycle.create)
    {
        return false;
    }
    if let Some(encounter) = &effects.encounter {
        let in_room = |p: Position| encounter.contains(p);
        let unique_from: BTreeSet<_> = encounter.players.iter().map(|p| p.from).collect();
        let unique_to: BTreeSet<_> = encounter.players.iter().map(|p| p.to).collect();
        let unique_monsters: BTreeSet<_> = encounter.monsters.iter().map(|p| p.position).collect();
        let source_party =
            encounter.phase == Some(EncounterPhase::Cults) || encounter.shared_staging;
        let personal_cooldown = encounter.personal_cooldown_seconds.is_some();
        if (encounter.minimum_level == 0 && !source_party && !personal_cooldown)
            || encounter.death_phases.as_ref().is_some_and(|plan| {
                callback.trigger != Trigger::Use
                    || callback.has_personal_state()
                    || !valid_death_phases(encounter, plan, in_room)
            })
            || (encounter.shared_staging
                && (callback.trigger != Trigger::Use
                    || encounter.timing.is_some()
                    || callback.has_personal_state()))
            || encounter.personal_cooldown_seconds.is_some_and(|seconds| {
                !(1..=30 * 24 * 60 * 60).contains(&seconds)
                    || callback.personal_room_entry().is_none()
                    || !callback.return_on_failure
                    || encounter.phase.is_some()
                    || encounter.monsters.len() != 1
            })
            || encounter.personal_timeout_ms.is_some_and(|milliseconds| {
                !(1..=3_600_000).contains(&milliseconds) || !personal_cooldown
            })
            || if let Some(timing) = encounter.timing {
                !encounter.players.is_empty()
                    || !callback.offerings.is_empty()
                    || callback.item_on_target.is_some()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.native_rule.is_some()
                    || callback.item_reward().is_some()
                    || effects.outfit_reward.is_some()
                    || effects.mount_reward.is_some()
                    || !(1..=60_000).contains(&timing.spawn_after_ms)
                    || timing.reset_after_ms <= timing.spawn_after_ms
                    || timing.reset_after_ms > 3_600_000
                    || (u64::from(encounter.room_max.x.saturating_sub(encounter.room_min.x)) + 1)
                        * (u64::from(encounter.room_max.y.saturating_sub(encounter.room_min.y)) + 1)
                        > 4096
            } else {
                !(1..=if encounter.phase.is_some() || encounter.shared_staging {
                    16
                } else {
                    8
                })
                    .contains(&encounter.players.len())
            }
            || !(1..=if encounter.phase.is_some() { 16 } else { 8 })
                .contains(&encounter.monsters.len())
            || (encounter.phase.is_some() && encounter.timing.is_some())
            || (callback.reversible && !source_party)
            || unique_from.len() != encounter.players.len()
            || (unique_to.len() != encounter.players.len() && !source_party)
            || unique_monsters.len() != encounter.monsters.len()
            || !unique_to.is_disjoint(&unique_monsters)
            || !pos(encounter.room_min)
            || !pos(encounter.room_max)
            // Lore Keeper stages one party against actors on Crystal floors 14 and 15.
            // actions_the_last_lore_keeper.lua:3-7,51-62 (Crystal 201189f8).
            || (encounter.room_min.floor != encounter.room_max.floor
                && (!encounter.shared_staging
                    || i16::from(encounter.room_min.floor) - i16::from(encounter.room_max.floor)
                        != 1))
            || encounter.room_min.x > encounter.room_max.x
            || encounter.room_min.y > encounter.room_max.y
            || encounter.players.iter().any(|p| {
                !pos(p.from)
                    || !pos(p.to)
                    || !in_room(p.to)
                    || (in_room(p.from) && callback.personal_room_entry() != Some(p.from))
            })
            || encounter.monsters.iter().any(|m| {
                !pos(m.position)
                    || !in_room(m.position)
                    || !super::super::valid_quest_key(&m.creature)
            })
        {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod reward_door_tests {
    use super::*;

    #[test]
    fn reward_door_checks_the_claim_without_shadow_progress_or_other_effects() -> Result<(), String>
    {
        let callback = super::super::load()?
            .into_iter()
            .find(|row| row.key == "oteryn:quest-callback/annihilator-reward-door")
            .ok_or("reward door")?;
        assert!(valid(&callback));
        for change in 0..5 {
            let mut invalid = callback.clone();
            match change {
                0 => {
                    invalid.effects.reward_door.as_mut().ok_or("door")?.claim =
                        "oteryn:quest.shadow".into()
                }
                1 => invalid.reversible = false,
                2 => invalid.selector.action = None,
                3 => invalid.effects.transform = Some(5114),
                _ => invalid.storage.push(Storage {
                    set_if: None,
                    track: "oteryn:quest.shadow".into(),
                    minimum: 0,
                    maximum: 1,
                    set: 0,
                    preserve: true,
                    increment: false,
                    at_least: false,
                    add: None,
                    read_only: false,
                }),
            }
            assert!(!valid(&invalid));
        }
        Ok(())
    }
}

#[cfg(test)]
mod party_transfer_tests {
    use super::*;

    #[test]
    fn source_party_rite_is_bounded_and_exclusive() -> Result<(), String> {
        let source = super::super::load()?
            .into_iter()
            .find(|row| row.key.ends_with("desert-dungeon-party-rite"))
            .ok_or("source rite")?;
        assert!(valid(&source));
        for change in 0..8 {
            let mut row = source.clone();
            match change {
                0 => row
                    .effects
                    .party_transfer
                    .as_mut()
                    .ok_or("party")?
                    .stations
                    .clear(),
                1 => {
                    let party = row.effects.party_transfer.as_mut().ok_or("party")?;
                    party.stations[1].to = party.stations[0].to;
                }
                2 => {
                    row.effects.party_transfer.as_mut().ok_or("party")?.stations[0].vocation =
                        "none".into()
                }
                3 => {
                    row.effects
                        .party_transfer
                        .as_mut()
                        .ok_or("party")?
                        .activation_appearance = 2774
                }
                4 => row.offerings.pop().map(|_| ()).ok_or("offering")?,
                5 => row.storage.clear(),
                6 => {
                    row.effects.encounter = super::super::load()?
                        .into_iter()
                        .find(|row| row.key.ends_with("annihilator-entry"))
                        .ok_or("encounter")?
                        .effects
                        .encounter
                        .clone()
                }
                7 => row.reversible = false,
                _ => unreachable!(),
            }
            assert!(!valid(&row), "invalid party change {change}");
        }
        Ok(())
    }
}

#[cfg(test)]
mod speech_tests {
    use super::*;

    #[test]
    fn qm46_public_speech_is_bounded_and_player_only() -> Result<(), String> {
        let original = super::super::load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/children-grease")
            .ok_or("grease callback")?;
        assert!(valid(&original));
        for text in [
            String::new(),
            "x".repeat(256),
            " whitespace ".into(),
            "tab\t".into(),
            "line\nbreak".into(),
            "nul\0".into(),
        ] {
            let mut callback = original.clone();
            callback.effects.player_speech = Some(text);
            assert!(!valid(&callback));
        }
        let mut monster = original;
        monster.actor_kind = ActorKind::Monster;
        assert!(!valid(&monster));
        Ok(())
    }
}

#[cfg(test)]
mod unicode_speech_tests {
    use super::*;

    #[test]
    fn qm46_authored_public_speech_accepts_chat_unicode_boundaries() -> Result<(), String> {
        let mut callback = super::super::load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/children-grease")
            .ok_or("grease callback")?;
        for scalar in ["x", "é", "界", "🦋"] {
            callback.effects.player_speech = Some(scalar.repeat(255));
            let document = serde_json::json!({"schema": 1, "callbacks": [&callback]});
            super::super::parse(&document.to_string())?;
            callback.effects.player_speech = Some(scalar.repeat(256));
            let document = serde_json::json!({"schema": 1, "callbacks": [&callback]});
            assert!(super::super::parse(&document.to_string()).is_err());
        }
        Ok(())
    }

    #[test]
    fn qm46_public_speech_is_bounded_and_player_only() -> Result<(), String> {
        let original = super::super::load()?
            .into_iter()
            .find(|c| c.key == "oteryn:quest-callback/children-grease")
            .ok_or("grease callback")?;
        assert!(valid(&original));
        for text in [
            String::new(),
            "x".repeat(256),
            " whitespace ".into(),
            "tab\t".into(),
            "line\nbreak".into(),
            "nul\0".into(),
        ] {
            let mut callback = original.clone();
            callback.effects.player_speech = Some(text);
            assert!(!valid(&callback));
        }
        let mut monster = original;
        monster.actor_kind = ActorKind::Monster;
        assert!(!valid(&monster));
        Ok(())
    }
}

#[cfg(test)]
mod phase_tests {
    #[test]
    fn production_cults_callback_is_source_qualified() -> Result<(), String> {
        let callbacks = super::super::load()?;
        let callback = callbacks
            .iter()
            .find(|c| c.key == "oteryn:quest-callback/engenc2/cults-essence")
            .ok_or("Cults callback is not registered")?;
        let encounter = callback
            .effects
            .encounter
            .as_ref()
            .ok_or("Cults encounter missing")?;
        assert_eq!(encounter.phase, Some(super::EncounterPhase::Cults));
        assert_eq!(encounter.players.len(), 5);
        assert_eq!(encounter.monsters.len(), 10);
        assert!(
            encounter
                .players
                .iter()
                .all(|p| p.to == encounter.players[0].to)
        );
        Ok(())
    }

    #[test]
    fn cults_phase_is_an_authored_encounter_input() -> Result<(), serde_json::Error> {
        let encounter: super::Encounter = serde_json::from_value(serde_json::json!({
            "minimum_level": 0, "players": [], "monsters": [],
            "room_min": {"x":33084,"y":31907,"floor":-15},
            "room_max": {"x":33114,"y":31933,"floor":-15},
            "phase": "cults_boss_phase_closures"
        }))?;
        let encoded = serde_json::to_value(encounter)?;
        assert_eq!(encoded["phase"], "cults_boss_phase_closures");
        Ok(())
    }
}
