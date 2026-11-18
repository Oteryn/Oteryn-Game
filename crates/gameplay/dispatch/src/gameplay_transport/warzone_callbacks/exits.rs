//! Crystal 201189f8 creaturescripts_bosses_mission_depths.lua.
use super::ComposedFreshAdmission;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, RuntimeScopeRefV1};
use crate::map::{
    boot::BundleMap,
    facts::QuestCallbackProjection,
    overlay::TilePos,
    view::{EntrySource, MapFacts, compose_tile},
};

const DURATION_US: u64 = 20 * 60 * 1_000_000;
const RETURN_TELEPORT: u16 = 1949;
const REWARD_TELEPORT: u16 = 22761;

#[derive(Debug, Clone, Copy)]
pub(in crate::gameplay_transport) struct WarzoneExit {
    trigger: TilePos,
    reward: TilePos,
    channel: TilePos,
}

impl WarzoneExit {
    pub(in crate::gameplay_transport) fn for_creature(creature: &str) -> Option<Self> {
        let tile = |(x, y)| TilePos { x, y, floor: -15 };
        let (trigger, reward, channel) = match creature {
            "oteryn:creature.the_baron_from_below" => {
                ((33650, 32312), (33668, 32301), (33462, 32267))
            }
            "oteryn:creature.the_count_of_the_core" => {
                ((33681, 32340), (33682, 32315), (33324, 32111))
            }
            "oteryn:creature.the_duke_of_the_depths" => {
                ((33719, 32302), (33691, 32301), (33275, 32318))
            }
            _ => return None,
        };
        Some(Self {
            trigger: tile(trigger),
            reward: tile(reward),
            channel: tile(channel),
        })
    }

    fn open(self, map: &BundleMap, due: u64) -> bool {
        let Ok(tile) = compose_tile(&map.overlay, &map.facts, self.trigger) else {
            return false;
        };
        let mut portals = tile.entries.iter().filter_map(|entry| {
            let EntrySource::Base { placement_key, .. } = entry.source else {
                return None;
            };
            (entry.facts.appearance_id == RETURN_TELEPORT
                && map.facts.quest_callback_native_teleport(
                    placement_key,
                    RETURN_TELEPORT,
                    self.channel,
                ))
            .then_some((placement_key, entry.facts))
        });
        let Some((key, restore)) = portals.next() else {
            return false;
        };
        if portals.next().is_some()
            || map.facts.house_tile(self.reward)
            || map
                .overlay
                .base()
                .tile(self.reward.x, self.reward.y, self.reward.floor)
                .is_none()
        {
            return false;
        }
        let Some(after) = map.facts.quest_callback_replacement(key, REWARD_TELEPORT) else {
            return false;
        };
        !after.pickupable
            && map.facts.apply_quest_callback(
                key,
                map.facts.object_revision(key),
                after,
                &[],
                &[],
                false,
                Some(&QuestCallbackProjection::TimedEntrance {
                    due,
                    restore,
                    position: self.trigger,
                    destination: self.reward,
                }),
            )
    }
}

#[derive(Debug, Default)]
pub(in crate::gameplay_transport) struct WarzoneExits(
    std::sync::Mutex<Vec<(ExactActorRef, WarzoneExit, u64)>>,
);

impl WarzoneExits {
    pub(in crate::gameplay_transport) fn enqueue(
        &self,
        boss: ExactActorRef,
        exit: WarzoneExit,
        at_us: u64,
    ) {
        if matches!(boss.runtime_scope(), RuntimeScopeRefV1::Instance { .. })
            && let Some(due) = at_us.checked_add(DURATION_US)
            && let Ok(mut pending) = self.0.lock()
            && pending.len() < 3
        {
            pending.push((boss, exit, due));
        }
    }

    pub(in crate::gameplay_transport) fn publish(
        &self,
        owner: &ComposedFreshAdmission<'_, '_, '_>,
        runtime: &ChannelRuntimeV1,
        now_us: u64,
    ) {
        let Some(map) = owner.bundle_map() else {
            return;
        };
        let Ok(mut pending) = self.0.lock() else {
            return;
        };
        let binding = runtime.binding();
        for (boss, exit, due) in pending.drain(..) {
            if now_us < due
                && boss.runtime_scope() == binding.runtime_scope()
                && boss.scope_generation() == binding.scope_generation()
            {
                exit.open(map, due);
            }
        }
    }
}
