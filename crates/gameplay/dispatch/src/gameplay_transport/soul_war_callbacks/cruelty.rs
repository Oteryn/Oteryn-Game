//! Crystal201189f goshnars_cruelty.lua136–152; soul_war.lua428/1477–1493.
//! One accepted Instance owns the source type counter (BOSS_RAID0 isolation).
use super::*;
pub(in crate::gameplay_transport) mod maw;
const BOSS: &str = "oteryn:creature.goshnar_s_cruelty";
const EYE: &str = "oteryn:creature.a_greedy_eye";
#[derive(Debug)]
pub(super) struct CrueltyCallbacks {
    boss: ExactActorRef,
    eye: ExactActorRef,
    content: [u8; 32],
    last: Option<ThinkOccurrence>,
    disappeared: bool,
}
impl CrueltyCallbacks {
    pub(super) fn initialize(
        runtime: &mut ChannelRuntimeV1,
        native: &NativeGameplayState,
        room: &QualifiedNativeEntryRoom,
        boss: ExactActorRef,
    ) -> Result<Self, ()> {
        use crate::content::bosses::activity_template::PinnedActivitySourceRoom as Role;
        if room.pinned_activity_source_room() != Some(Role::GoshnarsCruelty)
            || !qualifies_source(runtime, native)
            || source_actor(runtime, native, boss).as_deref() != Some(BOSS)
            || runtime.content_pin().frame_binding_digest() != room.frame_binding().digest()
            || runtime.map_revision_digest() != room.map_revision_digest()
            || runtime
                .read_actor_position(boss)
                .map_err(|_| ())?
                .position()
                != (MovementLocalPosition {
                    x: 33856,
                    y: 31866,
                    floor: 7,
                })
        {
            return Err(());
        }
        // BossLever.lua228–229: extended=true, force=true. The requested real
        // source tile exists even when occupied; no free-cell admission is added.
        let at = MovementLocalPosition {
            x: 33856,
            y: 31858,
            floor: 7,
        };
        let eye = runtime
            .spawn_soul_war_child(boss, EYE, at, |_, at| {
                super::super::soul_war_scene::current_ground(room, at).map(|_| true)
            })
            .map_err(|_| ())?;
        if source_actor(runtime, native, eye).as_deref() != Some(EYE) {
            return Err(());
        }
        Ok(Self {
            boss,
            eye,
            content: native.source_digest(),
            last: None,
            disappeared: false,
        })
    }
}
pub(super) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    occurrences: &[ThinkOccurrence],
) -> Result<(), ()> {
    // The original source USE owns its clock and target until its fused Item/
    // Quest receipt is known. Unknown SQL must not advance a competing Think.
    if states.soul_war_callbacks.maw.pending.is_some() {
        return Ok(());
    }
    // The existing owners stay in their slots while the current UTC read is
    // pending. Cancelling a Map read must not discard the boss or its clock.
    let SoulWarCallbacks { cruelty, maw, .. } = &mut states.soul_war_callbacks;
    let Some(owner) = cruelty.as_mut() else {
        return Err(());
    };
    if owner.content != runtime.content_pin().server_artifact_digest()
        || !qualifies_source(runtime, native)
    {
        return Err(());
    }
    runtime.owner_fence().map_err(|_| ())?;
    let boss = runtime
        .companion_snapshot(owner.boss)
        .ok()
        .filter(|boss| boss.health > 0 && boss.state.policy.definition_key == BOSS);
    let Some(boss) = boss else {
        if !owner.disappeared {
            // The source global name lookup removes one Eye, not every actor
            // of that type. This literal lever child is the oldest admitted
            // Eye in this private source room.
            if runtime.contains_live_creature(owner.eye)
                && source_actor(runtime, native, owner.eye).as_deref() == Some(EYE)
            {
                runtime.remove_encounter_actor(owner.eye).map_err(|_| ())?;
            }
            owner.disappeared = true;
        }
        return Ok(());
    };
    for &occurrence in occurrences {
        if occurrence.actor != owner.boss {
            continue;
        }
        if owner
            .last
            .is_some_and(|last| occurrence.sequence <= last.sequence)
        {
            continue;
        }
        let utc = map.source_observed_seconds(runtime).await.ok_or(())?;
        let mut next = maw.clock.clone();
        if next.think(utc, 1000).is_some() {
            let expected = runtime.soul_war_cruelty_drain(&boss).map_err(|_| ())?;
            runtime
                .increase_soul_war_cruelty(&boss, expected)
                .map_err(|_| ())?;
        }
        maw.clock = next;
        owner.last = Some(occurrence);
    }
    Ok(())
}
