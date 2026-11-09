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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ItemDisguiseRequirement {
    pub item: String,
    pub condition: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CallbackEffects {
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
    /// Complete a separately owned child quest with this native once-only reward.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quest_completion: Option<String>,
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
    pub outfit_reward: Option<super::super::reward_callbacks::OutfitReward>,
    /// Source-authored public player MONSTER_SAY, emitted only after commit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub player_speech: Option<String>,
    pub transform: Option<u16>,
    /// Restore the exact pre-transform source revision after this delay.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reset_after_ms: Option<u32>,
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

pub(super) fn valid(callback: &Callback) -> bool {
    let effects = &callback.effects;
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
    if effects.player_speech.as_ref().is_some_and(|text| {
        callback.actor_kind != ActorKind::Player
            || text.is_empty()
            // CHAT0-RL-01: 255 Unicode scalars, each at most four UTF-8 bytes.
            || text.len() > 1_020
            || text.chars().count() > 255
            || text.trim() != text.as_str()
            || text.chars().any(char::is_control)
    }) {
    if effects.item_reward_per_use && !super::per_use_reward::admitted(callback) {
        return false;
    }
    if effects.player_speech.as_ref().is_some_and(|text| {
        callback.actor_kind != ActorKind::Player
            || text.is_empty()
            || text.len() > 255
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
    if effects.quest_completion.as_ref().is_some_and(|quest| {
        !super::super::valid_quest_key(quest)
            || !quest.starts_with("oteryn:quest.")
            || quest == &callback.quest
            || callback.trigger != Trigger::Use
            || callback.reversible
            || callback.opening_only
            || callback.actor_kind != ActorKind::Player
            || effects.item_reward.is_none()
            || effects.item_reward_cycle.is_some()
            || callback.storage.iter().all(|s| s.preserve)
    }) {
        return false;
    }
    let pos = |p: Position| (-15..=0).contains(&p.floor);
    let lever = callback.trigger == Trigger::Use && callback.lever.is_some();
    if effects.item_reward.as_ref().is_some_and(|reward| {
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
        callback.trigger != Trigger::Use
            || !callback.has_personal_state()
            || (callback
                .storage
                .iter()
                .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set))
                && !callback.state_ops.iter().any(|operation| {
                    matches!(
                        operation.effect,
                        crate::quest::reward_callbacks::StateMutation::SetNow
                            | crate::quest::reward_callbacks::StateMutation::Add { value: 1.. }
                            | crate::quest::reward_callbacks::StateMutation::Add { value: ..=-1 }
                    )
                })
                && !effects
                    .item_reward_cycle
                    .as_deref()
                    .is_some_and(|track| callback.craft_reward_epoch(track)))
            || (!effects.item_reward_per_use
                && callback
                    .storage
                    .iter()
                    .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set)))
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
                || !callback.has_personal_state()
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
            || (callback.has_personal_state()
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
        + usize::from(effects.encounter.is_some());
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
            && ((!personal_encounter && (!lever || callback.has_personal_state()))
            && (((!lever || callback.has_personal_state())
                && !super::personal_trial::admitted(callback))
                || !callback.passages.is_empty()
                || callback.teleport.is_some()))
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
        let in_room = |p: Position| {
            p.floor == encounter.room_min.floor
                && (encounter.room_min.x..=encounter.room_max.x).contains(&p.x)
                && (encounter.room_min.y..=encounter.room_max.y).contains(&p.y)
        };
        let unique_from: BTreeSet<_> = encounter.players.iter().map(|p| p.from).collect();
        let unique_to: BTreeSet<_> = encounter.players.iter().map(|p| p.to).collect();
        let unique_monsters: BTreeSet<_> = encounter.monsters.iter().map(|p| p.position).collect();
        let source_party = encounter.phase == Some(EncounterPhase::Cults);
        if (encounter.minimum_level == 0 && !source_party)
            || if let Some(timing) = encounter.timing {
                !encounter.players.is_empty()
                    || !callback.offerings.is_empty()
                    || callback.item_on_target.is_some()
                    || callback.experience.is_some()
                    || callback.monk_reward.is_some()
                    || callback.native_rule.is_some()
                    || effects.item_reward.is_some()
                    || effects.outfit_reward.is_some()
                    || effects.mount_reward.is_some()
                    || !(1..=60_000).contains(&timing.spawn_after_ms)
                    || timing.reset_after_ms <= timing.spawn_after_ms
                    || timing.reset_after_ms > 3_600_000
                    || (u64::from(encounter.room_max.x.saturating_sub(encounter.room_min.x)) + 1)
                        * (u64::from(encounter.room_max.y.saturating_sub(encounter.room_min.y)) + 1)
                        > 4096
            } else {
                !(1..=if encounter.phase.is_some() { 16 } else { 8 })
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

#[cfg(test)]
mod speech_tests {
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
