//! Crystal 201189f8, actions_oasis_lever_door.lua:1-49.
use super::*;
use crate::map::view;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

const DOOR: TilePos = TilePos {
    x: 33122,
    y: 32765,
    floor: -14,
};

fn wins(draw: u64) -> bool {
    crate::spell::uniform_draw(draw, 1, 3) == 1
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_rahemos_oasis(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        resolved: &super::super::world_map::MapObjectTarget<()>,
        target: &WorldObjectTarget,
    ) -> UseOutcome {
        UseOutcome {
            disposition: self
                .rahemos_oasis(actor, command, map, resolved, target)
                .await
                .unwrap_or(UseDisposition::Rejected),
            ..UseOutcome::rejected()
        }
    }

    async fn rahemos_oasis(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        resolved: &super::super::world_map::MapObjectTarget<()>,
        target: &WorldObjectTarget,
    ) -> Option<UseDisposition> {
        let permit = self.spell_lane_permit().await?;
        let fence = self
            .current_quest_fence(command.game_session_id)
            .await
            .ok()??;
        if command.item_fence != Some(fence.item_fence()) {
            return None;
        }
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.try_lock().ok()?;
        let session = command.game_session_id;
        if runtime
            .player_control_facts(actor, session)
            .ok()?
            .control_loss
            .is_some()
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
            || super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                .is_none()
            || !self.states.player_trades.physical(session, None)
            || map.facts.object_revision(resolved.placement_key) != target.expected_revision
        {
            return None;
        }
        let at = native(runtime.read_actor_position(actor).ok()?.position())?;
        if !super::super::map_door::reachable(at, resolved.pos) {
            return Some(UseDisposition::TooFar);
        }
        if !self
            .states
            .world_gate
            .lock()
            .ok()?
            .admit(actor, self.clock_origin.elapsed())
        {
            return Some(UseDisposition::Exhausted);
        }
        let current = view::compose_tile(&map.overlay, &map.facts.pinned(), resolved.pos)
            .ok()??
            .entries
            .into_iter()
            .find(|entry| entry.source.key() == view::MapEntryKey::Base(resolved.placement_key))?
            .facts;
        match current.appearance_id {
            1662 => {
                self.states.chat.push_quest_feedback(
                    session,
                    "Find the carrot beneath one of the hats to open this door.",
                );
                return Some(UseDisposition::Committed);
            }
            2772 => {}
            _ => return Some(UseDisposition::Committed),
        }
        let hash = Sha256::new()
            .chain_update(b"oteryn:rahemos-oasis:v1")
            .chain_update(session.as_bytes())
            .chain_update(command.command_id.to_be_bytes())
            .chain_update(resolved.placement_key.to_be_bytes())
            .finalize();
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&hash[..16]);
        let root = GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest());
        let draw = deterministic_decision_u64(
            &root,
            DecisionOccurrenceId::from_bytes(occurrence),
            "quest.rahemos.hat",
            0,
        )
        .ok()?;
        let now = self.owner_now();
        if !wins(draw) {
            states.apply_creature_damage_kind(
                &runtime,
                actor,
                session,
                200,
                crate::player_lethal::PlayerHitKind::Physical,
                crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now.get()),
                oteryn_protocol_oteryn::analyser::AnalyserDamageSource::None,
            )?;
            map.facts.emit_movement_effect(at, 3);
            self.states
                .chat
                .push_quest_feedback(session, "Wrong hat. The carrot is still hidden.");
            return Some(UseDisposition::Committed);
        }

        let hat = TilePos {
            x: resolved.pos.x.checked_sub(1)?,
            ..resolved.pos
        };
        map.overlay.base().tile(hat.x, hat.y, hat.floor)?;
        if map.facts.house_tile(hat) {
            return None;
        }
        let callback = callbacks().iter().find(|callback| {
            callback.key == "oteryn:quest-callback/ancient-tombs-rahemos-carrot"
        })?;
        let result = Box::pin(self.execute_quest_callback(
            &permit,
            &mut runtime,
            &mut states,
            actor,
            session,
            map,
            callback,
            resolved.placement_key,
            target.expected_revision,
            QuestCause::Use(CommandRef::new(
                session,
                CommandId::new(command.command_id).ok()?,
            )),
            Some(fence),
            Some(&mut slot),
            None,
            None,
            None,
            None,
        ))
        .await;
        if result == UseDisposition::Committed {
            map.facts.emit_movement_effect(hat, 15);
            map.facts.emit_movement_effect(DOOR, 15);
        }
        Some(result)
    }
}
