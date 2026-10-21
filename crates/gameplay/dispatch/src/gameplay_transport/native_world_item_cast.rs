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
use crate::spell::native::CompiledNativeSpellPlanExt;
use oteryn_gameplay_spell_rules::native_items::{
    ItemOperation, ItemWorldSnapshot, NativeItemRef, WorldItemFacts,
};

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
        if let std::collections::btree_map::Entry::Vacant(e) = tiles.entry(p) {
            e.insert(read_tile(tx, authority, room, runtime, objects, p).await?);
        }
    }
    if let std::collections::btree_map::Entry::Vacant(e) = tiles.entry(target) {
        e.insert(read_tile(tx, authority, room, runtime, objects, target).await?);
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
    if let Some(selected) = rune.and_then(|r| r.target_item)
        && !current.items.first().is_some_and(|row| {
            row.item_instance_id == selected.item_instance
                && row.state_revision == selected.expected_state_revision
        })
    {
        return Err(SpellCastDisposition::TargetIllegal);
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
            let condition = oteryn_gameplay_foundation::foundation::condition::ConditionDefinition::new(
                "spell.chameleon",
                0,
                oteryn_gameplay_foundation::foundation::condition::ConditionValues::ItemOutfit {
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
            let application = oteryn_gameplay_foundation::foundation::condition::ApplicationFacts {
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
                oteryn_gameplay_foundation::foundation::condition::ConditionSourceKind::SelfUse,
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
    let mut batch=OwnerCombatBatch{caster:b.actor,attacker:CharacterId::decode(&b.character).map_err(|_|SpellCastDisposition::Rejected)?,current_lease_generation:b.lease_generation,command,occurrence:occurrence.clone().into(),anchor:Some(paid.anchor.clone()),now_us:Some(now),now_ms:now.get()/1000,effects,deferred:None,
        binding:serde_json::to_vec(&json!({"intent":source_cast_intent(intent,rune),"source_definition":format!("{profile:?}"),"source_tile_items":format!("{current:?}"),"source_item_facts":format!("{facts:?}"),"source_item_plan":format!("{plan:?}"),"item_operations":format!("{operations:?}")})).map_err(|_|SpellCastDisposition::Rejected)?};
    let mut cues = Vec::new();
    let cue_target = match plan.success_presentation.position {
        oteryn_gameplay_spell_rules::native_items::EffectPosition::Caster => {
            CueTarget::Actor(b.actor)
        }
        oteryn_gameplay_spell_rules::native_items::EffectPosition::TargetTile => {
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
        && let Some(binding) = &p.cast_cue
    {
        cues.push(LocatedCueRequest {
            binding: binding.clone(),
            target: CueTarget::Actor(b.actor),
        });
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
        if let std::collections::btree_map::Entry::Vacant(e) = tiles.entry(p) {
            e.insert(read_tile(tx, authority, room, runtime, objects, p).await?);
        }
    }
    if let std::collections::btree_map::Entry::Vacant(e) = tiles.entry(target) {
        e.insert(read_tile(tx, authority, room, runtime, objects, target).await?);
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
    let name =
        crate::durability::character_chat_name::read_character_name_for_share_in_transaction(
            tx,
            &authority.character_id_bytes(),
        )
        .await
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
    let facts = oteryn_gameplay_spell_rules::native_items::BarrierTileFacts {
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
        oteryn_gameplay_durability::durability::native_world_items::QualifiedItemDefinition::from_native_policy(policy)
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
    let mut batch=OwnerCombatBatch{caster:b.actor,attacker:CharacterId::decode(&b.character).map_err(|_|SpellCastDisposition::Rejected)?,current_lease_generation:b.lease_generation,command,occurrence:occurrence.clone().into(),anchor:Some(paid.anchor.clone()),now_us:Some(now),now_ms:now.get()/1000,effects:Vec::new(),deferred:None,binding:serde_json::to_vec(&json!({"intent":source_cast_intent(intent,rune),"source_definition":format!("{profile:?}"),"source_tile_items":format!("{current:?}"),"source_world_optional_pvp":optional_pvp,"source_caster_name":name})).map_err(|_|SpellCastDisposition::Rejected)?};
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
        && let Some(binding) = &p.cast_cue
    {
        cues.push(LocatedCueRequest {
            binding: binding.clone(),
            target: CueTarget::Actor(b.actor),
        });
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
                oteryn_gameplay_durability::durability::native_world_items::QualifiedItemDefinition::from_native_policy(p)
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
        target: oteryn_gameplay_spell_rules::native_items::ItemTarget::ContainerOrEquipment,
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
    if item.key != record.authoring.item.key || item.revision != record.authoring.item.revision {
        return reject();
    }
    let production = &record.production_definition;
    let definition = oteryn_gameplay_foundation::foundation::condition::ConditionDefinition::new(
        "spell.chameleon",
        0,
        oteryn_gameplay_foundation::foundation::condition::ConditionValues::ItemOutfit {
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
    let application = oteryn_gameplay_foundation::foundation::condition::ApplicationFacts {
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
        oteryn_gameplay_foundation::foundation::condition::ConditionSourceKind::SelfUse,
        &[],
        &application,
    )
    .map_err(|_| SpellCastDisposition::Rejected)?;
    let mut batch=OwnerCombatBatch{caster:b.actor,attacker:CharacterId::decode(&b.character).map_err(|_|SpellCastDisposition::Rejected)?,current_lease_generation:b.lease_generation,command,occurrence:occurrence.clone().into(),anchor:Some(paid.anchor.clone()),now_us:Some(now),now_ms:now.get()/1000,effects:vec![OwnerCombatEffect{target:b.actor,sub_ordinal:0,change:OwnerCombatChange::PlayerConditions{expected:Box::new(expected),next:Box::new(next)}}],deferred:None,binding:serde_json::to_vec(&json!({"intent":source_cast_intent(intent,rune),"source_definition":format!("{profile:?}"),"source_carried_item":format!("{carried:?}"),"source_item_facts":format!("{facts:?}"),"source_item_plan":format!("{plan:?}")})).map_err(|_|SpellCastDisposition::Rejected)?};
    let mut cues = vec![LocatedCueRequest {
        binding: plan.success_presentation.asset_binding.clone(),
        target: CueTarget::Actor(b.actor),
    }];
    if let Some(p) = spell
        .authored
        .as_ref()
        .and_then(|a| a.header.presentation.as_ref())
        && let Some(binding) = &p.cast_cue
    {
        cues.push(LocatedCueRequest {
            binding: binding.clone(),
            target: CueTarget::Actor(b.actor),
        });
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
                oteryn_gameplay_durability::durability::native_world_items::QualifiedItemDefinition::from_native_policy(p)
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

#[cfg(test)]
mod tests {
    use crate::bestiary_postgres_harness::{self as harness, TestResult, debug, id};
    use crate::durability::admission_journal::party_target_binding::read_connection_generation_for_share_in_transaction;
    use crate::durability::character_chat_name::read_character_name_for_share_in_transaction;
    use crate::durability::character_familiar::read_familiar_unix_seconds_in_transaction;
    use crate::foundation::GameSessionId;

    #[test]
    fn retained_sql_reads_postgres_preserve_values_missing_rows_and_share_locks() -> TestResult {
        let admin = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL")?;
        harness::runtime()?.block_on(async move {
            use sqlx::Connection;
            let options: sqlx::postgres::PgConnectOptions = admin.parse()?;
            if options.get_host() != "127.0.0.1"
                || ![5432, 55432].contains(&options.get_port())
                || options.get_database() != Some("postgres")
            {
                return Err("SQL read regression requires the local test PostgreSQL admin".into());
            }
            let name = format!(
                "dispatch_sql_reads_{}_{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_nanos(),
            );
            let mut admin = sqlx::PgConnection::connect_with(&options).await?;
            sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {name}")))
                .execute(&mut admin)
                .await?;
            let result = run_in_test_database(options.database(&name)).await;
            sqlx::query(sqlx::AssertSqlSafe(format!(
                "DROP DATABASE {name} WITH (FORCE)"
            )))
            .execute(&mut admin)
            .await?;
            result
        })
    }

    async fn run_in_test_database(options: sqlx::postgres::PgConnectOptions) -> TestResult {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(3)
            .connect_with(options)
            .await?;
        let result = async {
            let version: String = sqlx::query_scalar("SHOW server_version_num")
                .fetch_one(&pool)
                .await?;
            assert_eq!(version, "170006", "canonical target is PostgreSQL 17.6");
            oteryn_gameplay_durability::GAME_MIGRATOR.run(&pool).await?;
            seed_character(&pool).await?;
            check_retained_reads(&pool).await
        }
        .await;
        pool.close().await;
        result
    }

    async fn check_retained_reads(pool: &sqlx::PgPool) -> TestResult {
        let mut tx = pool.begin().await?;
        let session = GameSessionId::decode(&id(harness::SESSION)).map_err(debug)?;
        assert_eq!(
            read_character_name_for_share_in_transaction(&mut tx, &id(harness::CHARACTER)).await?,
            "Fixture Hero",
        );
        assert_eq!(
            read_connection_generation_for_share_in_transaction(&mut tx, session).await?,
            "1",
        );
        let before: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(pool)
                .await?;
        let sampled = read_familiar_unix_seconds_in_transaction(&mut tx).await?;
        let after: i64 =
            sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
                .fetch_one(pool)
                .await?;
        assert!((before..=after).contains(&sampled));

        // Both caller-owned share locks must remain live after each helper returns.
        for statement in [
            "UPDATE game_character_roots SET name=name WHERE character_id=encode($1,'hex')::uuid",
            "UPDATE game_durability_reconnect_sessions SET current_generation=current_generation WHERE character_id=encode($1,'hex')::uuid",
        ] {
            let mut writer = pool.begin().await?;
            sqlx::query("SET LOCAL lock_timeout='50ms'")
                .execute(&mut *writer)
                .await?;
            let blocked = sqlx::query(statement)
                .bind(id(harness::CHARACTER).as_slice())
                .execute(&mut *writer)
                .await;
            assert!(
                matches!(blocked, Err(sqlx::Error::Database(ref error)) if error.code().as_deref() == Some("55P03"))
            );
            writer.rollback().await?;
        }
        tx.rollback().await?;

        // A missing identity is still an error, rather than a synthesized name or generation.
        let mut tx = pool.begin().await?;
        assert!(matches!(
            read_character_name_for_share_in_transaction(&mut tx, &id(99)).await,
            Err(sqlx::Error::RowNotFound),
        ));
        assert!(matches!(
            read_connection_generation_for_share_in_transaction(
                &mut tx,
                GameSessionId::decode(&id(99)).map_err(debug)?,
            )
            .await,
            Err(sqlx::Error::RowNotFound),
        ));
        tx.rollback().await?;
        Ok(())
    }

    fn bootstrap_binding() -> Vec<u8> {
        let mut binding = vec![2];
        binding.extend_from_slice(&id(31));
        binding.extend_from_slice(&1_i64.to_be_bytes());
        binding.extend_from_slice(&id(30));
        binding.extend_from_slice(&id(40));
        binding.extend_from_slice(&id(harness::WORLD));
        binding.extend_from_slice(&1_i64.to_be_bytes());
        binding.extend_from_slice(&120_i64.to_be_bytes());
        for value in ["profile-1", "ruleset-1", "content-1", "starter-1"] {
            let length = u16::try_from(value.len()).unwrap_or(u16::MAX);
            binding.extend_from_slice(&length.to_be_bytes());
            binding.extend_from_slice(value.as_bytes());
        }
        // Contract version 2 binds the requested name last (CHAR-NAME-1).
        binding.extend_from_slice(&12_u16.to_be_bytes());
        binding.extend_from_slice(b"Fixture Hero");
        binding
    }

    async fn seed_character(pool: &sqlx::PgPool) -> TestResult {
        sqlx::query(
            "INSERT INTO game_character_interpretations VALUES \
             (1,'profile-1','ruleset-1','content-1','starter-1',1)",
        )
        .execute(pool)
        .await?;
        sqlx::query("INSERT INTO game_character_account_guards VALUES (encode($1,'hex')::uuid)")
            .bind(id(40).as_slice())
            .execute(pool)
            .await?;
        sqlx::query(
            "INSERT INTO game_character_roots VALUES \
             (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
              1,1,'profile-1','ruleset-1','content-1','starter-1','Fixture Hero')",
        )
        .bind(id(harness::CHARACTER).as_slice())
        .bind(id(40).as_slice())
        .bind(id(harness::WORLD).as_slice())
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO game_character_operation_receipts(\
               operation_id,command_binding,account_id,character_id,world_id,character_revision,\
               event_id,occurred_at,server_build_id,transaction_id,issuer_decision_id,\
               intent_source_revision,issued_at_source,expires_at_source) \
             VALUES (encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,\
               encode($5,'hex')::uuid,1,encode($6,'hex')::uuid,1,'test/1',\
               encode($7,'hex')::uuid,encode($8,'hex')::uuid,1,1,120)",
        )
        .bind(id(30).as_slice())
        .bind(bootstrap_binding())
        .bind(id(40).as_slice())
        .bind(id(harness::CHARACTER).as_slice())
        .bind(id(harness::WORLD).as_slice())
        .bind(id(32).as_slice())
        .bind(id(33).as_slice())
        .bind(id(31).as_slice())
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO game_character_bootstrap_intent_floors VALUES \
             (1,1,encode($1,'hex')::uuid,$2)",
        )
        .bind(id(31).as_slice())
        .bind(bootstrap_binding())
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO game_durability_reconnect_sessions(\
               game_session_id,account_id,character_id,world_id,runtime_scope_kind,\
               runtime_scope_world_id,runtime_scope_channel_id,control_loss_epoch,\
               original_grace_deadline,predecessor_generation,character_lease_generation,\
               scope_ownership_generation,current_generation,current_transport_ref,session_state) \
             VALUES (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
               encode($4,'hex')::uuid,1,encode($4,'hex')::uuid,encode($5,'hex')::uuid,\
               1,999999,1,1,1,1,$6,1)",
        )
        .bind(id(harness::SESSION).as_slice())
        .bind(id(40).as_slice())
        .bind(id(harness::CHARACTER).as_slice())
        .bind(id(harness::WORLD).as_slice())
        .bind(id(harness::CHANNEL).as_slice())
        .bind([9_u8; 16].as_slice())
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO game_durability_admission_account_guards VALUES \
             (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
              1,'test',1,'account-current',1,0,'{}')",
        )
        .bind(id(40).as_slice())
        .bind(id(harness::CHARACTER).as_slice())
        .bind(id(harness::SESSION).as_slice())
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO game_durability_admission_character_guards VALUES \
             (encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,\
              true,1,encode($4,'hex')::uuid,1,'test',1,'character-current',1,0,'{}')",
        )
        .bind(id(harness::CHARACTER).as_slice())
        .bind(id(40).as_slice())
        .bind(id(harness::WORLD).as_slice())
        .bind(id(harness::SESSION).as_slice())
        .execute(pool)
        .await?;
        Ok(())
    }
}
