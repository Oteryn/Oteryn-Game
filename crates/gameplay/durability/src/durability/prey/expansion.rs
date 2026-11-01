//! Store-only entitlement delivery; no matching action exists in the client codec.
use super::super::{
    DurabilityError, DurabilityRoot,
    character_authority::{ReconciledCharacterAuthority, assert_recovery_fence},
    character_progression::{CharacterProgressionError, CurrentCharacterGameplayFence},
    character_wheel::WheelPremiumGate,
    db::{begin_semantic_transaction, commit_semantic_transaction},
    runtime_scope_assignment::NodeIncarnationProof,
    store::DeliveryReceipt,
};
use super::{PreyCatalogue, PreyExpansionError, PreyMutation, PreyStoreDelivery};

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn grant_prey_list_expansion<G: WheelPremiumGate>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        delivery: PreyStoreDelivery,
        catalogue: &PreyCatalogue,
        gate: &G,
    ) -> Result<DeliveryReceipt, PreyExpansionError> {
        if delivery.operation_id[6] >> 4 != 7
            || delivery.operation_id[8] & 0xc0 != 0x80
            || delivery.fulfilment_unit_id.is_empty()
            || delivery.fulfilment_unit_id.len() > 512
            || delivery.fulfilment_unit_id.chars().any(char::is_control)
        {
            return Err(CharacterProgressionError::InvalidInput.into());
        }
        let operation = delivery.operation_id;
        let (_, _, receipt) = self
            .commit_prey_inner(
                authority,
                node,
                fence,
                operation,
                PreyMutation::Initialize,
                catalogue,
                None,
                gate,
                Some(delivery),
            )
            .await?;
        receipt.ok_or(PreyExpansionError::AlreadyOwned)
    }

    pub async fn prey_list_expansion_owned(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: crate::domain::CharacterId,
    ) -> Result<bool, CharacterProgressionError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        Ok(self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let owned: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_prey_expansions WHERE character_id=encode($1,'hex')::uuid)")
                .bind(character.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok::<_, DurabilityError>(owned)
        })).await?)
    }
}
