//! Crystal 201189f8 data/scripts/systems/concoctions.lua:7-12 grants 60 minutes once;
//! player_functions.cpp:2642-2647 caps stamina at 2520. Compose the existing build participant
//! with the canonical one-unit burn so a crash/replay cannot lose or repeat the grant.
use super::super::*;
use super::{Concoction, admit_kind, start};
use crate::durability::character_build::{
    BuildCause, BuildChangeRequest, BuildOccurrence, DurableBuildState, apply_build_in_transaction,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::offline_training_state::{self, OfflineChange};

impl DurabilityRoot {
    pub async fn commit_stamina_extension(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemUseRequest,
    ) -> Result<(ItemUseOutcome, u16)> {
        request.validate()?;
        if request.kind != ItemUseKind::Concoction
            || request.source_definition.family != "Item"
            || request.source_definition.production_key
                != Concoction::StaminaExtension.production_key()
            || !matches!(request.flask, ItemUseFlaskPlan::None)
        {
            return Err(ItemUseError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let prior = match replay(&mut tx, &fence, &request).await {
                Ok(prior) => prior,
                Err(ItemUseError::Unavailable(error)) => return Err(error),
                Err(error) => return Ok(Err(error)),
            };
            if let Some(outcome) = prior {
                let stamina = offline_training_state::load(&mut tx, fence.character_id.as_bytes()).await?.stamina_minutes;
                tx.commit(deadline).await?;
                return Ok(Ok((outcome, stamina)));
            }
            let root = match assert_item_only_fence(&mut tx, &fence, &node).await? {
                Ok(root) => root,
                Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
                Err(_) => return Ok(Err(ItemUseError::AuthorityRejected)),
            };
            if root.content_revision != request.content_revision || root.ruleset_revision != request.ruleset_revision {
                return Ok(Err(ItemUseError::StaleState));
            }
            if let Err(error) = admit_kind(&mut tx, &fence, Concoction::StaminaExtension).await {
                return match error {
                    ItemUseError::Unavailable(error) => Err(error),
                    error => Ok(Err(error)),
                };
            }
            let offline = offline_training_state::load(&mut tx, fence.character_id.as_bytes()).await?;
            let stamina = offline.stamina_minutes.saturating_add(60).min(2520);
            if stamina != offline.stamina_minutes {
                let row = sqlx::query("SELECT vocation, magic_level, mana_spent, fist_level, fist_tries, club_level, club_tries, sword_level, sword_tries, axe_level, axe_tries, distance_level, distance_tries, shielding_level, shielding_tries, fishing_level, fishing_tries FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
                let before = row.as_ref().map(|row| DurableBuildState::stored(row, "vocation", ""))
                    .transpose()?.unwrap_or_default();
                let build_fence = CurrentCharacterGameplayFence {
                    character_id: fence.character_id, game_session_id: fence.game_session_id,
                    connection_generation: fence.connection_generation, character_lease_generation: fence.character_lease_generation,
                    runtime_scope: fence.runtime_scope, scope_ownership_generation: fence.scope_ownership_generation,
                    expected_character_revision: crate::domain::CharacterRevision::new(root.revision)
                        .map_err(|_| DurabilityError::InvalidStoredState)?,
                };
                let occurrence = BuildOccurrence::from_bytes(request.transaction_id)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                let build = BuildChangeRequest { occurrence, cause: BuildCause::RestingStamina,
                    after: before.clone(), before, pruned_stance: None };
                let outcome = apply_build_in_transaction(&mut tx, &node, &build_fence, build,
                    Sha256::digest(b"w09fx-stamina-extension-v1").into(), root.content_revision,
                    request.transaction_id.to_vec(), Some(OfflineChange { before: offline,
                        after: offline_training_state::OfflineTrainingState { stamina_minutes: stamina, ..offline } })).await?;
                if let Err(error) = outcome {
                    return match error {
                        CharacterProgressionError::Unavailable(error) => Err(error),
                        _ => Ok(Err(ItemUseError::StaleState)),
                    };
                }
            }
            let tuple = tx.tuple();
            let outcome = match write(&mut tx, &fence, &request, tuple).await {
                Ok(outcome) => outcome,
                Err(ItemUseError::Unavailable(error)) => return Err(error),
                Err(error) => return Ok(Err(error)),
            };
            if let Err(error) = start(&mut tx, &fence, &request).await {
                return match error {
                    ItemUseError::Unavailable(error) => Err(error),
                    error => Ok(Err(error)),
                };
            }
            tx.commit(deadline).await?;
            Ok(Ok((outcome, stamina)))
        })).await?
    }
}
