//! House death composes genuine House authority with the unchanged death pricing writer.
use super::*;
use crate::durability::character_progression::assert_house_death_fence;
use crate::durability::house_scope_handoff::HouseId;
impl DurabilityRoot {
    #[allow(
        clippy::too_many_arguments,
        reason = "the same death inputs plus genuine named-House authority"
    )]
    pub async fn settle_runtime_house_player_death_with_items<const N: usize>(
        &self,
        sequencer: &CharacterRevisionSequencer,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        house: &HouseId,
        fence: CurrentCharacterGameplayFence,
        mut request: CharacterDeathRequest<N>,
        formula: Option<&dyn BuildFormula>,
        premium: Option<&dyn promotion::DeathPremiumOwner>,
        selection_root: Option<[u8; 32]>,
    ) -> Result<CommittedCharacterDeath> {
        if house.runtime_scope().ok() != Some(fence.runtime_scope)
            || request.death_cell.runtime_scope != fence.runtime_scope
        {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        validate_request(&fence, &request)?;
        let mut slot = sequencer.acquire(fence.character_id).await;
        let retained = self
            .reconcile_character_death(authority, request.occurrence)
            .await?;
        let original = retained.as_ref().map(|r| r.original_character_revision);
        if premium.is_some() {
            request.held_blessings = match retained {
                Some(r) => r.blessings_before,
                None => {
                    self.house_death_blessings(authority, node, house, fence)
                        .await?
                }
            };
        }
        let fence = if original.is_none() {
            CurrentCharacterGameplayFence {
                expected_character_revision: slot_cursor(&mut slot, self, authority).await?,
                ..fence
            }
        } else {
            fence
        };
        let formula = if original.is_some() {
            None
        } else {
            formula.filter(|f| f.content_revision() == request.context.content)
        };
        // House admission initializes progression and flushes training under its
        // genuine Channel predecessor. Never synthesize either inside a House.
        let outcome = slot
            .commit_house_premium_death(
                self,
                authority,
                node,
                fence,
                request,
                formula,
                original,
                premium,
                selection_root,
                house,
            )
            .await?;
        let (CharacterDeathOutcome::Committed(result)
        | CharacterDeathOutcome::AlreadyCommitted(result)) = outcome;
        Ok(result)
    }
    async fn house_death_blessings(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        house: &HouseId,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Vec<String>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let house = house.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error)=assert_house_death_fence(&mut tx,&fence.item_fence(),None,&node,&house).await? {return Ok(Err(error));}
            let held=sqlx::query_scalar("SELECT blessing_key COLLATE \"C\" FROM game_character_blessings WHERE character_id=encode($1,'hex')::uuid ORDER BY 1 LIMIT 33")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;Ok(Ok(held))
        })).await?
    }
    pub async fn read_house_death_home_town(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        house: &HouseId,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<u32>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let house = house.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            if let Err(error)=assert_house_death_fence(&mut tx,&fence.item_fence(),None,&node,&house).await? {return Ok(Err(error));}
            let town:Option<i32>=sqlx::query_scalar("SELECT town_id FROM game_character_home_towns WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let town=town.map(|id|u32::try_from(id).map_err(|_|DurabilityError::InvalidStoredState)).transpose()?;
            commit_semantic_transaction(tx,deadline).await?;Ok(Ok(town))
        })).await?
    }
}

impl DurabilityRoot {
    /// A House reconnect restores an already priced death; this read neither
    /// creates another death nor chooses or consumes its Channel respawn.
    pub async fn read_house_pending_player_death(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        house: &HouseId,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<Option<CommittedCharacterDeath>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let house = house.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;
            if let Err(error)=assert_house_death_fence(&mut tx,&fence.item_fence(),None,&node,&house).await? {return Ok(Err(error));}
            let pending:Option<(Vec<u8>,Vec<u8>)>=sqlx::query_as("SELECT uuid_send(death_occurrence_id),respawn_position FROM game_character_pending_respawns WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let result=if let Some((occurrence,position))=pending {
                let occurrence=PlayerDeathOccurrence::from_bytes(occurrence.as_slice().try_into().map_err(|_|DurabilityError::InvalidStoredState)?)
                    .map_err(|_|DurabilityError::InvalidStoredState)?;
                let row=load_receipt(&mut tx,occurrence).await?.ok_or(DurabilityError::InvalidStoredState)?;
                let receipt=decode_receipt(&row)?;
                if receipt.character_id!=fence.character_id || receipt.death_cell.runtime_scope!=fence.runtime_scope
                    || receipt.respawn_position!=position {return Ok(Err(CharacterProgressionError::AuthorityRejected));}
                Some(receipt)
            }else {None};
            commit_semantic_transaction(tx,deadline).await?;Ok(Ok(result))
        })).await?
    }
}
