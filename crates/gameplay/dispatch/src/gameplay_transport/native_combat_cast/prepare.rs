//! Owner qualification, spatial facts, preview and live native combat phases.
use super::*;
use crate::gameplay_transport::spell_presentations::monk_bond_impact;
use crate::spell::native::CompiledNativeSpellPlanExt;
use crate::spell::native_combat::Element;
use oteryn_gameplay_spell_rules::native_actor_states::{EquipmentAttackPlan, EquipmentFacts};

struct SpatialPhase {
    position: TilePosition,
    bindings: Vec<LiveActorBinding>,
    tiles: BTreeMap<TilePosition, TileFlags>,
    operational: OperationalCastFacts,
    facts: NativeCombatFacts,
    delayed: Option<crate::spell::native_delayed::NativeDelayedFacts>,
    equipment_geometry: Vec<BTreeSet<TilePosition>>,
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

/// WU26-14: the TARGET or CURSOR option of a native wave or beam, by the ordinary path's rule. A
/// delayed strike keeps its own target and `aim_at_target` handling.
fn native_aim(
    key: &str,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
) -> Result<Option<super::ordinary_combat::Aim>, SpellCastDisposition> {
    if key == "delayed_strike" {
        Ok(None)
    } else {
        super::ordinary_combat::aim(spell, intent)
    }
}

#[cfg(test)]
#[path = "aim_tests.rs"]
mod aim_tests;

#[cfg(test)]
#[path = "weapon_damage_tests.rs"]
mod weapon_damage_tests;

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
    charms: &crate::content::charm_source::CanonicalCharmCatalogue,
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
    let aim = native_aim(key, spell, intent)?;
    if aim.is_none()
        && matches!(intent.target, SpellTarget::Position(_))
        && !spell
            .authored
            .as_ref()
            .is_some_and(|p| p.header.targeting.cast_at_position == Some(true))
    {
        return reject();
    }
    if !(monk_equipment::applies(spell)
        || matches!(
            key,
            "wheel_combat"
                | "avatar_state"
                | "monster_ai_override"
                | "mass_spirit_mend"
                | "mana_shield_capacity"
                | "delayed_strike"
        ))
        || (key != "delayed_strike"
            && aim.is_none()
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
    let equipment_facts = monk_equipment::applies(spell)
        .then(|| monk_equipment::facts(content, owned, spell, &caster, now))
        .transpose()?;
    let equipment_plan = equipment_facts
        .as_ref()
        .map(|facts| {
            let Plan::Equipment(mut plan) = profile
                .plan(Facts::Equipment(facts), &mut |minimum, _| minimum)
                .map_err(|_| SpellCastDisposition::Rejected)?
            else {
                return reject();
            };
            monk_equipment::apply_parity(spell, &caster, facts, &mut plan)?;
            Ok(plan)
        })
        .transpose()?;
    let monk_bond = if spell.key == "candidate:spell/spiritual_outburst" {
        Some(ordinary_combat::monk_weapon_element(
            content,
            owned.equipment(),
        )?)
    } else {
        None
    };
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
        equipment_geometry,
    } = prepare_spatial(
        tx,
        authority,
        runtime,
        states,
        room,
        objects,
        spell,
        intent,
        aim,
        parameters,
        equipment_plan.as_deref(),
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
    let equipment = equipment_facts
        .as_ref()
        .map(|facts| (facts, equipment_geometry.as_slice()));
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
        charms,
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
        equipment,
        monk_bond,
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
        charms,
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
        equipment,
        monk_bond,
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
        outgoing_charms: None,
        leech_inputs: Vec::new(),
        item_spell_leech: item_spell_augments::leech(owned, spell, before, now),
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
    aim: Option<super::ordinary_combat::Aim>,
    parameters: &Value,
    equipment_plan: Option<&EquipmentAttackPlan>,
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
    let needs_direction = equipment_plan.is_some()
        || spell
            .authored
            .as_ref()
            .is_some_and(|p| p.header.targeting.needs_direction)
        || parameters["area"]["directional"] == true;
    let facing = current.facing();
    // WU26-14: an aimed wave or beam turns toward its target or cursor tile from the
    // caster-adjacent origin, as on the ordinary path; without one it keeps the facing.
    let aimed = match aim {
        Some(aim) => Some(super::ordinary_combat::aimed_origin(
            position,
            facing.map(direction),
            super::ordinary_combat::aim_point(
                runtime,
                states,
                actor,
                current.position(),
                aim,
                attack_target,
                now,
            )?,
        )?),
        None => None,
    };
    if needs_direction && facing.is_none() && aimed.is_none() {
        return reject();
    }
    let facing_direction = match aimed {
        Some((_, d)) => d,
        None => facing.map(direction).unwrap_or(Direction::North), // unused by symmetric/self recipes
    };
    let is_delayed = parameters.get("delay_ms").is_some();
    let is_divine_grenade = parameters["spell_name"] == "divine grenade";
    // Crystal 201189f spells.cpp:1252-1257 uses the caster tile when no position is sent.
    let held = (is_delayed && !is_divine_grenade)
        .then(|| delayed_strikes::held_target(runtime, attack_target))
        .flatten();
    let origin = if let Some(saved) = saved {
        saved.scheduled.position
    } else if let Some((origin, _)) = aimed {
        origin
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
            // Crystal spells.cpp:1254–1257 uses the caster when there is no cursor tile.
            SpellTarget::None | SpellTarget::AttackTarget
                if parameters["spell_name"] == "death echo" =>
            {
                position
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
    let equipment_geometry = equipment_plan
        .map(|plan| monk_equipment::geometry(plan, origin, facing_direction))
        .transpose()?
        .unwrap_or_default();
    let mut candidates = if equipment_plan.is_some() {
        equipment_geometry.iter().flatten().copied().collect()
    } else {
        area_candidates(geometry_parameters, origin, facing_direction)?
    };
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
        _ if aimed.is_some() => None,
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
        direction_available: facing.is_some() || aimed.is_some(),
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
    let mut facts = NativeCombatFacts {
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
    // Positional Tactics reads the same complete current spatial census as this plan.
    crate::spell::wheel_formula::WheelFormulaMagic::from_position(
        owned,
        runtime,
        before,
        now.get(),
    )
    .map_err(|_| SpellCastDisposition::Rejected)?
    .native_inputs(
        &mut facts.inputs,
        caster.melee_weapon,
        parameters["element"].as_str(),
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
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
            is_divine_grenade,
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
        equipment_geometry,
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
fn qualify_magnitude(
    runtime: &ChannelRuntimeV1,
    before: &PlayerSpellState,
    owned: &OwnedCastFacts,
    (actor, session): (ExactActorRef, GameSessionId),
    spell: &SpellDefinition,
    content: &NativeGameplayState,
    plan: &NativeCombatPlan,
    bindings: &[LiveActorBinding],
    players: &[(ExactActorRef, &PlayerSpellState)],
    now_ms: u64,
) -> Result<PreparedMagnitudeOwner, SpellCastDisposition> {
    let mut owner = PreparedMagnitudeOwner::qualify(
        runtime,
        before,
        actor,
        session,
        owned.magnitude(),
        plan,
        bindings,
        players,
        now_ms,
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    if let Some(weapon) = owned.intrinsic_weapon_for_spell(spell, content) {
        owner.bind_intrinsic_weapon_damage(weapon);
    }
    Ok(owner)
}

fn combat_impact(spell: &SpellDefinition, plan: &NativeCombatPlan) -> Option<&'static str> {
    match plan {
        NativeCombatPlan::Combat(combat) => monk_bond_impact(spell, combat.element),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn paid_combat(
    before: &PlayerSpellState,
    spell: &SpellDefinition,
    operational: &OperationalCastFacts,
    world: &NativeCombatFacts,
    delayed: Option<&crate::spell::native_delayed::NativeDelayedFacts>,
    equipment: Option<(&EquipmentFacts, &[BTreeSet<TilePosition>])>,
    monk_bond: Option<Element>,
    now: SemanticTimeMicros,
    draw: &mut dyn FnMut(i64, i64) -> i64,
    caster: &crate::spell::CasterState,
) -> Result<
    (
        crate::spell::cast::PaidNativeCast,
        Option<crate::spell::native_delayed::NativeDelayedPlan>,
    ),
    SpellCastDisposition,
> {
    let facts = if let Some((facts, _)) = equipment {
        Facts::Equipment(facts)
    } else {
        delayed.map_or(Facts::Combat(world), Facts::Delayed)
    };
    let (mut paid, delayed_plan) = delayed_strikes::paid(
        before,
        spell,
        operational,
        facts,
        monk_bond,
        now,
        draw,
        caster,
    )?;
    if let Some((_, geometry)) = equipment {
        let Plan::Equipment(equipment) = paid.plan else {
            return reject();
        };
        paid.plan = Plan::Combat(Box::new(monk_equipment::combat(
            &equipment, geometry, world, draw,
        )?));
    }
    Ok((paid, delayed_plan))
}

#[allow(clippy::too_many_arguments)]
fn prepare_preview(
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    content: &NativeGameplayState,
    charms: &crate::content::charm_source::CanonicalCharmCatalogue,
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
    equipment: Option<(&EquipmentFacts, &[BTreeSet<TilePosition>])>,
    monk_bond: Option<Element>,
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
    let (mut preview, delayed_plan) = paid_combat(
        before,
        spell,
        operational,
        facts,
        delayed,
        equipment,
        monk_bond,
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
        Some(qualify_magnitude(
            runtime,
            before,
            owned,
            (actor, session),
            spell,
            content,
            &preview_plan,
            bindings,
            players,
            facts.now_ms,
        )?)
    } else {
        None
    };
    if let Some(owner) = preview_magnitude.as_mut() {
        owner
            .bind_native_health_observations(runtime, owned.native_health_observations())
            .map_err(|_| SpellCastDisposition::Rejected)?;
        owner
            .select_item_spell(spell)
            .map_err(|_| SpellCastDisposition::Rejected)?;
        crate::gameplay_transport::attack::charm_combat::select_spell_critical(
            states,
            runtime,
            charms,
            owned.binding().actor,
            command,
            owner,
        )
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
    if let Some(owner) = preview_magnitude.as_mut() {
        owner
            .append_wheel_healing_echo(&mut preview_lowered)
            .map_err(|_| SpellCastDisposition::Rejected)?;
        if !matches!(
            delayed_plan.as_ref(),
            Some(crate::spell::native_delayed::NativeDelayedPlan::Cast {
                immediate: None,
                ..
            })
        ) {
            owner
                .apply_wheel_cast_state(&mut preview.next)
                .map_err(|_| SpellCastDisposition::Rejected)?;
        }
    }
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
        combat_impact(spell, &preview_plan),
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
    crate::gameplay_transport::actor_spell::stage_player_batch_with_magnitude(
        runtime,
        states,
        &preview_lowered.batch,
        Some(preview.next.clone()),
        preview_magnitude.as_ref(),
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let preview_requests = delayed_strikes::requests(
        runtime,
        &preview_lowered,
        delayed_plan.as_ref(),
        delayed,
        monk_bond,
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
    charms: &crate::content::charm_source::CanonicalCharmCatalogue,
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
    equipment: Option<(&EquipmentFacts, &[BTreeSet<TilePosition>])>,
    monk_bond: Option<Element>,
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
    let (mut paid, delayed_plan) = paid_combat(
        before,
        spell,
        operational,
        facts,
        delayed,
        equipment,
        monk_bond,
        now,
        draw,
        caster,
    )?;
    let Plan::Combat(plan) = &paid.plan else {
        return reject();
    };
    let mut magnitude = if needs_magnitude {
        Some(qualify_magnitude(
            runtime,
            before,
            owned,
            (actor, session),
            spell,
            content,
            plan,
            bindings,
            players,
            facts.now_ms,
        )?)
    } else {
        None
    };
    if let Some(owner) = magnitude.as_mut() {
        owner
            .bind_native_health_observations(runtime, owned.native_health_observations())
            .map_err(|_| SpellCastDisposition::Rejected)?;
        owner
            .select_item_spell(spell)
            .map_err(|_| SpellCastDisposition::Rejected)?;
        crate::gameplay_transport::attack::charm_combat::select_spell_critical(
            states,
            runtime,
            charms,
            owned.binding().actor,
            command,
            owner,
        )
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
    if let Some(owner) = magnitude.as_mut() {
        owner
            .append_wheel_healing_echo(&mut lowered)
            .map_err(|_| SpellCastDisposition::Rejected)?;
        if !matches!(
            delayed_plan.as_ref(),
            Some(crate::spell::native_delayed::NativeDelayedPlan::Cast {
                immediate: None,
                ..
            })
        ) {
            owner
                .apply_wheel_cast_state(&mut paid.next)
                .map_err(|_| SpellCastDisposition::Rejected)?;
        }
    }
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
        combat_impact(spell, final_plan),
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
        runtime,
        &lowered,
        delayed_plan.as_ref(),
        delayed,
        monk_bond,
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
    charms: &crate::content::charm_source::CanonicalCharmCatalogue,
    owned: &OwnedCastFacts,
    book: &SpellBook,
    saved: &crate::spell::delayed_execution::SavedStrike,
    now: SemanticTimeMicros,
    talisman: &crate::durability::task_board::BountyTalismanSnapshot,
    decision: oteryn_simulation_determinism::DecisionOccurrenceId,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<Option<crate::spell::delayed_execution::ResolvedStrike>, SpellCastDisposition> {
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
        None,
        parameters,
        None,
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
    // A current planner refusal retires the pending strike. A valid empty area
    // remains a real Combat: Crystal combat.cpp:1525-1527 consumes Focus before
    // enumerating victims, and its presentation/receipt still belong to this due.
    let Ok(Plan::Delayed(plan)) = saved.binding.spell.plan(Facts::Delayed(facts), draw) else {
        return Ok(None);
    };
    let crate::spell::native_delayed::NativeDelayedPlan::Occurrence { strike, .. } = plan.as_ref()
    else {
        return reject();
    };
    let Some(strike) = strike.as_ref() else {
        return Ok(None);
    };
    let plan = delayed_strikes::combat(Some(strike), None, saved.monk_bond);
    let players: Vec<_> = states
        .actors
        .iter()
        .filter(|(a, s, _)| states.get(runtime, *a, *s).is_some())
        .map(|(a, _, state)| (*a, state))
        .collect();
    let mut magnitude = qualify_magnitude(
        runtime,
        &before,
        owned,
        (
            saved.binding.caster,
            saved.binding.command.game_session_id(),
        ),
        spell,
        content,
        &plan,
        &spatial.bindings,
        &players,
        now.get() / 1000,
    )?;
    magnitude
        .bind_native_health_observations(runtime, owned.native_health_observations())
        .map_err(|_| SpellCastDisposition::Rejected)?;
    magnitude
        .select_item_spell(spell)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    crate::gameplay_transport::attack::charm_combat::select_spell_critical(
        states,
        runtime,
        charms,
        saved.binding.caster,
        saved.binding.command,
        &mut magnitude,
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    if strike.modifier_timing
        == crate::spell::native_delayed::ModifierTiming::CastBuffsAndOccurrenceModifiers
    {
        magnitude
            .select_planted_strike(&saved.cast_modifiers)
            .map_err(|_| SpellCastDisposition::Rejected)?;
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
    let mut leech = item_spell_augments::leech_projection::capture(
        runtime,
        states,
        charms,
        &lowered.batch,
        item_spell_augments::leech(owned, spell, &before, now),
        Some(&magnitude),
        &oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(content.source_digest()),
        decision,
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let candidates = super::outgoing_charms::due_candidates(
        tx,
        authority,
        runtime,
        room,
        objects,
        &lowered.batch,
    )
    .await?;
    let bounty_leech =
        crate::gameplay_transport::task_board::bounty_talisman::apply_spell_bonuses_with_leech(
            runtime,
            &mut lowered.batch,
            talisman,
            &oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                content.source_digest(),
            ),
            decision,
            &mut leech,
            (states, Some(&mut magnitude)),
        )
        .ok_or(SpellCastDisposition::Rejected)?;
    let cues = delayed_strikes::strike_cues(
        room,
        runtime,
        Some(strike),
        &spatial.bindings,
        saved
            .monk_bond
            .and_then(|element| monk_bond_impact(spell, element)),
    )?;
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
        augments: crate::spell::delayed_execution::DueSpellAugments {
            leech,
            candidates,
            talisman: talisman.clone(),
            children: Default::default(),
        },
        batch: lowered.batch,
        bounty_leech,
        presentation: std::sync::Arc::new(presentation),
        magnitude: std::sync::Arc::new(magnitude),
        owned: owned.clone(),
    }))
}
