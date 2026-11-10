//! Crystal teleport_step_event.lua:76–221. Each original STEP retains two
//! independent callbacks. Each callback re-reads the named boss and its target;
//! a committed second hit retains its spawn cursor before the next Map await.
use super::*;
use crate::creature_damage_spell::native_event::rotten_massive::RottenMassiveSource;
use crate::durability::creature_source_items::RottenPortalCallback;
use crate::foundation::owner_timer::SemanticTimeMicros;

pub(super) const MAX_PENDING: usize = 16;

#[derive(Debug, Clone)]
enum Stage {
    Attack,
    Respawn {
        source: Arc<RottenBloodSpawnSource>,
        parent: ExactActorRef,
        count: u8,
        next: u8,
    },
}

#[derive(Debug, Clone)]
pub(super) struct Pending {
    callback: Arc<RottenPortalCallback>,
    pub(super) centre: RoomPosition,
    key: &'static str,
    pub(super) due_us: u64,
    pub(super) phase: u8,
    stage: Stage,
}

pub(super) fn deadlines(step_us: u64) -> Result<[u64; 2], ()> {
    Ok([
        step_us.checked_add(5_000_000).ok_or(())?,
        step_us.checked_add(15_000_000).ok_or(())?,
    ])
}

pub(super) fn set_paused(advanced: &mut AdvancedCallbacks, centre: RoomPosition, paused: bool) {
    if paused {
        if !advanced.paused_centres.contains(&centre) {
            advanced.paused_centres.push(centre);
        }
    } else {
        advanced.paused_centres.retain(|current| *current != centre);
    }
    for boss in &mut advanced.bosses {
        if center(&boss.key) == centre {
            boss.paused = paused;
        }
    }
}

fn named_boss(
    runtime: &ChannelRuntimeV1,
    centre: RoomPosition,
    key: &str,
) -> Result<Option<ExactActorRef>, ()> {
    Ok(runtime
        .source_registered_creature_census()
        .map_err(|_| ())?
        .into_iter()
        .find_map(|(actor, at)| {
            (near(centre, pos(at.position()), 15)
                && runtime
                    .companion_snapshot_including_dead(actor)
                    .is_ok_and(|snapshot| snapshot.state.policy.definition_key == key))
            .then_some(actor)
        }))
}

fn main_target(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    boss: ExactActorRef,
) -> Result<Option<(ExactActorRef, GameSessionId)>, ()> {
    let Some(target) = states
        .monster_melee
        .creature_ai
        .state(boss)
        .and_then(|state| state.target())
    else {
        return Ok(None);
    };
    Ok(runtime
        .positioned_melee_census()
        .map_err(|_| ())?
        .into_iter()
        .find_map(|(actor, _, session)| {
            (actor == target)
                .then_some(session)
                .flatten()
                .map(|session| (actor, session))
        }))
}

pub(super) fn schedule(
    owner: &super::super::super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    callback: Arc<RottenPortalCallback>,
    centre: RoomPosition,
) -> Result<(), ()> {
    let key = [vem::VEMIATH_KEY, chag::CHAGORZ_KEY, bak::BAKRAGORE_KEY]
        .into_iter()
        .find(|key| center(key) == centre)
        .ok_or(())?;
    let Some(boss) = named_boss(runtime, centre, key)? else {
        return Ok(());
    };
    if main_target(runtime, states, boss)?.is_none() {
        return Ok(());
    }
    let due = deadlines(callback.due_us())?;
    let recipients = runtime.positioned_melee_census().map_err(|_| ())?;
    let advanced = &mut states.rotten_blood_callbacks.advanced;
    if advanced.massive.len() + 2 > MAX_PENDING {
        return Err(());
    }
    for (actor, position, session) in recipients {
        if let Some(session) = session
            && near(centre, pos(position.position()), 15)
            && let Some(position) = owner.wire_observation(runtime, super::super::at(centre))
        {
            owner.states.chat.publish_rotten_massive_warning(
                runtime,
                actor,
                session,
                position.actor_position,
            );
        }
    }
    for (index, due_us) in due.into_iter().enumerate() {
        advanced.massive.push(Pending {
            callback: Arc::clone(&callback),
            centre,
            key,
            due_us,
            phase: index as u8 + 1,
            stage: Stage::Attack,
        });
    }
    states.owner_wake.notify_one();
    Ok(())
}

fn offset(pending: &Pending, native: &NativeGameplayState, ordinal: u8) -> Result<(i32, i32), ()> {
    let root = GameplayDecisionRoot::from_bytes(
        Sha256::new()
            .chain_update(b"oteryn:rotten-massive-respawn:v1")
            .chain_update(native.source_digest())
            .chain_update(pending.callback.item_instance_id())
            .finalize()
            .into(),
    );
    let mut bytes = [0; 16];
    bytes[..8].copy_from_slice(&pending.due_us.to_be_bytes());
    bytes[8..].copy_from_slice(&u64::from(ordinal).to_be_bytes());
    let occurrence = DecisionOccurrenceId::from_bytes(bytes);
    let draw = |axis: &str| {
        for retry in 0..64 {
            let value =
                deterministic_decision_u64(&root, occurrence, axis, retry).map_err(|_| ())?;
            if value < u64::MAX - u64::MAX % 3 {
                return Ok((value % 3) as i32 - 1);
            }
        }
        Err(())
    };
    Ok((draw("offset_x")?, draw("offset_y")?))
}

#[allow(
    clippy::too_many_arguments,
    reason = "source callbacks share the canonical physical, map and combat owners"
)]
pub(super) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut super::super::super::attack::ChannelAttackStates,
    notices: &super::super::super::raid_notices::ServerMessageHub,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    now: u64,
) -> Result<(), ()> {
    drive_through(
        runtime,
        states,
        attack,
        notices,
        room,
        native,
        map,
        now,
        now,
        MAX_PENDING,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn drive_one_due(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut super::super::super::attack::ChannelAttackStates,
    notices: &super::super::super::raid_notices::ServerMessageHub,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    through: u64,
    now: u64,
) -> Result<(), ()> {
    drive_through(
        runtime, states, attack, notices, room, native, map, through, now, 1,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
async fn drive_through(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut super::super::super::attack::ChannelAttackStates,
    notices: &super::super::super::raid_notices::ServerMessageHub,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    through: u64,
    now: u64,
    budget: usize,
) -> Result<(), ()> {
    for _ in 0..budget {
        let Some(index) = states
            .rotten_blood_callbacks
            .advanced
            .massive
            .iter()
            .enumerate()
            .filter(|(_, pending)| pending.due_us <= now && pending.due_us <= through)
            .min_by_key(|(_, pending)| pending.due_us)
            .map(|(index, _)| index)
        else {
            return Ok(());
        };
        let pending = states.rotten_blood_callbacks.advanced.massive[index].clone();
        match pending.stage.clone() {
            Stage::Attack => {
                let Some(boss) = named_boss(runtime, pending.centre, pending.key)? else {
                    states.rotten_blood_callbacks.advanced.massive.remove(index);
                    continue;
                };
                let Some((target, session)) = main_target(runtime, states, boss)? else {
                    states.rotten_blood_callbacks.advanced.massive.remove(index);
                    continue;
                };
                // Qualify every later fallible input before the irreversible HP step.
                // Both taint reads and the canonical source stamp survive that step.
                let respawn = if pending.phase == 2 {
                    let mut maximum = 0;
                    for (actor, at, session) in runtime.positioned_melee_census().map_err(|_| ())? {
                        if let Some(session) = session
                            && near(pending.centre, pos(at.position()), 15)
                        {
                            maximum = maximum.max(
                                states
                                    .rotten_blood_callbacks
                                    .taint(actor, session)
                                    .ok_or(())?,
                            );
                        }
                    }
                    Some(Stage::Respawn {
                        source: Arc::new(
                            runtime
                                .qualify_rotten_blood_spawn_source(boss, native)
                                .map_err(|_| ())?,
                        ),
                        parent: boss,
                        count: elder::bloodjaw_target_count(maximum),
                        next: 0,
                    })
                } else {
                    None
                };
                let source = RottenMassiveSource::from_current_callback(
                    runtime,
                    native,
                    pending.callback.item_instance_id(),
                    pending.due_us,
                    pending.phase,
                    boss,
                )
                .map_err(|_| ())?;
                let stamp = runtime.issue_owner_work().map_err(|_| ())?;
                let semantic = SemanticTimeMicros::from_micros(now);
                let mut reader = super::super::super::monster_profile_cycle::physical_spell::prepare_rotten_massive_reader(
                    runtime, states, attack, room, native, map, &source, target, session, now, stamp,
                ).await.ok_or(())?;
                let plan = source
                    .prepare_player(
                        runtime,
                        &mut reader,
                        states,
                        target,
                        session,
                        stamp,
                        semantic,
                    )
                    .map_err(|_| ())?;
                let visuals = super::super::super::soul_war_scene::source_map(room).ok_or(())?;
                let cue = match plan.visual_effect() {
                    Some(effect) => Some(
                        crate::map::facts::CreatureVisualCue::effect_from_registered_source(
                            runtime,
                            stamp,
                            boss,
                            super::super::super::soul_war_scene::wire_position(
                                runtime
                                    .read_actor_position(target)
                                    .map_err(|_| ())?
                                    .position(),
                            )
                            .ok_or(())?,
                            effect,
                        )
                        .ok_or(())?,
                    ),
                    None => None,
                };
                if !map.validate_transaction(runtime).await {
                    return Err(());
                }
                #[cfg(test)]
                let before = states
                    .get(runtime, target, session)
                    .ok_or(())?
                    .vitals()
                    .health;
                if let Some(cue) = cue {
                    visuals.facts.emit_creature_visual(runtime, cue);
                }
                let (requested, hit) = states
                    .commit_rotten_massive_hit(runtime, plan, semantic)
                    .map_err(|_| ())?;
                // No await or fallible operation separates HP from the retained
                // continuation. Retrying map/spawn work cannot repeat the hit.
                if let Some(stage) = respawn {
                    states.rotten_blood_callbacks.advanced.massive[index].stage = stage;
                } else {
                    states.rotten_blood_callbacks.advanced.massive.remove(index);
                }
                if let Some(hit) = hit {
                    super::super::super::monster_profile_cycle::physical_spell::record_native_player_hit(
                        runtime, attack, Some(boss), target, session, &hit, now,
                    );
                    #[cfg(test)]
                    {
                        let commits = &mut states.rotten_blood_callbacks.advanced.massive_commits;
                        if commits.len() >= 64 {
                            commits.remove(0);
                        }
                        commits.push((target, pending.phase, requested, before, hit.health_after));
                    }
                }
                let element = if pending.key == bak::BAKRAGORE_KEY {
                    "agony"
                } else {
                    "energy"
                };
                let phase = if pending.phase == 1 {
                    "first"
                } else {
                    "second"
                };
                notices.send_current(
                    session,
                    oteryn_protocol_oteryn::zones_nologout::ServerMessage::Advance(format!(
                        "You received {requested} {element} damage from the {phase} massive attack!"
                    )),
                );
                states.owner_wake.notify_one();
            }
            Stage::Respawn {
                source,
                parent,
                count,
                next,
            } => {
                if next == count {
                    set_paused(
                        &mut states.rotten_blood_callbacks.advanced,
                        pending.centre,
                        false,
                    );
                    states.rotten_blood_callbacks.advanced.massive.remove(index);
                    continue;
                }
                let (dx, dy) = offset(&pending, native, next)?;
                let candidate = RoomPosition {
                    x: pending.centre.x.checked_add(dx).ok_or(())?,
                    y: pending.centre.y.checked_add(dy).ok_or(())?,
                    floor: pending.centre.floor,
                };
                map.read(runtime, super::super::at(candidate)).await;
                let destination = if map
                    .source_is_walkable(runtime, super::super::at(candidate))
                    .ok_or(())?
                {
                    candidate
                } else {
                    pending.centre
                };
                // One random candidate, then literal centre fallback, then the
                // existing default C++ N8 placement. Failed placement still counts.
                super::super::spawn_child(
                    runtime,
                    states,
                    native,
                    map,
                    &source,
                    parent,
                    pending.due_us,
                    u64::from(next) + 1,
                    vem::ELDER_BLOODJAW_KEY,
                    destination,
                    0,
                    false,
                    now,
                )
                .await?;
                if let Stage::Respawn { next, .. } =
                    &mut states.rotten_blood_callbacks.advanced.massive[index].stage
                {
                    *next += 1;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_step_retains_both_independent_deadlines() {
        assert_eq!(deadlines(83_000_001), Ok([88_000_001, 98_000_001]));
        assert!(deadlines(u64::MAX - 14_999_999).is_err());
    }
}
