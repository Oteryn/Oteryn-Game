//! Accepted source lever roster, before the existing party handoff exposes Players.
use super::*;
use crate::durability::activity_handoff::boss_room::{
    QualifiedBossPadPrefix, SourceBossLeverExtra,
};
#[derive(Debug)]
pub(super) struct Entry {
    handoff: [u8; 16],
    sparks: [Option<(ExactActorRef, u64)>; 4],
}
#[allow(
    clippy::too_many_arguments,
    reason = "accepted prefix, durable handoff, current destination runtime, native policy, AI owner and birth clock stay distinct"
)]
pub(super) fn initialize(
    owner: &mut NativeCallbackState,
    runtime: &mut ChannelRuntimeV1,
    active: &crate::content::ActiveGeneration,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    ai: &mut oteryn_gameplay_monster_ai::ai_think::CreatureAiTable,
    prefix: &QualifiedBossPadPrefix,
    handoff: [u8; 16],
    now: u64,
) -> Result<(), ()> {
    if !matches!(
        runtime.binding().runtime_scope(),
        crate::foundation::RuntimeScopeRefV1::Instance { .. }
    ) || prefix.refusal().is_some()
        || prefix.pads().is_empty()
        || prefix
            .pads()
            .iter()
            .any(|pad| pad.kind() != SourceBossLeverExtra::Outburst)
    {
        return Err(());
    }
    let context = active
        .qualify_callback_source_context(
            room,
            runtime.content_pin(),
            crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackOutburst,
        )
        .ok_or(())?;
    let source = native.outburst_events_source().map_err(|_| ())?;
    if let Some(entry) = &owner.outburst_entry {
        if entry.handoff != handoff {
            return Err(());
        }
    } else {
        if runtime
            .positioned_actor_census()
            .map_err(|_| ())?
            .iter()
            .any(|(_, _, session)| session.is_some())
        {
            return Err(());
        }
        let sample = runtime
            .native_null_health_entry_sample(native, source.callback_source())
            .map_err(|_| ())?;
        owner.outburst.reset_entry_sample(sample).ok_or(())?;
        owner.outburst_entry = Some(Entry {
            handoff,
            sparks: [None; 4],
        });
    }
    let mut parents = runtime
        .positioned_actor_census()
        .map_err(|_| ())?
        .into_iter()
        .filter_map(|(actor, _, session)| {
            (session.is_none()
                && runtime.matches_live_creature_identity(actor, source.outburst().key.as_bytes()))
            .then_some(actor)
        });
    let parent = parents.next().ok_or(())?;
    if parents.next().is_some() {
        return Err(());
    }
    let spec = oteryn_gameplay_entities::NativeEncounterSpawnSpec::qualified(
        &source.outburst().key,
        &source.spark().key,
        &source.spark().revision,
        native.source_digest(),
    )
    .map_err(|_| ())?;
    for (ordinal, (x, y)) in [
        (32229, 31282),
        (32230, 31287),
        (32237, 31287),
        (32238, 31282),
    ]
    .into_iter()
    .enumerate()
    {
        let entry = owner.outburst_entry.as_mut().ok_or(())?;
        let (actor, born) = if let Some(birth) = entry.sparks[ordinal] {
            birth
        } else {
            let (x, y, floor) = context
                .source_frame()
                .physical_position([x, y, 14])
                .ok_or(())?;
            let at = MovementLocalPosition { x, y, floor };
            // This destination is a fresh accepted Instance. Prove the literal
            // centre is already eligible; no force grant or fallback is invented.
            if runtime
                .current_top_actor_on_cell(at)
                .map_err(|_| ())?
                .is_some()
                || room
                    .movement_cells()
                    .index()
                    .lookup(
                        room.movement_cells().scope(),
                        crate::content::LogicalCell {
                            x,
                            y,
                            z: i32::from(floor),
                        },
                    )
                    .map_err(|_| ())?
                    != crate::content::CollisionClass::Walkable
            {
                return Err(());
            }
            let stamp = runtime.issue_owner_work().map_err(|_| ())?;
            let actor = runtime
                .admit_current_native_encounter_masterless(
                    stamp,
                    parent,
                    &spec,
                    at,
                    |current, at| {
                        Some(
                            current.current_top_actor_on_cell(at).ok()?.is_none()
                                && room
                                    .movement_cells()
                                    .index()
                                    .lookup(
                                        room.movement_cells().scope(),
                                        crate::content::LogicalCell {
                                            x: at.x,
                                            y: at.y,
                                            z: i32::from(at.floor),
                                        },
                                    )
                                    .ok()?
                                    == crate::content::CollisionClass::Walkable,
                        )
                    },
                )
                .map_err(|_| ())?;
            entry.sparks[ordinal] = Some((actor, now));
            (actor, now)
        };
        if ai.state(actor).is_none() {
            ai.admit_native_creature(runtime, native, actor, source.spark(), born)
                .map_err(|_| ())?;
        }
    }
    Ok(())
}
