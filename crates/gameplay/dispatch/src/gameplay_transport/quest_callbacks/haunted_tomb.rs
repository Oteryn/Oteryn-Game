//! Crystal sarcophagi share the Channel-owned source and its fenced cooldown.
use super::*;
use crate::durability::reward_claim_mint::{HauntedTombSource, HauntedTombUse};
use crate::map::view::MapFacts;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_items::{haunted_tomb as rules, reward_bag::reward_bag_bounded_draw};
use oteryn_protocol_oteryn::world_map::MapDefinition;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_haunted_tomb(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let active = self.active_generation?;
        let native = active.native_gameplay()?;
        let sarcophagus = native.item_index()?.definition_ref(
            "Item",
            "oteryn:item.tibia.i29349",
            "definition-r1",
        )?;
        let view = super::super::world_map::MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: active.identity().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let resolved =
            super::super::world_map::resolve_map_target(&view, target, |_| None::<()>).ok()?;
        let position = resolved.pos;
        let key = resolved.placement_key;
        if !rules::is_source(
            rules::SARCOPHAGUS,
            map.facts.action_at(key),
            position.x,
            position.y,
            position.floor,
        ) || map.facts.house_tile(position)
        {
            return None;
        }
        let base = map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)?;
        let matching = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(resolved.ordinal))
            .and_then(|(id, _)| map.facts.base_entry(position, resolved.ordinal, *id))
            .is_some_and(|entry| entry.definition == MapDefinition::Item(sarcophagus));
        if !matching {
            return None;
        }
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let Some(fence) = command.item_fence else {
            return outcome(UseDisposition::Rejected);
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return outcome(UseDisposition::Rejected);
        };
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if runtime.owner_fence().is_err()
            || map.overlay.runtime_scope() != runtime.binding().runtime_scope()
            || map.facts.object_revision(key) != target.expected_revision
            || map.facts.action_at(key) != Some(rules::ACTION_ID)
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || !self
                .states
                .player_trades
                .physical(command.game_session_id, None)
        {
            return outcome(UseDisposition::Rejected);
        }
        let at = runtime.read_actor_position(actor).ok()?.position();
        let origin = MovementLocalPosition {
            x: i32::from(position.x),
            y: i32::from(position.y),
            floor: -i16::from(position.floor),
        };
        if !oteryn_gameplay_items::reach::within_reach(
            (at.x, at.y, i32::from(at.floor)),
            (origin.x, origin.y, i32::from(origin.floor)),
        ) {
            return outcome(UseDisposition::TooFar);
        }
        let mut seed = [0; 16];
        seed[..8].copy_from_slice(&key.to_be_bytes());
        seed[8..].copy_from_slice(&target.expected_revision.to_be_bytes());
        let roll = reward_bag_bounded_draw(
            *command.game_session_id.as_bytes(),
            command.command_id,
            seed,
            48,
            10,
        )? + 1;
        let ordinal = CommandId::new(command.command_id).ok()?;
        let result = self
            .root
            .commit_haunted_tomb_use(
                &permit,
                self.character,
                self.holder,
                fence,
                CommandRef::new(command.game_session_id, ordinal),
                HauntedTombSource {
                    placement: key.to_be_bytes(),
                    map_digest: map.overlay.base().digest(),
                    expected_revision: target.expected_revision,
                    roll: u32::try_from(roll).ok()?,
                },
                native,
                &super::super::item_ref_admission::NativeItemWeights(native)
                    .with_wheel(self.premium_coordinator),
            )
            .await;
        match result {
            Ok(HauntedTombUse::Curse) => {
                self.spawn_haunted_tomb_mummy(&mut runtime, map, origin);
                states
                    .sounds
                    .record_item_effect(&runtime, actor, origin, 3, false);
                self.states.chat.push_quest_feedback(
                    command.game_session_id,
                    "You have disturbed the tomb and awakened its guardian.",
                );
            }
            Ok(HauntedTombUse::Key) => {
                self.states.chat.push_quest_feedback(
                    command.game_session_id,
                    "You find a strangely ornamented key.",
                );
                states.owner_wake.notify_one();
            }
            Ok(HauntedTombUse::CoolingDown) => {
                self.states.chat.push_quest_feedback(
                    command.game_session_id,
                    "Give the sarcophagus a moment. Its presence still unsettles you.",
                );
            }
            Ok(HauntedTombUse::NoRoom) => return outcome(UseDisposition::NoRoom),
            Ok(HauntedTombUse::Ineligible | HauntedTombUse::Replay) => {}
            Err(crate::durability::DurabilityError::CommitRejected) => {
                return outcome(UseDisposition::Rejected);
            }
            Err(_) => {
                runtime.retire_owner_cycle();
                return outcome(UseDisposition::Rejected);
            }
        }
        outcome(UseDisposition::Committed)
    }

    fn spawn_haunted_tomb_mummy(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        origin: MovementLocalPosition,
    ) {
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return;
        };
        if native.install_companion_policies(runtime).is_err() {
            return;
        }
        let Ok(policy) = runtime.companion_policy(rules::MUMMY) else {
            return;
        };
        let health = policy.maximum_health;
        if let Some(landing) = self.quest_summon_landing(runtime, map, origin, true) {
            let _ = runtime.commit_quest_creature_spawn(&[(rules::MUMMY, health, landing)]);
        }
    }
}
