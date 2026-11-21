//! Literal Soul War World triggers after the canonical physical STEP commits.
use super::ComposedFreshAdmission;
use crate::durability::raid_schedule::ClaustrophobicRaidStart;
use crate::foundation::{ExactActorRef, GameSessionId, MovementPositionSnapshot};
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_gameplay_encounter_rules::raid::source_claustrophobic::{Position, Raid, spec};
use sha2::{Digest, Sha256};

pub(super) async fn committed_step(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    origin: Option<MovementPositionSnapshot>,
    committed: MovementPositionSnapshot,
    _now: u64,
) {
    let Some(origin) = origin else { return };
    if hunt_entry(owner, actor, session, origin, committed).await {
        return;
    }
    let Some(room) = owner.effective_rooms().gate_monster_ai() else {
        return;
    };
    let Some(active) = owner.active_generation else {
        return;
    };
    let Some(native) = active.native_gameplay() else {
        return;
    };
    let runtime = owner.runtime.lock().await;
    let Ok(current) = runtime.read_actor_position(actor) else {
        return;
    };
    if current != committed
        || runtime.bound_attacker_lease(actor, session).is_err()
        || runtime.owner_fence().is_err()
        || !matches!(
            runtime.binding().runtime_scope(),
            crate::foundation::RuntimeScopeRefV1::Channel { .. }
        )
        || !native.qualifies_soul_war_callbacks(runtime.content_pin().server_artifact_digest())
    {
        return;
    }
    let Some(context) = active.qualify_published_world_context(room, runtime.content_pin()) else {
        return;
    };
    let frame = context.source_frame();
    let convert = |snapshot: MovementPositionSnapshot| {
        let p = snapshot.position();
        frame
            .source_position((p.x, p.y, p.floor))
            .map(|[x, y, z]| Position {
                x,
                y,
                floor: z as i16,
            })
    };
    let (Some(from), Some(to)) = (convert(origin), convert(committed)) else {
        return;
    };
    let Some(raid) = Raid::from_sand(to).filter(|raid| spec(*raid).ingress(from, to)) else {
        return;
    };
    let generation = runtime.binding().scope_generation();
    let mut hash = Sha256::new();
    hash.update(b"oteryn:soul-war:claustrophobic:committed-step:v1");
    hash.update(owner.world_id.as_bytes());
    hash.update(owner.channel_id.as_bytes());
    hash.update(owner.reset_epoch.to_be_bytes());
    hash.update(actor.placement_identity());
    hash.update(session.as_bytes());
    hash.update(generation.get().to_be_bytes());
    hash.update(origin.revision().to_be_bytes());
    hash.update(committed.revision().to_be_bytes());
    for position in [from, to] {
        hash.update(position.x.to_be_bytes());
        hash.update(position.y.to_be_bytes());
        hash.update(position.floor.to_be_bytes());
    }
    hash.update(runtime.content_pin().server_artifact_digest());
    hash.update(runtime.content_pin().frame_binding_digest());
    hash.update(runtime.content_pin().map_revision_digest());
    let digest = hash.finalize();
    let mut firing = [0u8; 16];
    firing.copy_from_slice(&digest[..16]);
    firing[6] = (firing[6] & 0x0f) | 0x70;
    firing[8] = (firing[8] & 0x3f) | 0x80;
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        room.movement_cells().index()
    else {
        return;
    };
    let request = ClaustrophobicRaidStart {
        world: owner.world_id,
        channel: owner.channel_id,
        generation,
        reset_epoch: owner.reset_epoch,
        bundle_digest: index.map().overlay.base().digest(),
        firing_id: firing,
        raid,
        from,
        to,
        now_ms: match crate::world_clock::now_utc_ms().and_then(|v| i64::try_from(v).ok()) {
            Some(v) => v,
            None => return,
        },
    };
    // The retained movement lane prevents another physical command overtaking
    // this observation. SQL independently validates its current ownership facts.
    drop(runtime);
    if owner
        .root
        .start_claustrophobic_raid(owner.character, owner.holder, request)
        .await
        .is_err()
    {
        super::operator_event("soul_war_claustrophobic_step_retry");
    }
}

/// Crystal 201189f moveevent-soul_war_entrances.lua:1-49. These are
/// position registrations in the published World, independent of boss rooms.
async fn hunt_entry(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    origin: MovementPositionSnapshot,
    committed: MovementPositionSnapshot,
) -> bool {
    let Some(room) = owner.rooms().gate_monster_ai() else {
        return false;
    };
    let Some(active) = owner.active_generation else {
        return false;
    };
    let runtime = owner.runtime.lock().await;
    let Some(context) = active.qualify_published_world_context(room, runtime.content_pin()) else {
        return false;
    };
    if !matches!(
        runtime.binding().runtime_scope(),
        crate::foundation::RuntimeScopeRefV1::Channel { .. }
    ) || runtime.read_actor_position(actor) != Ok(committed)
        || runtime.player_control_facts(actor, session).is_err()
    {
        return false;
    }
    let at = committed.position();
    let frame = context.source_frame();
    let destination = match frame.source_position((at.x, at.y, at.floor)) {
        Some([33615, 31422, 10]) => [34009, 31014, 9],
        Some([33618, 31422, 10]) => [33972, 31041, 11],
        Some([33621, 31422, 10]) => [33894, 31019, 8],
        Some([33624, 31422, 10]) => [33858, 31831, 3],
        Some([33627, 31422, 10]) => [33887, 31188, 10],
        _ => return false,
    };
    drop(runtime);
    let Ok(Some(fence)) = owner.current_quest_fence(session).await else {
        return true;
    };
    let Ok(state) = owner
        .root
        .read_soul_war_state(owner.character, owner.holder, fence)
        .await
    else {
        return true;
    };
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    if !context.matches(room, runtime.content_pin())
        || runtime.owner_fence().is_err()
        || runtime.read_actor_position(actor) != Ok(committed)
        || !runtime
            .player_control_facts(actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
        || !runtime
            .bound_attacker_lease(actor, session)
            .is_ok_and(|lease| {
                lease.character_id().as_bytes() == fence.character_id.as_bytes()
                    && lease.generation() == fence.character_lease_generation
            })
        || states.is_dead(actor)
    {
        return true;
    }
    let Some(player) = states.get(&runtime, actor, session) else {
        return true;
    };
    let message = if player.character_facts().level < 250 {
        Some("You need level 250 to enter here.")
    } else if !state.access() {
        Some("Your soul does not yet resonate with the frequency required to enter here.")
    } else {
        None
    };
    let destination = if message.is_some() {
        origin.position()
    } else {
        let Some((x, y, floor)) = frame.physical_position(destination) else {
            return true;
        };
        crate::foundation::MovementLocalPosition { x, y, floor }
    };
    let Some(map) = owner.bundle_map() else {
        return true;
    };
    let closed = crate::map::floor::BundleDoors(&map.facts);
    let blocked = super::pvp_live::pz_blocked(&states, actor, session, owner.owner_now());
    let Ok(proof) = crate::map::floor::prepare_scripted_quest_teleport_facing(
        &runtime,
        owner.movement_cells(),
        actor,
        session,
        committed,
        destination,
        blocked,
        &closed,
        committed.facing(),
        true,
    ) else {
        return true;
    };
    if runtime.commit_source_step(proof).is_err() {
        return true;
    }
    if let Some(message) = message {
        owner.states.server_messages.send_current(
            session,
            oteryn_protocol_oteryn::zones_nologout::ServerMessage::Advance(message.into()),
        );
    }
    // Convert the current physical frame to the map's negative source floor.
    let effect = |at: crate::foundation::MovementLocalPosition| {
        if let Some([x, y, z]) = frame.source_position((at.x, at.y, at.floor))
            && let Some(z) = z.checked_neg()
            && let (Ok(x), Ok(y), Ok(floor)) = (x.try_into(), y.try_into(), z.try_into())
        {
            map.facts
                .emit_movement_effect(crate::map::overlay::TilePos { x, y, floor }, 11);
        }
    };
    if message.is_none() {
        effect(origin.position());
    }
    effect(destination);
    true
}
