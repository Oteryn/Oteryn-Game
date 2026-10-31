//! Owner qualification, spatial facts, preview and live native combat phases.
use super::*;

struct SpatialPhase {
    position: TilePosition,
    bindings: Vec<LiveActorBinding>,
    tiles: BTreeMap<TilePosition, TileFlags>,
    operational: OperationalCastFacts,
    facts: NativeCombatFacts,
    delayed: Option<crate::spell::native_delayed::NativeDelayedFacts>,
}

struct PreviewPhase {
    needs_magnitude: bool,
    binding: Vec<u8>,
    attacker: CharacterId,
    stamp: Option<crate::foundation::RuntimeWorkStamp>,
}

struct PaidPreparation {
    paid: crate::spell::cast::PaidNativeCast,
    lowered: crate::spell::combat_batch::LoweredNativeCombat,
    training: PreparedPlayerTraining,
    timers: Option<ValidatedTimerInstall>,
    magnitude: Option<PreparedMagnitudeOwner>,
    presentation: crate::gameplay_transport::spell_presentations::PreparedPresentation,
}

#[allow(clippy::too_many_arguments)]
pub async fn prepare_from_owners(
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
    rune: Option<&ItemSpellCastIntent>,
    attack_target: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    let Execution::NativeProfile(profile) = &spell.execution else {
        return reject();
    };
    let recipe = &profile.spell()["execution"]["native_behavior"];
    let key = recipe["key"]
        .as_str()
        .ok_or(SpellCastDisposition::Rejected)?;
    if matches!(intent.target, SpellTarget::Position(_))
        && !spell
            .authored
            .as_ref()
            .is_some_and(|p| p.header.targeting.cast_at_position == Some(true))
    {
        return reject();
    }
    if !matches!(
        key,
        "wheel_combat"
            | "avatar_state"
            | "monster_ai_override"
            | "mass_spirit_mend"
            | "mana_shield_capacity"
            | "delayed_strike"
    ) || (key != "delayed_strike"
        && (intent.aim_at_target || intent.target != SpellTarget::None))
        || book.indexed(intent.spell) != Some(spell)
        || content.spell_book().indexed(intent.spell) != Some(spell)
        || !matches!(spell.carrier, crate::spell::Carrier::Instant { .. })
    {
        return reject();
    }
    let parameters = &recipe["parameters"];
    let b = owned.binding();
    let actor = b.actor;
    let session = command.game_session_id();
    let actual = runtime.binding();
    if b.session != session
        || authority.command() != command
        || authority.character_id_bytes() != b.character
        || authority.compatible_content_digest() != content.source_digest()
        || authority.runtime_scope() != actual.runtime_scope()
        || authority.scope_generation() != actual.scope_generation().get()
        || b.content_digest != runtime.content_pin().server_artifact_digest()
    {
        return reject();
    }
    let before_snapshot = states
        .get(runtime, actor, session)
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    let before = &before_snapshot;
    let party = if may_heal_player(spell)
        || matches!(
            before.source_party_vocation(),
            crate::spell::Vocation::Monk | crate::spell::Vocation::ExaltedMonk
        ) {
        Some(
            crate::gameplay_transport::party_spell_owner::read_source_party_world_in_transaction(
                tx, root, authority, runtime, states, b.actor, b.session,
            )
            .await
            .map_err(|_| SpellCastDisposition::Rejected)?,
        )
    } else {
        None
    };
    let caster = owned
        .caster(
            before,
            spell,
            content,
            before.owned_harmony_multiplier(spell)?,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let training_formula = content
        .training_formula()
        .ok_or(SpellCastDisposition::Rejected)?;
    let stage = if key == "delayed_strike"
        && spell
            .authored
            .as_ref()
            .is_some_and(|p| p.header.requirements.wheel_unlock == Some(true))
    {
        owned
            .wheel_stage(&spell.name, now.get())
            .map_err(|_| SpellCastDisposition::Rejected)?
    } else {
        match parameters.get("wheel") {
            Some(w) => owned
                .wheel_stage(
                    w["perk"].as_str().ok_or(SpellCastDisposition::Rejected)?,
                    now.get(),
                )
                .or_else(|error| {
                    wheel_stage_without_owner(w, error, owned.has_current_wheel(now.get()))
                })?,
            None => 0,
        }
    };
    let elemental = if parameters.get("elemental_stance").is_some() {
        owned
            .elemental_stance(now.get())
            .ok_or(SpellCastDisposition::Rejected)?
    } else {
        ElementalStance::None
    }; // unconsumed for these recipes
    let roster = runtime
        .positioned_actor_census()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let creatures = roster
        .iter()
        .filter(|(_, _, s)| s.is_none())
        .map(|(actor, _, _)| {
            runtime
                .companion_snapshot(*actor)
                .map_err(|_| SpellCastDisposition::Rejected)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let player_predecessors: Vec<(ExactActorRef, GameSessionId, PlayerSpellState)> = states
        .actors
        .iter()
        .filter(|(a, s, _)| states.get(runtime, *a, *s).is_some())
        .cloned()
        .collect();
    let SpatialPhase {
        position,
        bindings,
        tiles,
        operational,
        facts,
        delayed,
    } = prepare_spatial(
        tx,
        authority,
        runtime,
        states,
        room,
        objects,
        spell,
        intent,
        parameters,
        actor,
        stage,
        elemental,
        now,
        &caster,
        &roster,
        attack_target,
        None,
        owned,
        before,
    )
    .await?;
    let players: Vec<_> = player_predecessors
        .iter()
        .map(|(a, _, p)| (*a, p))
        .collect();
    let PreviewPhase {
        needs_magnitude,
        binding,
        attacker,
        stamp,
    } = prepare_preview(
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
        profile,
        before,
        &caster,
        &operational,
        &facts,
        delayed.as_ref(),
        &bindings,
        &players,
        training_formula,
        position,
        stage,
    )?;
    let PaidPreparation {
        paid,
        lowered,
        training,
        timers,
        magnitude,
        presentation,
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
        profile,
        before,
        &caster,
        &operational,
        &facts,
        delayed.as_ref(),
        &bindings,
        &players,
        training_formula,
        position,
        needs_magnitude,
        binding,
        attacker,
        stamp,
        draw,
    )?;
    Ok(PreparedNativeCombatCast {
        item_spell_leech: item_spell_augments::leech(owned, spell, before),
        corpse_companion: None,
        direct_companion: None,
        field_policy_revision: None,
        carried_target: None,
        item_operations: Vec::new(),
        item_creations: Vec::new(),
        party,
        before: before.clone(),
        paid: paid.next,
        batch: lowered.batch,
        tile_facts: tiles,
        roster,
        training,
        timers,
        magnitude: magnitude.map(Box::new),
        facts_binding: b.clone(),
        creatures,
        players: player_predecessors,
        equipment: owned.equipment().clone(),
        presentation: Some(presentation),
        installation: None,
    })
}

#[allow(clippy::too_many_arguments)]
async fn prepare_spatial(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    parameters: &Value,
    actor: ExactActorRef,
    stage: u8,
    elemental: ElementalStance,
    now: SemanticTimeMicros,
    caster: &crate::spell::CasterState,
    roster: &[(
        ExactActorRef,
        MovementPositionSnapshot,
        Option<GameSessionId>,
    )],
    attack_target: Option<oteryn_protocol_oteryn::world_spatial_entities::EntityRef>,
    saved: Option<&crate::spell::delayed_execution::SavedStrike>,
    owned: &OwnedCastFacts,
    before: &PlayerSpellState,
) -> Result<SpatialPhase, SpellCastDisposition> {
    let current = runtime
        .read_actor_position(actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let position = tile(current.position());
    let needs_direction = spell
        .authored
        .as_ref()
        .is_some_and(|p| p.header.targeting.needs_direction)
        || parameters["area"]["directional"] == true;
    let facing = current.facing();
    if needs_direction && facing.is_none() {
        return reject();
    }
    let facing_direction = facing.map(direction).unwrap_or(Direction::North); // unused by symmetric/self recipes
    let is_delayed = parameters.get("delay_ms").is_some();
    let held = is_delayed
        .then(|| delayed_strikes::held_target(runtime, attack_target))
        .flatten();
    let origin = if let Some(saved) = saved {
        saved.scheduled.position
    } else {
        match intent.target {
            SpellTarget::Position(p) => TilePosition {
                x: p.x,
                y: p.y,
                floor: p.floor,
            },
            SpellTarget::None if needs_direction => {
                let (dx, dy) = delta(facing_direction);
                offset(position, dx, dy)?
            }
            SpellTarget::None | SpellTarget::AttackTarget if is_delayed => {
                if spell
                    .authored
                    .as_ref()
                    .is_some_and(|p| p.header.targeting.cast_at_position == Some(true))
                {
                    held.map_or(position, |(_, p)| p)
                } else {
                    position
                }
            }
            SpellTarget::None => position,
            SpellTarget::AttackTarget => return reject(),
        }
    };
    if origin.floor != position.floor && saved.is_none() {
        return reject();
    }
    let sight_origin = if saved.is_some() && parameters.get("chain").is_some() {
        position
    } else if needs_direction && origin != position {
        let (dx, dy) = delta(facing_direction);
        offset(origin, -dx, -dy)?
    } else {
        origin
    };
    let area_parameters;
    let geometry_parameters = if parameters["area"].is_array() {
        area_parameters = json!({"area":{"directional":false,"orthogonal":parameters["area"]}});
        &area_parameters
    } else {
        parameters
    };
    let mut candidates = area_candidates(geometry_parameters, origin, facing_direction)?;
    candidates.insert(position);
    candidates.insert(origin);
    let TargetPhase {
        bindings,
        attackable,
        mut targets,
    } = collect_targets(
        runtime,
        states,
        if saved.is_some() && parameters.get("chain").is_none() {
            origin
        } else {
            position
        },
        roster,
        &mut candidates,
    )?;
    let caster_id = u64::from(actor.actor_local_id());
    if saved.is_none() && !bindings.iter().any(|b| b.actor == actor) {
        return reject();
    }
    let mut paths = BTreeMap::new();
    if saved.is_none() && position.floor == origin.floor {
        paths.insert((position, origin), sight_steps(position, origin)?);
    }
    for target in &candidates {
        // At due time only the fixed area's sight (or the current chain's sight)
        // matters. A teleport must not introduce a new caster-to-old-origin ray.
        if saved.is_some()
            && *target != sight_origin
            && (*target == position || (parameters.get("chain").is_some() && *target == origin))
        {
            continue;
        }
        if target.floor != sight_origin.floor {
            continue;
        }
        paths.insert((sight_origin, *target), sight_steps(sight_origin, *target)?);
    }
    if parameters.get("chain").is_some() {
        for a in targets
            .iter()
            .map(|t| t.position)
            .chain(std::iter::once(position))
        {
            for b in targets.iter().map(|t| t.position) {
                if a.x.abs_diff(b.x) <= 7 && a.y.abs_diff(b.y) <= 7 {
                    paths.insert((a, b), sight_steps(a, b)?);
                }
            }
        }
    }
    for steps in paths.values() {
        for (p, _) in steps {
            candidates.insert(*p);
        }
    }
    if candidates.len() > MAX_WORLD_TILES {
        return reject();
    }
    let mut tiles = BTreeMap::new();
    for p in candidates {
        tiles.insert(
            p,
            read_tile(tx, authority, room, runtime, objects, p).await?,
        );
    }
    let caster_tile = tiles.get(&position).ok_or(SpellCastDisposition::Rejected)?;
    if !caster_tile.present {
        return reject();
    }
    let mut clear_sight = BTreeSet::new();
    for (pair, steps) in paths {
        if steps
            .iter()
            .all(|(p, exempt)| *exempt || tiles.get(p).is_some_and(|t| !t.projectile))
        {
            clear_sight.insert(pair);
        }
    }
    for target in &mut targets {
        let t = tiles
            .get(&target.position)
            .ok_or(SpellCastDisposition::Rejected)?;
        target.legal = cast_plans::target_legal(
            t,
            caster_tile,
            target.health,
            spell.aggressive,
            target.kind == TargetKind::MasterlessMonster,
            attackable.contains(&target.id),
        );
    }
    let target_position = match intent.target {
        _ if is_delayed
            && spell
                .authored
                .as_ref()
                .is_some_and(|p| p.header.targeting.cast_at_position == Some(true)) =>
        {
            Some(origin)
        }
        SpellTarget::Position(_) => Some(origin),
        _ => None,
    };
    let operational = OperationalCastFacts {
        caster_position: position,
        target_position,
        target: None,
        line_of_sight_clear: target_position
            .map(|p| p == position || clear_sight.contains(&(position, p))),
        direction_available: facing.is_some(),
        wheel_unlocked: (parameters.get("wheel").is_some()
            || spell
                .authored
                .as_ref()
                .is_some_and(|p| p.header.requirements.wheel_unlock == Some(true)))
        .then_some(stage > 0),
        in_protection_zone: caster_tile.protection,
        target_tile_solid: tiles.get(&origin).map(|t| t.solid),
        target_tile_creature: Some(
            targets
                .iter()
                .any(|t| t.position == origin && t.id != caster_id),
        ),
    };
    let facts = NativeCombatFacts {
        now_ms: now.get() / 1000,
        stage,
        inputs: FormulaInputs {
            level: caster.level,
            magic_level: caster.magic_level,
            base_power: spell.base_power,
            attack_skill: caster.attack_skill,
            attack_value: caster.attack_value,
            attack_factor: caster.attack_factor,
            shielding_skill: caster.shielding_skill,
            shield_defense: caster.shield_defense,
        },
        maximum_mana: caster.max_mana,
        caster: caster_id,
        caster_position: position,
        area_origin: origin,
        area_sight_origin: sight_origin,
        direction: facing_direction,
        explicit_target: None,
        attacked_target: None,
        targets,
        clear_sight,
        floor_change_tiles: tiles
            .iter()
            .filter_map(|(p, t)| t.floor_change.then_some(*p))
            .collect(),
        elemental_stance: elemental,
        incoming_hit: None,
    };
    let delayed = if is_delayed {
        Some(delayed_strikes::facts(
            runtime,
            owned,
            before,
            caster,
            spell,
            &facts,
            &tiles,
            held.map(|(a, _)| u64::from(a.actor_local_id())),
            saved,
        )?)
    } else {
        None
    };
    Ok(SpatialPhase {
        position,
        bindings,
        tiles,
        operational,
        facts,
        delayed,
    })
}

struct TargetPhase {
    bindings: Vec<LiveActorBinding>,
    attackable: BTreeSet<u64>,
    targets: Vec<NativeTargetFact>,
}

fn collect_targets(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    position: TilePosition,
    roster: &[(
        ExactActorRef,
        MovementPositionSnapshot,
        Option<GameSessionId>,
    )],
    candidates: &mut BTreeSet<TilePosition>,
) -> Result<TargetPhase, SpellCastDisposition> {
    let mut bindings = Vec::new();
    let mut attackable = BTreeSet::new();
    let mut targets = Vec::new();
    for (target, snapshot, target_session) in roster {
        let p = tile(snapshot.position());
        if p.floor != position.floor
            || p.x.abs_diff(position.x) > MAX_SIGHT_DISTANCE
            || p.y.abs_diff(position.y) > MAX_SIGHT_DISTANCE
        {
            continue;
        }
        let id = u64::from(target.actor_local_id());
        let (name, kind, reward_boss, preferred_distance, health, maximum_health, atom) =
            if let Some(s) = target_session {
                let state = states
                    .get(runtime, *target, *s)
                    .ok_or(SpellCastDisposition::Rejected)?;
                let v = state.vitals();
                (
                    String::new(),
                    TargetKind::Player,
                    false,
                    0,
                    v.health,
                    v.max_health,
                    crate::spell::combat_execution::actor_atom(*target),
                )
            } else {
                let creature = runtime
                    .companion_snapshot(*target)
                    .map_err(|_| SpellCastDisposition::Rejected)?;
                let policy = &creature.state.policy;
                if policy.flags.attackable {
                    attackable.insert(id);
                }
                let kind = match &creature.state.master {
                    None => TargetKind::MasterlessMonster,
                    Some(_) => TargetKind::PlayerSummon,
                };
                (
                    policy.display_name.clone(),
                    kind,
                    policy.reward_boss.ok_or(SpellCastDisposition::Rejected)?,
                    policy
                        .preferred_distance
                        .ok_or(SpellCastDisposition::Rejected)?,
                    u32::try_from(creature.health).map_err(|_| SpellCastDisposition::Rejected)?,
                    u32::try_from(creature.maximum_health)
                        .map_err(|_| SpellCastDisposition::Rejected)?,
                    runtime
                        .creature_spell_target_atom(*target)
                        .map_err(|_| SpellCastDisposition::Rejected)?,
                )
            };
        bindings.push(LiveActorBinding {
            source_id: id,
            actor: *target,
            target_atom: atom,
        });
        targets.push(NativeTargetFact {
            id,
            position: p,
            kind,
            name,
            reward_boss,
            preferred_distance,
            health,
            maximum_health,
            legal: false,
        });
        candidates.insert(p);
    }
    Ok(TargetPhase {
        bindings,
        attackable,
        targets,
    })
}

#[allow(clippy::too_many_arguments)]
fn prepare_preview(
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
    profile: &crate::spell::native::CompiledNativeSpell,
    before: &PlayerSpellState,
    caster: &crate::spell::CasterState,
    operational: &OperationalCastFacts,
    facts: &NativeCombatFacts,
    delayed: Option<&crate::spell::native_delayed::NativeDelayedFacts>,
    bindings: &[LiveActorBinding],
    players: &[(ExactActorRef, &PlayerSpellState)],
    training_formula: &crate::content::training_formula::CompiledTrainingFormula,
    position: TilePosition,
    stage: u8,
) -> Result<PreviewPhase, SpellCastDisposition> {
    let b = owned.binding();
    let actor = b.actor;
    let session = command.game_session_id();
    let caster_id = u64::from(actor.actor_local_id());
    // A lower-bound preview is never committed and consumes no live random stream. It validates
    // common costs, source shape, all magnitude prerequisites and training before the first draw.
    let (mut preview, delayed_plan) = delayed_strikes::paid(
        before,
        spell,
        operational,
        facts,
        delayed,
        now,
        &mut |minimum, _| minimum,
        caster,
    )?;
    let Plan::Combat(preview_plan) = &preview.plan else {
        return reject();
    };
    let preview_plan = preview_plan.clone();
    if let NativeCombatPlan::Combat(p) = preview_plan.as_ref()
        && (p.use_weapon_charges || p.weapon_missile || p.hits.iter().any(|h| h.delay_ms > 0))
    {
        return reject();
    }
    let needs_magnitude = matches!(
        preview_plan.as_ref(),
        NativeCombatPlan::Combat(_) | NativeCombatPlan::MassSpiritMend { .. }
    );
    let mut preview_magnitude = if needs_magnitude {
        Some(
            PreparedMagnitudeOwner::qualify(
                runtime,
                before,
                actor,
                session,
                owned.magnitude(),
                &preview_plan,
                bindings,
                players,
                facts.now_ms,
            )
            .map_err(|_| SpellCastDisposition::Rejected)?,
        )
    } else {
        None
    };
    if let Some(owner) = preview_magnitude.as_mut() {
        owner
            .select_item_spell(spell)
            .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    let mut captured = json!({"intent":source_cast_intent(intent,rune),"spell":profile.spell(),
        "equipment_revision":b.equipment_revision,"character_revision":b.character_revision,
        "source_stage":stage,"capture_format":"owner-debug-r21",
        // Immutable receipts only; these captures grant no owner authority.
        "native_facts":format!("{facts:?}"),
        "magnitude_facts":format!("{:?}",owned.magnitude()),
        "equipment_facts":format!("{:?}",owned.equipment())});
    if let Some(delayed) = delayed {
        captured["delayed_facts"] = json!(format!("{delayed:?}"));
    }
    let binding = serde_json::to_vec(&captured).map_err(|_| SpellCastDisposition::Rejected)?;
    let attacker = CharacterId::decode(&b.character).map_err(|_| SpellCastDisposition::Rejected)?;
    let preview_batch = OwnerCombatBatch {
        caster: actor,
        attacker,
        current_lease_generation: b.lease_generation,
        command,
        occurrence: occurrence.clone().into(),
        binding: binding.clone(),
        anchor: Some(preview.anchor.clone()),
        now_us: Some(now),
        now_ms: facts.now_ms,
        effects: Vec::new(),
        deferred: None,
    };
    let mut preview_lowered = lower_native(
        runtime,
        preview_batch,
        caster_id,
        &preview_plan,
        bindings,
        0,
        &mut |plan, hit, target| {
            preview_magnitude
                .as_mut()
                .ok_or(Error::InvalidBatch)?
                .finish_with_damage_healing(plan, hit, target, &mut |minimum, _| minimum)
        },
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    crate::spell::native_cast_commit::finalize_paid_combat_cast(
        before,
        spell,
        book,
        &mut preview,
        &mut preview_lowered,
        now,
    )?;
    item_spell_augments::cooldown(owned, spell, &mut preview.next, &mut preview.anchor, now)?;
    preview_lowered.batch.anchor = Some(preview.anchor.clone());
    if preview_lowered.batch.effects.iter().any(|effect| {
        effect.target != actor
            && matches!(
                effect.change,
                crate::foundation::runtime_actor_spell_types::OwnerCombatChange::Heal { .. }
            )
    }) && runtime
        .current_player_reentry_protection(actor, session, now.get())
        .map_err(|_| SpellCastDisposition::Rejected)?
    {
        return reject();
    }
    let preview_cues = delayed_strikes::cues(
        room,
        runtime,
        &preview_plan,
        delayed_plan.as_ref(),
        profile,
        bindings,
        actor,
    )?;
    let presentation_owner = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?;
    presentation_owner
        .reserve_before_draw(runtime, preview_cues.len())
        .map_err(|_| SpellCastDisposition::Rejected)?;
    presentation_owner
        .prepare_positions(
            runtime,
            content,
            intent.spell,
            profile,
            &mut preview_lowered.batch,
            preview_cues,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    runtime
        .stage_spell_batch(&preview_lowered.batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    stage_player_batch(
        runtime,
        states,
        &preview_lowered.batch,
        Some(preview.next.clone()),
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let preview_requests = delayed_strikes::requests(
        &preview_lowered,
        delayed_plan.as_ref(),
        delayed,
        profile,
        occurrence,
        position,
        now,
        owned,
        before,
    )?;
    let stamp = if preview_requests.is_empty() {
        None
    } else {
        let stamp = runtime
            .issue_owner_work()
            .map_err(|_| SpellCastDisposition::Rejected)?;
        timer_preflight(
            runtime,
            states,
            &preview_lowered.batch,
            preview_requests,
            stamp,
        )?;
        Some(stamp)
    };
    before
        .prepare_paid_training(
            &preview.next,
            &preview.anchor,
            training_formula,
            training_occurrence,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    Ok(PreviewPhase {
        needs_magnitude,
        binding,
        attacker,
        stamp,
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
    profile: &crate::spell::native::CompiledNativeSpell,
    before: &PlayerSpellState,
    caster: &crate::spell::CasterState,
    operational: &OperationalCastFacts,
    facts: &NativeCombatFacts,
    delayed: Option<&crate::spell::native_delayed::NativeDelayedFacts>,
    bindings: &[LiveActorBinding],
    players: &[(ExactActorRef, &PlayerSpellState)],
    training_formula: &crate::content::training_formula::CompiledTrainingFormula,
    position: TilePosition,
    needs_magnitude: bool,
    binding: Vec<u8>,
    attacker: CharacterId,
    stamp: Option<crate::foundation::RuntimeWorkStamp>,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PaidPreparation, SpellCastDisposition> {
    let b = owned.binding();
    let actor = b.actor;
    let session = command.game_session_id();
    let caster_id = u64::from(actor.actor_local_id());
    let (mut paid, delayed_plan) = delayed_strikes::paid(
        before,
        spell,
        operational,
        facts,
        delayed,
        now,
        draw,
        caster,
    )?;
    let Plan::Combat(plan) = &paid.plan else {
        return reject();
    };
    let mut magnitude = if needs_magnitude {
        Some(
            PreparedMagnitudeOwner::qualify(
                runtime,
                before,
                actor,
                session,
                owned.magnitude(),
                plan,
                bindings,
                players,
                facts.now_ms,
            )
            .map_err(|_| SpellCastDisposition::Rejected)?,
        )
    } else {
        None
    };
    if let Some(owner) = magnitude.as_mut() {
        owner
            .select_item_spell(spell)
            .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    let batch = OwnerCombatBatch {
        caster: actor,
        attacker,
        current_lease_generation: b.lease_generation,
        command,
        occurrence: occurrence.clone().into(),
        binding,
        anchor: Some(paid.anchor.clone()),
        now_us: Some(now),
        now_ms: facts.now_ms,
        effects: Vec::new(),
        deferred: None,
    };
    let mut lowered = lower_native(
        runtime,
        batch,
        caster_id,
        plan,
        bindings,
        0,
        &mut |plan, hit, target| {
            magnitude
                .as_mut()
                .ok_or(Error::InvalidBatch)?
                .finish_with_damage_healing(plan, hit, target, draw)
        },
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    crate::spell::native_cast_commit::finalize_paid_combat_cast(
        before,
        spell,
        book,
        &mut paid,
        &mut lowered,
        now,
    )?;
    item_spell_augments::cooldown(owned, spell, &mut paid.next, &mut paid.anchor, now)?;
    lowered.batch.anchor = Some(paid.anchor.clone());
    let Plan::Combat(final_plan) = &paid.plan else {
        return reject();
    };
    let cues = delayed_strikes::cues(
        room,
        runtime,
        final_plan,
        delayed_plan.as_ref(),
        profile,
        bindings,
        actor,
    )?;
    let presentation = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .prepare_positions(
            runtime,
            content,
            intent.spell,
            profile,
            &mut lowered.batch,
            cues,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let training = before
        .prepare_paid_training(
            &paid.next,
            lowered
                .batch
                .anchor
                .as_ref()
                .ok_or(SpellCastDisposition::Rejected)?,
            training_formula,
            training_occurrence,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let requests = delayed_strikes::requests(
        &lowered,
        delayed_plan.as_ref(),
        delayed,
        profile,
        occurrence,
        position,
        now,
        owned,
        before,
    )?;
    // Physical/timer capacity was also checked by the non-executed source preview before RNG.
    runtime
        .stage_spell_batch(&lowered.batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let timers = if requests.is_empty() {
        None
    } else {
        Some(timer_preflight(
            runtime,
            states,
            &lowered.batch,
            requests,
            stamp.ok_or(SpellCastDisposition::Rejected)?,
        )?)
    };
    Ok(PaidPreparation {
        paid,
        lowered,
        training,
        timers,
        magnitude,
        presentation,
    })
}

/// Due-only planning has no payment anchor and cannot consume a second resource cost.
#[allow(
    clippy::too_many_arguments,
    reason = "fresh due SQL and physical owner inputs"
)]
pub(in crate::gameplay_transport) async fn prepare_strike_due(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    book: &SpellBook,
    saved: &crate::spell::delayed_execution::SavedStrike,
    now: SemanticTimeMicros,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<Option<crate::spell::delayed_execution::ResolvedStrike>, SpellCastDisposition> {
    use crate::spell::native::CompiledNativeSpellPlanExt;
    let index = (1_u32..)
        .map_while(std::num::NonZeroU32::new)
        .take_while(|i| book.source_indexed(*i).is_some())
        .find(|i| {
            book.source_indexed(*i).is_some_and(|(s, active)| {
                active
                    && matches!(&s.execution,
            Execution::NativeProfile(p) if p.spell() == saved.binding.spell.spell())
            })
        })
        .ok_or(SpellCastDisposition::Rejected)?;
    let spell = book.indexed(index).ok_or(SpellCastDisposition::Rejected)?;
    let before = states
        .get(
            runtime,
            saved.binding.caster,
            saved.binding.command.game_session_id(),
        )
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    let caster = owned
        .numerical_caster(
            &before,
            spell,
            content,
            before.owned_harmony_multiplier(spell)?,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let parameters = &saved.binding.spell.spell()["execution"]["native_behavior"]["parameters"];
    let stage = if spell
        .authored
        .as_ref()
        .is_some_and(|p| p.header.requirements.wheel_unlock == Some(true))
    {
        owned
            .wheel_stage(&spell.name, now.get())
            .map_err(|_| SpellCastDisposition::Rejected)?
    } else {
        0
    };
    let roster = runtime
        .positioned_actor_census()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let intent = SpellCastIntent {
        spell: index,
        target: SpellTarget::None,
        aim_at_target: false,
    };
    let spatial = prepare_spatial(
        tx,
        authority,
        runtime,
        states,
        room,
        objects,
        spell,
        &intent,
        parameters,
        saved.binding.caster,
        stage,
        ElementalStance::None,
        now,
        &caster,
        &roster,
        None,
        Some(saved),
        owned,
        &before,
    )
    .await?;
    let facts = spatial
        .delayed
        .as_ref()
        .ok_or(SpellCastDisposition::Rejected)?;
    // A due callback has no cost to roll back. A current planner refusal or no
    // remaining victim consumes the pending strike instead of blocking the lane.
    let Ok(Plan::Delayed(plan)) = saved.binding.spell.plan(Facts::Delayed(facts), draw) else {
        return Ok(None);
    };
    let crate::spell::native_delayed::NativeDelayedPlan::Occurrence { strike, .. } = plan.as_ref()
    else {
        return reject();
    };
    let Some(strike) = strike.as_ref().filter(|s| !s.hits.is_empty()) else {
        return Ok(None);
    };
    let plan = delayed_strikes::combat(Some(strike), None);
    let players: Vec<_> = states
        .actors
        .iter()
        .filter(|(a, s, _)| states.get(runtime, *a, *s).is_some())
        .map(|(a, _, state)| (*a, state))
        .collect();
    let mut magnitude = PreparedMagnitudeOwner::qualify(
        runtime,
        &before,
        saved.binding.caster,
        saved.binding.command.game_session_id(),
        owned.magnitude(),
        &plan,
        &spatial.bindings,
        &players,
        now.get() / 1000,
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    magnitude
        .select_item_spell(spell)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    if saved.scheduled.captured_raw_bounds.is_some() {
        magnitude.select_planted_strike(&saved.cast_modifiers);
    }
    if let Some(stance) = &saved.scheduled.stance {
        magnitude
            .select_echo_stance(stance)
            .map_err(|_| SpellCastDisposition::Rejected)?;
    }
    let mut binding: Value = serde_json::from_slice(&saved.binding.parent_binding)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let object = binding
        .as_object_mut()
        .ok_or(SpellCastDisposition::Rejected)?;
    object.remove("source_presentation_bytes");
    object.insert("delayed_strike_due".into(), json!(true));
    let batch = OwnerCombatBatch {
        caster: saved.binding.caster,
        attacker: saved.binding.attacker,
        current_lease_generation: owned.binding().lease_generation,
        command: saved.binding.command,
        occurrence: saved.binding.occurrence.clone().into(),
        binding: serde_json::to_vec(&binding).map_err(|_| SpellCastDisposition::Rejected)?,
        anchor: None,
        now_us: Some(now),
        now_ms: now.get() / 1000,
        effects: Vec::new(),
        deferred: None,
    };
    let mut lowered = lower_native(
        runtime,
        batch,
        u64::from(saved.binding.caster.actor_local_id()),
        &plan,
        &spatial.bindings,
        saved.first_sub_ordinal,
        &mut |plan, hit, target| magnitude.finish_with_damage_healing(plan, hit, target, draw),
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    states.apply_prey_spell_damage(runtime, &mut lowered.batch);
    let cues = delayed_strikes::strike_cues(room, runtime, Some(strike), &spatial.bindings)?;
    let presentation = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .prepare_positions(
            runtime,
            content,
            index,
            &saved.binding.spell,
            &mut lowered.batch,
            cues,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .hold_prepared_before_sql(runtime, &presentation, &lowered.batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    Ok(Some(crate::spell::delayed_execution::ResolvedStrike {
        batch: lowered.batch,
        presentation: std::sync::Arc::new(presentation),
        magnitude: std::sync::Arc::new(magnitude),
        owned: owned.clone(),
    }))
}
