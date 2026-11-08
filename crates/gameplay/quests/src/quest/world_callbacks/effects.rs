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

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CallbackEffects {
    /// Distinct carried definitions consumed atomically by the Quest receipt owner.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub carried_inputs: Vec<super::super::reward_callbacks::ItemAmount>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outfit_reward: Option<super::super::reward_callbacks::OutfitReward>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub item_reward: Option<super::super::reward_callbacks::ItemReward>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene: Option<super::SceneEdits>,
    pub transform: Option<u16>,
    pub facing: Option<Facing>,
    pub magic: Option<MagicEffect>,
    pub portals: Option<PortalPair>,
    pub item_cycle: Option<ItemCycle>,
    pub encounter: Option<Encounter>,
}

impl CallbackEffects {
    pub(super) fn is_empty(&self) -> bool {
        self == &Self::default()
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
                || callback.storage.is_empty()
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
    if effects.item_reward.as_ref().is_some_and(|reward| {
        callback.trigger != Trigger::Use
            || callback.is_read_only()
            || !callback.offerings.is_empty()
            || callback.item_on_target.as_ref().is_some_and(|input| {
                input.reusable || input.replacement.is_some() || input.target_replacement.is_some()
            })
            || callback.storage.is_empty()
            || callback
                .storage
                .iter()
                .all(|s| s.preserve || (s.minimum..=s.maximum).contains(&s.set))
            || !super::super::reward_callbacks::valid_item_reward(reward)
    }) || (callback.completes && (callback.storage.is_empty() || callback.is_read_only()))
    {
        return false;
    }

    let pos = |p: Position| (-15..=0).contains(&p.floor);
    let lever = callback.trigger == Trigger::Use && callback.lever.is_some();
    if effects.outfit_reward.as_ref().is_some_and(|r| {
        callback.trigger != Trigger::Use
            || callback.storage.is_empty()
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
    }) || effects.transform == Some(0)
        || (effects.facing.is_some() && callback.teleport.is_none())
        || effects
            .magic
            .as_ref()
            .is_some_and(|m| !pos(m.position) || m.effect == 0)
        || (callback.trigger == Trigger::StepOut && callback.return_on_failure)
    {
        return false;
    }
    let operations = usize::from(effects.portals.is_some())
        + usize::from(effects.item_cycle.is_some())
        + usize::from(effects.encounter.is_some())
        + usize::from(effects.scene.is_some());
    if operations > 1
        || (operations > 0
            && (!lever
                || (effects.scene.is_none() && !callback.storage.is_empty())
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
        if encounter.minimum_level == 0
            || !(1..=8).contains(&encounter.players.len())
            || !(1..=8).contains(&encounter.monsters.len())
            || callback.reversible
            || unique_from.len() != encounter.players.len()
            || unique_to.len() != encounter.players.len()
            || unique_monsters.len() != encounter.monsters.len()
            || !unique_to.is_disjoint(&unique_monsters)
            || !pos(encounter.room_min)
            || !pos(encounter.room_max)
            || encounter.room_min.floor != encounter.room_max.floor
            || encounter.room_min.x > encounter.room_max.x
            || encounter.room_min.y > encounter.room_max.y
            || encounter
                .players
                .iter()
                .any(|p| !pos(p.from) || !pos(p.to) || !in_room(p.to) || in_room(p.from))
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
