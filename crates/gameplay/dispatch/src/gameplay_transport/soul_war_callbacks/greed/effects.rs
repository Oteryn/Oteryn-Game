//! A committed statement is removed before any later statement can await.
use super::*;

pub(super) async fn drain(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    native: &NativeGameplayState,
    map: &mut CurrentMonsterMap<'_, '_>,
    execution_now: u64,
) -> Result<(), ()> {
    next_source_due_time(runtime, states)?;
    if !current_context(runtime, states, room, native) || !map.validate_transaction(runtime).await {
        return Err(());
    }
    if let Some(read) = states
        .soul_war_callbacks
        .greed_work
        .pending
        .as_ref()
        .ok_or(())?
        .sphere
    {
        // An absent source tile is a known Lua Tile(nil); a failed current read
        // is not. Keep the exact original sphere callback until that read works.
        let exists = map.source_tile_exists(runtime, read.west).ok_or(())?;
        if exists {
            map.read(runtime, read.west).await.ok_or(())?;
        }
        if !map.validate_transaction(runtime).await {
            return Err(());
        }
        let owner = &mut states.soul_war_callbacks;
        let pending = owner.greed_work.pending.as_mut().ok_or(())?;
        pending.effects.extend(owner.greed.sphere_think(
            read.instance,
            crate::ai_think::CREATURE_THINK_INTERVAL_MILLIS,
            read.position,
            exists && sphere_cell_has_boss(runtime, pending.boss, read.west),
        ));
        pending.sphere = None;
    }
    loop {
        let pending = states
            .soul_war_callbacks
            .greed_work
            .pending
            .as_ref()
            .ok_or(())?;
        let Some(effect) = pending.effects.front().copied() else {
            states.soul_war_callbacks.greed_work.pending = None;
            return Ok(());
        };
        let boss = pending.boss;
        match effect {
            GreedEffect::FinishBossThink { interval_ms } => {
                let owner = &mut states.soul_war_callbacks;
                let followup = owner.greed.finish_boss_think(interval_ms);
                let pending = owner.greed_work.pending.as_mut().ok_or(())?;
                pending.effects.pop_front();
                for effect in followup.into_iter().rev() {
                    pending.effects.push_front(effect);
                }
                continue;
            }
            GreedEffect::TeleportMagic { position } => {
                crate::gameplay_transport::soul_war_scene::emit_effect(room, at(position), 11);
            }
            GreedEffect::SpawnMonster {
                creature_key,
                position,
                forced,
            } => {
                // Observe cannot fail after a successful physical spawn merely
                // because the association counter was already exhausted.
                let owner = &states.soul_war_callbacks;
                owner.next_id.checked_add(1).ok_or(())?;
                let created = spawn::apply(
                    runtime,
                    room,
                    native,
                    map,
                    owner.spawn_source.as_ref().ok_or(())?,
                    boss,
                    creature_key,
                    at(position),
                    forced,
                    pending.sampled_at,
                    owner.next_id,
                )
                .await?;
                let owner = &mut states.soul_war_callbacks;
                if let Some(actor) = created {
                    observe(
                        owner,
                        actor,
                        GreedMinion::from_creature_key(creature_key),
                        creature_key == greed_model::SOUL_SPHERE,
                    )?;
                } else if creature_key == greed_model::SOUL_SPHERE {
                    owner.greed.sphere_spawn_failed();
                }
            }
            GreedEffect::RemoveMonster { instance } => {
                if let Some(entry) = states
                    .soul_war_callbacks
                    .observed
                    .iter()
                    .find(|entry| entry.id == instance)
                    && runtime.contains_live_creature(entry.actor)
                {
                    runtime
                        .remove_encounter_actor(entry.actor)
                        .map_err(|_| ())?;
                }
            }
            GreedEffect::PlayerHealthLoss { actor, amount } => {
                let target = pending
                    .players
                    .iter()
                    .find(|(target, _)| u64::from(target.0.actor_local_id.0) == actor)
                    .copied();
                let source = match pending.origin {
                    Origin::Think { occurrence, .. } => Some(occurrence.actor),
                    Origin::Timer => None,
                };
                if let (Some((target, session)), Some(source)) = (target, source) {
                    // Lua ignores addHealth(false), including a player killed by
                    // an earlier statement of this same retained callback.
                    let _ = states.apply_soul_war_lua_health_loss(
                        runtime,
                        native,
                        source,
                        target,
                        session,
                        amount,
                        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(
                            execution_now,
                        ),
                    );
                }
            }
            GreedEffect::BossBuff {
                reflect_percent_per_element,
                defense,
                max_health,
                health,
            } => change(
                runtime,
                boss,
                Change::Buff {
                    defense: u32::try_from(defense).map_err(|_| ())?,
                    reflection: u32::from(reflect_percent_per_element),
                    maximum_health: u32::try_from(max_health).map_err(|_| ())?,
                    health: u32::try_from(health).map_err(|_| ())?,
                },
            )?,
            GreedEffect::BossImmune { immune } => change(runtime, boss, Change::Immune(immune))?,
            GreedEffect::BossSpeed { speed } => change(runtime, boss, Change::Speed(speed))?,
            GreedEffect::BossHealFull => change(runtime, boss, Change::HealFull)?,
            GreedEffect::BossTeleport { position } => {
                if runtime.contains_live_creature(boss) {
                    let destination = at(position);
                    if map.source_tile_exists(runtime, destination).ok_or(())? {
                        map.read(runtime, destination).await.ok_or(())?;
                        if !map.validate_transaction(runtime).await {
                            return Err(());
                        }
                        // The existing relocation owner applies Lua's default
                        // x-priority facing, including north for a same-cell move.
                        let _ = map
                            .view()
                            .relocate_source_creature(runtime, boss, destination);
                    }
                }
            }
            GreedEffect::SphereMove { instance, position } => {
                if let Some(actor) = states
                    .soul_war_callbacks
                    .observed
                    .iter()
                    .find(|entry| entry.id == instance)
                    .map(|entry| entry.actor)
                    && runtime.contains_live_creature(actor)
                {
                    let destination = at(position);
                    if map.source_tile_exists(runtime, destination).ok_or(())? {
                        map.read(runtime, destination).await.ok_or(())?;
                        if !map.validate_transaction(runtime).await {
                            return Err(());
                        }
                        // Source pushMovement=true uses the adjacent westward
                        // move; Map::moveCreature applies its normal facing.
                        let before = runtime.read_actor_position(actor).map_err(|_| ())?;
                        if before.position() != destination && map.view().matches(runtime) {
                            let _ = runtime
                                .borrow_movement_position()
                                .commit_creature_relocation(actor, before, destination);
                        }
                    }
                }
            }
            GreedEffect::RemoveSoulSphere { instance } => {
                if let Some(entry) = instance.and_then(|id| {
                    states
                        .soul_war_callbacks
                        .observed
                        .iter()
                        .find(|entry| entry.id == id)
                }) && entry.sphere
                    && runtime.contains_live_creature(entry.actor)
                {
                    runtime
                        .remove_encounter_actor(entry.actor)
                        .map_err(|_| ())?;
                }
            }
        }
        // There is no await between a physical success/known Lua refusal and
        // this cursor advancement. Unknown current facts returned above instead.
        states
            .soul_war_callbacks
            .greed_work
            .pending
            .as_mut()
            .ok_or(())?
            .effects
            .pop_front();
        #[cfg(test)]
        if let Some(notify) = retention_test::after_effect(states, effect)? {
            notify.notify_one();
            std::future::pending::<()>().await;
        }
    }
}
