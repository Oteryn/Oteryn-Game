//! Animate Dead is an ordinary acquired Skeleton with its real master/corpse
//! receipt. It never creates or changes the separate Familiar snapshot owner.
//! The root supplies independently current acquisition restrictions; absence
//! of the actual skull/group producer is a refusal before physical allocation.
use super::*;
use crate::durability::spell_item_transaction as item_tx;
use crate::durability::spell_items_abi::SpellItemOperation;
use crate::spell::companion_lifecycle::{AcquisitionOwnerFacts, CompanionSpawnReservation};

pub(super) fn applicable(spell: &SpellDefinition) -> bool {
    matches!(&spell.execution,Execution::NativeProfile(profile) if profile.spell()["execution"]["native_behavior"]["key"]=="acquire_summon" && profile.spell()["execution"]["native_behavior"]["parameters"]["source"]=="corpse_tile")
}
pub(super) fn direct_applicable(spell: &SpellDefinition) -> bool {
    matches!(&spell.execution,Execution::NativeProfile(profile) if profile.spell()["execution"]["native_behavior"]["key"]=="acquire_summon" && matches!(profile.spell()["execution"]["native_behavior"]["parameters"]["source"].as_str(),Some("named_creature"|"target_creature")))
}
#[derive(Debug)]
pub(super) struct PreparedDirectCompanionState {
    pub(super) reservation: crate::spell::companion_lifecycle::DirectCompanionReservation,
    pub(super) restrictions: AcquisitionOwnerFacts,
}

/// Summon/Convince source scripts do not consume a skull predicate. Explicit
/// source N/A is distinct from a current projection that says clear skull.
pub(super) async fn read_current_direct_restrictions(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    owned: &OwnedCastFacts,
    _now: SemanticTimeMicros,
    spell: &SpellDefinition,
) -> Result<AcquisitionOwnerFacts, SpellCastDisposition> {
    let Execution::NativeProfile(profile) = &spell.execution else {
        return reject();
    };
    if !direct_applicable(spell)
        || profile.spell()["execution"]["native_behavior"]["parameters"]["refuse_black_skull"]
            != false
    {
        return reject();
    }
    let b = owned.binding();
    let before = states
        .get(runtime, b.actor, b.session)
        .ok_or(SpellCastDisposition::Rejected)?;
    let group = crate::durability::spell_familiar_group::read_current_group_in_transaction(
        tx,
        authority,
        &crate::durability::fresh_admission::FreshAdmissionStore::from_root(root.clone()),
        b.session,
    )
    .await
    .map_err(|_| SpellCastDisposition::NotAvailable)?;
    Ok(AcquisitionOwnerFacts {
        mana: u64::from(before.vitals().mana),
        has_infinite_mana: group
            .flag("HasInfiniteMana")
            .ok_or(SpellCastDisposition::NotAvailable)?,
        can_summon_all: group
            .flag("CanSummonAll")
            .ok_or(SpellCastDisposition::NotAvailable)?,
        can_convince_all: group
            .flag("CanConvinceAll")
            .ok_or(SpellCastDisposition::NotAvailable)?,
        // Planner's bit is unconsumed for these two qualified profiles. The
        // persisted source binding below explicitly records not_applicable.
        black_skull: false,
    })
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare_direct(
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
    parameter: Option<&ParameterSpellCastIntent>,
    _draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    use crate::spell::combat_batch::{OwnerCombatChange, OwnerCombatEffect};
    use crate::spell::companion_lifecycle::{
        AcquisitionSource, prepare_direct_acquisition_excluding,
    };
    if !direct_applicable(spell)
        || book.indexed(intent.spell) != Some(spell)
        || content.spell_book().indexed(intent.spell) != Some(spell)
        || intent.aim_at_target
    {
        return reject();
    }
    let Execution::NativeProfile(profile) = &spell.execution else {
        return reject();
    };
    let named =
        profile.spell()["execution"]["native_behavior"]["parameters"]["source"] == "named_creature";
    let b = owned.binding();
    let scope = runtime.binding();
    if b.session != command.game_session_id()
        || authority.command() != command
        || authority.character_id_bytes() != b.character
        || authority.compatible_content_digest() != content.source_digest()
        || b.content_digest != runtime.content_pin().server_artifact_digest()
        || authority.runtime_scope()
            != RuntimeScopeRefV1::channel(scope.world_id(), scope.channel_id())
        || authority.scope_generation() != scope.scope_generation().get()
    {
        return reject();
    }
    let restrictions =
        read_current_direct_restrictions(tx, root, authority, runtime, states, owned, now, spell)
            .await?;
    let before = states
        .get(runtime, b.actor, b.session)
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    let caster = owned
        .caster(
            &before,
            spell,
            content,
            before.owned_harmony_multiplier(spell)?,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let position = runtime
        .read_actor_position(b.actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let origin = tile(position.position());
    let roster = runtime
        .positioned_actor_census()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let mut creatures = Vec::new();
    let (source, target) = if named {
        if rune.is_some()
            || !matches!(spell.carrier, crate::spell::Carrier::Instant { .. })
            || intent.target != SpellTarget::None
        {
            return reject();
        }
        let input = parameter.ok_or(SpellCastDisposition::TargetRequired)?;
        if input.intent != *intent {
            return reject();
        }
        // Exact source name lookup is resolved only in the active Creature owner.
        let name = input
            .parameter
            .as_ref()
            .filter(|s| !s.is_empty())
            .ok_or(SpellCastDisposition::TargetRequired)?;
        (AcquisitionSource::Named(name.clone()), None)
    } else {
        if parameter.is_some()
            || rune.is_none()
            || !matches!(spell.carrier, crate::spell::Carrier::Rune { .. })
        {
            return reject();
        }
        let p = match intent.target {
            SpellTarget::Position(p) => TilePosition {
                x: p.x,
                y: p.y,
                floor: p.floor,
            },
            SpellTarget::AttackTarget => return Err(SpellCastDisposition::NotAvailable),
            _ => return Err(SpellCastDisposition::TargetRequired),
        };
        let group = crate::durability::spell_familiar_group::read_current_group_in_transaction(
            tx,
            authority,
            &crate::durability::fresh_admission::FreshAdmissionStore::from_root(root.clone()),
            b.session,
        )
        .await
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
        let sees_invisible = group.access()
            || group
                .flag("CanSenseInvisibility")
                .ok_or(SpellCastDisposition::NotAvailable)?;
        // Match the actual qualified physical creature on the requested tile.
        // A missing attack-selector owner remains unavailable rather than guessed.
        let mut visible = Vec::new();
        for (actor, pos, session) in &roster {
            if session.is_none() && tile(pos.position()) == p {
                let snapshot = runtime
                    .companion_snapshot(*actor)
                    .map_err(|_| SpellCastDisposition::TargetIllegal)?;
                if sees_invisible || !snapshot.state.conditions.invisible_at(now.get()) {
                    visible.push(snapshot);
                }
            }
        }
        let snapshot = visible
            .into_iter()
            .min_by_key(|m| m.actor.actor_local_id())
            .ok_or(SpellCastDisposition::TargetIllegal)?;
        let selected_actor = snapshot.actor;
        let target = crate::spell::target::CastTarget {
            caster: u64::from(b.actor.actor_local_id()),
            creature: u64::from(snapshot.actor.actor_local_id()),
            actor: crate::spell::combat_execution::actor_atom(snapshot.actor),
            master: snapshot
                .state
                .master
                .map(|m| u64::from(m.actor.actor_local_id())),
        };
        creatures.push(snapshot);
        (AcquisitionSource::Target(selected_actor), Some((p, target)))
    };
    let mut tiles = BTreeMap::new();
    tiles.insert(
        origin,
        read_tile(tx, authority, room, runtime, objects, origin).await?,
    );
    let mut sight = true;
    if let Some((p, _)) = &target {
        for (cell, exempt) in sight_steps(origin, *p)? {
            if !tiles.contains_key(&cell) {
                tiles.insert(
                    cell,
                    read_tile(tx, authority, room, runtime, objects, cell).await?,
                );
            }
            if !exempt && tiles[&cell].projectile {
                sight = false;
            }
        }
        if !tiles.contains_key(p) {
            tiles.insert(
                *p,
                read_tile(tx, authority, room, runtime, objects, *p).await?,
            );
        }
    }
    let target_position = target.as_ref().map(|(p, _)| *p);
    let operational = OperationalCastFacts {
        caster_position: origin,
        target_position,
        target: target.as_ref().map(|(_, t)| t.clone()),
        line_of_sight_clear: Some(sight),
        direction_available: position.facing().is_some(),
        wheel_unlocked: None,
        in_protection_zone: tiles[&origin].protection,
        target_tile_solid: target_position.map(|p| tiles[&p].solid),
        target_tile_creature: target_position
            .map(|p| roster.iter().any(|(_, v, _)| tile(v.position()) == p)),
    };
    // Dynamic Item/door blockers participate before selecting the extended
    // source position. Exclusions can only narrow the immutable map proof.
    let mut excluded = Vec::new();
    if named {
        for (dx, dy) in [
            (0, 0),
            (-1, 0),
            (0, -1),
            (1, 0),
            (0, 1),
            (-1, -1),
            (1, -1),
            (-1, 1),
            (1, 1),
        ] {
            let p = offset(origin, dx, dy)?;
            let flags = match tiles.get(&p).copied() {
                Some(flags) => Some(flags),
                None => read_tile(tx, authority, room, runtime, objects, p)
                    .await
                    .ok(),
            };
            match flags {
                Some(flags) => {
                    tiles.insert(p, flags);
                    if !flags.present || flags.solid || flags.floor_change {
                        excluded.push(cell(p));
                    }
                }
                None => excluded.push(cell(p)),
            }
        }
    }
    let reservation = prepare_direct_acquisition_excluding(
        runtime,
        room.movement_cells(),
        b.actor,
        b.session,
        crate::domain::CharacterId::from_bytes(b.character)
            .map_err(|_| SpellCastDisposition::Rejected)?,
        profile,
        source,
        &restrictions,
        &excluded,
    )
    .map_err(|_| SpellCastDisposition::TargetIllegal)?;
    if reservation.consume_rune_charge() != !named {
        return reject();
    }
    let physical = tile(reservation.position());
    if !tiles.contains_key(&physical) {
        tiles.insert(
            physical,
            read_tile(tx, authority, room, runtime, objects, physical).await?,
        );
    }
    if !tiles[&physical].present
        || (named && (tiles[&physical].solid || tiles[&physical].floor_change))
    {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    let paid = crate::spell::cast::prepare_direct_acquisition_owner_cast_with_caster(
        &before,
        spell,
        &operational,
        now,
        &caster,
        &reservation,
    )?;
    let training = before
        .prepare_direct_acquisition_training(
            &paid.next,
            &paid.anchor,
            content
                .training_formula()
                .ok_or(SpellCastDisposition::NotAvailable)?,
            training_occurrence,
            now.get(),
            &reservation,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let mut effects = Vec::new();
    if let Some(assignment) = reservation.assignment() {
        effects.push(OwnerCombatEffect {
            target: assignment.actor(),
            sub_ordinal: 0,
            change: OwnerCombatChange::CompanionMaster(Box::new(assignment.clone())),
        });
    }
    let mut source = source_original_intent(intent, rune, parameter)?;
    source["acquisition_black_skull"] = json!("not_applicable");
    let mut binding = json!({"intent":source,"source_definition":format!("{profile:?}"),"source_direct_reservation":format!("{reservation:?}"),"source_current_acquisition_restrictions":format!("{restrictions:?}")});
    if parameter.is_some() {
        binding["parameter_result"] = json!(
            encode_parameter_spell_cast_result(&ParameterSpellCastResult {
                disposition: SpellCastDisposition::Cast,
                feedback: None,
                editor: None
            })
            .map_err(|_| SpellCastDisposition::Rejected)?
        );
    }
    let mut batch = OwnerCombatBatch {
        caster: b.actor,
        attacker: CharacterId::decode(&b.character).map_err(|_| SpellCastDisposition::Rejected)?,
        current_lease_generation: b.lease_generation,
        command,
        occurrence: occurrence.into(),
        anchor: Some(paid.anchor),
        now_ms: now.get() / 1000,
        effects,
        deferred: None,
        binding: serde_json::to_vec(&binding).map_err(|_| SpellCastDisposition::Rejected)?,
    };
    use crate::gameplay_transport::spell_presentations::{
        CueTarget, LocatedCueRequest, QualifiedCueTile,
    };
    let mut cues = Vec::new();
    if let Some(p) = spell
        .authored
        .as_ref()
        .and_then(|a| a.header.presentation.as_ref())
    {
        for binding in &p.cast_cue {
            cues.push(LocatedCueRequest {
                binding: binding.clone(),
                target: CueTarget::Actor(b.actor),
            });
        }
    }
    cues.push(LocatedCueRequest {
        binding: "appearance:effect/magic_blue".into(),
        target: CueTarget::Actor(b.actor),
    });
    if named {
        let fact = crate::spell::world_execution::qualified_combat_tile_in_transaction(
            tx,
            authority,
            room,
            runtime,
            objects,
            reservation.position(),
        )
        .await
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
        let cue = QualifiedCueTile::from_current_tile(room, runtime, &fact)
            .map_err(|_| SpellCastDisposition::NotAvailable)?;
        cues.push(LocatedCueRequest {
            binding: "appearance:effect/teleport".into(),
            target: CueTarget::Tile(cue),
        });
    }
    let presentation = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .prepare_source_definition(runtime, content, intent.spell, spell, &mut batch, cues)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    stage_player_batch(runtime, states, &batch, Some(paid.next.clone()))?;
    runtime
        .stage_spell_batch(&batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    Ok(PreparedNativeCombatCast {
        direct_companion: Some(PreparedDirectCompanionState {
            reservation,
            restrictions,
        }),
        corpse_companion: None,
        field_policy_revision: None,
        carried_target: None,
        item_operations: Vec::new(),
        item_creations: Vec::new(),
        party: None,
        before: before.clone(),
        paid: paid.next,
        batch,
        tile_facts: tiles,
        roster,
        training,
        timers: None,
        magnitude: None,
        facts_binding: b.clone(),
        creatures,
        players: vec![(b.actor, b.session, before)],
        equipment: owned.equipment().clone(),
        presentation: Some(presentation),
        installation: None,
    })
}
/// The returned real reservation still needs pre-SQL physical slot reservation
/// and the matching actual common COMMIT proof before infallible installation.
#[derive(Debug)]
pub(super) struct PreparedCorpseCompanion {
    pub(super) cast: PreparedNativeCombatCast,
    pub(super) spawn: CompanionSpawnReservation,
    pub(super) restrictions: AcquisitionOwnerFacts,
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare_with_current_restrictions(
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
    current: &AcquisitionOwnerFacts,
) -> Result<PreparedCorpseCompanion, SpellCastDisposition> {
    if !applicable(spell)
        || book.indexed(intent.spell) != Some(spell)
        || content.spell_book().indexed(intent.spell) != Some(spell)
        || rune.is_none()
        || intent.aim_at_target
    {
        return reject();
    }
    let Execution::NativeProfile(profile) = &spell.execution else {
        return reject();
    };
    let b = owned.binding();
    let actual = runtime.binding();
    if authority.command() != command
        || b.session != command.game_session_id()
        || authority.character_id_bytes() != b.character
        || authority.compatible_content_digest() != content.source_digest()
        || authority.runtime_scope()
            != RuntimeScopeRefV1::channel(actual.world_id(), actual.channel_id())
        || authority.scope_generation() != actual.scope_generation().get()
    {
        return reject();
    }
    let group = crate::durability::spell_familiar_group::read_current_group_in_transaction(
        tx,
        authority,
        &crate::durability::fresh_admission::FreshAdmissionStore::from_root(root.clone()),
        b.session,
    )
    .await
    .map_err(|_| SpellCastDisposition::NotAvailable)?;
    if group.flag("HasInfiniteMana") != Some(current.has_infinite_mana)
        || group.flag("CanSummonAll") != Some(current.can_summon_all)
        || group.flag("CanConvinceAll") != Some(current.can_convince_all)
    {
        return Err(SpellCastDisposition::NotAvailable);
    }
    let before = states
        .get(runtime, b.actor, b.session)
        .ok_or(SpellCastDisposition::Rejected)?
        .clone();
    if current.mana != u64::from(before.vitals().mana) {
        return reject();
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
    let position = runtime
        .read_actor_position(b.actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let origin = tile(position.position());
    let target = match intent.target {
        SpellTarget::Position(p) => TilePosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        },
        _ => return Err(SpellCastDisposition::TargetRequired),
    };
    let mut tiles = BTreeMap::new();
    tiles.insert(
        origin,
        read_tile(tx, authority, room, runtime, objects, origin).await?,
    );
    // The corpse must be a real existing placed map cell, with the current
    // initialized map/door owners checked independently of the Item receipt.
    let target_fact = crate::spell::world_execution::qualified_combat_tile_in_transaction(
        tx,
        authority,
        room,
        runtime,
        objects,
        cell(target),
    )
    .await
    .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let target_flags = TileFlags {
        projectile: target_fact.block_projectile(),
        solid: target_fact.block_solid(),
        floor_change: target_fact.floor_change(),
        protection: target_fact.protection_zone(),
        present: target_fact.ground_present(),
    };
    if !target_flags.present {
        return Err(SpellCastDisposition::TargetIllegal);
    }
    for (p, exempt) in sight_steps(origin, target)? {
        if !exempt && p != target && !tiles.contains_key(&p) {
            tiles.insert(
                p,
                read_tile(tx, authority, room, runtime, objects, p).await?,
            );
        }
    }
    tiles.insert(target, target_flags);
    let sight = sight_steps(origin, target)?
        .iter()
        .all(|(p, exempt)| *exempt || tiles.get(p).is_some_and(|t| !t.projectile));
    let roster = runtime
        .positioned_actor_census()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let occupied = roster.iter().any(|(_, p, _)| tile(p.position()) == target);
    let operational = OperationalCastFacts {
        caster_position: origin,
        target_position: Some(target),
        target: None,
        line_of_sight_clear: Some(sight),
        direction_available: position.facing().is_some(),
        wheel_unlocked: None,
        in_protection_zone: tiles[&origin].protection,
        target_tile_solid: Some(target_flags.solid),
        target_tile_creature: Some(occupied),
    };
    let paid = crate::spell::cast::prepare_native_callback_owner_cast_with_caster(
        &before,
        spell,
        &operational,
        now,
        &caster,
    )?;
    let address = SpellGroundTarget::for_native_tile_read(room, runtime, cell(target))
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let reference = item_tx::current_top_corpse_definition_in_transaction(tx, authority, &address)
        .await
        .map_err(|_| SpellCastDisposition::TargetIllegal)?;
    let policy = content
        .item_policy(&reference.production_key, &reference.revision_ref)
        .ok_or(SpellCastDisposition::NotAvailable)?;
    let definition =
        crate::spell::world_items_execution::QualifiedItemDefinition::from_native_policy(policy)
            .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let corpse = item_tx::prepare_corpse_in_transaction(tx, authority, &address, &definition)
        .await
        .map_err(|_| SpellCastDisposition::TargetIllegal)?;
    let spawn = crate::spell::companion_lifecycle::prepare_animate_dead(
        runtime,
        room.movement_cells(),
        b.actor,
        b.session,
        crate::domain::CharacterId::from_bytes(b.character)
            .map_err(|_| SpellCastDisposition::Rejected)?,
        profile,
        &corpse,
        current,
    )
    .map_err(|_| SpellCastDisposition::TargetIllegal)?;
    if !spawn.consume_rune_charge() {
        return reject();
    }
    let mut operations = corpse
        .contents
        .iter()
        .cloned()
        .map(SpellItemOperation::RetireContained)
        .collect::<Vec<_>>();
    operations.push(SpellItemOperation::ConsumeCorpse(corpse));
    let mut batch=OwnerCombatBatch{caster:b.actor,attacker:CharacterId::decode(&b.character).map_err(|_|SpellCastDisposition::Rejected)?,current_lease_generation:b.lease_generation,command,occurrence:occurrence.clone().into(),anchor:Some(paid.anchor.clone()),now_ms:now.get()/1000,effects:Vec::new(),deferred:None,binding:serde_json::to_vec(&json!({"intent":source_cast_intent(intent,rune),"source_definition":format!("{profile:?}"),"source_corpse_operations":format!("{operations:?}"),"source_prepared_spawn":format!("{spawn:?}"),"source_current_acquisition_restrictions":format!("{current:?}"),"source_current_group_revision":group.revision()})).map_err(|_|SpellCastDisposition::Rejected)?};
    let mut cues = Vec::new();
    if let Some(p) = spell
        .authored
        .as_ref()
        .and_then(|a| a.header.presentation.as_ref())
    {
        for binding in &p.cast_cue {
            cues.push(
                crate::gameplay_transport::spell_presentations::LocatedCueRequest {
                    binding: binding.clone(),
                    target: crate::gameplay_transport::spell_presentations::CueTarget::Actor(
                        b.actor,
                    ),
                },
            );
        }
    }
    cues.push(crate::gameplay_transport::spell_presentations::LocatedCueRequest {
        binding: "appearance:effect/magic_blue".into(),
        target: crate::gameplay_transport::spell_presentations::CueTarget::Tile(
            crate::gameplay_transport::spell_presentations::QualifiedCueTile::from_current_tile(room,runtime,&target_fact)
                .map_err(|_|SpellCastDisposition::NotAvailable)?,
        ),
    });
    let presentation = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .prepare_source_definition(runtime, content, intent.spell, spell, &mut batch, cues)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let training = before
        .prepare_paid_training(
            &paid.next,
            &paid.anchor,
            content
                .training_formula()
                .ok_or(SpellCastDisposition::Rejected)?,
            training_occurrence,
            now.get(),
        )
        .map_err(|_| SpellCastDisposition::Rejected)?;
    stage_player_batch(runtime, states, &batch, Some(paid.next.clone()))?;
    runtime
        .stage_spell_batch(&batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let players = vec![(b.actor, b.session, before.clone())];
    let cast = PreparedNativeCombatCast {
        direct_companion: None,
        corpse_companion: None,
        field_policy_revision: None,
        carried_target: None,
        item_operations: operations,
        item_creations: Vec::new(),
        party: None,
        before,
        paid: paid.next,
        batch,
        tile_facts: tiles,
        roster,
        training,
        timers: None,
        magnitude: None,
        facts_binding: b.clone(),
        creatures: Vec::new(),
        players,
        equipment: owned.equipment().clone(),
        presentation: Some(presentation),
        installation: None,
    };
    Ok(PreparedCorpseCompanion {
        cast,
        spawn,
        restrictions: current.clone(),
    })
}

#[derive(Debug)]
pub(super) struct PreparedCorpseCompanionState {
    pub(super) spawn: CompanionSpawnReservation,
    pub(super) restrictions: AcquisitionOwnerFacts,
}
#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare(
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
    _draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    let restrictions =
        read_current_restrictions(tx, root, authority, runtime, states, owned, now).await?;
    let PreparedCorpseCompanion {
        mut cast,
        spawn,
        restrictions,
    } = prepare_with_current_restrictions(
        tx,
        root,
        authority,
        runtime,
        states,
        room,
        objects,
        content,
        owned,
        book,
        spell,
        intent,
        command,
        occurrence,
        training_occurrence,
        now,
        rune,
        &restrictions,
    )
    .await?;
    cast.corpse_companion = Some(PreparedCorpseCompanionState {
        spawn,
        restrictions,
    });
    Ok(cast)
}
pub(super) async fn read_current_restrictions(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
    owned: &OwnedCastFacts,
    now: SemanticTimeMicros,
) -> Result<AcquisitionOwnerFacts, SpellCastDisposition> {
    let b = owned.binding();
    let before = states
        .get(runtime, b.actor, b.session)
        .ok_or(SpellCastDisposition::Rejected)?;
    let group = crate::durability::spell_familiar_group::read_current_group_in_transaction(
        tx,
        authority,
        &crate::durability::fresh_admission::FreshAdmissionStore::from_root(root.clone()),
        b.session,
    )
    .await
    .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let black = qualify_current_black_skull(owned.current_black_skull(now.get()), b, now.get())?;
    Ok(AcquisitionOwnerFacts {
        mana: u64::from(before.vitals().mana),
        has_infinite_mana: group
            .flag("HasInfiniteMana")
            .ok_or(SpellCastDisposition::NotAvailable)?,
        can_summon_all: group
            .flag("CanSummonAll")
            .ok_or(SpellCastDisposition::NotAvailable)?,
        can_convince_all: group
            .flag("CanConvinceAll")
            .ok_or(SpellCastDisposition::NotAvailable)?,
        black_skull: black,
    })
}

fn qualify_current_black_skull(
    projection: Option<&crate::spell::owned_cast_facts::CurrentProjection<bool>>,
    binding: &crate::spell::owned_cast_facts::CastFactsBinding,
    now: u64,
) -> Result<bool, SpellCastDisposition> {
    projection
        .filter(|p| p.current(binding, now))
        .map(|p| p.value)
        .ok_or(SpellCastDisposition::NotAvailable)
}

#[cfg(test)]
mod restriction_authority_tests {
    use super::*;
    use crate::spell::owned_cast_facts::{CastFactsBinding, CurrentProjection};

    fn current_binding() -> CastFactsBinding {
        let (_, actor, session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(93);
        CastFactsBinding {
            actor,
            session,
            character: [7; 16],
            character_revision: 4,
            lease_generation: 1,
            connection_generation: 1,
            player_revision: 3,
            content_digest: [9; 32],
            equipment_revision: 2,
        }
    }

    #[test]
    fn missing_expired_or_unissued_skull_projection_is_unavailable() {
        let binding = current_binding();
        assert_eq!(
            qualify_current_black_skull(None, &binding, 99),
            Err(SpellCastDisposition::NotAvailable)
        );
        let mut projection = CurrentProjection {
            binding: binding.clone(),
            authority_revision: 7,
            valid_until_micros: 100,
            value: false,
        };
        assert_eq!(
            qualify_current_black_skull(Some(&projection), &binding, 99),
            Ok(false)
        );
        assert_eq!(
            qualify_current_black_skull(Some(&projection), &binding, 100),
            Err(SpellCastDisposition::NotAvailable)
        );
        projection.authority_revision = 0;
        assert_eq!(
            qualify_current_black_skull(Some(&projection), &binding, 99),
            Err(SpellCastDisposition::NotAvailable)
        );
    }

    #[test]
    fn skull_projection_cannot_substitute_any_caster_owner_binding() {
        let binding = current_binding();
        let projection = CurrentProjection {
            binding: binding.clone(),
            authority_revision: 7,
            valid_until_micros: 100,
            value: true,
        };
        assert_eq!(
            qualify_current_black_skull(Some(&projection), &binding, 99),
            Ok(true)
        );
        let (mut other_runtime, _, other_session) =
            crate::gameplay_transport::actor_spell::tests::runtime_with_player(94);
        let reservation = other_runtime
            .reserve_fresh_session(binding.session)
            .expect("real second slot");
        let other_actor = other_runtime
            .commit_fresh_session(reservation)
            .expect("real second actor");
        for field in 0..9 {
            let mut actual = binding.clone();
            match field {
                0 => actual.actor = other_actor,
                1 => actual.session = other_session,
                2 => actual.character = [8; 16],
                3 => actual.character_revision += 1,
                4 => actual.lease_generation += 1,
                5 => actual.connection_generation += 1,
                6 => actual.player_revision += 1,
                7 => actual.content_digest = [8; 32],
                _ => actual.equipment_revision += 1,
            }
            assert_eq!(
                qualify_current_black_skull(Some(&projection), &actual, 99),
                Err(SpellCastDisposition::NotAvailable),
                "substituted field {field}"
            );
        }
    }
}
