//! Field publication contacts have their own retained occurrence, independent of STEP.
use super::player_field;
use crate::durability::{
    DurabilityError, fresh_admission::FreshAdmissionStore,
    item_transfer::CurrentCharacterItemFence, spell_item_transaction as storage,
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{
    ChannelRuntimeV1, ExactActorRef, GameSessionId, GameSessionState, MovementLocalPosition,
    MovementPositionSnapshot,
};
use crate::gameplay_transport::ComposedFreshAdmission;
use oteryn_gameplay_domain::domain::condition::ConditionRefusal;
use oteryn_gameplay_foundation::foundation::condition::{
    ApplicationFacts, ConditionDefinition, ConditionLineage, ConditionSourceKind, TickFacts,
};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy)]
pub(in crate::gameplay_transport) struct CreatedFieldContact {
    actor: ExactActorRef,
    session: Option<GameSessionId>,
    position: MovementPositionSnapshot,
    item: [u8; 16],
}

pub(in crate::gameplay_transport) fn capture(
    runtime: &ChannelRuntimeV1,
    fields: &[(MovementLocalPosition, [u8; 16])],
) -> Result<Vec<CreatedFieldContact>, DurabilityError> {
    if fields.is_empty() {
        return Ok(Vec::new());
    }
    let actors = runtime
        .positioned_actor_census()
        .map_err(|_| DurabilityError::Unavailable)?;
    let mut contacts = Vec::new();
    for (position, item) in fields {
        for (actor, snapshot, session) in &actors {
            if snapshot.position() == *position {
                contacts
                    .try_reserve(1)
                    .map_err(|_| DurabilityError::Unavailable)?;
                contacts.push(CreatedFieldContact {
                    actor: *actor,
                    session: *session,
                    position: *snapshot,
                    item: *item,
                });
            }
        }
    }
    Ok(contacts)
}

fn definition(
    native: &crate::content::native_gameplay::NativeGameplayState,
    item: &crate::durability::spell_items_abi::DurableTileItem,
) -> Result<Option<ConditionDefinition>, DurabilityError> {
    let policy = native
        .item_policy(
            &item.definition.production_key,
            &item.definition.revision_ref,
        )
        .ok_or(DurabilityError::Unavailable)?;
    let Some(recipe) = &policy.record().attributes.field_condition else {
        return Ok(None);
    };
    let Some(values) = recipe
        .condition_values()
        .map_err(|_| DurabilityError::Unavailable)?
    else {
        return Ok(None);
    };
    ConditionDefinition::new(
        &format!(
            "native.field.{}",
            policy.record().production_binding.external_id
        ),
        1,
        values,
    )
    .map(Some)
    .ok_or(DurabilityError::Unavailable)
}

fn lineage(origin: &crate::durability::spell_items_abi::DurableFieldOrigin) -> ConditionLineage {
    ConditionLineage {
        source_character: origin.character_id,
        source_session: origin.game_session_id,
        source_lease_generation: origin.character_lease_generation,
        source_actor_placement: origin.actor_placement_digest,
        source_scope_generation: origin.source_scope_generation,
        source_content_digest: origin.content_digest,
        source_catalog_digest: origin.catalog_digest,
        source_creation_command: origin.creation_command_id,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn drain_created_field_contacts(
        &self,
        permit: &mut SpellLanePermit,
    ) -> bool {
        loop {
            if permit.has_unresolved() {
                return false;
            }
            let mut runtime = self.runtime.lock().await;
            let mut states = self.states.spell_states.lock().await;
            let Some(contact) = states.created_field_contacts.front().copied() else {
                return true;
            };
            if runtime.read_actor_position(contact.actor).ok() != Some(contact.position)
                || contact.session.is_some_and(|session| {
                    !runtime
                        .player_control_facts(contact.actor, session)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                })
                || (contact.session.is_none() && !runtime.contains_live_creature(contact.actor))
            {
                states.created_field_contacts.pop_front();
                continue;
            }
            let now = self.owner_now().get();
            let Ok(pass) = self.root.try_issue_semantic_pass() else {
                return false;
            };
            let mut context = (self, &mut *runtime, &mut *states, &mut *permit);
            let result = pass.run_with_context(&mut context, |holder, deadline, ctx| Box::pin(async move {
                let (owner, runtime, states, permit) = ctx;
                let native = owner.active_generation.and_then(|generation| generation.native_gameplay()).ok_or(DurabilityError::Unavailable)?;
                let room = owner.rooms().gate_field_step_ingress().ok_or(DurabilityError::Unavailable)?;
                let mut tx = storage::begin_field_owner_transaction(holder, deadline).await?;
                let target = crate::durability::spell_items_abi::SpellGroundTarget::for_native_tile_read(room, runtime, contact.position.position()).map_err(|_| DurabilityError::Unavailable)?;
                if let Some(session) = contact.session {
                    let current = FreshAdmissionStore::from_root(owner.root.clone()).current_session_in_transaction(&mut tx, session).await?;
                    let fence = CurrentCharacterItemFence {
                        character_id: crate::domain::CharacterId::from_bytes(*current.commit().character_id().as_bytes()).map_err(|_| DurabilityError::Unavailable)?,
                        game_session_id: session, connection_generation: current.current_connection_generation(), character_lease_generation: current.current_character_lease().generation(),
                        runtime_scope: current.current_runtime_scope(), scope_ownership_generation: current.current_scope_generation(),
                    };
                    if current.session_state() != GameSessionState::Active
                        || fence.runtime_scope != runtime.binding().runtime_scope()
                        || fence.scope_ownership_generation != runtime.binding().scope_generation()
                        || !runtime.bound_attacker_lease(contact.actor, session).is_ok_and(|lease| lease.character_id().as_bytes() == fence.character_id.as_bytes() && lease.generation() == fence.character_lease_generation)
                    {
                        tx.into_inner().rollback().await?;
                        states.created_field_contacts.pop_front();
                        return Ok(());
                    }
                    let read = storage::read_standing_player_tile_in_transaction(&mut tx, permit, owner.root, owner.character, owner.holder, &fence, native.source_digest(), &target).await.map_err(|_| DurabilityError::Unavailable)?;
                    let item = read.items().iter().find(|item| item.item_instance_id == contact.item);
                    let entry = if let Some(item) = item {
                        match (definition(native, item)?, item.field_origin.as_ref()) {
                            (Some(definition), Some(origin)) if !matches!(origin.source_item_key.as_str(), "oteryn:item.tibia.i2134" | "oteryn:item.tibia.i2135" | "oteryn:item.tibia.i21465") => {
                                let recipe = native.item_policy(&item.definition.production_key, &item.definition.revision_ref).and_then(|policy| policy.record().attributes.field_condition.as_ref()).ok_or(DurabilityError::Unavailable)?;
                                Some(player_field::PlayerFieldContact { origin: origin.clone(), item: item.item_instance_id, definition, element: recipe.element().map_err(|_| DurabilityError::Unavailable)?, attributed: true })
                            }
                            _ => None,
                        }
                    } else { None };
                    let pending = if let Some(entry) = entry {
                        owner.stage_player_field_entry(&mut tx, permit, runtime, states, &read, contact.actor, session, contact.position.position(), now, entry).await?
                    } else { None };
                    if let Some(pending) = pending {
                        // From this point the commit window retains this exact contact on
                        // cancellation or an unknown outcome. The queue must not apply it twice.
                        states.created_field_contacts.pop_front();
                        let mut window = permit.open_commit_window(pending, player_field::park);
                        crate::durability::pvp_live::commit_live_pvp_transaction(tx.into_inner(), deadline, &mut window).await?;
                        if !window.attempt().matches_current(runtime, states) { return Err(DurabilityError::Unavailable); }
                        window.install().install(runtime, states);
                    } else {
                        tx.into_inner().rollback().await?;
                        states.created_field_contacts.pop_front();
                    }
                } else {
                    let binding = runtime.binding();
                    let authority = storage::assert_spell_item_scope_in_transaction(&mut tx, permit, owner.root, owner.character, owner.holder, binding.runtime_scope(), binding.scope_generation().get()).await.map_err(|_| DurabilityError::Unavailable)?;
                    let read = storage::read_creature_scope_tile_in_transaction(&mut tx, &authority, &target).await.map_err(|_| DurabilityError::Unavailable)?;
                    let cells = room.movement_cells();
                    let position = contact.position.position();
                    let tile = cells.spell_tiles().lookup(cells.scope(), crate::content::LogicalCell { x: position.x, y: position.y, z: i32::from(position.floor) }).map_err(|_| DurabilityError::Unavailable)?;
                    if !tile.flags().protection_zone
                        && let Some(item) = read.items.iter().find(|item| item.item_instance_id == contact.item)
                        && let Some(definition) = definition(native, item)?
                    {
                        let before = runtime.companion_snapshot(contact.actor).map_err(|_| DurabilityError::Unavailable)?;
                        let source = item.field_origin.as_ref().and_then(|origin| states.actors.iter().find(|(actor, session, _)| actor.placement_identity() == origin.actor_placement_digest && *session.as_bytes() == origin.game_session_id))
                            .filter(|(actor, session, _)| runtime.player_control_facts(*actor, *session).is_ok_and(|facts| facts.control_loss.is_none())).map(|(actor, _, _)| *actor);
                        let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(native.source_digest());
                        let digest = Sha256::new().chain_update(b"oteryn:created-field-contact:v1").chain_update(contact.actor.placement_identity()).chain_update(contact.item).finalize();
                        let mut bytes = [0;16]; bytes.copy_from_slice(&digest[..16]);
                        let facts = ApplicationFacts { now, base_speed: u16::try_from(before.state.policy.base_speed).map_err(|_| DurabilityError::Unavailable)?, mana_shield_capacity: 0, target_reentry_protected: false, source_reentry_protected: false, target_is_player: false, decision_root: &root, occurrence: oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(bytes) };
                        let mut next = before.state.clone();
                        let applied = if let Some(origin) = source.and(item.field_origin.as_ref()) {
                            next.conditions.apply_with_lineage(&definition, source, ConditionSourceKind::Field, &before.state.policy.condition_immunities, &facts, &lineage(origin))
                        } else {
                            next.conditions.apply(&definition, None, ConditionSourceKind::Field, &before.state.policy.condition_immunities, &facts)
                        };
                        if !matches!(applied, Ok(_) | Err(ConditionRefusal::Immune | ConditionRefusal::KeptCurrent | ConditionRefusal::ReentryProtected)) {
                            return Err(DurabilityError::Unavailable);
                        }
                        runtime.compare_companion_state(&before, next).map_err(|_| DurabilityError::Unavailable)?;
                        // Publication is not movement: do not consume or reset field_entry_revision.
                        states.created_field_contacts.pop_front();
                        let recipe = native.item_policy(&item.definition.production_key, &item.definition.revision_ref).and_then(|policy| policy.record().attributes.field_condition.as_ref()).ok_or(DurabilityError::Unavailable)?;
                        states.tick_source_creature_conditions(runtime, contact.actor, TickFacts { in_protection_zone: false, standing_on_field: recipe.element().map_err(|_| DurabilityError::Unavailable)? }, crate::foundation::owner_timer::SemanticTimeMicros::from_micros(now)).map_err(|_| DurabilityError::Unavailable)?;
                        tx.into_inner().rollback().await?;
                    } else {
                        tx.into_inner().rollback().await?;
                        states.created_field_contacts.pop_front();
                    }
                }
                Ok(())
            })).await;
            if result.is_err() {
                return false;
            }
        }
    }
}
