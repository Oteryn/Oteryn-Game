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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quest_completion: Option<QuestCompletion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party_transfer: Option<PartyTransfer>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reward_door: Option<RewardDoor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outfit_reward: Option<super::super::reward_callbacks::OutfitReward>,
    pub transform: Option<u16>,
    pub facing: Option<Facing>,
    pub magic: Option<MagicEffect>,
    pub portals: Option<PortalPair>,
    pub item_cycle: Option<ItemCycle>,
    pub encounter: Option<Encounter>,
}

/// Durable once-claim identities required by a terminal world transition.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct QuestCompletion {
    #[serde(default)]
    pub claimed_rewards: Vec<String>,
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
        + usize::from(effects.reward_door.is_some())
        + usize::from(effects.party_transfer.is_some());
    if let Some(completion) = &effects.quest_completion {
        let unique: std::collections::BTreeSet<_> = completion.claimed_rewards.iter().collect();
        if completion.claimed_rewards.len() > 8
            || unique.len() != completion.claimed_rewards.len()
            || completion.claimed_rewards.iter().any(|key| {
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
    if operations > 1
        || (operations > 0
            && (!lever
                || (!callback.storage.is_empty() && effects.party_transfer.is_none())
                || !callback.passages.is_empty()
                || callback.teleport.is_some()))
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
                    track: "oteryn:quest.shadow".into(),
                    minimum: 0,
                    maximum: 1,
                    set: 0,
                    preserve: true,
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
                }
                7 => row.reversible = false,
                _ => unreachable!(),
            }
            assert!(!valid(&row), "invalid party change {change}");
        }
        Ok(())
    }
}
