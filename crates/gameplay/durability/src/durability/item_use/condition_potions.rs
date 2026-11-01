//! Q4/Q6: the existing item-use burn and a reserved real skill-condition application commit
//! together. The immutable application receipt resolves a lost acknowledgement; it is not a
//! second condition store. The retained actual owner batch keeps its original semantic expiry.
use super::*;
use crate::foundation::runtime_actor_spell_types::{OwnerCombatChange, PlayerBatchProof};
use crate::foundation::{
    ApplicationFacts, ConditionDefinition, ConditionSourceKind, StagedSpellBatch,
};
use oteryn_gameplay_progression::condition_potions::{
    ConditionPotion, DEFINITION_REVISION, DURATION_MS,
};
use oteryn_simulation_determinism::{DecisionOccurrenceId, GameplayDecisionRoot};

/// Only a staged actual player-owner proof can create this application. Its fields are private
/// so a decoded request cannot claim an effect merely by naming a potion or an achievement.
#[derive(Debug, Clone)]
pub struct PreparedConditionPotion {
    potion: ConditionPotion,
    character: crate::domain::CharacterId,
    session: crate::foundation::GameSessionId,
    lease_generation: u64,
    command: u64,
    owner_micros: u64,
    content_digest: [u8; 32],
    item_binding: [u8; 32],
    binding: [u8; 32],
}

impl PreparedConditionPotion {
    pub fn capture(
        physical: &StagedSpellBatch,
        player: &impl PlayerBatchProof,
        request: &ItemUseRequest,
        content_digest: [u8; 32],
    ) -> Result<Self> {
        let batch = physical.batch();
        let potion = ConditionPotion::from_key(&request.source_definition.production_key)
            .ok_or(ItemUseError::InvalidInput)?;
        let now = batch.now_us.ok_or(ItemUseError::InvalidInput)?.get();
        let [effect] = batch.effects.as_slice() else {
            return Err(ItemUseError::InvalidInput);
        };
        let OwnerCombatChange::PlayerConditions { expected, next } = &effect.change else {
            return Err(ItemUseError::InvalidInput);
        };
        if !physical.will_apply()
            || !player.matches_batch(batch)
            || effect.target != batch.caster
            || effect.sub_ordinal != 0
            || batch.anchor.is_some()
            || batch.deferred.is_some()
            || batch.command.command_id().get() != request.command_id
            || request.kind != ItemUseKind::Potion
            || request.flask != ItemUseFlaskPlan::None
            || request.source_definition.family != "Item"
            || content_digest == [0; 32]
            || now.checked_add(u64::from(DURATION_MS) * 1_000).is_none()
        {
            return Err(ItemUseError::InvalidInput);
        }
        // Reproduce the precise non-random transition through the one existing condition engine.
        // No unrelated condition change or KeptCurrent can earn the progress increment.
        let definition =
            ConditionDefinition::new(potion.key(), DEFINITION_REVISION, potion.values())
                .ok_or(ItemUseError::InvalidInput)?;
        let root = GameplayDecisionRoot::from_bytes(content_digest);
        let mut reproduced = (**expected).clone();
        reproduced
            .apply(
                &definition,
                None,
                ConditionSourceKind::SelfUse,
                &[],
                &ApplicationFacts {
                    now,
                    base_speed: 0,
                    mana_shield_capacity: 0,
                    target_reentry_protected: false,
                    source_reentry_protected: false,
                    target_is_player: true,
                    decision_root: &root,
                    occurrence: DecisionOccurrenceId::from_bytes(request.event_id),
                },
            )
            .map_err(|_| ItemUseError::InvalidInput)?;
        if &reproduced != next.as_ref() || expected == next {
            return Err(ItemUseError::InvalidInput);
        }
        let item_binding = request.binding();
        let binding = Sha256::new()
            .chain_update(b"oteryn:condition-potion-application:v1")
            .chain_update(format!("{batch:?}"))
            .chain_update(item_binding)
            .chain_update(content_digest)
            .finalize()
            .into();
        Ok(Self {
            potion,
            character: crate::domain::CharacterId::from_bytes(*batch.attacker.as_bytes())
                .map_err(|_| ItemUseError::InvalidInput)?,
            session: batch.command.game_session_id(),
            lease_generation: batch.current_lease_generation,
            command: request.command_id,
            owner_micros: now,
            content_digest,
            item_binding,
            binding,
        })
    }

    fn matches(&self, fence: CurrentCharacterItemFence, request: &ItemUseRequest) -> bool {
        self.character == fence.character_id
            && self.session == fence.game_session_id
            && self.lease_generation == fence.character_lease_generation
            && self.command == request.command_id
            && self.item_binding == request.binding()
            && ConditionPotion::from_key(&request.source_definition.production_key)
                == Some(self.potion)
    }
}

impl DurabilityRoot {
    pub async fn commit_condition_potion(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemUseRequest,
        application: PreparedConditionPotion,
    ) -> Result<ItemUseOutcome> {
        self.condition_potion_operation(authority, node, fence, request, application, true)
            .await?
            .ok_or(ItemUseError::StaleState)
    }

    /// The same root/session locks wait for the original transaction before proving absence.
    /// This path never retries a burn or extends a condition after an uncertain commit.
    pub async fn resolve_condition_potion(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemUseRequest,
        application: PreparedConditionPotion,
    ) -> Result<bool> {
        Ok(self
            .condition_potion_operation(authority, node, fence, request, application, false)
            .await?
            .is_some())
    }

    #[allow(clippy::too_many_arguments)]
    async fn condition_potion_operation(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemUseRequest,
        application: PreparedConditionPotion,
        write_new: bool,
    ) -> Result<Option<ItemUseOutcome>> {
        request.validate()?;
        if !application.matches(fence, &request) {
            return Err(ItemUseError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            use crate::durability::character_equipment::{assert_current_equipment_in_transaction, EquipmentError};
            let command = crate::foundation::CommandRef::new(fence.game_session_id,
                crate::foundation::CommandId::new(request.command_id).map_err(|_| DurabilityError::InvalidStoredState)?);
            match assert_current_equipment_in_transaction(&mut tx, &recovery, &node, &fence,
                Some(command), application.content_digest).await {
                Ok(_) => {}
                Err(EquipmentError::Durability(error)) => return Err(error),
                Err(EquipmentError::Database(error)) => return Err(error.into()),
                Err(EquipmentError::Rejected(_)) => return Ok(Err(ItemUseError::AuthorityRejected)),
            }
            match assert_item_only_fence(&mut tx, &fence, &node).await? {
                Ok(root) if root.content_revision == request.content_revision && root.ruleset_revision == request.ruleset_revision => {}
                Ok(_) => return Ok(Err(ItemUseError::StaleState)),
                Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
                Err(_) => return Ok(Err(ItemUseError::AuthorityRejected)),
            }
            match replay(&mut tx, &fence, &request).await {
                Ok(Some(outcome)) => {
                    let stored: Option<Vec<u8>> = sqlx::query_scalar("SELECT application_binding FROM game_item_condition_potion_receipts WHERE transaction_id=encode($1,'hex')::uuid")
                        .bind(request.transaction_id.as_slice()).fetch_optional(&mut **tx).await?;
                    if stored.as_deref() != Some(application.binding.as_slice()) {
                        return Ok(Err(ItemUseError::ConflictingCommand));
                    }
                    tx.commit(deadline).await?;
                    return Ok(Ok(Some(outcome)));
                }
                Ok(None) => {}
                Err(ItemUseError::Unavailable(error)) => return Err(error),
                Err(error) => return Ok(Err(error)),
            }
            if !write_new {
                tx.commit(deadline).await?;
                return Ok(Ok(None));
            }
            let tuple = tx.tuple();
            let outcome = match write(&mut tx, &fence, &request, tuple).await {
                Ok(outcome @ ItemUseOutcome::Written { .. }) => outcome,
                Ok(_) => return Ok(Err(ItemUseError::ConflictingCommand)),
                Err(ItemUseError::Unavailable(error)) => return Err(error),
                Err(error) => return Ok(Err(error)),
            };
            sqlx::query("INSERT INTO game_item_condition_potion_receipts(transaction_id,condition_key,definition_revision,applied_owner_micros,expires_owner_micros,content_digest,application_binding) VALUES(encode($1,'hex')::uuid,$2,$3,$4::text::numeric,$5::text::numeric,$6,$7)")
                .bind(request.transaction_id.as_slice()).bind(application.potion.key())
                .bind(i64::from(DEFINITION_REVISION)).bind(application.owner_micros.to_string())
                .bind((application.owner_micros + u64::from(DURATION_MS) * 1_000).to_string())
                .bind(application.content_digest.as_slice()).bind(application.binding.as_slice())
                .execute(&mut **tx).await?;
            let counted = application.potion.achievement();
            add_progress(&mut tx, fence.character_id, counted.key, counted.total,
                item_use_achievement::SOURCE_KIND, &request.event_id).await?;
            tx.commit(deadline).await?;
            Ok(Ok(Some(outcome)))
        })).await?
    }
}
