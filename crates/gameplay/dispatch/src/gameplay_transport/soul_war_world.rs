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
