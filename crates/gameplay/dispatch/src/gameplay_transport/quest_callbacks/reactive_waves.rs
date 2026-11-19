use super::*;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::QuestTrackChange;
use world_callbacks::reactive_waves::{Chance, Retaliation};

// Crystal's Demon Oak retaliation is part of the accepted axe blow, with no
// additional timer. Use the same placement and native Creature owner as timed waves.
pub(super) fn selected_wave<'p>(
    plan: &'p Retaliation,
    changes: &[QuestTrackChange],
) -> &'p [String] {
    let Some(hit) = changes.iter().find(|change| change.track == plan.progress) else {
        return &[];
    };
    let progress = plan
        .progress_floor
        .map_or(hit.before, |floor| hit.before.max(floor));
    if let Some(wave) = plan.special.iter().find(|wave| wave.before == progress) {
        let last = plan.weak_points.iter().all(|track| {
            changes
                .iter()
                .any(|change| change.track == *track && change.after == plan.completed_at)
        });
        if last {
            return std::slice::from_ref(&plan.last);
        }
        return &wave.creatures;
    }
    if wins(plan.ordinary_chance) {
        &plan.ordinary
    } else {
        &[]
    }
}

fn wins(chance: Chance) -> bool {
    super::periodic_bosses::random_below(u32::from(chance.rolls))
        .is_some_and(|roll| roll < u32::from(chance.wins))
}

#[allow(
    clippy::too_many_arguments,
    reason = "all effects stay inside the admitted world callback turn"
)]
pub(super) fn publish(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    map: &BundleMap,
    actor: ExactActorRef,
    session: GameSessionId,
    callback: &Callback,
    wave: &[String],
) {
    if let Some(plan) = &callback.effects.retaliation
        && !wave.is_empty()
        && let Some(native) = owner
            .active_generation
            .and_then(|active| active.native_gameplay())
        && native.install_companion_policies(runtime).is_ok()
    {
        for creature in wave {
            if let Some(index) = super::periodic_bosses::random_below(plan.positions.len() as u32) {
                // Crystal actions_demon_oak.lua:106-118 passes extended=false, force=true.
                let _ = super::periodic_bosses::spawn_forced(
                    owner,
                    runtime,
                    map,
                    native,
                    creature,
                    plan.positions[index as usize],
                    false,
                );
            }
        }
    }
    if let Some(hit) = callback.effects.earth_hit
        && wins(hit.chance)
        && let Some(offset) =
            super::periodic_bosses::random_below(u32::from(hit.maximum - hit.minimum) + 1)
    {
        let committed = states.apply_creature_damage_kind(
            runtime,
            actor,
            session,
            u32::from(hit.minimum) + offset,
            crate::player_lethal::PlayerHitKind::Earth,
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                owner.owner_now().get(),
            ),
            oteryn_protocol_oteryn::analyser::AnalyserDamageSource::Creature(None),
        );
        if committed.is_some()
            && let Ok(position) = runtime.read_actor_position(actor)
            && let Some(at) = native(position.position())
        {
            map.facts.emit_movement_effect(at, hit.effect);
        }
    }
}

pub(super) fn finish_area_clear(runtime: &ChannelRuntimeV1, callback: &Callback) -> bool {
    let Some(room) = &callback.effects.finish_area else {
        return true;
    };
    runtime.current_live_creature_census().is_ok_and(|actors| {
        actors.into_iter().all(|actor| {
            runtime
                .read_actor_position(actor)
                .ok()
                .and_then(|at| native(at.position()))
                .is_some_and(|at| {
                    !room.contains(Position {
                        x: at.x,
                        y: at.y,
                        floor: at.floor,
                    }) || runtime
                        .companion_snapshot(actor)
                        .is_ok_and(|snapshot| snapshot.state.master.is_some())
                })
        })
    })
}
