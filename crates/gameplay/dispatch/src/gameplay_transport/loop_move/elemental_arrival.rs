//! M49 live item-arrival adapter; the lane owns all nine map observations through SQL.
use super::*;
use crate::durability::{
    DurabilityError,
    item_loop_move::{ElementalArrivalProbe, ElementalArrivalTarget, LoopMoveReceipt},
    spell_owner_commit::SpellLanePermit,
};
use oteryn_gameplay_quests::quest::{
    elemental_arrival::{self as rules, ArrivalFence},
    world_callbacks::Position,
};

type MoveCommit = Result<Result<LoopMoveReceipt, LoopMoveRefusal>, DurabilityError>;

pub(in crate::gameplay_transport) struct PendingElementalArrival {
    command: CommandRef,
    fence: CurrentCharacterItemFence,
    request: LoopMoveRequest,
    destination: WorldTilePosition,
    _reservation: crate::map::facts::ElementalArrivalReservation,
}
impl std::fmt::Debug for PendingElementalArrival {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PendingElementalArrival")
            .field("command", &self.command)
            .field("request", &self.request)
            .finish_non_exhaustive()
    }
}
fn park(pending: PendingElementalArrival) -> Box<dyn std::any::Any + Send> {
    Box::new(
        super::super::actor_spell::native_combat_cast::UnresolvedSpellCommit::ElementalArrival(
            Box::new(pending),
        ),
    )
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn commit_elemental_arrival(
        &self,
        permit: &mut SpellLanePermit,
        actor: ExactActorRef,
        command: UseCommand,
        native: &crate::content::native_gameplay::NativeGameplayState,
        request: &LoopMoveRequest,
        ground: Option<&LoopGroundOverlay>,
    ) -> Option<MoveCommit> {
        let LoopDestination::Ground(to) = &request.destination else {
            return None;
        };
        let source = ground?.facts.get(&request.item)?;
        if !rules::RECIPES
            .iter()
            .any(|recipe| recipe.corpse == source.appearance_id)
        {
            return None;
        }
        let at =
            crate::durability::spell_items_abi::decode_ground_cell(&to.spatial_position).ok()?;
        let position = Position {
            x: at.x.try_into().ok()?,
            y: at.y.try_into().ok()?,
            floor: at.z.try_into().ok()?,
        };
        let destination = WorldTilePosition {
            x: at.x,
            y: at.y,
            floor: at.z.try_into().ok()?,
        };
        let fence = command.item_fence?;
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).ok()?,
        );
        let map = self.bundle_map()?;
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        let corpse = match self
            .root
            .item_loop_elemental_corpse(
                permit,
                self.character,
                self.holder,
                fence,
                &weights,
                ElementalArrivalProbe {
                    command: command_ref,
                    item: request.item,
                    position,
                },
            )
            .await
        {
            Ok(Some(corpse)) => corpse,
            Ok(None) => return None,
            Err(error) => return Some(Err(error)),
        };
        let (plan, reservation) = {
            let mut runtime = self.runtime.lock().await;
            let states = self.states.spell_states.lock().await;
            if runtime.owner_fence().is_err()
                || runtime.binding().runtime_scope() != fence.runtime_scope
                || runtime.binding().scope_generation() != fence.scope_ownership_generation
                || states
                    .get(&runtime, actor, command.game_session_id)
                    .is_none()
                || states.is_dead(actor)
                || states.has_pending_spell_commit(actor, command.game_session_id)
                || native_for_move(&runtime, self.active_generation, false, false).is_none()
            {
                return Some(Ok(Err(LoopMoveRefusal::Stale)));
            }
            let wake = Arc::clone(&states.owner_wake);
            drop(states);
            let map_digest: [u8; 32] =
                Sha256::digest(map_revision(map.overlay.base()).as_bytes()).into();
            if runtime.content_pin().map_revision_digest() != map_digest {
                return Some(Ok(Err(LoopMoveRefusal::Stale)));
            }
            let actor_at =
                super::super::bundle_map_position(&mut runtime, self.movement_cells(), actor)?;
            if !in_reach(
                (actor_at.at.x, actor_at.at.y, actor_at.at.floor),
                (at.x, at.y, destination.floor),
            ) {
                return Some(Ok(Err(LoopMoveRefusal::Stale)));
            }
            let binding = runtime.binding();
            let channel = binding.channel_id()?;
            let recipe = rules::RECIPES
                .iter()
                .find(|recipe| recipe.corpse == corpse.item)?;
            let busy = runtime.boss_spawns().iter().any(|spawn| {
                spawn.creature == recipe.creature
                    && spawn
                        .actor
                        .is_some_and(|actor| runtime.contains_live_creature(actor))
            });
            let scope = ArrivalFence {
                world: *binding.world_id().as_bytes(),
                channel: *channel.as_bytes(),
                scope_generation: binding.scope_generation().get(),
                map: map.overlay.base().digest(),
                content: native.source_digest(),
                reset_epoch: self.reset_epoch,
            };
            let plan = map.facts.observe_elemental_arrival(
                map.overlay.base(),
                scope,
                TilePos {
                    x: position.x,
                    y: position.y,
                    floor: position.floor,
                },
                corpse,
                busy,
            )?;
            let Some(reservation) =
                map.facts
                    .reserve_elemental_arrival(map.overlay.base(), &plan, wake)
            else {
                return Some(Ok(Err(LoopMoveRefusal::Stale)));
            };
            (plan, reservation)
        };
        let target = ElementalArrivalTarget::new(plan);
        let pending = PendingElementalArrival {
            command: command_ref,
            fence,
            request: request.clone(),
            destination,
            _reservation: reservation,
        };
        let mut window = permit.open_commit_window(pending, park);
        let result = self
            .root
            .commit_item_loop_elemental_arrival(
                &mut window,
                self.character,
                self.holder,
                fence,
                command_ref,
                &weights,
                request.clone(),
                target,
            )
            .await;
        match &result {
            Ok(Ok(_)) => {
                drop(window.install());
            }
            Ok(Err(_)) => {
                drop(window.release());
            }
            Err(_) => {} // Drop parks the entire reservation until the exact receipt is known.
        }
        Some(result)
    }

    pub(in crate::gameplay_transport) async fn resolve_elemental_arrival(
        &self,
        permit: &mut SpellLanePermit,
        pending: PendingElementalArrival,
    ) {
        let window = permit.open_commit_window(pending, park);
        let pending = window.attempt();
        let result = self
            .root
            .reconcile_item_loop_move(
                self.character,
                pending.fence.character_id,
                pending.command,
                pending.request.clone(),
            )
            .await;
        match result {
            Ok(Some(Ok(_))) => {
                self.states.loop_ground.invalidate();
                self.states.browse_fields.committed_move(
                    ItemKey::Instance(pending.request.item),
                    Some(pending.destination),
                );
                drop(window.install());
            }
            Ok(_) => {
                drop(window.release());
            }
            Err(_) => {} // Cancellation and unavailable truth keep the lane fenced.
        }
    }
}
