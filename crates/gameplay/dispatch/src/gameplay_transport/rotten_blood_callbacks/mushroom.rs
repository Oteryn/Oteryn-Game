//! Real reserved Mushroom thinks, retained fuse, normal HP and native visual owner.
use super::*;
use crate::creature_damage_spell::native_event::{
    MushroomThinkSource, NativeCreatureEventDamagePlan, prepare_native_creature_damage,
};
use crate::map::boot::BundleCollisionIndexExt;
use oteryn_gameplay_encounters::bosses::rotten_blood::{RoomTarget, mushroom_think as rule};

#[derive(Debug)]
struct Tracked {
    actor: ExactActorRef,
    source: MushroomThinkSource,
    owner: rule::MushroomThinkOwner,
    last: Option<u64>,
    pending: Option<PreparedExplosion>,
}
impl Tracked {
    fn fresh_draft(&self) -> Result<Self, ()> {
        if self.pending.is_some() {
            return Err(());
        }
        Ok(Self {
            actor: self.actor,
            source: self.source.clone(),
            owner: self.owner.clone(),
            last: self.last,
            pending: None,
        })
    }
}
#[derive(Debug)]
enum Hit {
    SelfVisual(Option<crate::map::facts::CreatureVisualCue>),
    Player(
        Box<NativeCreatureEventDamagePlan>,
        Option<crate::map::facts::CreatureVisualCue>,
    ),
    Creature {
        target: ExactActorRef,
        amount: u32,
        target_atom: String,
        occurrence: String,
        binding: Vec<u8>,
        cue: Option<crate::map::facts::CreatureVisualCue>,
    },
}
#[derive(Debug)]
struct PreparedExplosion {
    cues: Vec<crate::map::facts::CreatureVisualCue>,
    hits: Vec<Hit>,
    cursor: usize,
}
#[derive(Debug, Default)]
pub(super) struct Mushrooms {
    tracked: Vec<Tracked>,
    #[cfg(test)]
    pause_after_player_commit: bool,
}
impl Mushrooms {
    #[cfg(test)]
    pub(super) fn pause_after_player_commit_for_test(&mut self) {
        self.pause_after_player_commit = true;
    }
    #[cfg(test)]
    pub(super) fn progress_for_test(&self) -> Option<super::MushroomProgressForTest> {
        self.tracked.iter().find_map(|tracked| {
            let pending = tracked.pending.as_ref()?;
            Some(super::MushroomProgressForTest {
                actor: tracked.actor,
                original_due: tracked.owner.explode_at(1)?.checked_mul(1000)?,
                cursor: pending.cursor,
                total: pending.hits.len(),
                players: pending
                    .hits
                    .iter()
                    .filter_map(|hit| match hit {
                        Hit::Player(plan, _) => Some(plan.target()),
                        _ => None,
                    })
                    .collect(),
            })
        })
    }
    pub(super) fn next_due_time(&self) -> Option<u64> {
        self.tracked
            .iter()
            .filter_map(|m| m.owner.explode_at(1))
            .map(|ms| ms.saturating_mul(1000))
            .min()
    }
}
fn position(runtime: &ChannelRuntimeV1, actor: ExactActorRef) -> Option<MovementLocalPosition> {
    Some(runtime.read_actor_position(actor).ok()?.position())
}
fn cue(
    runtime: &ChannelRuntimeV1,
    stamp: crate::foundation::RuntimeWorkStamp,
    actor: ExactActorRef,
    at: MovementLocalPosition,
    effect: u16,
) -> Option<crate::map::facts::CreatureVisualCue> {
    crate::map::facts::CreatureVisualCue::effect_from_registered_source(
        runtime,
        stamp,
        actor,
        crate::gameplay_transport::soul_war_scene::wire_position(at)?,
        effect,
    )
}
fn emit(
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    cues: Vec<crate::map::facts::CreatureVisualCue>,
) -> Result<(), ()> {
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(bundle) =
        room.movement_cells().index()
    else {
        return Err(());
    };
    for cue in cues {
        bundle.map().facts.emit_creature_visual(runtime, cue);
    }
    Ok(())
}
fn remove(
    runtime: &mut ChannelRuntimeV1,
    native: &NativeGameplayState,
    actor: ExactActorRef,
) -> Result<(), ()> {
    if runtime.contains_live_creature(actor) {
        let stamp = runtime.issue_owner_work().map_err(|_| ())?;
        runtime
            .remove_current_native_encounter_subject(
                stamp,
                actor,
                rule::MUSHROOM_KEY,
                native.source_digest(),
            )
            .map_err(|_| ())?;
    }
    Ok(())
}
#[allow(
    clippy::too_many_arguments,
    reason = "actual Think occurrences share the existing physical, spell-state and map owners"
)]
pub(super) async fn drive_one_due(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut super::super::attack::ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    through: u64,
    now: u64,
) -> Result<(), ()> {
    states
        .rotten_blood_callbacks
        .mushrooms
        .tracked
        .retain(|m| m.source.current(runtime));
    let selected = states
        .rotten_blood_callbacks
        .mushrooms
        .tracked
        .iter()
        .filter_map(|m| {
            m.owner
                .explode_at(1)
                .and_then(|at| at.checked_mul(1000))
                .filter(|at| *at <= through && *at <= now)
                .map(|at| (at, m.actor))
        })
        .min_by_key(|(at, _)| *at)
        .map(|(_, actor)| actor);
    if let Some(actor) = selected {
        drive_fuse(runtime, states, attack, room, native, map, actor, now).await?;
    }
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "actual Think occurrences share the existing physical, spell-state and map owners"
)]
pub(super) async fn drive(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut super::super::attack::ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    occurrences: &[ThinkOccurrence],
    now: u64,
) -> Result<Option<u64>, ()> {
    states
        .rotten_blood_callbacks
        .mushrooms
        .tracked
        .retain(|m| m.source.current(runtime));
    let due = states
        .rotten_blood_callbacks
        .mushrooms
        .tracked
        .iter()
        .filter(|m| m.owner.explode_at(1).is_some_and(|at| at <= now / 1000))
        .map(|m| m.actor)
        .collect::<Vec<_>>();
    for actor in due {
        drive_fuse(runtime, states, attack, room, native, map, actor, now).await?;
    }
    for occurrence in occurrences {
        if super::super::soul_war_callbacks::source_actor(runtime, native, occurrence.actor)
            .as_deref()
            != Some(rule::MUSHROOM_KEY)
            || !runtime.contains_live_creature(occurrence.actor)
        {
            continue;
        }
        let current = states
            .rotten_blood_callbacks
            .mushrooms
            .tracked
            .iter()
            .position(|m| m.actor == occurrence.actor);
        let mut tracked = match current {
            Some(index) => states.rotten_blood_callbacks.mushrooms.tracked[index].fresh_draft()?,
            None => {
                if states.rotten_blood_callbacks.mushrooms.tracked.len() >= 512 {
                    return Err(());
                }
                Tracked {
                    actor: occurrence.actor,
                    source: MushroomThinkSource::from_current_reserved_think(
                        runtime,
                        native,
                        *occurrence,
                    )
                    .map_err(|_| ())?,
                    owner: Default::default(),
                    last: None,
                    pending: None,
                }
            }
        };
        if tracked.last.is_some_and(|last| occurrence.sequence <= last) {
            continue;
        }
        let own = position(runtime, tracked.actor).ok_or(())?;
        let census = runtime.positioned_actor_census().map_err(|_| ())?;
        let mut players = Vec::new();
        let mut monsters = Vec::new();
        for (index, (actor, _, session)) in census.iter().enumerate() {
            let Some(at) = position(runtime, *actor) else {
                continue;
            };
            if let Some(session) = session {
                if states
                    .get(runtime, *actor, *session)
                    .ok_or(())?
                    .vitals()
                    .health
                    > 0
                {
                    players.push(RoomTarget {
                        actor: index as u64 + 2,
                        position: pos(at),
                    });
                }
            } else if runtime.contains_live_creature(*actor)
                && let Some(key) =
                    super::super::soul_war_callbacks::source_actor(runtime, native, *actor)
            {
                monsters.push((key, pos(at)));
            }
        }
        let refs = monsters
            .iter()
            .map(|(key, at)| (key.as_str(), *at))
            .collect::<Vec<_>>();
        let before_due = tracked.owner.explode_at(1);
        let effects = tracked
            .owner
            .think(1, pos(own), now / 1000, &players, &refs);
        if tracked.owner.explode_at(1) != before_due && tracked.owner.explode_at(1).is_some() {
            tracked.source =
                MushroomThinkSource::from_current_reserved_think(runtime, native, *occurrence)
                    .map_err(|_| ())?;
        }
        let stamp = runtime.issue_owner_work().map_err(|_| ())?;
        if !map.validate_transaction(runtime).await {
            return Err(());
        }
        for effect in effects {
            match effect {
                rule::MushroomEffect::Primed { .. } => {
                    let a = cue(runtime, stamp, tracked.actor, own, 15).ok_or(())?;
                    let b = cue(runtime, stamp, tracked.actor, own, 15).ok_or(())?;
                    emit(runtime, room, vec![a, b])?;
                }
                rule::MushroomEffect::Despawn { .. } => remove(runtime, native, tracked.actor)?,
                _ => return Err(()),
            }
        }
        tracked.last = Some(occurrence.sequence);
        if runtime.contains_live_creature(tracked.actor) {
            match current {
                Some(index) => states.rotten_blood_callbacks.mushrooms.tracked[index] = tracked,
                None => states
                    .rotten_blood_callbacks
                    .mushrooms
                    .tracked
                    .push(tracked),
            }
        } else {
            states
                .rotten_blood_callbacks
                .mushrooms
                .tracked
                .retain(|current| current.actor != tracked.actor);
        }
    }
    Ok(states.rotten_blood_callbacks.mushrooms.next_due_time())
}
#[allow(
    clippy::too_many_arguments,
    reason = "a source fuse uses the same physical runtime, native facts, HP owner and map transaction"
)]
async fn drive_fuse(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut super::super::attack::ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    actor: ExactActorRef,
    now: u64,
) -> Result<(), ()> {
    let index = states
        .rotten_blood_callbacks
        .mushrooms
        .tracked
        .iter()
        .position(|m| m.actor == actor)
        .ok_or(())?;
    if states.rotten_blood_callbacks.mushrooms.tracked[index]
        .pending
        .is_none()
    {
        let draft = states.rotten_blood_callbacks.mushrooms.tracked[index].fresh_draft()?;
        let prepared =
            prepare_explosion(runtime, states, attack, room, native, map, draft, now).await?;
        states.rotten_blood_callbacks.mushrooms.tracked[index].pending = Some(prepared);
    }
    // Taking the continuation is confined to this synchronous section. It is
    // restored on every refusal, including a partially committed spectator.
    let source = states.rotten_blood_callbacks.mushrooms.tracked[index]
        .source
        .clone();
    let mut pending = states.rotten_blood_callbacks.mushrooms.tracked[index]
        .pending
        .take()
        .ok_or(())?;
    let result = finish_explosion(
        runtime,
        states,
        attack,
        room,
        native,
        &source,
        &mut pending,
        now,
    );
    if result.is_ok() {
        states
            .rotten_blood_callbacks
            .mushrooms
            .tracked
            .remove(index);
    } else {
        states.rotten_blood_callbacks.mushrooms.tracked[index].pending = Some(pending);
    }
    result
}

#[allow(
    clippy::too_many_arguments,
    reason = "one actual fuse preflights its full spectator cohort before any published effect"
)]
async fn prepare_explosion(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut super::super::attack::ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    tracked: Tracked,
    now: u64,
) -> Result<PreparedExplosion, ()> {
    if !tracked.source.current(runtime) {
        return Err(());
    }
    let centre = position(runtime, tracked.actor).ok_or(())?;
    let stamp = runtime.issue_owner_work().map_err(|_| ())?;
    let semantic = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now);
    let mut reader =
        crate::gameplay_transport::monster_profile_cycle::physical_spell::prepare_mushroom_reader(
            runtime,
            states,
            attack,
            room,
            native,
            map,
            &tracked.source,
            now,
            stamp,
        )
        .await
        .ok_or(())?;
    let mut hits: Vec<(u32, Hit)> = Vec::new();
    let mut cues = Vec::new();
    let mut recipients = Vec::new();
    let mut source_draws = Vec::new();
    let census = runtime.positioned_actor_census().map_err(|_| ())?;
    let mut ids = Vec::new();
    for (actor, _, session) in &census {
        let Some(at) = position(runtime, *actor) else {
            continue;
        };
        if at.floor != centre.floor || at.x.abs_diff(centre.x) > 3 || at.y.abs_diff(centre.y) > 3 {
            continue;
        }
        if *actor != tracked.actor {
            let id = ids.len() as u64 + 2;
            ids.push((*actor, *session, at, actor.actor_local_id()));
            recipients.push(RoomTarget {
                actor: id,
                position: pos(at),
            });
            source_draws.push(tracked.source.normal_damage_draw(*actor).map_err(|_| ())?);
        }
    }
    let mut draft = tracked.owner.clone();
    let mut draws = source_draws.into_iter();
    let effects = draft
        .explode_due(1, pos(centre), now / 1000, &recipients, || {
            draws.next().unwrap_or(0)
        })
        .map_err(|_| ())?;
    if effects.is_empty() {
        return Err(());
    }
    hits.push((
        tracked.actor.actor_local_id(),
        Hit::SelfVisual(Some(
            cue(runtime, stamp, tracked.actor, centre, 9).ok_or(())?,
        )),
    ));
    let issuer_pz = room
        .movement_cells()
        .spell_tiles()
        .lookup(
            room.movement_cells().scope(),
            crate::content::LogicalCell {
                x: centre.x,
                y: centre.y,
                z: i32::from(centre.floor),
            },
        )
        .map_err(|_| ())?
        .flags()
        .protection_zone;
    for effect in effects {
        match effect {
            rule::MushroomEffect::BlastVisual { .. } => {
                for dy in -2i32..=2 {
                    let width = if dy.abs() == 2 { 1 } else { 2 };
                    for dx in -width..=width {
                        let at = MovementLocalPosition {
                            x: centre.x.checked_add(dx).ok_or(())?,
                            y: centre.y.checked_add(dy).ok_or(())?,
                            floor: centre.floor,
                        };
                        map.read(runtime, at).await.ok_or(())?;
                        cues.push(cue(runtime, stamp, tracked.actor, at, 9).ok_or(())?);
                    }
                }
            }
            rule::MushroomEffect::LifeDrain { target, amount, .. } => {
                let (actor, session, at, order) = *ids
                    .get(usize::try_from(target.checked_sub(2).ok_or(())?).map_err(|_| ())?)
                    .ok_or(())?;
                if let Some(session) = session {
                    if !reader.event_players().contains(&(actor, session)) {
                        continue;
                    }
                    match tracked.source.prepare_player(
                        runtime,
                        &mut reader,
                        actor,
                        session,
                        stamp,
                        semantic,
                    ) {
                        Ok(plan) => {
                            hits.push((
                                order,
                                Hit::Player(
                                    Box::new(plan),
                                    Some(cue(runtime, stamp, tracked.actor, at, 9).ok_or(())?),
                                ),
                            ));
                        }
                        Err(crate::creature_auto_attack::AttackError::Protected) => {}
                        Err(_) => return Err(()),
                    }
                } else if runtime.contains_live_creature(actor) {
                    use crate::monster_summon::{
                        CurrentSourceCreatureRole, current_source_creature_role,
                    };
                    let role = current_source_creature_role(runtime, actor).map_err(|_| ())?;
                    if matches!(
                        role,
                        CurrentSourceCreatureRole::Ordinary
                            | CurrentSourceCreatureRole::IntrinsicMonsterSummon(_)
                    ) {
                        continue;
                    }
                    let snapshot = runtime.companion_snapshot(actor).map_err(|_| ())?;
                    let master = snapshot.state.master.ok_or(())?;
                    let control = runtime
                        .player_control_facts(master.actor, master.session)
                        .map_err(|_| ())?;
                    if control.control_loss.is_some()
                        || attack
                            .creature_target_protection(
                                runtime,
                                master.actor,
                                master.session,
                                oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now),
                            )
                            .ok_or(())?
                        || !snapshot.state.policy.flags.attackable
                    {
                        continue;
                    }
                    map.read(runtime, at).await.ok_or(())?;
                    let target_tile = room
                        .movement_cells()
                        .spell_tiles()
                        .lookup(
                            room.movement_cells().scope(),
                            crate::content::LogicalCell {
                                x: at.x,
                                y: at.y,
                                z: i32::from(at.floor),
                            },
                        )
                        .map_err(|_| ())?;
                    if issuer_pz || target_tile.flags().protection_zone {
                        continue;
                    }
                    let amount = prepare_native_creature_damage(
                        &snapshot.state.policy,
                        "life_drain",
                        amount,
                    )
                    .ok_or(())?;
                    let occurrence = format!(
                        "native:mushroom-fuse:{}:{}",
                        crate::creature_damage_spell::hex(&tracked.actor.placement_identity()),
                        crate::creature_damage_spell::hex(&actor.placement_identity())
                    );
                    let binding = Sha256::new()
                        .chain_update(b"oteryn:mushroom-creature-hit:v1")
                        .chain_update(occurrence.as_bytes())
                        .chain_update(amount.to_be_bytes())
                        .finalize()
                        .to_vec();
                    hits.push((
                        order,
                        Hit::Creature {
                            target: actor,
                            amount,
                            target_atom: runtime
                                .creature_spell_target_atom(actor)
                                .map_err(|_| ())?,
                            occurrence,
                            binding,
                            cue: Some(cue(runtime, stamp, tracked.actor, at, 9).ok_or(())?),
                        },
                    ));
                }
            }
            rule::MushroomEffect::Despawn { .. } => {}
            rule::MushroomEffect::Primed { .. } => return Err(()),
        }
    }
    if !map.validate_transaction(runtime).await {
        return Err(());
    }
    hits.sort_by_key(|(order, _)| *order);
    for (_, hit) in &mut hits {
        if let Hit::Player(plan, _) = hit {
            plan.retain(runtime, states).map_err(|_| ())?;
        }
    }
    Ok(PreparedExplosion {
        cues,
        hits: hits.into_iter().map(|(_, hit)| hit).collect(),
        cursor: 0,
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "a retained source callback uses the same exact HP, physical, native and visual owners"
)]
fn finish_explosion(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    attack: &mut super::super::attack::ChannelAttackStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    source: &MushroomThinkSource,
    pending: &mut PreparedExplosion,
    now: u64,
) -> Result<(), ()> {
    if !source.current(runtime)
        || !matches!(
            room.movement_cells().index(),
            crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)
        )
    {
        return Err(());
    }
    let semantic = crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now);
    emit(runtime, room, std::mem::take(&mut pending.cues))?;
    while let Some(hit) = pending.hits.get_mut(pending.cursor) {
        match hit {
            Hit::SelfVisual(cue) => {
                if let Some(cue) = cue.take() {
                    emit(runtime, room, vec![cue])?;
                }
            }
            Hit::Player(plan, cue) => {
                let (target, session) = plan.target();
                if runtime.player_control_facts(target, session).is_err() {
                    pending.cursor += 1;
                    continue;
                }
                if let Some(cue) = cue.take() {
                    emit(runtime, room, vec![cue])?;
                }
                let receipt = plan
                    .commit_retained(runtime, states, semantic)
                    .map_err(|_| ())?;
                if let Some(hit) = receipt {
                    crate::gameplay_transport::monster_profile_cycle::physical_spell::record_native_player_hit(runtime,attack,Some(source.actor()),target,session,&hit,now);
                }
                pending.cursor += 1;
                #[cfg(test)]
                if receipt.is_some()
                    && states
                        .rotten_blood_callbacks
                        .mushrooms
                        .pause_after_player_commit
                {
                    states
                        .rotten_blood_callbacks
                        .mushrooms
                        .pause_after_player_commit = false;
                    return Err(());
                }
                continue;
            }
            Hit::Creature {
                target,
                amount,
                target_atom,
                occurrence,
                binding,
                cue,
            } => {
                if !runtime.contains_live_creature(*target) {
                    pending.cursor += 1;
                    continue;
                }
                if let Some(cue) = cue.take() {
                    emit(runtime, room, vec![cue])?;
                }
                if *amount > 0 {
                    runtime
                        .borrow_exact_actor_commit_at_us(now)
                        .commit_damage(
                            *target,
                            crate::foundation::OwnerDamageCommand {
                                target: target_atom.as_bytes(),
                                occurrence: occurrence.as_bytes(),
                                binding,
                                damage: i64::from(*amount),
                            },
                        )
                        .map_err(|_| ())?;
                }
            }
        }
        pending.cursor += 1;
    }
    // HP0 remains registered for its existing canonical SourceDeath owner.
    // Only a live source is administratively retired by this literal callback.
    remove(runtime, native, source.actor())
}
