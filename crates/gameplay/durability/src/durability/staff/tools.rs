//! NX-1 durable enforcement and captured world effects. Every live-session access
//! re-proves the exact recovery, node, lease, connection and runtime-scope fence.
use super::*;
use crate::foundation::GameSessionId;
use sqlx::PgConnection;
use std::{future::Future, pin::Pin};

mod inspect;
mod live;
mod live_fence;
mod world;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StaffLiveStatus {
    pub banned: bool,
    pub kicked: bool,
    pub ghost: bool,
    pub invisible: bool,
    /// Donor groups 4 GM, 5 CM, 6 God; zero is an ordinary player.
    pub group: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaffWorldEffectTarget {
    pub session: GameSessionId,
    pub connection_generation: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StaffWorldEffectState {
    Pending,
    Applied,
    Refused,
}
type QueryFuture<'a, T> = Pin<Box<dyn Future<Output = Result<T, DurabilityError>> + Send + 'a>>;

impl DurabilityRoot {
    async fn with_staff_tools_fence<T, F>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        query: F,
    ) -> Result<T, DurabilityError>
    where
        T: Send + 'static,
        F: for<'a> FnOnce(
                &'a mut PgConnection,
                CurrentCharacterItemFence,
                Option<String>,
                Option<String>,
            ) -> QueryFuture<'a, T>
            + Send
            + 'static,
    {
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        let trust = self.staff_trust();
        let environment = trust.as_ref().map(|trust| trust.environment().to_owned());
        let activation = trust.as_ref().map(|trust| trust.activation().to_owned());
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !live_fence::current(&mut tx, &fence, &node).await? {
                        return Err(DurabilityError::Unavailable);
                    }
                    let result = query(&mut tx, fence, environment, activation).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
}
