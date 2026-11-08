//! Source rules: docs/reference/tibia-manual/combat.md:269-278. The physical
//! arena cadence reads only current run members and committed native deaths.
use super::*;
use crate::spell::cast::DromeStatus;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Modifier {
    Somersault,
    GoingDown,
    ExplodingCorpses,
    Escalated,
    Lava,
    Beam,
    Tanked,
    SownSorrow,
    BadRoots,
}
impl Modifier {
    pub(super) fn draw(run: [u8; 16], wave: u32) -> Self {
        const ALL: [Modifier; 9] = [
            Modifier::Somersault,
            Modifier::GoingDown,
            Modifier::ExplodingCorpses,
            Modifier::Escalated,
            Modifier::Lava,
            Modifier::Beam,
            Modifier::Tanked,
            Modifier::SownSorrow,
            Modifier::BadRoots,
        ];
        ALL[(sample(draw(run, wave, 0, 0)) % ALL.len() as u64) as usize]
    }
}

pub(super) struct Mark {
    cells: Vec<MovementLocalPosition>,
    deadline: Instant,
    next_visual: Instant,
    resolved: Vec<GameSessionId>,
}

fn sample(bytes: [u8; 32]) -> u64 {
    let mut prefix = [0; 8];
    prefix.copy_from_slice(&bytes[..8]);
    u64::from_be_bytes(prefix)
}

fn draw(run: [u8; 16], wave: u32, sequence: u64, purpose: u8) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"oteryn:tibiadrome:modifier:v1");
    hash.update(run);
    hash.update(wave.to_be_bytes());
    hash.update(sequence.to_be_bytes());
    hash.update([purpose]);
    hash.finalize().into()
}

fn members(
    live: &HashMap<GameSessionId, Live>,
    run: [u8; 16],
) -> Vec<(GameSessionId, ExactActorRef)> {
    let mut members: Vec<_> = live
        .iter()
        .filter(|(_, member)| member.run == run && !member.ended)
        .map(|(session, member)| (*session, member.actor))
        .collect();
    members.sort_by_key(|(session, _)| *session.as_bytes());
    members
}

fn adjacent(a: MovementLocalPosition, b: MovementLocalPosition) -> bool {
    // Oteryn working parameter for the manual's unquantified "nearby": one
    // adjacent tile in the existing Chebyshev combat grid, on the same floor.
    a.floor == b.floor && a.x.abs_diff(b.x) <= 1 && a.y.abs_diff(b.y) <= 1
}

fn apply(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    states: &mut super::super::actor_spell::ChannelSpellStates,
    actor: ExactActorRef,
    session: GameSessionId,
    percent: Option<u8>,
    status: Option<DromeStatus>,
) -> bool {
    if !runtime.is_tibiadrome(actor)
        || runtime.player_control_facts(actor, session).is_err()
        || runtime.assert_actor_spell_unreserved(actor).is_err()
        || states.has_pending_spell_commit(actor, session)
    {
        return false;
    }
    if states.is_dead(actor) {
        return true;
    }
    let Some(before) = states.get(runtime, actor, session) else {
        return false;
    };
    let next = if let Some(percent) = percent {
        before.after_drome_percentage_hit(percent)
    } else if let Some(status) = status {
        use oteryn_gameplay_domain::domain::condition::ConditionRefusal;
        match before.try_after_drome_status(status, owner.owner_now().get()) {
            Ok(next) => Some(next),
            Err(ConditionRefusal::Immune | ConditionRefusal::KeptCurrent) => return true,
            Err(_) => return false,
        }
    } else {
        return false;
    };
    next.is_some_and(|next| states.commit(runtime, actor, session, next))
}

pub(super) fn deaths(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    live: &HashMap<GameSessionId, Live>,
    battle: &mut Battle,
    run: [u8; 16],
) -> bool {
    let percent = match battle.modifier {
        Modifier::ExplodingCorpses => 25,
        Modifier::GoingDown => 40,
        _ => return true,
    };
    let Ok(mut states) = owner.states.spell_states.try_lock() else {
        return false;
    };
    let members = members(live, run);
    let mut complete = true;
    for target in &battle.opponents {
        let Ok((_, corpse)) = runtime.borrow_combat_death().projected_death(*target) else {
            continue;
        };
        let center = if battle.modifier == Modifier::GoingDown {
            let Some(center) = runtime.tibiadrome_last_hitter(*target).and_then(|session| {
                members
                    .iter()
                    .find(|(member, _)| *member == session)
                    .and_then(|(_, actor)| runtime.read_actor_position(*actor).ok())
                    .map(|position| position.position())
            }) else {
                continue;
            };
            center
        } else {
            corpse
        };
        for (session, actor) in &members {
            if battle.exploded.contains(&(*target, *session)) {
                continue;
            }
            let hit = runtime
                .read_actor_position(*actor)
                .is_ok_and(|position| adjacent(position.position(), center));
            if !hit
                || apply(
                    owner,
                    runtime,
                    &mut states,
                    *actor,
                    *session,
                    Some(percent),
                    None,
                )
            {
                battle.exploded.push((*target, *session));
            } else {
                complete = false;
            }
        }
    }
    complete
}

fn candidates(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &ChannelRuntimeV1,
    center: MovementLocalPosition,
    run: [u8; 16],
    wave: u32,
    sequence: u64,
) -> Vec<MovementLocalPosition> {
    let mut cells = Vec::new();
    let seed = draw(run, wave, sequence, 1);
    for at in battle_cells(owner, runtime, center) {
        let mut hash = Sha256::new();
        hash.update(seed);
        hash.update(at.x.to_be_bytes());
        hash.update(at.y.to_be_bytes());
        cells.push((<[u8; 32]>::from(hash.finalize()), at));
    }
    cells.sort_by_key(|(key, _)| *key);
    cells.into_iter().map(|(_, cell)| cell).collect()
}

fn visual(
    states: &mut super::super::actor_spell::ChannelSpellStates,
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    cells: &[MovementLocalPosition],
    seed: bool,
) {
    // Crystal 201189f src/utils/utils_definitions.hpp:63,101: yellow rings8,
    // small plants46. Existing native MagicEffect journal supplies the warning.
    for cell in cells {
        states
            .sounds
            .record_item_effect(runtime, actor, *cell, if seed { 46 } else { 8 }, false);
    }
}

pub(super) fn timed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    live: &HashMap<GameSessionId, Live>,
    battle: &mut Battle,
    run: [u8; 16],
    now: Instant,
) {
    let seed = matches!(battle.modifier, Modifier::SownSorrow | Modifier::BadRoots);
    if !seed
        && !matches!(
            battle.modifier,
            Modifier::Lava | Modifier::Beam | Modifier::Tanked
        )
    {
        return;
    }
    let members = members(live, run);
    let Some((_, presenter)) = members.first().copied() else {
        return;
    };
    let Ok(mut states) = owner.states.spell_states.try_lock() else {
        return;
    };
    if let Some(mark) = &mut battle.mark {
        // A participant stepping on a live seed defuses it for the whole group.
        if seed
            && members.iter().any(|(_, actor)| {
                runtime
                    .read_actor_position(*actor)
                    .is_ok_and(|position| mark.cells.contains(&position.position()))
            })
        {
            battle.mark = None;
        } else if now >= mark.deadline {
            for (session, actor) in &members {
                if mark.resolved.contains(session) {
                    continue;
                }
                let affected = seed
                    || runtime
                        .read_actor_position(*actor)
                        .is_ok_and(|position| mark.cells.contains(&position.position()));
                let applied = if !affected {
                    true
                } else {
                    match battle.modifier {
                        Modifier::Lava => apply(
                            owner,
                            runtime,
                            &mut states,
                            *actor,
                            *session,
                            Some(60),
                            None,
                        ),
                        Modifier::Tanked => apply(
                            owner,
                            runtime,
                            &mut states,
                            *actor,
                            *session,
                            None,
                            Some(DromeStatus::Drunk),
                        ),
                        Modifier::SownSorrow => apply(
                            owner,
                            runtime,
                            &mut states,
                            *actor,
                            *session,
                            None,
                            Some(DromeStatus::Fear),
                        ),
                        Modifier::BadRoots => apply(
                            owner,
                            runtime,
                            &mut states,
                            *actor,
                            *session,
                            None,
                            Some(DromeStatus::Root),
                        ),
                        Modifier::Beam => {
                            if runtime.assert_actor_spell_unreserved(*actor).is_err()
                                || states.has_pending_spell_commit(*actor, *session)
                            {
                                continue;
                            }
                            let before = runtime.read_actor_position(*actor).ok();
                            let cells = candidates(
                                owner,
                                runtime,
                                battle.center,
                                run,
                                battle.wave,
                                battle.effect_sequence,
                            );
                            before.is_some_and(|before| {
                                cells
                                    .into_iter()
                                    .find(|cell| runtime.native_summon_cell_free(*cell))
                                    .is_some_and(|destination| {
                                        runtime
                                            .relocate_arena_player(
                                                *actor,
                                                *session,
                                                before,
                                                destination,
                                            )
                                            .is_ok()
                                    })
                            })
                        }
                        _ => true,
                    }
                };
                if applied {
                    mark.resolved.push(*session);
                }
            }
            if members
                .iter()
                .all(|(session, _)| mark.resolved.contains(session))
            {
                battle.mark = None;
            }
        } else if now >= mark.next_visual {
            visual(&mut states, runtime, presenter, &mark.cells, seed);
            mark.next_visual = now + Duration::from_secs(1);
        }
    }
    if battle.mark.is_none() && now >= battle.next_modifier {
        battle.effect_sequence = battle.effect_sequence.saturating_add(1);
        let mut cells = candidates(
            owner,
            runtime,
            battle.center,
            run,
            battle.wave,
            battle.effect_sequence,
        );
        if cells.is_empty() || (!seed && cells.len() < 100) {
            // A partial warning area cannot silently weaken a source modifier.
            battle.ended = true;
            for target in battle.opponents.drain(..) {
                let _ = runtime.remove_arena_opponent(target);
            }
            return;
        }
        cells.truncate(if seed { 1 } else { 100 });
        visual(&mut states, runtime, presenter, &cells, seed);
        battle.mark = Some(Mark {
            cells,
            deadline: now + Duration::from_secs(if seed { 6 } else { 3 }),
            next_visual: now + Duration::from_secs(1),
            resolved: Vec::new(),
        });
        let interval = if seed {
            [20, 17, 14, 11, 8]
                .get(members.len().saturating_sub(1))
                .copied()
                .unwrap_or(20)
        } else {
            15
        };
        battle.next_modifier = now + Duration::from_secs(interval);
    }
}

pub(in crate::gameplay_transport) fn somersault(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    sequence: u64,
    states: &super::super::actor_spell::ChannelSpellStates,
) -> Option<(MovementLocalPosition, MovementLocalPosition)> {
    let live = owner.states.tibiadrome.live.lock().ok()?;
    let current = live.values().find(|member| {
        member.battle.lock().is_ok_and(|battle| {
            !battle.ended
                && !battle.preparing
                && battle.modifier == Modifier::Somersault
                && battle.opponents.contains(&actor)
        })
    })?;
    let mut battle = current.battle.lock().ok()?;
    if battle
        .somersaults
        .iter()
        .any(|(held, last)| *held == actor && *last >= sequence)
    {
        return None;
    }
    if let Some((_, last)) = battle
        .somersaults
        .iter_mut()
        .find(|(held, _)| *held == actor)
    {
        *last = sequence;
    } else {
        battle.somersaults.push((actor, sequence));
    }
    let mut hash = Sha256::new();
    hash.update(draw(current.run, battle.wave, sequence, 2));
    hash.update(actor.placement_identity());
    if sample(hash.finalize().into()) % 100 >= 15 {
        return None;
    }
    let before = runtime.read_actor_position(actor).ok()?;
    let origin = before.position();
    let (_, farthest) = members(&live, current.run)
        .into_iter()
        .filter_map(|(session, player)| {
            if states.get(runtime, player, session)?.vitals().health == 0 {
                return None;
            }
            runtime
                .read_actor_position(player)
                .ok()
                .map(|at| (session, at.position()))
        })
        .filter(|(_, at)| at.floor == origin.floor)
        .max_by_key(|(session, at)| {
            (
                at.x.abs_diff(origin.x).max(at.y.abs_diff(origin.y)),
                *session.as_bytes(),
            )
        })?;
    let area = battle_cells(owner, runtime, battle.center);
    let destination = (-1..=1)
        .flat_map(|dy| (-1..=1).map(move |dx| (dx, dy)))
        .filter(|(dx, dy)| *dx != 0 || *dy != 0)
        .filter_map(|(dx, dy)| {
            Some(MovementLocalPosition {
                x: farthest.x.checked_add(dx)?,
                y: farthest.y.checked_add(dy)?,
                floor: farthest.floor,
            })
        })
        .find(|at| area.contains(at) && runtime.native_summon_cell_free(*at))?;
    runtime
        .relocate_tibiadrome_opponent(actor, before, destination)
        .ok()?;
    Some((origin, destination))
}
