//! Current tile-item plans use the existing retained caster transaction. The
//! current Item owner supplies identity, revision, source tags and real custody;
//! removal and temporary item display never turn numeric IDs into authority.
use super::*;
use crate::content::{ReferenceItemField as F, ReferenceItemType};
use crate::durability::spell_item_transaction as item_tx;
use crate::durability::spell_items_abi::SpellItemOperation;
use crate::gameplay_transport::spell_presentations::{
    CueTarget, LocatedCueRequest, QualifiedCueTile,
};
use crate::spell::combat_batch::{OwnerCombatChange, OwnerCombatEffect};
use crate::spell::native_items::{ItemOperation, ItemWorldSnapshot, NativeItemRef, WorldItemFacts};

pub(super) fn applicable(spell: &SpellDefinition) -> bool {
    matches!(&spell.execution, Execution::NativeProfile(p) if p.spell()["execution"]["native_behavior"]["key"]=="tile_item_operation" || is_barrier(p))
}
fn is_barrier(profile: &crate::spell::native::CompiledNativeSpell) -> bool {
    profile.spell()["execution"]
        .get("native_behavior")
        .is_none()
        && profile.dependencies()["effects"]
            .as_array()
            .is_some_and(|effects| {
                effects.len() == 1
                    && effects[0]["operation"] == "create_item"
                    && effects[0]["duration_selection"] == "uniform_integer_seconds"
                    && effects[0].get("pvp_safe_item").is_some()
            })
}
fn item_fact(
    content: &NativeGameplayState,
    row: &crate::durability::spell_items_abi::DurableTileItem,
    tags: (bool, bool),
) -> Result<(WorldItemFacts, bool), SpellCastDisposition> {
    let policy = content
        .item_policy(&row.definition.production_key, &row.definition.revision_ref)
        .ok_or(SpellCastDisposition::NotAvailable)?;
    let record = policy.record();
    let id = record
        .production_binding
        .external_id
        .parse::<u32>()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    if id == 0 {
        return reject();
    }
    let F::Known(physical) = &record.semantics.physical else {
        return Err(SpellCastDisposition::NotAvailable);
    };
    let F::Known(movable) = physical.movable else {
        return Err(SpellCastDisposition::NotAvailable);
    };
    let F::Known(class) = &record.semantics.classification else {
        return Err(SpellCastDisposition::NotAvailable);
    };
    let field = match class.item_type {
        F::Known(t) => t == ReferenceItemType::MagicField,
        F::NotApplicable => false,
        _ => return Err(SpellCastDisposition::NotAvailable),
    };
    Ok((
        WorldItemFacts {
            instance_key: row
                .item_instance_id
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect(),
            item: NativeItemRef {
                key: record.authoring.item.key.clone(),
                revision: record.authoring.item.revision.clone(),
            },
            movable,
            script_tagged: tags.0,
            action_tagged: tags.1,
        },
        field,
    ))
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn prepare(
    tx: &mut Transaction<'_, Postgres>,
    _root: &crate::durability::DurabilityRoot,
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
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
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
    if rune.and_then(|r| r.target_item).is_some() && intent.target == SpellTarget::None {
        return prepare_carried(
            tx,
            authority,
            runtime,
            states,
            room,
            objects,
            content,
            owned,
            spell,
            intent,
            command,
            occurrence,
            training_occurrence,
            now,
            rune,
            draw,
        )
        .await;
    }
    if is_barrier(profile) {
        return prepare_barrier(
            tx,
            _root,
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
            draw,
        )
        .await;
    }

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
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let source = runtime
        .read_actor_position(b.actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let origin = tile(source.position());
    let target = match intent.target {
        SpellTarget::Position(p) => TilePosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        },
        SpellTarget::AttackTarget => return Err(SpellCastDisposition::TargetIllegal),
        _ => return Err(SpellCastDisposition::TargetRequired),
    };
    let mut tiles = BTreeMap::new();
    tiles.insert(
        origin,
        read_tile(tx, authority, room, runtime, objects, origin).await?,
    );
    for (p, _) in sight_steps(origin, target)? {
        if !tiles.contains_key(&p) {
            tiles.insert(
                p,
                read_tile(tx, authority, room, runtime, objects, p).await?,
            );
        }
    }
    if !tiles.contains_key(&target) {
        tiles.insert(
            target,
            read_tile(tx, authority, room, runtime, objects, target).await?,
        );
    }
    let target_flags = *tiles.get(&target).ok_or(SpellCastDisposition::Rejected)?;
    let caster_flags = *tiles.get(&origin).ok_or(SpellCastDisposition::Rejected)?;
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
        direction_available: source.facing().is_some(),
        wheel_unlocked: None,
        in_protection_zone: caster_flags.protection,
        target_tile_solid: Some(target_flags.solid),
        target_tile_creature: Some(occupied),
    };
    // All common header/caster guards precede source plan or real randomness.
    let paid = crate::spell::cast::prepare_native_callback_owner_cast_with_caster(
        &before,
        spell,
        &operational,
        now,
        &caster,
    )?;
    let address = SpellGroundTarget::for_native_tile_read(room, runtime, cell(target))
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let current = item_tx::read_spell_tile_in_transaction(tx, authority, &address)
        .await
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    if let Some(selected) = rune.and_then(|r| r.target_item) {
        if !current.items.first().is_some_and(|row| {
            row.item_instance_id == selected.item_instance
                && row.state_revision == selected.expected_state_revision
        }) {
            return Err(SpellCastDisposition::TargetIllegal);
        }
    }
    let mut facts = ItemWorldSnapshot {
        caster_in_pz: caster_flags.protection,
        tile_exists: target_flags.present,
        ..Default::default()
    };
    for row in current.items.iter().take(500) {
        let tags = item_tx::current_tile_protection_tags_in_transaction(tx, authority, row)
            .await
            .map_err(|_| SpellCastDisposition::NotAvailable)?;
        let (fact, field) = item_fact(content, row, tags)?;
        if facts.first_magic_field.is_none() && field {
            facts.first_magic_field = Some(fact.clone());
        }
        facts.tile_items.push(fact);
    }
    facts.top_item = facts.tile_items.first().cloned();
    let Plan::Item(plan) = profile
        .plan(Facts::Item(&facts), draw)
        .map_err(|_| SpellCastDisposition::TargetIllegal)?
    else {
        return reject();
    };
    let mut operations = Vec::new();
    let mut effects = Vec::new();
    match &plan.operation {
        ItemOperation::Disintegrate {
            remove_instances, ..
        } => {
            for key in remove_instances {
                let row = current
                    .items
                    .iter()
                    .find(|r| {
                        r.item_instance_id
                            .iter()
                            .map(|b| format!("{b:02x}"))
                            .collect::<String>()
                            == *key
                    })
                    .ok_or(SpellCastDisposition::Rejected)?;
                operations.push(SpellItemOperation::RemoveGround(
                    item_tx::prepare_ground_removal_in_transaction(
                        tx,
                        authority,
                        &address,
                        row.item_instance_id,
                    )
                    .await
                    .map_err(|_| SpellCastDisposition::Rejected)?,
                ));
            }
        }
        ItemOperation::RemoveField { remove_instance } => {
            let row = current
                .items
                .iter()
                .find(|r| {
                    r.item_instance_id
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                        == *remove_instance
                })
                .ok_or(SpellCastDisposition::Rejected)?;
            operations.push(SpellItemOperation::RemoveGround(
                item_tx::prepare_ground_removal_in_transaction(
                    tx,
                    authority,
                    &address,
                    row.item_instance_id,
                )
                .await
                .map_err(|_| SpellCastDisposition::Rejected)?,
            ));
        }
        ItemOperation::MimicItem {
            item, duration_ms, ..
        } => {
            let policy = content
                .item_policy(&item.key, &item.revision)
                .ok_or(SpellCastDisposition::NotAvailable)?;
            let definition = &policy.record().production_definition;
            let condition = crate::ability::condition::ConditionDefinition::new(
                "spell.chameleon",
                0,
                crate::ability::condition::ConditionValues::ItemOutfit {
                    duration_ms: *duration_ms,
                },
            )
            .and_then(|d| {
                d.with_item_appearance(
                    &definition.production_key,
                    &definition.revision_ref,
                    runtime.content_pin().server_artifact_digest(),
                )
            })
            .ok_or(SpellCastDisposition::Rejected)?;
            let expected = before.owned_conditions().clone();
            let mut next = expected.clone();
            let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
                content.source_digest(),
            );
            let decision = oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(nonce(
                b"oteryn:native-item-condition:v1",
                b.actor,
                b.session,
                command.command_id().get(),
                &[],
            ));
            let application = crate::ability::condition::ApplicationFacts {
                now: now.get(),
                base_speed: u16::try_from(before.owned_base_speed())
                    .map_err(|_| SpellCastDisposition::Rejected)?,
                mana_shield_capacity: 0,
                target_reentry_protected: runtime
                    .current_player_reentry_protection(b.actor, b.session, now.get())
                    .map_err(|_| SpellCastDisposition::Rejected)?,
                source_reentry_protected: runtime
                    .current_player_reentry_protection(b.actor, b.session, now.get())
                    .map_err(|_| SpellCastDisposition::Rejected)?,
                target_is_player: true,
                decision_root: &root,
                occurrence: decision,
            };
            next.apply(
                &condition,
                Some(crate::spell::combat_execution::actor_atom(b.actor)),
                crate::ability::condition::ConditionSourceKind::SelfUse,
                &[],
                &application,
            )
            .map_err(|_| SpellCastDisposition::Rejected)?;
            effects.push(OwnerCombatEffect {
                target: b.actor,
                sub_ordinal: 0,
                change: OwnerCombatChange::PlayerConditions {
                    expected: Box::new(expected),
                    next: Box::new(next),
                },
            });
        }
    }
    let mut batch=OwnerCombatBatch{caster:b.actor,attacker:CharacterId::decode(&b.character).map_err(|_|SpellCastDisposition::Rejected)?,current_lease_generation:b.lease_generation,command,occurrence:occurrence.clone().into(),anchor:Some(paid.anchor.clone()),now_ms:now.get()/1000,effects,deferred:None,
        binding:serde_json::to_vec(&json!({"intent":source_cast_intent(intent,rune),"source_definition":format!("{profile:?}"),"source_tile_items":format!("{current:?}"),"source_item_facts":format!("{facts:?}"),"source_item_plan":format!("{plan:?}"),"item_operations":format!("{operations:?}")})).map_err(|_|SpellCastDisposition::Rejected)?};
    let mut cues = Vec::new();
    let cue_target = match plan.success_presentation.position {
        crate::spell::native_items::EffectPosition::Caster => CueTarget::Actor(b.actor),
        crate::spell::native_items::EffectPosition::TargetTile => {
            CueTarget::Tile(cue_tile(tx, authority, room, runtime, objects, target).await?)
        }
    };
    cues.push(LocatedCueRequest {
        binding: plan.success_presentation.asset_binding.clone(),
        target: cue_target,
    });
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
    Ok(PreparedNativeCombatCast {
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
    })
}

#[allow(clippy::too_many_arguments)]
async fn prepare_barrier(
    tx: &mut Transaction<'_, Postgres>,
    root: &crate::durability::DurabilityRoot,
    authority: &SpellItemAuthority,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    _book: &SpellBook,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    command: CommandRef,
    occurrence: AbilityOccurrence,
    training_occurrence: BuildOccurrence,
    now: SemanticTimeMicros,
    rune: Option<&ItemSpellCastIntent>,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    use sqlx::Row;
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
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let source = runtime
        .read_actor_position(b.actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let origin = tile(source.position());
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
    for (p, _) in sight_steps(origin, target)? {
        if !tiles.contains_key(&p) {
            tiles.insert(
                p,
                read_tile(tx, authority, room, runtime, objects, p).await?,
            );
        }
    }
    if !tiles.contains_key(&target) {
        tiles.insert(
            target,
            read_tile(tx, authority, room, runtime, objects, target).await?,
        );
    }
    let flags = *tiles.get(&target).ok_or(SpellCastDisposition::Rejected)?;
    let caster_flags = *tiles.get(&origin).ok_or(SpellCastDisposition::Rejected)?;
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
        direction_available: source.facing().is_some(),
        wheel_unlocked: None,
        in_protection_zone: caster_flags.protection,
        target_tile_solid: Some(flags.solid),
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
    let current = item_tx::read_spell_tile_in_transaction(tx, authority, &address)
        .await
        .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let world = crate::durability::spell_field_policy::read_world_field_policy_in_transaction(
        tx,
        root,
        authority.runtime_scope(),
        authority.scope_generation(),
    )
    .await
    .map_err(|_| SpellCastDisposition::NotAvailable)?
    .ok_or(SpellCastDisposition::NotAvailable)?;
    let (optional_pvp, world_revision) = {
        let read = world.bind(&*tx);
        (
            read.mode() == crate::durability::spell_field_policy::FieldWorldType::NoPvp,
            read.revision().to_owned(),
        )
    };
    let name: String = sqlx::query(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR SHARE",
    )
    .bind(authority.character_id_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| SpellCastDisposition::Rejected)?
    .try_get("name")
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let effect = &profile.dependencies()["effects"][0];
    let minimum = effect["duration_range_ms"]["minimum"]
        .as_u64()
        .and_then(|v| u32::try_from(v / 1000).ok())
        .ok_or(SpellCastDisposition::Rejected)?;
    let maximum = effect["duration_range_ms"]["maximum"]
        .as_u64()
        .and_then(|v| u32::try_from(v / 1000).ok())
        .ok_or(SpellCastDisposition::Rejected)?;
    let facts = crate::spell::native_items::BarrierTileFacts {
        exists: flags.present,
        floor_change: flags.floor_change,
        creature_on_tile: occupied,
    };
    let Plan::Barrier(preview) = profile
        .plan(
            Facts::Barrier {
                tile: &facts,
                optional_pvp,
                sample_seconds: minimum,
                caster_name: &name,
            },
            &mut |low, _| low,
        )
        .map_err(|_| SpellCastDisposition::TargetIllegal)?
    else {
        return reject();
    };
    let policy = content
        .item_policy(&preview.item.key, &preview.item.revision)
        .ok_or(SpellCastDisposition::NotAvailable)?;
    let definition =
        crate::spell::world_items_execution::QualifiedItemDefinition::from_native_policy(policy)
            .map_err(|_| SpellCastDisposition::NotAvailable)?;
    let record = policy.record();
    let blocks_movement = record
        .attributes
        .blocks_movement
        .ok_or(SpellCastDisposition::NotAvailable)?;
    let blocks_projectile = record
        .attributes
        .blocks_projectile
        .ok_or(SpellCastDisposition::NotAvailable)?;
    if !definition.ground_destination
        || definition.stack_maximum != 1
        || record.attributes.immovable_block_solid != Some(!definition.movable && blocks_movement)
    {
        return Err(SpellCastDisposition::NotAvailable);
    }
    let mut operations = Vec::new();
    // Exact first existing field is inspected before live duration selection.
    for row in &current.items {
        let p = content
            .item_policy(&row.definition.production_key, &row.definition.revision_ref)
            .ok_or(SpellCastDisposition::NotAvailable)?;
        let F::Known(c) = &p.record().semantics.classification else {
            return Err(SpellCastDisposition::NotAvailable);
        };
        let field = match c.item_type {
            F::Known(t) => t == ReferenceItemType::MagicField,
            F::NotApplicable => false,
            _ => return Err(SpellCastDisposition::NotAvailable),
        };
        if field {
            if p.record().attributes.field_replaceable != Some(true) {
                return Err(SpellCastDisposition::TargetIllegal);
            }
            operations.push(SpellItemOperation::RemoveGround(
                item_tx::prepare_ground_removal_in_transaction(
                    tx,
                    authority,
                    &address,
                    row.item_instance_id,
                )
                .await
                .map_err(|_| SpellCastDisposition::Rejected)?,
            ));
            break;
        }
    }
    states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .reserve_before_draw(runtime, 3)
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
    let mut batch=OwnerCombatBatch{caster:b.actor,attacker:CharacterId::decode(&b.character).map_err(|_|SpellCastDisposition::Rejected)?,current_lease_generation:b.lease_generation,command,occurrence:occurrence.clone().into(),anchor:Some(paid.anchor.clone()),now_ms:now.get()/1000,effects:Vec::new(),deferred:None,binding:serde_json::to_vec(&json!({"intent":source_cast_intent(intent,rune),"source_definition":format!("{profile:?}"),"source_tile_items":format!("{current:?}"),"source_world_optional_pvp":optional_pvp,"source_caster_name":name})).map_err(|_|SpellCastDisposition::Rejected)?};
    stage_player_batch(runtime, states, &batch, Some(paid.next.clone()))?;
    runtime
        .stage_spell_batch(&batch)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let sample = u32::try_from(draw(i64::from(minimum), i64::from(maximum)))
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let Plan::Barrier(plan) = profile
        .plan(
            Facts::Barrier {
                tile: &facts,
                optional_pvp,
                sample_seconds: sample,
                caster_name: &name,
            },
            draw,
        )
        .map_err(|_| SpellCastDisposition::Rejected)?
    else {
        return reject();
    };
    operations.push(SpellItemOperation::MintGround {
        item_instance_id: nonce(
            b"oteryn:native-barrier-item:v1",
            b.actor,
            b.session,
            command.command_id().get(),
            &[],
        ),
        definition,
        quantity: plan.count,
        placement: address.placement(Vec::new()),
        lifetime_millis: Some(plan.duration_ms),
        blocks_movement,
        blocks_projectile,
        description: Some(plan.description.clone()),
    });
    let mut binding: Value =
        serde_json::from_slice(&batch.binding).map_err(|_| SpellCastDisposition::Rejected)?;
    binding["source_barrier_plan"] = json!(format!("{plan:?}"));
    binding["source_item_operations"] = json!(format!("{operations:?}"));
    batch.binding = serde_json::to_vec(&binding).map_err(|_| SpellCastDisposition::Rejected)?;
    let mut cues = vec![LocatedCueRequest {
        binding: plan.projectile_asset_binding.clone(),
        target: CueTarget::Tile(cue_tile(tx, authority, room, runtime, objects, target).await?),
    }];
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
    let presentation = states
        .presentations
        .as_mut()
        .ok_or(SpellCastDisposition::Rejected)?
        .prepare_source_definition(runtime, content, intent.spell, spell, &mut batch, cues)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let players = vec![(b.actor, b.session, before.clone())];
    Ok(PreparedNativeCombatCast {
        direct_companion: None,
        corpse_companion: None,
        field_policy_revision: Some(world_revision),
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
    })
}

#[allow(clippy::too_many_arguments)]
async fn prepare_carried(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    runtime: &mut ChannelRuntimeV1,
    states: &mut ChannelSpellStates,
    room: &QualifiedNativeEntryRoom,
    objects: &LocalObjectRuntime,
    content: &NativeGameplayState,
    owned: &OwnedCastFacts,
    spell: &SpellDefinition,
    intent: &SpellCastIntent,
    command: CommandRef,
    occurrence: AbilityOccurrence,
    training_occurrence: BuildOccurrence,
    now: SemanticTimeMicros,
    rune: Option<&ItemSpellCastIntent>,
    draw: &mut (dyn FnMut(i64, i64) -> i64 + Send),
) -> Result<PreparedNativeCombatCast, SpellCastDisposition> {
    let Execution::NativeProfile(profile) = &spell.execution else {
        return reject();
    };
    if profile.spell()["execution"]["native_behavior"]["parameters"]["operation"] != "mimic_item"
        || intent.target != SpellTarget::None
    {
        return reject();
    }
    let selected = rune
        .and_then(|r| r.target_item)
        .ok_or(SpellCastDisposition::TargetRequired)?;
    let qualify = |r: &crate::durability::item_mint::TypedDefinitionRef| {
        content
            .item_policy(&r.production_key, &r.revision_ref)
            .and_then(|p| {
                crate::spell::world_items_execution::QualifiedItemDefinition::from_native_policy(p)
                    .ok()
            })
    };
    let carried = item_tx::read_carried_spell_target_in_transaction(
        tx,
        authority,
        selected.item_instance,
        selected.expected_state_revision,
        &qualify,
    )
    .await
    .map_err(|_| SpellCastDisposition::TargetIllegal)?;
    carried
        .check_transaction(tx, authority)
        .await
        .map_err(|_| SpellCastDisposition::Rejected)?;
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
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let source = runtime
        .read_actor_position(b.actor)
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let origin = tile(source.position());
    let flags = read_tile(tx, authority, room, runtime, objects, origin).await?;
    let operational = OperationalCastFacts {
        caster_position: origin,
        target_position: None,
        target: None,
        line_of_sight_clear: None,
        direction_available: source.facing().is_some(),
        wheel_unlocked: None,
        in_protection_zone: flags.protection,
        target_tile_solid: None,
        target_tile_creature: None,
    };
    let paid = crate::spell::cast::prepare_native_callback_owner_cast_with_carried_item(
        &before,
        spell,
        &operational,
        now,
        &caster,
        &carried,
    )?;
    let policy = content
        .item_policy(
            &carried.definition().production_key,
            &carried.definition().revision_ref,
        )
        .ok_or(SpellCastDisposition::NotAvailable)?;
    let record = policy.record();
    let id = record
        .production_binding
        .external_id
        .parse::<u32>()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let F::Known(physical) = &record.semantics.physical else {
        return Err(SpellCastDisposition::NotAvailable);
    };
    let F::Known(movable) = physical.movable else {
        return Err(SpellCastDisposition::NotAvailable);
    };
    let fact = WorldItemFacts {
        instance_key: carried
            .item_instance()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
        item: NativeItemRef {
            key: record.authoring.item.key.clone(),
            revision: record.authoring.item.revision.clone(),
        },
        movable,
        script_tagged: false,
        action_tagged: false,
    };
    // Tag protection is irrelevant to Mimic, whose source contract requires
    // only current movable identity; these fields never qualify Disintegrate.
    if fact
        .item
        .numeric_id()
        .map_err(|_| SpellCastDisposition::Rejected)?
        != id
    {
        return reject();
    }
    let mut facts = ItemWorldSnapshot {
        caster_in_pz: flags.protection,
        target: crate::spell::native_items::ItemTarget::ContainerOrEquipment,
        ..Default::default()
    };
    match carried.custody() {
        crate::durability::spell_items_abi::InventoryCustody::Container { .. } => {
            facts.container_exists = true;
            facts.container_slot_item = Some(fact);
        }
        crate::durability::spell_items_abi::InventoryCustody::Equipment { .. } => {
            facts.container_exists = false;
            facts.equipment_slot_item = Some(fact);
        }
    }
    let Plan::Item(plan) = profile
        .plan(Facts::Item(&facts), draw)
        .map_err(|_| SpellCastDisposition::TargetIllegal)?
    else {
        return reject();
    };
    let ItemOperation::MimicItem {
        item, duration_ms, ..
    } = &plan.operation
    else {
        return reject();
    };
    if &item.key != &record.authoring.item.key || &item.revision != &record.authoring.item.revision
    {
        return reject();
    }
    let production = &record.production_definition;
    let definition = crate::ability::condition::ConditionDefinition::new(
        "spell.chameleon",
        0,
        crate::ability::condition::ConditionValues::ItemOutfit {
            duration_ms: *duration_ms,
        },
    )
    .and_then(|d| {
        d.with_item_appearance(
            &production.production_key,
            &production.revision_ref,
            runtime.content_pin().server_artifact_digest(),
        )
    })
    .ok_or(SpellCastDisposition::Rejected)?;
    let expected = before.owned_conditions().clone();
    let mut next = expected.clone();
    let stream =
        oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(content.source_digest());
    let decision = oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(nonce(
        b"oteryn:native-item-condition:v1",
        b.actor,
        b.session,
        command.command_id().get(),
        &[],
    ));
    let protected = runtime
        .current_player_reentry_protection(b.actor, b.session, now.get())
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let application = crate::ability::condition::ApplicationFacts {
        now: now.get(),
        base_speed: u16::try_from(before.owned_base_speed())
            .map_err(|_| SpellCastDisposition::Rejected)?,
        mana_shield_capacity: 0,
        target_reentry_protected: protected,
        source_reentry_protected: protected,
        target_is_player: true,
        decision_root: &stream,
        occurrence: decision,
    };
    next.apply(
        &definition,
        Some(crate::spell::combat_execution::actor_atom(b.actor)),
        crate::ability::condition::ConditionSourceKind::SelfUse,
        &[],
        &application,
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let mut batch=OwnerCombatBatch{caster:b.actor,attacker:CharacterId::decode(&b.character).map_err(|_|SpellCastDisposition::Rejected)?,current_lease_generation:b.lease_generation,command,occurrence:occurrence.clone().into(),anchor:Some(paid.anchor.clone()),now_ms:now.get()/1000,effects:vec![OwnerCombatEffect{target:b.actor,sub_ordinal:0,change:OwnerCombatChange::PlayerConditions{expected:Box::new(expected),next:Box::new(next)}}],deferred:None,binding:serde_json::to_vec(&json!({"intent":source_cast_intent(intent,rune),"source_definition":format!("{profile:?}"),"source_carried_item":format!("{carried:?}"),"source_item_facts":format!("{facts:?}"),"source_item_plan":format!("{plan:?}")})).map_err(|_|SpellCastDisposition::Rejected)?};
    let mut cues = vec![LocatedCueRequest {
        binding: plan.success_presentation.asset_binding.clone(),
        target: CueTarget::Actor(b.actor),
    }];
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
    let roster = runtime
        .positioned_actor_census()
        .map_err(|_| SpellCastDisposition::Rejected)?;
    let players = vec![(b.actor, b.session, before.clone())];
    Ok(PreparedNativeCombatCast {
        direct_companion: None,
        corpse_companion: None,
        field_policy_revision: None,
        carried_target: Some(CarriedTargetBinding::from_current(&carried)),
        item_operations: Vec::new(),
        item_creations: Vec::new(),
        party: None,
        before,
        paid: paid.next,
        batch,
        tile_facts: BTreeMap::from([(origin, flags)]),
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
    })
}

async fn cue_tile(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    room: &QualifiedNativeEntryRoom,
    runtime: &ChannelRuntimeV1,
    objects: &LocalObjectRuntime,
    target: TilePosition,
) -> Result<QualifiedCueTile, SpellCastDisposition> {
    let cells = room.movement_cells();
    let logical = crate::content::LogicalCell {
        x: target.x,
        y: target.y,
        z: i32::from(target.floor),
    };
    if matches!(
        cells.spell_tiles().lookup(cells.scope(), logical),
        Err(SpellTileLookupError::Absent)
    ) {
        return QualifiedCueTile::from_known_absent_cell(room, runtime, cell(target))
            .map_err(|_| SpellCastDisposition::NotAvailable);
    }
    let fact = crate::spell::world_execution::qualified_combat_tile_in_transaction(
        tx,
        authority,
        room,
        runtime,
        objects,
        cell(target),
    )
    .await
    .map_err(|_| SpellCastDisposition::NotAvailable)?;
    QualifiedCueTile::from_current_tile(room, runtime, &fact)
        .map_err(|_| SpellCastDisposition::NotAvailable)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct CarriedTargetBinding {
    instance: [u8; 16],
    revision: u64,
    definition: crate::durability::item_mint::TypedDefinitionRef,
    custody: crate::durability::spell_items_abi::InventoryCustody,
}
impl CarriedTargetBinding {
    fn from_current(target: &item_tx::CarriedSpellTarget) -> Self {
        Self {
            instance: target.item_instance(),
            revision: target.state_revision(),
            definition: target.definition().clone(),
            custody: target.custody().clone(),
        }
    }
}
pub(super) async fn carried_target_is_current(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    content: &NativeGameplayState,
    target: Option<&CarriedTargetBinding>,
) -> bool {
    let Some(target) = target else { return true };
    let qualify = |r: &crate::durability::item_mint::TypedDefinitionRef| {
        content
            .item_policy(&r.production_key, &r.revision_ref)
            .and_then(|p| {
                crate::spell::world_items_execution::QualifiedItemDefinition::from_native_policy(p)
                    .ok()
            })
    };
    match item_tx::read_carried_spell_target_in_transaction(
        tx,
        authority,
        target.instance,
        target.revision,
        &qualify,
    )
    .await
    {
        Ok(current) => CarriedTargetBinding::from_current(&current) == *target,
        Err(_) => false,
    }
}
