//! Free source NPC teleports use the existing held, recoverable arrival projection.
use super::*;
use crate::durability::character_last_position::LastPositionCell;
use crate::durability::npc_travel::{PendingArrival, TravelOccurrence};
use crate::durability::quest_state::npc_program::teleport::NpcTeleportAdmission;
use crate::gameplay_transport::{
    newhaven_travel::{Departure, Invocation},
    npc_travel::{FrozenTravel, TravelCapture, TravelHoldSlot},
};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, npc_rules::NpcRulePosition};

#[derive(Clone)]
pub(in crate::gameplay_transport) struct DepartureProof {
    pub request: NpcProgramRequest,
    pub catalogue: Arc<QuestStateCatalogue>,
    pub intent: NpcTalkIntent,
    pub context: Context,
    pub magic: usize,
}

pub(super) fn start(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    command: UseCommand,
    proof: DepartureProof,
    destination: NpcRulePosition,
    runtime: &mut ChannelRuntimeV1,
) -> Option<()> {
    let destination = NpcTeleportAdmission::runtime_destination(destination)?;
    let session = command.game_session_id;
    let fence = command.item_fence?;
    if admission.states.npc_conversations.current_npc(
        &admission.states.chat,
        runtime,
        actor,
        session,
        &proof.intent,
    ) != Some(proof.context.clone())
    {
        return None;
    }
    let states = admission.states.spell_states.try_lock().ok()?;
    if states.is_dead(actor) || !states.spell_idle(runtime, actor, session) {
        return None;
    }
    let cell = crate::foundation::MovementLocalPosition {
        x: destination.x,
        y: destination.y,
        floor: destination.floor,
    };
    if !super::super::super::respawn_cell_admissible(
        admission.movement_cells().index(),
        admission.movement_cells().scope(),
        admission.world_id,
        runtime.content_pin().server_artifact_digest(),
        &std::collections::BTreeSet::new(),
        cell,
    ) || admission.bundle_map().is_some_and(|map| {
        crate::content::bundle_spell_tiles::tile_pos(crate::content::LogicalCell {
            x: cell.x,
            y: cell.y,
            z: i32::from(cell.floor),
        })
        .is_none_or(|tile| map.facts.house_tile(tile) || map.facts.doors().blocks(tile))
    }) {
        return None;
    }
    let position = runtime.read_actor_position(actor).ok()?;
    let capture = TravelCapture {
        session: fence,
        content: position.context(),
        position_revision: position.revision(),
    };
    let occurrence = TravelOccurrence::from_bytes(super::super::super::npc_trade_command::derived(
        b"oteryn:quest-npc-teleport:v1",
        session,
        command.command_id,
    ))
    .ok()?;
    let frozen = FrozenTravel {
        command: proof.request.command,
        npc: proof.request.npc.clone(),
        route: proof.request.transition_key.clone(),
        fare: 0,
        arrival: PendingArrival {
            occurrence,
            world_id: admission.world_id,
            destination: LastPositionCell {
                x: destination.x,
                y: destination.y,
                floor: destination.floor,
            },
        },
    };
    let work = runtime.issue_owner_work().ok()?;
    let owner = runtime.owner_fence().ok()?;
    let mut slot = TravelHoldSlot::default();
    slot.start(
        frozen.clone(),
        capture.clone(),
        work,
        owner,
        std::time::Instant::now(),
    )
    .ok()?;
    let mut holds = admission.states.newhaven_travel.holds.lock().ok()?;
    if holds.contains_key(&session) {
        return None;
    }
    holds.insert(
        session,
        Invocation {
            actor,
            npc: proof.intent.npc_actor,
            capture,
            frozen,
            route: Departure::Quest(Box::new(proof)),
            refusal: None,
            slot,
            committed: false,
            placed: None,
        },
    );
    Some(())
}

pub(in crate::gameplay_transport) async fn commit(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    proof: DepartureProof,
    arrival: PendingArrival,
    capture: &TravelCapture,
) -> Option<
    Result<
        Option<PendingArrival>,
        crate::durability::character_progression::CharacterProgressionError,
    >,
> {
    // Historical response is immutable evidence, not fresh conversation authority.
    match admission
        .root
        .reconcile_npc_program(
            admission.character,
            fence.character_id,
            proof.request.clone(),
        )
        .await
    {
        Ok(Some(_)) => return read_arrival(admission, fence, arrival).await,
        Ok(None) => {}
        Err(error) => return Some(Err(error)),
    }
    let permit = admission.spell_lane_permit().await?;
    let mut slot = admission
        .revision_sequencer
        .acquire(fence.character_id)
        .await;
    {
        let runtime = admission.runtime.lock().await;
        let states = admission.states.spell_states.try_lock().ok()?;
        let position = runtime.read_actor_position(actor).ok()?;
        if position.context() != capture.content
            || position.revision() != capture.position_revision
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, fence.game_session_id)
            || admission.states.npc_conversations.current_npc(
                &admission.states.chat,
                &runtime,
                actor,
                fence.game_session_id,
                &proof.intent,
            ) != Some(proof.context)
        {
            return Some(Err(
                crate::durability::character_progression::CharacterProgressionError::InvalidInput,
            ));
        }
    }
    let native = admission.active_generation?.native_gameplay()?;
    let result = slot
        .commit_npc_teleport(
            admission.root,
            admission.character,
            admission.holder,
            &permit,
            fence,
            proof.request,
            proof.catalogue,
            &super::super::super::item_ref_admission::NativeItemWeights(native)
                .with_wheel(admission.premium_coordinator),
            &admission.premium_coordinator,
            NpcTeleportAdmission { arrival },
        )
        .await;
    match result {
        Ok(commit)
            if commit.read_only
                || matches!(
                    commit.transition,
                    QuestTransitionOutcome::Committed(_)
                        | QuestTransitionOutcome::AlreadyCommitted(_)
                ) =>
        {
            read_arrival(admission, fence, arrival).await
        }
        Ok(_) => Some(Err(
            crate::durability::character_progression::CharacterProgressionError::InvalidInput,
        )),
        Err(error) => Some(Err(error)),
    }
}

async fn read_arrival(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    arrival: PendingArrival,
) -> Option<
    Result<
        Option<PendingArrival>,
        crate::durability::character_progression::CharacterProgressionError,
    >,
> {
    let pending = admission
        .root
        .read_npc_pending_arrival(admission.character, admission.holder, fence)
        .await
        .ok()?;
    if pending.is_some_and(|p| p != arrival) {
        return Some(Err(crate::durability::character_progression::CharacterProgressionError::ConflictingOccurrence));
    }
    Some(Ok(pending))
}
