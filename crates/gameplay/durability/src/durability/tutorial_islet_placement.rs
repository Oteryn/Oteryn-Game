//! Settle a first-login islet placement together with the existing durable last position.
//! The pending bit survives a crash between the progress CAS and Channel placement.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_last_position::{
    FinalLastPosition, LastPositionKey, LastPositionWrite, upsert_last_position,
};
use super::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence, assert_gameplay_fence,
};
use super::character_tutorial::{TutorialProgressOutcome, tutorial_progress_in_transaction};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_gameplay_quests::tutorial::ISLET_TEMPLE;

impl DurabilityRoot {
    /// `placed` is captured by the admitted owner after placement. Stale sessions and
    /// another pending obligation cannot acknowledge it or overwrite the saved position.
    pub async fn complete_tutorial_islet_placement(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        placed: FinalLastPosition,
    ) -> Result<bool, CharacterProgressionError> {
        let cell = placed.record.cell;
        if placed.record.world_id != fence.runtime_scope.world_id()
            || cell.floor != ISLET_TEMPLE.2
            || cell.x.abs_diff(ISLET_TEMPLE.0) > 3
            || cell.y.abs_diff(ISLET_TEMPLE.1) > 3
            || !oteryn_gameplay_content::content::newhaven_tutorial_land::contains(
                cell.x, cell.y, cell.floor,
            )
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error)=assert_gameplay_fence(&mut tx,&fence,&node).await? {return Ok(Err(error));}
            let TutorialProgressOutcome::Read(Some(stored))=tutorial_progress_in_transaction(&mut tx,fence.character_id,None).await? else {return Ok(Ok(false));};
            if !stored.pending_islet {return Ok(Ok(false));}
            let version=stored.version.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
            if upsert_last_position(&mut tx,fence.character_id.as_bytes(),LastPositionKey {character_lease_generation:fence.character_lease_generation,write_sequence:placed.write_sequence},&placed.record).await? !=LastPositionWrite::Written {
                return Err(DurabilityError::InvalidStoredState);
            }
            sqlx::query("UPDATE game_character_tutorial SET version=$2,pending_islet=FALSE WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).bind(version).execute(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(true))
        })).await?
    }
}
