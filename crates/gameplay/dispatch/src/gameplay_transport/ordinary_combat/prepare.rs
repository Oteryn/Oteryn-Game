//! Source, spatial, legality and payment phases of ordinary cast preparation.
use super::*;

struct SpatialPhase {
    position: MovementPositionSnapshot,
    caster_position: TilePosition,
    origin: TilePosition,
    facing: Direction,
    bindings: Vec<LiveActorBinding>,
    chain_creatures: Vec<ChainCreature>,
    sight_origin: TilePosition,
    tiles: BTreeMap<TilePosition, TileFlags>,
    sight: BTreeSet<(TilePosition, TilePosition)>,
    attack_target_actor: Option<ExactActorRef>,
    attack_target_tile: Option<TilePosition>,
    caster_protection: bool,
}

struct TargetPhase {
    world: World<Direction>,
    operational: OperationalCastFacts,
    pvp_targets: Vec<(ExactActorRef, bool)>,
    chain_start: ChainStart,
    bindings: Vec<LiveActorBinding>,
}

struct PaidPreparation {
    paid: PlayerSpellState,
    output: Output,
    training: PreparedPlayerTraining,
    timers: Option<ValidatedTimerInstall>,
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare_inner(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    book: &SpellBook,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    command: CommandRef,
    occurrence: AbilityOccurrence,
    training_occurrence: BuildOccurrence,
    now: SemanticTimeMicros,
    rune: Option<&oteryn_protocol_oteryn::actor_spell_item_v2::ItemSpellCastIntent>,
    legacy: Option<&crate::gameplay_transport::rune_use_bridge::LegacyRuneUse>,
    named: Option<(
        ExactActorRef,
        &oteryn_protocol_oteryn::actor_spell_v2::ParameterSpellCastIntent,
    )>,
    attack_target: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    pvp_modes: oteryn_protocol_oteryn::pvp_expert::ExpertFightModes,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    if !applicable(spell)
        || book.indexed(intent.spell) != Some(spell)
        || content.spell_book().indexed(intent.spell) != Some(spell)
        || (matches!(spell.carrier, crate::spell::Carrier::Rune { .. }) && rune.is_none())
    {
        return reject();
    }
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=bindings");
    let b = owned.binding();
    let actual = runtime.binding();
    if b.session != command.game_session_id()
        || authority.command() != command
        || authority.character_id_bytes() != b.character
        || authority.compatible_content_digest() != content.source_digest()
        || b.content_digest != runtime.content_pin().server_artifact_digest()
        || authority.runtime_scope()
            != RuntimeScopeRefV1::channel(actual.world_id(), actual.channel_id())
        || authority.scope_generation() != actual.scope_generation().get()
    {
        return reject();
    }
    let actor = b.actor;
    let session = b.session;
    let before = states
        .get(runtime, actor, session)
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    // CONDITIONS-0 §3.5: the gate families answer before the reentry and target checks.
    crate::spell::cast::admit_condition_gates(&before, spell, now)?;
    let source_protected = runtime
        .current_player_reentry_protection(actor, session, now.get())
        .map_err(|_| SpellCastDisposition::Rejected)?;
    if spell.aggressive && source_protected {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    let caster = owned
        .caster(
            &before,
            spell,
            content,
            before.owned_harmony_multiplier(spell)?,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=caster_qualified");
    let profile = spell
        .authored
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    let party = if matches!(spell.execution, Execution::PartyBuff(_))
        || may_heal_player(spell)
        || matches!(
            before.source_party_vocation(),
            crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk
        ) {
        Some(
            crate::gameplay_transport::party_spell_owner::read_source_party_world_in_transaction(
                tx, root, authority, runtime, states, actor, session,
            )
            .await
            .map_err(|_| SpellCastDisposition::Rejected)?,
        )
    } else {
        None
    };
    let roster = runtime
        .positioned_actor_census()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let creatures = roster
        .iter()
        .filter(|(_, _, s)| s.is_none())
        .map(|(a, _, _)| {
            runtime
                .companion_snapshot(*a)
                .map_err(|_| SpellCastDisposition::Rejected)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let players = states
        .actors
        .iter()
        .filter(|(a, s, _)| states.get(runtime, *a, *s).is_some())
        .cloned()
        .collect::<Vec<_>>();
    let spatial = prepare_spatial(
        tx,
        authority,
        runtime,
        room,
        objects,
        spell,
        intent,
        profile,
        actor,
        attack_target,
        &roster,
    )
    .await?;
    let caster_position = spatial.caster_position;
    let TargetPhase {
        world,
        operational,
        pvp_targets,
        chain_start,
        bindings,
    } = qualify_targets(
        tx, root, authority, runtime, states, room, objects, spell, intent, profile, actor, named,
        pvp_modes, &players, &creatures, spatial,
    )
    .await?;
    let PaidPreparation {
        paid,
        output,
        training,
        timers,
    } = prepare_payment(
        runtime,
        states,
        room,
        content,
        owned,
        book,
        spell,
        intent,
        command,
        &occurrence,
        training_occurrence,
        now,
        rune,
        legacy,
        named,
        &before,
        &caster,
        &party,
        &bindings,
        &world,
        &operational,
        &pvp_targets,
        chain_start,
        caster_position,
        draw,
    )?;
    Ok(PreparedNativeCombatCast {
        corpse_companion: None,
        direct_companion: None,
        field_policy_revision: None,
        carried_target: None,
        item_operations: Vec::new(),
        party,
        before,
        paid,
        batch: output.batch,
        tile_facts: world.tiles,
        roster,
        training,
        timers,
        magnitude: output.magnitude,
        facts_binding: b.clone(),
        creatures,
        players,
        equipment: owned.equipment().clone(),
        presentation: Some(output.presentation),
        installation: None,
        item_creations: output.items,
    })
}

#[allow(clippy::too_many_arguments)]
async fn prepare_spatial(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    profile: &crate::spell::executable_catalog::OperationalProfile,
    actor: ExactActorRef,
    attack_target: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    roster: &[(
        ExactActorRef,
        MovementPositionSnapshot,
        Option<GameSessionId>,
    )],
) -> Result<SpatialPhase, SpellCastDisposition> {
    let position = runtime
        .read_actor_position(actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let caster_position = tile(position.position());
    // SPELL-TARGET-1: the held ATTACK-0 §4 target. No target is `TargetRequired`; one that is no
    // longer a visible creature is `TargetIllegal`.
    let attack_target_actor = match intent.target {
        SpellTarget::AttackTarget => {
            let held = attack_target.ok_or(SpellCastDisposition::TargetRequired)?;
            let creature = runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|c| {
                    c.actor.placement_identity() == held.identity && c.generation == held.generation
                })
                .map(|c| c.actor)
                .ok_or(SpellCastDisposition::TargetIllegal)?;
            let seen = runtime
                .read_actor_position(creature)
                .map_err(|_| SpellCastDisposition::TargetIllegal)?;
            if !crate::gameplay_transport::attack::sees(position.position(), seen.position()) {
                return Err(SpellCastDisposition::TargetIllegal);
            }
            Some((creature, tile(seen.position())))
        }
        _ => None,
    };
    let attack_target_tile = attack_target_actor.map(|(_, t)| t);
    let attack_target_actor = attack_target_actor.map(|(a, _)| a);
    // A directional or aimed spell keeps the caster as geometry origin; the target only picks
    // the facing (`aim_at_target`). A single-target spell is centred on the target.
    let directional = profile.header.targeting.needs_direction || spell.target_or_direction;
    let origin = match intent.target {
        SpellTarget::Position(p) => TilePosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        },
        SpellTarget::None
            if profile.header.targeting.needs_direction || spell.target_or_direction =>
        {
            let d = position
                .facing()
                .map(direction)
                .ok_or(SpellCastDisposition::TargetRequired)?;
            let (dx, dy) = delta(d);
            offset(caster_position, dx, dy)?
        }
        SpellTarget::None => caster_position,
        SpellTarget::AttackTarget => {
            let target = attack_target_tile.ok_or(SpellCastDisposition::TargetRequired)?;
            attack_target_origin(caster_position, target, directional)?.0
        }
    };
    if origin.floor != caster_position.floor {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    let facing = if let Some(target) = attack_target_tile {
        attack_target_origin(caster_position, target, directional)?.1
    } else if intent.aim_at_target && origin != caster_position {
        Some(facing_to(caster_position, origin))
    } else {
        position.facing().map(direction)
    };
    let facing = facing.unwrap_or(Direction::North);
    let mut candidates = BTreeSet::from([caster_position, origin]);
    for a in &profile.dependencies.abilities {
        candidates.extend(geometry(a, origin, facing)?);
    }
    let mut bindings = Vec::new();
    let mut chain_creatures = Vec::new();
    for (a, p, _) in roster {
        let p = tile(p.position());
        if p.floor != caster_position.floor
            || p.x.abs_diff(caster_position.x) > MAX_SIGHT_DISTANCE
            || p.y.abs_diff(caster_position.y) > MAX_SIGHT_DISTANCE
        {
            continue;
        }
        let atom = if runtime.contains_live_creature(*a) {
            runtime
                .creature_spell_target_atom(*a)
                .map_err(|_| SpellCastDisposition::Rejected)?
        } else {
            crate::spell::combat_execution::actor_atom(*a)
        };
        let id = u64::from(a.actor_local_id());
        bindings.push(LiveActorBinding {
            source_id: id,
            actor: *a,
            target_atom: atom.clone(),
        });
        chain_creatures.push(ChainCreature {
            id,
            actor: atom,
            position: p,
        });
        candidates.insert(p);
        if spell.chain.is_some() {
            for dx in -9..=9 {
                for dy in -9..=9 {
                    candidates.insert(offset(p, dx, dy)?);
                }
            }
        }
    }
    if bindings.len() > crate::spell::combat_batch::MAX_EFFECTS {
        return reject();
    }
    let mut paths = BTreeMap::new();
    let sight_origin = if profile.header.targeting.needs_direction {
        caster_position
    } else {
        origin
    };
    for p in candidates.clone() {
        paths.insert((sight_origin, p), sight_steps(sight_origin, p)?);
    }
    paths.insert(
        (caster_position, origin),
        sight_steps(caster_position, origin)?,
    );
    if let Some(target) = attack_target_tile {
        paths.insert(
            (caster_position, target),
            sight_steps(caster_position, target)?,
        );
        candidates.insert(target);
    }
    if spell.chain.is_some() {
        for a in chain_creatures
            .iter()
            .chain(std::iter::once(&ChainCreature {
                id: u64::from(actor.actor_local_id()),
                actor: String::new(),
                position: caster_position,
            }))
        {
            for c in &chain_creatures {
                if a.position.x.abs_diff(c.position.x) <= 9
                    && a.position.y.abs_diff(c.position.y) <= 9
                {
                    paths.insert(
                        (a.position, c.position),
                        sight_steps(a.position, c.position)?,
                    );
                }
            }
        }
    }
    for steps in paths.values() {
        candidates.extend(steps.iter().map(|(p, _)| *p));
    }
    if candidates.len() > MAX_WORLD_TILES {
        return reject();
    }
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=tile_reads");
    let mut tiles = BTreeMap::new();
    for p in candidates {
        tiles.insert(
            p,
            read_tile(tx, authority, room, runtime, objects, p).await?,
        );
    }
    let caster_tile = tiles
        .get(&caster_position)
        .ok_or(SpellCastDisposition::Rejected)?;
    if !caster_tile.present {
        return reject();
    }
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=tiles_qualified");
    let caster_protection = caster_tile.protection;
    let sight = paths
        .into_iter()
        .filter_map(|(pair, steps)| {
            steps
                .iter()
                .all(|(p, exempt)| *exempt || tiles.get(p).is_some_and(|t| !t.projectile))
                .then_some(pair)
        })
        .collect::<BTreeSet<_>>();
    Ok(SpatialPhase {
        position,
        caster_position,
        origin,
        facing,
        bindings,
        chain_creatures,
        sight_origin,
        tiles,
        sight,
        attack_target_actor,
        attack_target_tile,
        caster_protection,
    })
}

#[allow(clippy::too_many_arguments)]
async fn qualify_targets(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    profile: &crate::spell::executable_catalog::OperationalProfile,
    actor: ExactActorRef,
    named: Option<(ExactActorRef, &ParameterSpellCastIntent)>,
    pvp_modes: oteryn_protocol_oteryn::pvp_expert::ExpertFightModes,
    players: &[(ExactActorRef, GameSessionId, PlayerSpellState)],
    creatures: &[crate::foundation::CompanionSnapshot],
    spatial: SpatialPhase,
) -> Result<TargetPhase, SpellCastDisposition> {
    let SpatialPhase {
        position,
        caster_position,
        origin,
        facing,
        bindings,
        chain_creatures,
        sight_origin,
        tiles,
        sight,
        attack_target_actor,
        attack_target_tile,
        caster_protection,
    } = spatial;
    let mut pvp_targets = Vec::new();
    // Basic direct damage only: delayed/chain/condition/field effects retain their refusals.
    let basic_pvp = spell.aggressive
        && spell.chain.is_none()
        && profile
            .dependencies
            .effects
            .iter()
            .all(|e| e.operation == "damage")
        && profile
            .dependencies
            .abilities
            .iter()
            .all(|a| a.chain.is_none());
    if basic_pvp {
        for (target, _, _) in players {
            if *target == actor {
                continue;
            }
            if let Ok(facts) = crate::gameplay_transport::pvp_live::legal_player(
                tx, root, authority, runtime, states, room, objects, actor, *target, pvp_modes,
            )
            .await
            {
                pvp_targets.push((*target, facts.target_black));
            }
        }
    }
    let legal = chain_creatures
        .iter()
        .filter(|c| {
            let Some(t) = tiles.get(&c.position) else {
                return false;
            };
            if !t.present || t.floor_change || t.solid {
                return false;
            }
            if spell.aggressive {
                creatures
                    .iter()
                    .find(|m| u64::from(m.actor.actor_local_id()) == c.id)
                    .is_some_and(|m| {
                        m.health > 0
                            && m.state.master.is_none()
                            && m.state.policy.flags.attackable
                            && !t.protection
                            && !caster_protection
                    })
                    || pvp_targets
                        .iter()
                        .any(|(a, _)| u64::from(a.actor_local_id()) == c.id)
            } else {
                true
            }
        })
        .map(|c| c.id)
        .collect::<BTreeSet<_>>();
    let selected = if matches!(
        intent.target,
        SpellTarget::Position(_) | SpellTarget::AttackTarget
    ) {
        chain_creatures
            .iter()
            .filter(|c| {
                attack_target_actor.is_none_or(|t| u64::from(t.actor_local_id()) == c.id)
                    && c.position == attack_target_tile.unwrap_or(origin)
                    && legal.contains(&c.id)
                    && named.is_none_or(|(a, _)| u64::from(a.actor_local_id()) == c.id)
            })
            .min_by_key(|c| c.id)
    } else {
        None
    };
    let selected_actor = selected.and_then(|c| bindings.iter().find(|b| b.source_id == c.id));
    if let (SpellTarget::AttackTarget, Some(target)) = (intent.target, attack_target_tile) {
        let distance = target
            .x
            .abs_diff(caster_position.x)
            .max(target.y.abs_diff(caster_position.y));
        if selected_actor.is_none()
            || !(caster_position == target || sight.contains(&(caster_position, target)))
            || profile
                .header
                .targeting
                .range_tiles
                .is_some_and(|range| distance > range)
        {
            return Err(SpellCastDisposition::TargetIllegal);
        }
    }
    if named.is_some_and(|(a, _)| selected_actor.is_none_or(|b| b.actor != a)) {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    if selected_actor.is_some_and(|t| t.actor == actor) && !profile.header.targeting.allow_on_self {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    // Name-directed healing uses player owners; rune self/own-summon rules
    // remain explicitly owned by allowed_targets and the actual master binding.
    if profile.header.targeting.parameter == "player_name"
        && selected_actor.is_some_and(|t| runtime.contains_live_creature(t.actor))
    {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    let target = selected.map(|c| crate::spell::target::CastTarget {
        caster: u64::from(actor.actor_local_id()),
        creature: c.id,
        actor: c.actor.clone(),
        master: creatures
            .iter()
            .find(|m| u64::from(m.actor.actor_local_id()) == c.id)
            .and_then(|m| m.state.master.map(|m| u64::from(m.actor.actor_local_id()))),
    });
    let operational = OperationalCastFacts {
        caster_position,
        target_position: matches!(
            intent.target,
            SpellTarget::Position(_) | SpellTarget::AttackTarget
        )
        .then_some(origin),
        target,
        line_of_sight_clear: Some(
            caster_position == origin || sight.contains(&(caster_position, origin)),
        ),
        direction_available: position.facing().is_some(),
        wheel_unlocked: None,
        in_protection_zone: caster_protection,
        target_tile_solid: tiles.get(&origin).map(|t| t.solid),
        target_tile_creature: Some(chain_creatures.iter().any(|c| c.position == origin)),
    };
    let world = World {
        origin,
        sight_origin,
        direction: facing,
        caster: ChainCreature {
            id: u64::from(actor.actor_local_id()),
            actor: crate::spell::combat_execution::actor_atom(actor),
            position: caster_position,
        },
        creatures: chain_creatures,
        legal,
        sight,
        tiles,
    };
    let chain_start = ChainStart {
        target: selected_actor.map(|b| b.source_id),
        attacked: None,
    };
    Ok(TargetPhase {
        world,
        operational,
        pvp_targets,
        chain_start,
        bindings,
    })
}

#[allow(clippy::too_many_arguments)]
fn prepare_payment(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    book: &SpellBook,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    command: CommandRef,
    occurrence: &AbilityOccurrence,
    training_occurrence: BuildOccurrence,
    now: SemanticTimeMicros,
    rune: Option<&ItemSpellCastIntent>,
    legacy: Option<&LegacyRuneUse>,
    named: Option<(ExactActorRef, &ParameterSpellCastIntent)>,
    before: &PlayerSpellState,
    caster: &crate::spell::CasterState,
    party: &Option<crate::gameplay_transport::party_spell_owner::SourcePartyWorld>,
    bindings: &[LiveActorBinding],
    world: &World<Direction>,
    operational: &OperationalCastFacts,
    pvp_targets: &[(ExactActorRef, bool)],
    chain_start: ChainStart,
    caster_position: TilePosition,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PaidPreparation, SpellCastDisposition> {
    let prepare_paid = |draw: &mut dyn FnMut(i64, i64) -> i64| {
        prepare_ordinary_owner_cast_with_caster(
            book,
            before,
            spell,
            operational,
            now,
            caster,
            party
                .as_ref()
                .map(|p| p as &dyn crate::spell::party::PartyWorld),
            spell
                .chain
                .as_ref()
                .map(|_| (world as &dyn ChainWorld, chain_start)),
            draw,
        )
    };
    // Entire footprint, magnitude prerequisites, owner stores and outbox capacity
    // are qualified using a non-executed preview before consuming the live stream.
    #[cfg(test)]
    eprintln!(
        "SEAM_EVIDENCE ordinary_combat_preview operational_target={} position_target={} direction={} protection={} needs_target={} target_or_direction={}",
        operational.target.is_some(),
        operational.target_position.is_some(),
        operational.direction_available,
        operational.in_protection_zone,
        spell.needs_target,
        spell.target_or_direction
    );
    #[cfg(test)]
    eprintln!(
        "SEAM_EVIDENCE ordinary_combat_preview caster_state pending_training={} level={} owned_magic={:?} qualified_level={} qualified_magic={} vocation_matches={}",
        before.pending_training_checkpoint().is_some(),
        before.character_facts().level,
        before.owned_effective_magic_level(now.get()),
        caster.level,
        caster.magic_level,
        caster.vocation == before.character_facts().vocation
    );
    let preview = prepare_paid(&mut |minimum, _| minimum).inspect_err(|_reason| {
        #[cfg(test)]
        eprintln!(
            "SEAM_EVIDENCE ordinary_combat_prepare stage=paid_preview_refused reason={_reason:?}"
        );
    })?;
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=paid_preview_qualified");
    let preview_output = lower(
        runtime,
        states,
        room,
        content,
        owned,
        spell,
        intent,
        rune,
        legacy,
        named,
        before,
        &preview,
        caster,
        operational,
        bindings,
        world,
        party,
        pvp_targets,
        command,
        occurrence,
        now,
        &mut |minimum, _| minimum,
    )
    .inspect_err(|_reason| {
        #[cfg(test)]
        eprintln!(
            "SEAM_EVIDENCE ordinary_combat_prepare stage=lower_preview_refused reason={_reason:?}"
        );
    })?;
    #[cfg(test)]
    eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=lower_preview_qualified");
    let formula = content
        .training_formula()
        .ok_or(SpellCastDisposition::Rejected)?;
    before
        .prepare_paid_training(
            &preview.next,
            &preview.anchor,
            formula,
            training_occurrence,
            now.get(),
        )
        .map_err(|_reason| {
            #[cfg(test)]
            eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=training_or_runtime_preview_refused reason={_reason:?}");
            SpellCastDisposition::Rejected
        })?;
    stage_player_batch(
        runtime,
        states,
        &preview_output.batch,
        Some(preview.next.clone()),
    ).inspect_err(|_reason| {
        #[cfg(test)]
        eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=player_batch_preview_refused reason={_reason:?}");
    })?;
    runtime
        .stage_spell_batch(&preview_output.batch)
        .map_err(|_reason| {
            #[cfg(test)]
            eprintln!("SEAM_EVIDENCE ordinary_combat_prepare stage=training_or_runtime_preview_refused reason={_reason:?}");
            SpellCastDisposition::Rejected
        })?;
    let preview_requests = crate::spell::ordinary_timer::ordinary_timer_requests(
        runtime,
        spell,
        &preview_output.batch,
        occurrence,
        caster_position,
        &preview_output.delayed,
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let stamp = if preview_requests.is_empty() {
        None
    } else {
        let stamp = runtime
            .issue_owner_work()
            .map_err(|_| SpellCastDisposition::Rejected)?;
        timer_preflight(
            runtime,
            states,
            &preview_output.batch,
            preview_requests,
            stamp,
        )?;
        Some(stamp)
    };
    let paid = prepare_paid(draw)?;
    let output = lower(
        runtime,
        states,
        room,
        content,
        owned,
        spell,
        intent,
        rune,
        legacy,
        named,
        before,
        &paid,
        caster,
        operational,
        bindings,
        world,
        party,
        pvp_targets,
        command,
        occurrence,
        now,
        draw,
    )?;
    let training = before
        .prepare_paid_training(
            &paid.next,
            &paid.anchor,
            formula,
            training_occurrence,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    stage_player_batch(runtime, states, &output.batch, Some(paid.next.clone()))?;
    runtime
        .stage_spell_batch(&output.batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let timers = if output.delayed.is_empty() {
        None
    } else {
        let requests = crate::spell::ordinary_timer::ordinary_timer_requests(
            runtime,
            spell,
            &output.batch,
            occurrence,
            caster_position,
            &output.delayed,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
        Some(timer_preflight(
            runtime,
            states,
            &output.batch,
            requests,
            stamp.ok_or(SpellCastDisposition::Rejected)?,
        )?)
    };
    Ok(PaidPreparation {
        paid: paid.next,
        output,
        training,
        timers,
    })
}
