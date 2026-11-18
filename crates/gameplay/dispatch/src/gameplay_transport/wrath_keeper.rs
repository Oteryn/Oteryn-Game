//! Exact ground corpse USE and fenced Quest reward admission.
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
    item_ref_admission::NativeItemWeights,
    item_view::ItemKey,
};
use crate::durability::{
    quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest},
    reward_claim_mint::{QuestRewardAdmission, quest_creature::QuestCreatureUseProof},
};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_quests::quest::{reward_callbacks, wrath_keeper};
use oteryn_protocol_oteryn::world_object::{UseDisposition, UseIntent};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_wrath_keeper_corpse(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        let (Some(ItemKey::Instance(item)), Some(item_fence), Some(catalogue), Some(native)) = (
            source,
            command.item_fence,
            self.quest_catalogue.as_ref(),
            self.active_generation.and_then(|g| g.native_gameplay()),
        ) else {
            return Ok(None);
        };
        let Some(ground) = self
            .root
            .read_ground_action_source(self.character, item_fence, item)
            .await
            .map_err(|_| ())?
            .filter(|ground| ground.definition().production_key == wrath_keeper::CORPSE)
        else {
            return Ok(None);
        };
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(outcome(UseDisposition::Rejected));
        };
        let Some(fence) = self
            .current_quest_fence(command.game_session_id)
            .await
            .map_err(|_| ())?
            .filter(|fence| fence.item_fence() == item_fence)
        else {
            return Ok(outcome(UseDisposition::Rejected));
        };
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let callback = reward_callbacks::load_for(catalogue)
            .map_err(|_| ())?
            .into_iter()
            .find(|callback| callback.key == wrath_keeper::TAIL_HANDOFF)
            .ok_or(())?;
        let mut sequence = self.revision_sequencer.acquire(fence.character_id).await;
        let runtime = self.runtime.lock().await;
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, None)
            || {
                let states = self.states.spell_states.lock().await;
                states.is_dead(actor)
                    || !states.spell_idle(&runtime, actor, command.game_session_id)
            }
        {
            return Ok(outcome(UseDisposition::Rejected));
        }
        let Some(proof) = QuestCreatureUseProof::read_corpse(
            &runtime,
            actor,
            fence,
            command_ref,
            &callback,
            intent,
            ground.quest_corpse_operand(),
            &permit,
        ) else {
            return Ok(outcome(UseDisposition::Rejected));
        };
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let result = sequence
            .commit_quest_reward(
                self.root,
                self.character,
                self.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: callback.key,
                    cause: QuestCause::Command(command_ref),
                },
                std::sync::Arc::clone(catalogue),
                QuestRewardAdmission {
                    native,
                    weights: &weights,
                    fee_payment: None,
                    actor_conditions: None,
                    carried_use: None,
                    creature_use: Some(&proof),
                    peer_use: None,
                },
                &self.premium_coordinator,
            )
            .await
            .map_err(|_| ())?;
        drop(runtime);
        drop(sequence);
        drop(permit);
        if matches!(
            result,
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
        ) {
            self.refresh_quest_session(command.game_session_id, false)
                .await;
            return Ok(outcome(UseDisposition::Committed));
        }
        Ok(outcome(UseDisposition::NotPossible))
    }
}

use super::actor_spell::ChannelSpellStates;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::{
    content::{CollisionClass, LogicalCell},
    map::{boot::BundleMap, overlay::TilePos},
};

#[derive(Default)]
pub(super) struct PoisonRound {
    uses: u8,
    active: Option<(ExactActorRef, u64)>,
}

pub(super) struct KeeperSpawn {
    health: i64,
    position: MovementLocalPosition,
    tile: TilePos,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Runs while holding the Channel runtime and spell-state locks.
    pub(super) fn prepare_wrath_keeper_poison(
        &self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        map: &BundleMap,
        actor: ExactActorRef,
        tool: [u8; 16],
        session: crate::foundation::GameSessionId,
    ) -> Result<KeeperSpawn, UseDisposition> {
        if !self.states.player_trades.physical(session, Some(tool)) {
            return Err(UseDisposition::Rejected);
        }
        let callbacks = &self.states.quest_callbacks;
        let pending = callbacks
            .records
            .lock()
            .map_err(|_| UseDisposition::Rejected)?;
        if pending
            .iter()
            .any(|(other, record)| *other != session && record.callback == wrath_keeper::POISON_USE)
        {
            return Err(UseDisposition::Occupied);
        }
        drop(pending);
        let mut round = callbacks
            .wrath_keeper_poison
            .lock()
            .map_err(|_| UseDisposition::Rejected)?;
        let now = self.owner_now().get();
        if let Some((keeper, deadline)) = round.active {
            if deadline > now
                && runtime.read_actor_position(keeper).is_ok()
                && !states.is_dead(keeper)
            {
                return Err(UseDisposition::Committed);
            }
            *round = PoisonRound::default();
        }
        if round.uses < 5 {
            let position = runtime
                .read_actor_position(actor)
                .map_err(|_| UseDisposition::Rejected)?
                .position();
            let position =
                super::quest_callbacks::native(position).ok_or(UseDisposition::Rejected)?;
            let _ = map
                .facts
                .emit_movement_speech(crate::map::facts::MovementSpeech {
                    speaker: actor,
                    position,
                    talk_type: 36,
                    text: "The plant twists violently, as though it could cry out.".into(),
                    captured_name: None,
                });
            round.uses += 1;
            return Err(UseDisposition::Committed);
        }
        let source = wrath_keeper::SPAWN;
        let tile = TilePos {
            x: source.x,
            y: source.y,
            floor: source.floor,
        };
        let position = super::quest_callbacks::local(source);
        let cells = &self.movement_cells;
        if cells.index().lookup(
            cells.scope(),
            LogicalCell {
                x: position.x,
                y: position.y,
                z: i32::from(position.floor),
            },
        ) != Ok(CollisionClass::Walkable)
            || map
                .facts
                .quest_callback_blocked(map.overlay.base(), tile, false)
            || runtime.positioned_melee_census().map_or(true, |actors| {
                actors.iter().any(|(_, at, _)| at.position() == position)
            })
        {
            return Err(UseDisposition::Occupied);
        }
        let policy = runtime
            .companion_policy(wrath_keeper::CREATURE)
            .map_err(|_| UseDisposition::Rejected)?;
        Ok(KeeperSpawn {
            health: policy.maximum_health,
            position,
            tile,
        })
    }

    pub(super) fn publish_wrath_keeper_spawn(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        target: TilePos,
        spawn: KeeperSpawn,
        project: impl FnOnce() -> bool,
    ) -> bool {
        let Ok(mut round) = self.states.quest_callbacks.wrath_keeper_poison.lock() else {
            return false;
        };
        let Some(deadline) = self.owner_now().get().checked_add(60_000_000) else {
            return false;
        };
        let Ok(actors) = runtime.commit_quest_creature_spawn_with_projection(
            &[(wrath_keeper::CREATURE, spawn.health, spawn.position)],
            project,
        ) else {
            return false;
        };
        round.active = Some((actors[0], deadline));
        let _ = map.facts.emit_movement_effect(target, 15);
        let _ = map.facts.emit_movement_effect(spawn.tile, 11);
        true
    }
}
