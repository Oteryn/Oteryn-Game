//! Resolve immutable Content before entering the deciding SQL transaction. The
//! decision still independently rechecks current configuration and activation.
use super::*;
use std::sync::Arc;
use std::time::Duration;

// A stalled local artifact read cannot accumulate blocking workers or prevent
// the command consumer from returning PENDING and draining retained receipts.
static ARTIFACT_READ: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

#[derive(Clone)]
pub(super) struct PreparedContent {
    path: String,
    digest: String,
    artifact: Arc<items::PreparedArtifact>,
}
impl PreparedContent {
    pub(super) fn bind(&self, policy: &mut Policy) {
        if policy.content_artifact_file.as_deref() == Some(&self.path)
            && policy.content_artifact_sha256.as_deref() == Some(&self.digest)
        {
            policy.preloaded_artifact = Some(self.artifact.clone());
        }
    }
}
impl DurabilityRoot {
    pub(super) async fn prepare_bazaar_content(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
    ) -> Option<PreparedContent> {
        let recovery = authority.record_for(self).ok()?;
        let proof = proof.clone();
        let policy=self.try_issue_semantic_pass().ok()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
            if !prove_current_incarnation(&mut tx,&proof).await?{return Err(DurabilityError::Unavailable);}
            let raw:String=sqlx::query_scalar("SELECT policy::text FROM game_bazaar_configuration WHERE singleton FOR SHARE").fetch_one(&mut *tx).await?;
            let policy:Policy=serde_json::from_str(&raw).map_err(|_|DurabilityError::InvalidStoredState)?;
            commit_semantic_transaction(tx,deadline).await?;Ok(policy)
        })).await.ok()?;
        let path = policy.content_artifact_file.clone()?;
        let digest = policy.content_artifact_sha256.clone()?;
        let read = async move {
            let permit = ARTIFACT_READ.acquire().await.ok()?;
            oteryn_task_supervisor::spawn_blocking_lifecycle_supervised(
                "bazaar_artifact_preload",
                None,
                move || {
                    let _permit = permit;
                    items::preload_artifact(&policy)
                },
            )
            .await
            .ok()?
        };
        let artifact = tokio::time::timeout(Duration::from_secs(10), read)
            .await
            .ok()??;
        Some(PreparedContent {
            path,
            digest,
            artifact,
        })
    }
}
