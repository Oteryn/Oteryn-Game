//! Ancient Tombs AddItem route. The move transaction owns custody and the exact
//! recipient observation; the Channel publishes the position from that receipt.
use super::*;
use crate::durability::item_loop_move::{CoalBasinRecipient, CoalBasinTarget, LoopMoveReceipt};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::terrain_tool::TerrainScope;
use crate::foundation::{ChannelRuntimeV1, GameSessionId, MovementLocalPosition};
use crate::map::boot::BundleMap;
use oteryn_gameplay_quests::quest::QuestRefusal;
use oteryn_gameplay_quests::quest::carried_step::{COAL_BASINS, CoalBasin};

fn scope(map: &BundleMap, content: [u8; 32], reset_epoch: u64) -> Option<TerrainScope> {
    Some(TerrainScope {
        map_revision: map_revision(map.overlay.base()),
        content_generation: content,
        reset_epoch: reset_epoch.try_into().ok()?,
    })
}

fn movement((x, y, floor): (i32, i32, i16)) -> MovementLocalPosition {
    MovementLocalPosition { x, y, floor }
}

fn at_basin(basin: &CoalBasin) -> TilePos {
    let (x, y, floor) = basin.basin;
    TilePos { x, y, floor }
}

pub(super) fn target(
    map: &BundleMap,
    runtime: &ChannelRuntimeV1,
    to: WorldTilePosition,
    reset_epoch: u64,
) -> Option<CoalBasinTarget> {
    let pos = TilePos {
        x: to.x.try_into().ok()?,
        y: to.y.try_into().ok()?,
        floor: to.floor.try_into().ok()?,
    };
    let basin = COAL_BASINS.iter().find(|basin| at_basin(basin) == pos)?;
    let facts = map.facts.pinned();
    let composed = view::compose_tile(&map.overlay, &facts, pos).ok()??;
    let entry = composed
        .entries
        .iter()
        .find(|entry| entry.facts.appearance_id == 2114)?;
    let view::EntrySource::Base {
        placement_key: key,
        ordinal: 1,
        ..
    } = entry.source
    else {
        return None;
    };
    let base = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
    let id = base
        .ids()
        .iter()
        .zip(base.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(1)?
        .0;
    if !map
        .facts
        .quest_callback_matches(key, Some(basin.unique), None, Some(2114))
        && !map.facts.quest_callback_native_object(key, *id, 2114)
    {
        return None;
    }
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let recipient = runtime
        .positioned_melee_census()
        .ok()?
        .into_iter()
        .find_map(|(actor, position, session)| {
            let session = session?;
            (position.position() == movement(basin.flame)
                && runtime.is_current_top_actor(actor) == Ok(true)
                && runtime.bound_attacker_lease(actor, session).is_ok())
            .then_some(CoalBasinRecipient {
                session: *session.as_bytes(),
                actor: actor.placement_identity(),
                actor_generation: actor.actor_local_generation(),
                movement_revision: position.revision(),
            })
        });
    Some(CoalBasinTarget {
        scope: scope(
            map,
            runtime.content_pin().server_artifact_digest(),
            reset_epoch,
        )?,
        placement,
        expected_revision: facts.object_revision(key).try_into().ok()?,
        uid: basin.unique,
        recipient,
    })
}

fn recipient_actor(
    runtime: &ChannelRuntimeV1,
    basin: &CoalBasin,
    recipient: &CoalBasinRecipient,
    session: GameSessionId,
) -> Option<ExactActorRef> {
    runtime
        .positioned_melee_census()
        .ok()?
        .into_iter()
        .find_map(|(actor, position, observed_session)| {
            (observed_session == Some(session)
                && actor.placement_identity() == recipient.actor
                && actor.actor_local_generation() == recipient.actor_generation
                && position.revision() == recipient.movement_revision
                && position.position() == movement(basin.flame)
                && runtime.is_current_top_actor(actor) == Ok(true)
                && runtime.bound_attacker_lease(actor, session).is_ok())
            .then_some(actor)
        })
}

async fn progress(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    uid: u16,
    transaction: [u8; 16],
) -> bool {
    let Some(catalogue) = admission.quest_catalogue.as_ref() else {
        return false;
    };
    let Ok(Some(initial)) = admission.current_quest_fence(session).await else {
        return false;
    };
    let Some(mut slot) = admission
        .revision_sequencer
        .try_acquire(initial.character_id)
    else {
        return false;
    };
    let additional = match uid {
        9035 => Some("diphtrahs_treasure/other_2"),
        9036 => Some("mahrdis_treasure/other_1"),
        _ => None,
    };
    for transition in std::iter::once("default_start/movement_1").chain(additional) {
        let Ok(Some(fence)) = admission.current_quest_fence(session).await else {
            return false;
        };
        if fence.character_id != initial.character_id {
            return false;
        }
        let hash = Sha256::new()
            .chain_update(b"oteryn:coal-basin-progress:v1")
            .chain_update(transaction)
            .chain_update(transition.as_bytes())
            .finalize();
        let Ok(bytes) = <[u8; 8]>::try_from(&hash[..8]) else {
            return false;
        };
        let Ok(id) = CommandId::new(u64::from_be_bytes(bytes).max(1)) else {
            return false;
        };
        let request = QuestTransitionRequest {
            transition_key: format!(
                "oteryn:quest-transition/quest/u7_4/the_ancient_tombs/{transition}"
            ),
            cause: QuestCause::Step(CommandRef::new(session, id)),
        };
        if !matches!(
            slot.commit_quest_transition(
                admission.root,
                admission.character,
                admission.holder,
                fence,
                request,
                Arc::clone(catalogue)
            )
            .await,
            Ok(QuestTransitionOutcome::Committed(_)
                | QuestTransitionOutcome::AlreadyCommitted(_)
                | QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch))
        ) {
            return false;
        }
    }
    drop(slot);
    admission.schedule_quest_refresh(session);
    true
}

pub(super) async fn finish_drop(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    target: &CoalBasinTarget,
    receipt: &LoopMoveReceipt,
    offered: bool,
) -> bool {
    if offered || receipt.replayed {
        return publish(admission, fence, command, !receipt.replayed).await;
    }
    if let Some(map) = admission.bundle_map()
        && let Some(basin) = COAL_BASINS.iter().find(|basin| basin.unique == target.uid)
    {
        map.facts.emit_movement_effect(at_basin(basin), 3);
    }
    true
}

/// Only the original top player on the source flame can receive the committed
/// route. Replaying the move can recover an interrupted projection.
pub(super) async fn publish(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    fresh: bool,
) -> bool {
    let Some(map) = admission.bundle_map() else {
        return true;
    };
    let content = {
        let runtime = admission.runtime.lock().await;
        if runtime.owner_fence().is_err()
            || runtime.encounter_owner_scope()
                != (fence.runtime_scope, fence.scope_ownership_generation)
        {
            return false;
        }
        runtime.content_pin().server_artifact_digest()
    };
    let Some(scope) = scope(map, content, admission.reset_epoch) else {
        return false;
    };
    let Ok(offering) = admission
        .root
        .read_coal_basin_offering(admission.character, fence, command, scope)
        .await
    else {
        return false;
    };
    let Some((uid, transaction, recipient)) = offering else {
        return true;
    };
    let Some(basin) = COAL_BASINS.iter().find(|basin| basin.unique == uid) else {
        return false;
    };
    if fresh {
        map.facts.emit_movement_effect(at_basin(basin), 16);
    }
    let Some(recipient) = recipient else {
        return true;
    };
    let Ok(session) = GameSessionId::decode(&recipient.session) else {
        return false;
    };
    {
        let runtime = admission.runtime.lock().await;
        if recipient_actor(&runtime, basin, &recipient, session).is_none() {
            return true;
        }
    }
    if !progress(admission, session, uid, transaction).await {
        return false;
    }
    let mut runtime = admission.runtime.lock().await;
    let Some(actor) = recipient_actor(&runtime, basin, &recipient, session) else {
        return true;
    };
    let Ok(expected) = runtime.read_actor_position(actor) else {
        return true;
    };
    let states = admission.states.spell_states.lock().await;
    if states.is_dead(actor) || !states.spell_idle(&runtime, actor, session) {
        return true;
    }
    let blocked =
        super::super::pvp_live::pz_blocked(&states, actor, session, admission.owner_now());
    let doors = crate::map::floor::BundleDoors(&map.facts);
    let landing = movement(basin.landing);
    let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
        &runtime,
        admission.movement_cells(),
        actor,
        session,
        expected,
        landing,
        blocked,
        &doors,
    ) else {
        return true;
    };
    if runtime.commit_source_step(proof).is_ok() {
        let (Ok(x), Ok(y), Ok(floor)) = (
            u16::try_from(landing.x),
            u16::try_from(landing.y),
            i8::try_from(-landing.floor),
        ) else {
            return false;
        };
        map.facts.emit_movement_effect(TilePos { x, y, floor }, 11);
    }
    true
}
