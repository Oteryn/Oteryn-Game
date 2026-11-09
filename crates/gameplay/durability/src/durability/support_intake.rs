//! Anonymous support evidence; current session authority is checked but never stored.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_session_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_protocol_oteryn::support_intake as wire;
use sha2::{Digest, Sha256};
impl DurabilityRoot {
    pub async fn submit_support_evidence(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        kind: u32,
        payload: &[u8],
    ) -> Result<bool, DurabilityError> {
        let Ok(preview) = wire::decode_submission(payload) else {
            return Ok(false);
        };
        if !matches!(kind, wire::CRASH | wire::SURVEY) {
            return Ok(false);
        }
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let preview = preview.to_owned();
        let mut hash = Sha256::new();
        hash.update(kind.to_be_bytes());
        hash.update(preview.as_bytes());
        let receipt = hash.finalize().to_vec();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &record).await?;
            lock_admission_relations(&mut tx).await?;
            if assert_session_fence(&mut tx, &fence, &node).await?.is_err() { return Ok(false); }
            sqlx::query("INSERT INTO game_support_intake(receipt,kind,preview) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
                .bind(&receipt).bind(kind as i32).bind(&preview).execute(&mut *tx).await?;
            let matches: bool = sqlx::query_scalar("SELECT kind=$2 AND preview=$3 FROM game_support_intake WHERE receipt=$1")
                .bind(&receipt).bind(kind as i32).bind(&preview).fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(matches)
        })).await
    }
}
