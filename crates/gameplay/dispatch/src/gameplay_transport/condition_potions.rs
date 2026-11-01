//! Q4/Q6 condition potions, composed with the existing item-use gate, physical actor
//! reservation, player condition owner, fenced burn and unresolved spell lane.
//! Crystal 201189f `potions.lua:1-31,52-54,63,78-80,125-129,141-147`.
use super::*;
use crate::durability::item_use::condition_potions::PreparedConditionPotion;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{
    ApplicationFacts, CommandId, CommandRef, ConditionDefinition, ConditionSourceKind,
    ConditionStore, StagedSpellBatch,
};
use crate::gameplay_transport::actor_spell::{
    PlayerBatchPreflight, check_owner_batch, install_owner_batch, stage_player_batch,
};
use oteryn_gameplay_progression::condition_potions::{ConditionPotion, DEFINITION_REVISION};

#[derive(Debug)]
pub(in crate::gameplay_transport) struct PendingConditionPotion {
    fence: CurrentCharacterItemFence,
    request: ItemUseRequest,
    application: PreparedConditionPotion,
    physical: StagedSpellBatch,
    player: PlayerBatchPreflight,
    content_digest: [u8; 32],
}

fn park(attempt: PendingConditionPotion) -> Box<dyn std::any::Any + Send> {
    Box::new(crate::gameplay_transport::actor_spell::native_combat_cast::UnresolvedSpellCommit::ConditionPotion(Box::new(attempt)))
}

fn vocation_name(vocation: Vocation) -> &'static str {
    match base_vocation(vocation) {
        ReferenceBaseVocation::None => "none",
        ReferenceBaseVocation::Sorcerer => "sorcerer",
        ReferenceBaseVocation::Druid => "druid",
        ReferenceBaseVocation::Paladin => "paladin",
        ReferenceBaseVocation::Knight => "knight",
        ReferenceBaseVocation::Monk => "monk",
    }
}

/// Typed source values enter the one existing condition store; its expiry, conflict slot,
/// suppression and KeptCurrent rules remain authoritative.
fn prepared_conditions(
    before: &ConditionStore<String>,
    potion: ConditionPotion,
    now_us: u64,
    digest: [u8; 32],
    event: [u8; 16],
) -> Result<ConditionStore<String>, UseDisposition> {
    let definition = ConditionDefinition::new(potion.key(), DEFINITION_REVISION, potion.values())
        .ok_or(UseDisposition::Rejected)?;
    let decision = GameplayDecisionRoot::from_bytes(digest);
    let mut next = before.clone();
    next.apply(
        &definition,
        None,
        ConditionSourceKind::SelfUse,
        &[],
        &ApplicationFacts {
            now: now_us,
            // Skill conditions do not consume movement or mana-shield formula facts.
            base_speed: 0,
            mana_shield_capacity: 0,
            target_reentry_protected: false,
            source_reentry_protected: false,
            target_is_player: true,
            decision_root: &decision,
            occurrence: DecisionOccurrenceId::from_bytes(event),
        },
    )
    .map_err(|_| UseDisposition::Rejected)?;
    Ok(next)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn use_condition_potion(
        &self,
        permit: &mut SpellLanePermit,
        actor: ExactActorRef,
        command: UseCommand,
        fence: CurrentCharacterItemFence,
        native: &NativeGameplayState,
        inventory: &ItemUseInventory,
        entry: &BackpackEntry,
        record: &ItemProfileRecord,
        potion: ConditionPotion,
    ) -> Result<UseDisposition, UseDisposition> {
        let session = command.game_session_id;
        let request = ItemUseRequest {
            kind: ItemUseKind::Potion,
            command_id: command.command_id,
            content_revision: inventory.content_revision.clone(),
            ruleset_revision: inventory.ruleset_revision.clone(),
            sim_revision: entry_chest::SIM_REVISION.to_owned(),
            transaction_id: crate::operator_files::uuid_v7()
                .map_err(|_| UseDisposition::Rejected)?,
            event_id: crate::operator_files::uuid_v7().map_err(|_| UseDisposition::Rejected)?,
            occurred_at_unix_ms: unix_ms().map_err(|_| UseDisposition::Rejected)?,
            server_build_id: SERVER_BUILD_ID.to_owned(),
            source_item_instance_id: entry.item.item_instance_id,
            source_definition: entry.item.definition.clone(),
            source_quantity_before: entry.item.quantity,
            flask: ItemUseFlaskPlan::None,
        };
        let now = self.owner_now();
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let lease = runtime
            .bound_attacker_lease(actor, session)
            .map_err(|_| UseDisposition::Rejected)?;
        if lease.character_id().as_bytes() != fence.character_id.as_bytes()
            || lease.generation() != fence.character_lease_generation
            || runtime.content_pin().server_artifact_digest() != native.source_digest()
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
        {
            return Err(UseDisposition::Rejected);
        }
        let state = states
            .get(&runtime, actor, session)
            .ok_or(UseDisposition::Rejected)?;
        let facts = state.character_facts();
        if !potion.allows_base_vocation(vocation_name(facts.vocation))
            || !requirements_met(&record.semantics.use_requirements, facts)
        {
            return Err(UseDisposition::RequirementNotMet);
        }
        let expected = state.conditions.clone();
        let next = prepared_conditions(
            &expected,
            potion,
            now.get(),
            native.source_digest(),
            request.event_id,
        )?;
        let batch = OwnerCombatBatch {
            caster: actor,
            attacker: lease.character_id(),
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(
                session,
                CommandId::new(command.command_id).map_err(|_| UseDisposition::Rejected)?,
            ),
            occurrence: SpellOccurrenceBinding {
                id: format!("{}:{}", potion.key(), command.command_id),
                revisions: [
                    request.ruleset_revision.clone(),
                    request.content_revision.clone(),
                    "oteryn:self-item-policy:v1".to_owned(),
                    "crystal:201189f:condition-potions:v1".to_owned(),
                    request.sim_revision.clone(),
                ],
            },
            binding: request.source_definition.revision_ref.as_bytes().to_vec(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1_000,
            effects: vec![OwnerCombatEffect {
                target: actor,
                sub_ordinal: 0,
                change: OwnerCombatChange::PlayerConditions {
                    expected: Box::new(expected),
                    next: Box::new(next),
                },
            }],
            deferred: None,
        };
        let mut physical = runtime
            .stage_spell_batch(&batch)
            .map_err(|_| UseDisposition::Rejected)?;
        let player = stage_player_batch(&runtime, &states, &batch, None)
            .map_err(|_| UseDisposition::Rejected)?;
        let application =
            PreparedConditionPotion::capture(&physical, &player, &request, native.source_digest())
                .map_err(|_| UseDisposition::Rejected)?;
        states
            .item_uses
            .gate(&runtime, actor, session)
            .ok_or(UseDisposition::Rejected)?
            .start(ItemUseKind::Potion, now.get() / 1_000)?;
        runtime
            .reserve_spell_batch(&mut physical)
            .map_err(|_| UseDisposition::Rejected)?;
        let attempt = PendingConditionPotion {
            fence,
            request,
            application,
            physical,
            player,
            content_digest: native.source_digest(),
        };
        let mut window = permit.open_commit_window(attempt, park);
        // The actual actor is reserved. Cancellation anywhere after this point retains the
        // whole immutable application; only the receipt resolver can prove it uncommitted.
        window.mark_already_committed();
        #[cfg(test)]
        if take_fault(1) {
            return Err(UseDisposition::Rejected);
        }
        let attempt = window.attempt();
        let result = self
            .root
            .commit_condition_potion(
                self.character,
                self.holder,
                fence,
                attempt.request.clone(),
                attempt.application.clone(),
            )
            .await;
        match result {
            Ok(_) => {
                #[cfg(test)]
                if take_fault(2) {
                    return Err(UseDisposition::Rejected);
                }
                let checked = check_owner_batch(
                    &runtime,
                    &states,
                    &window.attempt().physical,
                    Some(&window.attempt().player),
                )
                .map_err(|_| UseDisposition::Rejected)?;
                let attempt = window.install();
                install_owner_batch(
                    &mut runtime,
                    &mut states,
                    attempt.physical,
                    Some(attempt.player),
                    checked,
                );
                if let Some(gate) = states.item_uses.gate(&runtime, actor, session) {
                    gate.settle(command.command_id, UseDisposition::Committed, &[]);
                }
                Ok(UseDisposition::Committed)
            }
            Err(ItemUseError::Unavailable(_)) => Err(UseDisposition::Rejected),
            Err(error) => {
                let disposition = if matches!(error, ItemUseError::StaleState) {
                    UseDisposition::StaleState
                } else {
                    UseDisposition::Rejected
                };
                runtime
                    .release_definitely_uncommitted_spell_batch(&window.attempt().physical)
                    .map_err(|_| UseDisposition::Rejected)?;
                window.release();
                if let Some(gate) = states.item_uses.gate(&runtime, actor, session) {
                    gate.settle(command.command_id, disposition, &[]);
                }
                Err(disposition)
            }
        }
    }

    pub(in crate::gameplay_transport) async fn resolve_condition_potion_attempt(
        &self,
        permit: &mut SpellLanePermit,
        attempt: PendingConditionPotion,
    ) {
        let mut window = permit.open_commit_window(attempt, park);
        window.mark_already_committed();
        let original = window.attempt().fence;
        let command = window.attempt().physical.batch().command;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(self.root.clone())
                .current_session_at(original.game_session_id)
                .await
                .ok()
                .and_then(|(current, _)| crate::gameplay_transport::item_fence_of(current));
        let Some(fresh) = current else { return };
        if fresh != original {
            let mut expected = original;
            expected.connection_generation = fresh.connection_generation;
            if expected != fresh {
                return;
            }
            let Ok(character) = self
                .root
                .read_current_character(self.character, original.character_id)
                .await
            else {
                return;
            };
            let old = crate::durability::character_progression::CurrentCharacterGameplayFence {
                character_id: original.character_id,
                game_session_id: original.game_session_id,
                connection_generation: original.connection_generation,
                character_lease_generation: original.character_lease_generation,
                runtime_scope: original.runtime_scope,
                scope_ownership_generation: original.scope_ownership_generation,
                expected_character_revision: character.revision,
            };
            let Ok(pass) = self.root.try_issue_semantic_pass() else {
                return;
            };
            let digest = window.attempt().content_digest;
            let mut context = (self, window.permit());
            let proof = pass.run_with_context(&mut context, move |holder, deadline, context| Box::pin(async move {
                let (owner, permit) = context;
                use crate::durability::spell_item_transaction as storage;
                let mut tx = storage::begin_spell_owner_transaction(holder, deadline).await?;
                let authority = storage::assert_spell_item_authority_in_transaction(&mut tx, permit,
                    owner.root, owner.character, owner.holder, &fresh, command, digest).await
                    .map_err(|_| crate::durability::DurabilityError::Unavailable)?;
                crate::durability::admission_journal::spell_reconnect::prove_pending_spell_reconnect(&mut tx, &authority, &old, &fresh).await
            })).await;
            if !matches!(proof, Ok(Some(proof)) if proof.matches(original.connection_generation,
                fresh.connection_generation, command, fresh.character_id, fresh.character_lease_generation,
                fresh.runtime_scope, fresh.scope_ownership_generation.get()))
            {
                return;
            }
            window.attempt_mut().fence = fresh;
        }
        let attempt = window.attempt();
        let result = self
            .root
            .resolve_condition_potion(
                self.character,
                self.holder,
                fresh,
                attempt.request.clone(),
                attempt.application.clone(),
            )
            .await;
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let actor = window.attempt().physical.batch().caster;
        let disposition = match result {
            Ok(true) => {
                let Ok(checked) = check_owner_batch(
                    &runtime,
                    &states,
                    &window.attempt().physical,
                    Some(&window.attempt().player),
                ) else {
                    return;
                };
                let attempt = window.install();
                install_owner_batch(
                    &mut runtime,
                    &mut states,
                    attempt.physical,
                    Some(attempt.player),
                    checked,
                );
                UseDisposition::Committed
            }
            Ok(false) => {
                if runtime
                    .release_definitely_uncommitted_spell_batch(&window.attempt().physical)
                    .is_err()
                {
                    return;
                }
                window.release();
                UseDisposition::Rejected
            }
            Err(_) => return,
        };
        if let Some(gate) = states
            .item_uses
            .gate(&runtime, actor, original.game_session_id)
        {
            gate.settle(command.command_id().get(), disposition, &[]);
        }
    }
}

#[cfg(test)]
static FAULT: std::sync::atomic::AtomicU8 = std::sync::atomic::AtomicU8::new(0);
#[cfg(test)]
pub(in crate::gameplay_transport) fn inject_fault(point: u8) {
    FAULT.store(point, std::sync::atomic::Ordering::SeqCst);
}
#[cfg(test)]
fn take_fault(point: u8) -> bool {
    FAULT
        .compare_exchange(
            point,
            0,
            std::sync::atomic::Ordering::SeqCst,
            std::sync::atomic::Ordering::SeqCst,
        )
        .is_ok()
}

#[cfg(test)]
#[path = "condition_potions_tests.rs"]
mod tests;
