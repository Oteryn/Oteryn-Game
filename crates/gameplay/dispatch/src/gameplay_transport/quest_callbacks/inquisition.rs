//! The Shadow Nexus burn and reset sequence from Crystal 201189f8.
use super::*;
use crate::map::{
    facts::QuestCallbackProjection,
    view::{EntrySource, compose_tile},
};

pub(in crate::gameplay_transport) const HOLY_WATER: &str = "oteryn:item.tibia.i133";
const NEXUS: TilePos = TilePos {
    x: 33115,
    y: 31702,
    floor: -12,
};

pub(in crate::gameplay_transport) fn is_nexus(
    target: &super::super::world_map::MapObjectTarget<()>,
) -> bool {
    target.pos == NEXUS && target.ordinal == 3
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn damage_shadow_nexus(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        revision: u64,
        target: &super::super::world_map::MapObjectTarget<()>,
    ) -> Option<UseDisposition> {
        let _permit = self.spell_lane_permit().await?;
        let runtime = self.runtime.lock().await;
        let session = command.game_session_id;
        let states = self.states.spell_states.try_lock().ok()?;
        if runtime.owner_fence().is_err()
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
            || !self.states.player_trades.physical(session, None)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Some(UseDisposition::Rejected);
        }
        let at = native(runtime.read_actor_position(actor).ok()?.position())?;
        if !super::super::map_door::reachable(at, NEXUS) {
            return Some(UseDisposition::TooFar);
        }
        let now = self.owner_now().get();
        map.facts.expire_movement_items(now);
        let key = target.placement_key;
        let tile = compose_tile(&map.overlay, &map.facts, NEXUS).ok()??;
        let current = tile.entries.into_iter().find_map(|entry| {
            matches!(entry.source, EntrySource::Base { placement_key, .. } if key == placement_key)
                .then_some(entry.facts.appearance_id)
        })?;
        if !matches!(current, 7925 | 7927 | 7929) {
            return Some(UseDisposition::Rejected);
        }
        let after = map.facts.quest_callback_replacement(key, current + 1)?;
        let ready = map.facts.quest_callback_replacement(key, current + 2)?;
        let original = map.facts.quest_callback_replacement(key, 7925)?;
        let projection = QuestCallbackProjection::TimedTransformSequence {
            first: (now.checked_add(30_000_000)?, ready),
            second: (now.checked_add(50_000_000)?, original),
        };
        Some(
            if map.facts.apply_quest_callback(
                key,
                revision,
                after,
                &[],
                &[],
                false,
                Some(&projection),
            ) {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            },
        )
    }
}
