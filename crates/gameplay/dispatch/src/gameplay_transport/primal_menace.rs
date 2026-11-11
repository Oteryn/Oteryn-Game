//! Crystal 201189f the_primal_menace.lua:156-347: the actual Instance owner's
//! cadence schedules source beasts/pods and retires beasts into Fungosaurus.
use super::ComposedFreshAdmission;
use crate::content::QualifiedNativeEntryRoom;
use crate::content::native_gameplay::NativeGameplayState;
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, MovementLocalPosition, RuntimeScopeRefV1,
};
use oteryn_gameplay_encounters::bosses::hazard_primal::primal_menace_think::{
    self as rules, PrimalMenaceEffect, PrimalMenaceThinkFacts, PrimalMenaceThinkOwner, WaveKind,
};
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};

#[derive(Default)]
pub(super) struct PrimalMenace {
    // A leaf lock, taken only under this exact runtime owner. Exact actor refs
    // include scope/generation: another Instance or a reused slot inherits nothing.
    live: Mutex<Vec<(ExactActorRef, Fight)>>,
    next_tick_ms: AtomicU64,
    qualification: Mutex<Option<Qualification>>,
}
struct Qualification {
    scope: RuntimeScopeRefV1,
    generation: u64,
    digest: [u8; 32],
    ready: bool,
}
#[derive(Default)]
struct Fight {
    think: PrimalMenaceThinkOwner,
    draw: u64,
    next_beast: u64,
    beasts: Vec<(u64, ExactActorRef)>,
    pending: Vec<PrimalMenaceEffect>,
}
/// Ordinary qualified creatures, not an arena substitute or an alternate
/// encounter entry. Failed policy installation rolls the native admission back.
pub(super) fn spawn_creature(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    key: &str,
    position: MovementLocalPosition,
) -> Option<ExactActorRef> {
    runtime.owner_fence().ok()?;
    if !super::arena_match::walkable(owner, runtime, position)
        || !runtime.native_summon_cell_free(position)
    {
        return None;
    }
    admit_source_creature(runtime, key, position)
}

/// Crystal hazard_primal.lua:64 and game_functions.cpp:632-651, map.cpp:234-247:
/// the pod's timeout uses force=true on an existing tile, even if occupied or
/// newly blocked. This never grants a force flag to unrelated spawn callers.
pub(super) fn spawn_fungosaurus_from_pod(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    position: MovementLocalPosition,
) -> Option<ExactActorRef> {
    runtime.owner_fence().ok()?;
    let cells = &owner.movement_cells;
    if cells.scope().generation_digest != runtime.content_pin().server_artifact_digest()
        || cells
            .index()
            .lookup(
                cells.scope(),
                crate::content::LogicalCell {
                    x: position.x,
                    y: position.y,
                    z: i32::from(position.floor),
                },
            )
            .is_err()
    {
        return None;
    }
    admit_source_creature(runtime, rules::FUNGOSAURUS_KEY, position)
}

fn admit_source_creature(
    runtime: &mut ChannelRuntimeV1,
    key: &str,
    position: MovementLocalPosition,
) -> Option<ExactActorRef> {
    let policy = runtime.companion_policy(key).ok()?;
    if policy.definition_key != key {
        return None;
    }
    let actor = runtime
        .admit_encounter_actor(key, policy.maximum_health, position)
        .ok()?;
    if runtime.install_creature_policy(actor, key).is_err() {
        let _ = runtime.remove_encounter_actor(actor);
        return None;
    }
    Some(actor)
}

pub(super) fn tick(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    content: &NativeGameplayState,
    now_ms: u64,
) {
    if runtime.owner_fence().is_err()
        || room.movement_cells().scope().generation_digest
            != runtime.content_pin().server_artifact_digest()
        || content.source_digest() != runtime.content_pin().server_artifact_digest()
    {
        return;
    }
    let Ok(mut qualification) = owner.states.primal_menace.qualification.lock() else {
        return;
    };
    let digest = content.source_digest();
    let scope = runtime.binding().runtime_scope();
    let generation = runtime.binding().scope_generation().get();
    let reset = qualification.as_ref().is_none_or(|known| {
        known.digest != digest || known.scope != scope || known.generation != generation
    });
    if reset {
        *qualification = Some(Qualification {
            scope,
            generation,
            digest,
            ready: crate::content::bosses::primal_closure::qualify(content)
                .is_some_and(|closure| closure.source_digest() == digest),
        });
        owner
            .states
            .primal_menace
            .next_tick_ms
            .store(0, Ordering::Relaxed);
    }
    let qualified = qualification.as_ref().is_some_and(|known| known.ready);
    drop(qualification);
    if reset {
        let Ok(mut live) = owner.states.primal_menace.live.lock() else {
            return;
        };
        live.clear();
    }
    // Connections share the existing Creature think cadence; network traffic
    // cannot multiply the census, source scheduler or volatile pod damage turns.
    if now_ms
        < owner
            .states
            .primal_menace
            .next_tick_ms
            .load(Ordering::Relaxed)
    {
        return;
    }
    owner.states.primal_menace.next_tick_ms.store(
        now_ms.saturating_add(crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS),
        Ordering::Relaxed,
    );
    // The pod owner independently proves its pin and must also forget old
    // generation occurrences when this generation no longer qualifies Primal.
    super::primal_pods::tick(owner, runtime, now_ms);
    if !qualified {
        return;
    }
    let Ok(census) = runtime.positioned_melee_census() else {
        return;
    };
    let Ok(mut live) = owner.states.primal_menace.live.lock() else {
        return;
    };
    for (actor, _, session) in &census {
        if session.is_some() || live.iter().any(|(known, _)| known == actor) {
            continue;
        }
        if runtime.companion_snapshot(*actor).is_ok_and(|s| {
            s.state.policy.definition_key == rules::PRIMAL_MENACE_KEY && s.health > 0
        }) {
            live.push((*actor, Fight::default()));
        }
    }
    live.retain_mut(|(boss, fight)| {
        if boss.runtime_scope() != scope || boss.scope_generation().get() != generation {
            return false;
        }
        let snapshot = runtime
            .companion_snapshot(*boss)
            .ok()
            .filter(|s| s.health > 0);
        if let Some(snapshot) = snapshot {
            let shares = runtime
                .borrow_combat_death()
                .reward_damage_shares(*boss)
                .unwrap_or_default();
            let levels: Vec<_> = shares
                .iter()
                .filter_map(|share| {
                    let session = share.session?;
                    let player = census.iter().find_map(|(actor, _, bound)| {
                        (*bound == Some(session)).then_some(*actor)
                    })?;
                    runtime.hazard_level(player, session)
                })
                .collect();
            let root = GameplayDecisionRoot::from_bytes(content.source_digest());
            let occurrence = DecisionOccurrenceId::from_bytes(boss.placement_identity());
            let effects = fight.think.think(
                &PrimalMenaceThinkFacts {
                    position: snapshot.position,
                    now_ms,
                    health: u64::try_from(snapshot.health).unwrap_or(0),
                    max_health: u64::try_from(snapshot.maximum_health).unwrap_or(0),
                    damage_map_levels: &levels,
                },
                || {
                    let value = deterministic_decision_u64(
                        &root,
                        occurrence,
                        "primal-menace-think",
                        fight.draw,
                    );
                    fight.draw = fight.draw.saturating_add(1);
                    value.map_or(rules::DRAW_PPM, |value| {
                        ((u128::from(value) * u128::from(rules::DRAW_PPM)) >> 64) as u32
                    })
                },
                |at| {
                    super::arena_match::walkable(owner, runtime, at)
                        && runtime.native_summon_cell_free(at)
                },
                |token| {
                    fight
                        .beasts
                        .iter()
                        .find_map(|(id, beast)| (*id == token).then_some(*beast))
                        .and_then(|beast| {
                            runtime.live_creature_health(beast).and_then(|_| {
                                runtime
                                    .read_actor_position(beast)
                                    .ok()
                                    .map(|p| p.position())
                            })
                        })
                },
            );
            if let Ok(effects) = effects {
                fight.pending.extend(effects);
            }
        }
        let pending = std::mem::take(&mut fight.pending);
        for effect in pending {
            let due = match effect {
                PrimalMenaceEffect::SpawnSparkle { at_ms, .. }
                | PrimalMenaceEffect::ScheduledSpawn { at_ms, .. } => at_ms,
                PrimalMenaceEffect::ReplaceWithFungosaurus { .. } => now_ms,
            };
            if due > now_ms {
                fight.pending.push(effect);
                continue;
            }
            match effect {
                PrimalMenaceEffect::SpawnSparkle { position, .. } => {
                    super::primal_pods::sparkle(owner, runtime, position, now_ms);
                }
                PrimalMenaceEffect::ScheduledSpawn {
                    kind: WaveKind::Pod,
                    position,
                    ..
                } => {
                    // Source spawnPod ignores the boss ID: an already scheduled
                    // pod still appears if its boss died during the three seconds.
                    super::primal_pods::spawn(owner, runtime, position, now_ms);
                }
                PrimalMenaceEffect::ScheduledSpawn {
                    kind: WaveKind::Beast,
                    position,
                    ..
                } => {
                    // Source spawnMonster:273-277 requires the original monster.
                    if runtime.live_creature_health(*boss).is_none() {
                        continue;
                    }
                    let root = GameplayDecisionRoot::from_bytes(content.source_digest());
                    let value = deterministic_decision_u64(
                        &root,
                        DecisionOccurrenceId::from_bytes(boss.placement_identity()),
                        "primal-menace-beast",
                        fight.draw,
                    );
                    fight.draw = fight.draw.saturating_add(1);
                    let Some(key) = value.ok().and_then(|v| {
                        rules::pick_beast(
                            ((u128::from(v) * u128::from(rules::DRAW_PPM)) >> 64) as u32,
                        )
                    }) else {
                        continue;
                    };
                    if let Some(beast) = spawn_creature(owner, runtime, key, position) {
                        if let Some(token) = fight.next_beast.checked_add(1) {
                            fight.next_beast = token;
                            fight.beasts.push((token, beast));
                            fight.think.record_beast(token, now_ms);
                        } else {
                            let _ = runtime.remove_encounter_actor(beast);
                        }
                    }
                }
                PrimalMenaceEffect::ReplaceWithFungosaurus {
                    beast: token,
                    position,
                } => {
                    if let Some(index) = fight.beasts.iter().position(|(id, _)| *id == token) {
                        let beast = fight.beasts[index].1;
                        if runtime.remove_encounter_actor(beast).is_err() {
                            fight.pending.push(effect);
                            continue;
                        }
                        fight.beasts.swap_remove(index);
                        let _ = spawn_creature(owner, runtime, rules::FUNGOSAURUS_KEY, position);
                    }
                }
            }
        }
        fight
            .beasts
            .retain(|(_, beast)| runtime.live_creature_health(*beast).is_some());
        runtime.live_creature_health(*boss).is_some() || !fight.pending.is_empty()
    });
}
