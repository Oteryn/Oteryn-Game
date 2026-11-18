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

#[derive(Default)]
pub(super) struct NexusCharges {
    pub(super) remaining: u8,
    pub(super) expires: u64,
}

pub(in crate::gameplay_transport) fn is_nexus(
    target: &super::super::world_map::MapObjectTarget<()>,
) -> bool {
    target.pos == NEXUS && target.ordinal == 3
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn vampire_altars_ready(&self, map: &BundleMap) -> Option<bool> {
        use oteryn_gameplay_quests::quest::inquisition_actions::{ALTARS, SCRIPTURE};
        let index = {
            let runtime = self.runtime.lock().await;
            super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)?
        };
        let scripture = index.definition_ref("Item", SCRIPTURE, "definition-r1")?;
        let ground =
            super::super::loop_move::loop_ground(self, map.overlay.base(), Some(index)).await?;
        let facts = ground.facts(&map.facts);
        Some(ALTARS.iter().all(|&altar| {
            compose_tile(ground.overlay(), &facts, tile(altar))
                .ok()
                .flatten()
                .is_some_and(|tile| {
                    tile.entries.iter().any(|entry| {
                        entry.facts.definition
                            == oteryn_protocol_oteryn::world_map::MapDefinition::Item(scripture)
                            && matches!(entry.source, EntrySource::Ground(_))
                    })
                })
        }))
    }

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
        let mut charges = self.states.quest_callbacks.shadow_nexus.lock().ok()?;
        if current == 7931 {
            if charges.remaining > 0 && now < charges.expires {
                // The carried USE owner continues with the fenced inventory and quest receipt.
                return None;
            }
            let reset = map.facts.quest_callback_replacement(key, 7925)?;
            return Some(
                if map
                    .facts
                    .apply_quest_callback(key, revision, reset, &[], &[], false, None)
                {
                    UseDisposition::Committed
                } else {
                    UseDisposition::StaleState
                },
            );
        }
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
        let new_charges = if current == 7929 {
            use oteryn_simulation_determinism::{
                DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
            };
            use sha2::{Digest, Sha256};
            let digest = Sha256::new()
                .chain_update(key.to_be_bytes())
                .chain_update(revision.to_be_bytes())
                .chain_update(session.as_bytes())
                .finalize();
            let occurrence = DecisionOccurrenceId::from_bytes(digest[..16].try_into().ok()?);
            let root =
                GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
            let draw =
                deterministic_decision_u64(&root, occurrence, "quest.inquisition.nexus.charges", 0)
                    .ok()?;
            Some(NexusCharges {
                remaining: u8::try_from(crate::spell::uniform_draw(draw, 4, 5)).ok()?,
                expires: now.checked_add(50_000_000)?,
            })
        } else {
            None
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
                if let Some(new_charges) = new_charges {
                    *charges = new_charges;
                }
                states.owner_wake.notify_one();
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            },
        )
    }
}
