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
    /// Transfer the admitted roster still in the encounter room when it expires.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timed_group_transfer: Option<TimedGroupTransfer>,
    /// Create a nonportable map object until the shared deadline.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timed_display: Option<TimedDisplay>,
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
    /// Close the currently open remote passages after a native StepIn.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub close_passages_after_ms: Option<u32>,
    pub facing: Option<Facing>,
    pub magic: Option<MagicEffect>,
    /// A shared switch's ordered cues and its wrong-order response.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub switch_cues: Option<SwitchCues>,
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
pub struct SwitchCues {
    pub success: Vec<MagicEffect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wrong_order: Option<MagicEffect>,
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
pub struct Encounter {
    pub minimum_level: u32,
    pub players: Vec<EncounterPlayer>,
    pub monsters: Vec<EncounterMonster>,
    pub room_min: Position,
    pub room_max: Position,
    /// Crystal the_annihilator/lever.lua:28-39,52-60 reads the top creature on each pad.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub select_top_creature: bool,
    /// A value-free source switch schedules native monsters without moving a party.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<EncounterTiming>,
    /// Source phases run on the shared encounter owner after atomic party admission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phase: Option<EncounterPhase>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub enum EncounterPhase {
    #[serde(rename = "cults_boss_phase_closures")]
    Cults,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct EncounterTiming {
    pub spawn_after_ms: u32,
    pub reset_after_ms: u32,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TimedDisplay {
    pub position: Position,
    pub appearance: u16,
    pub after_ms: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TimedGroupTransfer {
    pub after_ms: u32,
    pub destination: Position,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub switches: Vec<GroupSwitch>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<GroupTransferWarning>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_speech: Option<GroupTransferSpeech>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub start_magic: Vec<MagicEffect>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroupTransferSpeech {
    pub position: Position,
    pub text: String,
}

fn valid_group_speech_text(text: &str) -> bool {
    !text.is_empty() && text.len() <= 256 && !text.contains(['\n', '\r', '\0'])
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroupTransferWarning {
    pub after_ms: u32,
    pub position: Position,
    pub room_min: Position,
    pub room_max: Position,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GroupSwitch {
    pub lever: Position,
    pub station: Position,
    pub wall_from: Position,
    pub wall_to: Position,
    pub wall_appearance: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wall_speech: Option<GroupTransferSpeech>,
}

impl GroupSwitch {
    pub fn wall_positions(&self) -> impl Iterator<Item = Position> + '_ {
        (self.wall_from.x..=self.wall_to.x).flat_map(move |x| {
            (self.wall_from.y..=self.wall_to.y).map(move |y| Position {
                x,
                y,
                floor: self.wall_from.floor,
            })
        })
    }
}

fn valid_group_switches(callback: &Callback, room: &Encounter, switches: &[GroupSwitch]) -> bool {
    let in_room = |p: Position| {
        p.floor == room.room_min.floor
            && (room.room_min.x..=room.room_max.x).contains(&p.x)
            && (room.room_min.y..=room.room_max.y).contains(&p.y)
    };
    let levers: BTreeSet<_> = switches.iter().map(|s| s.lever).collect();
    let stations: BTreeSet<_> = switches.iter().map(|s| s.station).collect();
    if !(2..=8).contains(&switches.len())
        || levers.len() != switches.len()
        || stations.len() != switches.len()
        || !levers.is_disjoint(&stations)
        || !callback
            .selector
            .position
            .is_some_and(|position| levers.contains(&position))
        || !room.players.is_empty()
        || !callback.passages.is_empty()
        || callback.effects.scene.is_some()
    {
        return false;
    }
    let mut walls = BTreeSet::new();
    switches.iter().all(|switch| {
        let (from, to) = (switch.wall_from, switch.wall_to);
        in_room(switch.lever)
            && in_room(switch.station)
            && switch.lever.x.abs_diff(switch.station.x) <= 1
            && switch.lever.y.abs_diff(switch.station.y) <= 1
            && in_room(from)
            && in_room(to)
            && from.x <= to.x
            && from.y <= to.y
            && (from.x == to.x || from.y == to.y)
            && from.x.abs_diff(to.x) + from.y.abs_diff(to.y) < 8
            && switch.wall_appearance != 0
            && switch.wall_speech.as_ref().is_none_or(|speech| {
                valid_group_speech_text(&speech.text)
                    && switch
                        .wall_positions()
                        .any(|position| position == speech.position)
            })
            && switch.wall_positions().all(|at| {
                !levers.contains(&at)
                    && !stations.contains(&at)
                    && !room.monsters.iter().any(|monster| monster.position == at)
                    && walls.insert(at)
            })
    })
}

fn valid_group_start_cues(room: &Encounter, transfer: &TimedGroupTransfer) -> bool {
    if transfer.start_speech.is_none() && transfer.start_magic.is_empty() {
        return true;
    }
    let in_room = |p: Position| {
        p.floor == room.room_min.floor
            && (room.room_min.x..=room.room_max.x).contains(&p.x)
            && (room.room_min.y..=room.room_max.y).contains(&p.y)
    };
    !transfer.switches.is_empty()
        && transfer
            .start_speech
            .as_ref()
            .is_none_or(|speech| in_room(speech.position) && valid_group_speech_text(&speech.text))
        && transfer.start_magic.len() <= 32
        && transfer
            .start_magic
            .iter()
            .all(|magic| in_room(magic.position) && (1..=255).contains(&magic.effect))
}

/// A source rite transports the exact players on bounded stations. The initiator
/// may stand elsewhere; no social party or monster room is inferred.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PartyTransfer {
    /// Run the rite only after the lever changes to this source appearance.
    pub activation_appearance: u16,
    pub stations: Vec<PartyStation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub room_entry: Option<PartyRoomEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PartyStation {
    pub from: Position,
    pub to: Position,
    /// Base vocation; promoted members retain their base eligibility.
    pub vocation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offering: Option<Position>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub carried_item: Option<String>,
}

/// Read each participant's progress and carried item before admitting the party
/// to the room shared with a coordinated switch window.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PartyRoomEntry {
    pub initiator: Position,
    pub room_min: Position,
    pub room_max: Position,
    pub window_lever: Position,
    pub track: String,
    pub minimum: i64,
    pub occupied_speech: GroupTransferSpeech,
    pub ineligible_speech: GroupTransferSpeech,
}

impl PartyRoomEntry {
    pub fn contains(&self, p: Position) -> bool {
        p.floor == self.room_min.floor
            && (self.room_min.x..=self.room_max.x).contains(&p.x)
            && (self.room_min.y..=self.room_max.y).contains(&p.y)
    }
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
        && if let Some(room) = &party.room_entry {
            !callback.has_state()
                && !callback.requires_world_admission
                && callback.offerings.is_empty()
                && callback
                    .lever
                    .is_some_and(|pair| pair[1] == party.activation_appearance)
                && pos(room.initiator)
                && pos(room.room_min)
                && pos(room.room_max)
                && room.room_min.floor == room.room_max.floor
                && room.contains(room.window_lever)
                && super::super::valid_quest_key(&room.track)
                && room.track.starts_with("oteryn:quest-progress/")
                && party.stations.iter().any(|s| s.from == room.initiator)
                && [&room.occupied_speech, &room.ineligible_speech]
                    .iter()
                    .all(|cue| {
                        pos(cue.position)
                            && cue.position.floor == room.initiator.floor
                            && valid_group_speech_text(&cue.text)
                    })
        } else {
            !callback.storage.is_empty()
                && callback.offerings.len() == party.stations.len()
                && unique(party.stations.iter().filter_map(|s| s.offering).collect())
        }
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
        && party.stations.iter().all(|station| {
            pos(station.from)
                && pos(station.to)
                && station.from != station.to
                && matches!(
                    station.vocation.as_str(),
                    "druid" | "sorcerer" | "paladin" | "knight" | "monk"
                )
                && if let Some(room) = &party.room_entry {
                    station.offering.is_none()
                        && station.from.floor == room.initiator.floor
                        && !room.contains(station.from)
                        && room.contains(station.to)
                        && station.carried_item.as_ref().is_some_and(|item| {
                            item.starts_with("oteryn:item.") && super::super::valid_quest_key(item)
                        })
                } else {
                    station.carried_item.is_none()
                        && station.offering.is_some_and(|offering| {
                            pos(offering)
                                && callback
                                    .offerings
                                    .iter()
                                    .filter(|o| o.position == offering && o.count == 1)
                                    .count()
                                    == 1
                        })
                }
        })
}

pub(super) fn valid(callback: &Callback) -> bool {
    let effects = &callback.effects;
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
        || effects.close_passages_after_ms.is_some_and(|delay| {
            !(1..=3_600_000).contains(&delay)
                || callback.trigger != Trigger::StepIn
                || callback.actor_kind != ActorKind::Player
                || callback.lever.is_some()
                || callback.reversible
                || callback.has_state()
                || callback.teleport.is_some()
                || callback.item_on_target.is_some()
                || callback.native_rule.is_some()
                || callback.passages.is_empty()
                || *effects
                    != CallbackEffectSet::from_shared(CallbackEffects {
                        close_passages_after_ms: Some(delay),
                        ..Default::default()
                    })
        })
        || effects.passages_reset_after_ms.is_some_and(|delay| {
            !(1..=3_600_000).contains(&delay)
                || callback.actor_kind != ActorKind::Player
                || callback.trigger != Trigger::Use
                || (callback.reversible && callback.lever.is_none())
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
    let operations = usize::from(effects.portals.is_some())
        + usize::from(effects.item_cycle.is_some())
        + usize::from(effects.encounter.is_some())
        + usize::from(effects.scene.is_some())
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
    if let Some(display) = effects.timed_display
        && (!pos(display.position)
            || display.appearance == 0
            || !(1..=3_600_000).contains(&display.after_ms)
            || callback.actor_kind != ActorKind::Player
            || callback.trigger != Trigger::Use
            || !callback.requires_world_admission
            || !callback.has_state_writes()
            || callback
                .item_on_target
                .as_ref()
                .is_none_or(|input| input.reusable || input.count != 1)
            || callback.lever.is_some()
            || callback.reversible
            || !callback.passages.is_empty()
            || effects.encounter.is_some()
            || effects.scene.is_some()
            || effects.target_transform.is_some()
            || effects.reset_after_ms.is_some()
            || effects.portals.is_some()
            || effects.item_presence.is_some()
            || effects.item_cycle.is_some())
    {
        return false;
    }
    if let Some(transfer) = &effects.timed_group_transfer
        && (!pos(transfer.destination)
            || !(1..=3_600_000).contains(&transfer.after_ms)
            || callback.trigger != Trigger::Use
            || callback.lever.is_none()
            || callback.has_personal_state()
            || effects.encounter.as_ref().is_none_or(|encounter| {
                (if transfer.switches.is_empty() {
                    encounter.players.is_empty()
                } else {
                    !valid_group_switches(callback, encounter, &transfer.switches)
                }) || encounter.timing.is_some()
                    || encounter.phase.is_some()
                    || !valid_group_start_cues(encounter, transfer)
                    || transfer.warning.as_ref().is_some_and(|warning| {
                        let (min, max) = (warning.room_min, warning.room_max);
                        warning.after_ms == 0
                            || warning.after_ms >= transfer.after_ms
                            || !pos(warning.position)
                            || !valid_group_speech_text(&warning.text)
                            || min.floor != encounter.room_min.floor
                            || max.floor != encounter.room_max.floor
                            || !(encounter.room_min.x <= min.x
                                && min.x <= max.x
                                && max.x <= encounter.room_max.x)
                            || !(encounter.room_min.y <= min.y
                                && min.y <= max.y
                                && max.y <= encounter.room_max.y)
                    })
                    || (transfer.destination.floor == encounter.room_min.floor
                        && (encounter.room_min.x..=encounter.room_max.x)
                            .contains(&transfer.destination.x)
                        && (encounter.room_min.y..=encounter.room_max.y)
                            .contains(&transfer.destination.y))
            }))
    {
        return false;
    }
    if let Some(cues) = &effects.switch_cues
        && (!callback.is_character_guarded_switch()
            || cues.success.len() > 8
            || (cues.success.is_empty() && cues.wrong_order.is_none())
            || cues
                .success
                .iter()
                .any(|cue| !pos(cue.position) || !(1..=255).contains(&cue.effect))
            || cues.wrong_order.as_ref().is_some_and(|cue| {
                callback.world_requirements.is_empty()
                    || callback.selector.position != Some(cue.position)
                    || !(1..=255).contains(&cue.effect)
            }))
    {
        return false;
    }
    if let Some(encounter) = &effects.encounter {
        let in_room = |p: Position| {
            p.floor == encounter.room_min.floor
                && (encounter.room_min.x..=encounter.room_max.x).contains(&p.x)
                && (encounter.room_min.y..=encounter.room_max.y).contains(&p.y)
        };
        let unique_from: BTreeSet<_> = encounter.players.iter().map(|p| p.from).collect();
        let unique_to: BTreeSet<_> = encounter.players.iter().map(|p| p.to).collect();
        let unique_monsters: BTreeSet<_> = encounter.monsters.iter().map(|p| p.position).collect();
        let source_party = encounter.phase == Some(EncounterPhase::Cults);
        let coordinated = effects
            .timed_group_transfer
            .as_ref()
            .is_some_and(|group| !group.switches.is_empty());
        if (encounter.minimum_level == 0 && !source_party)
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
                !coordinated
                    && !(1..=if encounter.phase.is_some() { 16 } else { 8 })
                        .contains(&encounter.players.len())
            }
            || !(1..=if encounter.phase.is_some() { 16 } else { 8 })
                .contains(&encounter.monsters.len())
            || (encounter.select_top_creature && encounter.players.is_empty())
            || (encounter.phase.is_some() && encounter.timing.is_some())
            || (callback.reversible && !source_party)
            || unique_from.len() != encounter.players.len()
            || (unique_to.len() != encounter.players.len() && !source_party)
            || unique_monsters.len() != encounter.monsters.len()
            || !unique_to.is_disjoint(&unique_monsters)
            || !pos(encounter.room_min)
            || !pos(encounter.room_max)
            || encounter.room_min.floor != encounter.room_max.floor
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
