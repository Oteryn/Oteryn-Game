//! Resolve immutable Content before entering the deciding SQL transaction. The
//! decision still independently rechecks current configuration and activation.
use super::*;
use std::sync::Arc;
use std::time::Duration;

// A stalled artifact or signing-key read cannot accumulate blocking workers or prevent
// the command consumer from returning PENDING and draining retained receipts.
static ARTIFACT_READ: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(1);

#[derive(Clone)]
pub(super) struct PreparedContent {
    path: String,
    digest: String,
    artifact: Arc<items::PreparedArtifact>,
    signing_path: Option<String>,
    signing_kid: Option<String>,
    signing_key: Option<Arc<ed25519_dalek::SigningKey>>,
}
impl PreparedContent {
    pub(super) fn bind(&self, policy: &mut Policy) {
        if policy.content_artifact_file.as_deref() == Some(&self.path)
            && policy.content_artifact_sha256.as_deref() == Some(&self.digest)
        {
            policy.preloaded_artifact = Some(self.artifact.clone());
            if policy.signing_key_file == self.signing_path
                && policy.signing_kid == self.signing_kid
            {
                policy.preloaded_signing_key = self.signing_key.clone();
            }
        }
    }
}
impl DurabilityRoot {
    pub(super) async fn prepare_bazaar_content(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        proof: &NodeIncarnationProof,
        needs_signing_key: bool,
    ) -> Option<PreparedContent> {
        let recovery = authority.record_for(self).ok()?;
        let proof = proof.clone();
        let policy = self
            .try_issue_semantic_pass()
            .ok()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    if !prove_current_incarnation(&mut tx, &proof).await? {
                        return Err(DurabilityError::Unavailable);
                    }
                    let raw: String = sqlx::query_scalar(
                        "SELECT policy FROM game_character_service_bazaar_policy()",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let policy: Policy = serde_json::from_str(&raw)
                        .map_err(|_| DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(policy)
                })
            })
            .await
            .ok()?;
        let path = policy.content_artifact_file.clone()?;
        let digest = policy.content_artifact_sha256.clone()?;
        let signing_path = policy.signing_key_file.clone();
        let signing_kid = policy.signing_kid.clone();
        let read = async move {
            let permit = ARTIFACT_READ.acquire().await.ok()?;
            oteryn_task_supervisor::spawn_blocking_lifecycle_supervised(
                "bazaar_artifact_preload",
                None,
                move || {
                    let _permit = permit;
                    let artifact = items::preload_artifact(&policy)?;
                    let key = if needs_signing_key {
                        policy
                            .signing_key_file
                            .as_deref()
                            .and_then(characters::private_seed)
                            .map(|seed| Arc::new(ed25519_dalek::SigningKey::from_bytes(&seed)))
                    } else {
                        None
                    };
                    Some((artifact, key))
                },
            )
            .await
            .ok()?
        };
        let (artifact, signing_key) = tokio::time::timeout(Duration::from_secs(10), read)
            .await
            .ok()??;
        Some(PreparedContent {
            path,
            digest,
            artifact,
            signing_path,
            signing_kid,
            signing_key,
        })
    }
}
