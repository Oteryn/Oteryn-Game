//! Authenticated account selection for the read-only Premium overview. No grant writes.
use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use sqlx::Row;
impl DurabilityRoot {
    pub async fn premium_overview_account(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<[u8; 16], CharacterAuthorityError> {
        let recovery = authority.record_for(self)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if super::character_progression::assert_item_only_fence(&mut tx,&fence,&node).await?.is_err() {return Ok(Err(CharacterAuthorityError::Rejected));}
            let row=sqlx::query("SELECT uuid_send(account_id) AS account FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND lifecycle=1 AND NOT game_account_closure_detached(character_id) FOR SHARE")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let Some(row)=row else {return Ok(Err(CharacterAuthorityError::Rejected));};
            let account:Vec<u8>=row.try_get("account")?;
            let account=account.try_into().map_err(|_|DurabilityError::InvalidStoredState)?;
            commit_semantic_transaction(tx,deadline).await?;Ok(Ok(account))
        })).await?
    }
}
