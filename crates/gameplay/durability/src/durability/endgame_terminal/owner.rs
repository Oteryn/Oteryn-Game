use super::*;
use sqlx::{Postgres, Transaction};
use std::future::Future;
use std::pin::Pin;

mod sealed {
    pub trait Sealed {}
}

/// Opaque transaction-bound evidence. W02's adapter must prove its existing
/// retained terminal record before constructing this value inside this module.
/// There is no public constructor and no client-accessible implementation.
pub struct ProvenTerminalOutcome {
    transaction_id: String,
    authority_binding: [u8; 32],
}
impl ProvenTerminalOutcome {
    pub(super) async fn matches(
        &self,
        tx: &mut Transaction<'_, Postgres>,
        request: &EndgameFailureRequest,
    ) -> Result<bool> {
        let current: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut **tx)
            .await
            .map_err(DurabilityError::from)?;
        Ok(current == self.transaction_id
            && self.authority_binding == request.authority_binding()?)
    }
}

/// Sealed composition boundary for the existing shared boss owner. The actual
/// adapter is deliberately absent on this base, awaiting coordinator composition
/// of W02. A timeout or unsuccessful terminal must be owned/proved there.
pub trait SharedTerminalOwner: sealed::Sealed + Send + Sync {
    fn prove<'a, 'db>(
        &'a self,
        tx: &'a mut Transaction<'db, Postgres>,
        request: &'a EndgameFailureRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ProvenTerminalOutcome>> + Send + 'a>>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct UnavailableSharedTerminalOwner;
impl UnavailableSharedTerminalOwner {
    pub(super) fn admission(&self) -> Result<ProvenTerminalOutcome> {
        Err(EndgameTerminalError::ContentUnavailable)
    }
}
impl sealed::Sealed for UnavailableSharedTerminalOwner {}
impl SharedTerminalOwner for UnavailableSharedTerminalOwner {
    fn prove<'a, 'db>(
        &'a self,
        _tx: &'a mut Transaction<'db, Postgres>,
        _request: &'a EndgameFailureRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ProvenTerminalOutcome>> + Send + 'a>> {
        Box::pin(async move { self.admission() })
    }
}

#[cfg(any(test, feature = "test-support"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TestOwnerState {
    Active,
    IndividualDeath,
    Disconnected,
    Victory,
    TerminalUnsuccessful,
}
#[cfg(any(test, feature = "test-support"))]
pub struct TestSharedTerminalOwner {
    request: EndgameFailureRequest,
    state: TestOwnerState,
}
#[cfg(any(test, feature = "test-support"))]
impl TestSharedTerminalOwner {
    pub fn new(mut request: EndgameFailureRequest, state: TestOwnerState) -> Self {
        request
            .participants
            .sort_by_key(|member| member.character_id);
        Self { request, state }
    }
    pub(super) fn admitted(&self, request: &EndgameFailureRequest) -> Result<()> {
        if self.state != TestOwnerState::TerminalUnsuccessful {
            return Err(EndgameTerminalError::NotTerminalUnsuccessful);
        }
        if self.request.authority_binding()? != request.authority_binding()? {
            return Err(EndgameTerminalError::AuthorityRejected);
        }
        Ok(())
    }
}
#[cfg(any(test, feature = "test-support"))]
impl sealed::Sealed for TestSharedTerminalOwner {}
#[cfg(any(test, feature = "test-support"))]
impl SharedTerminalOwner for TestSharedTerminalOwner {
    fn prove<'a, 'db>(
        &'a self,
        tx: &'a mut Transaction<'db, Postgres>,
        request: &'a EndgameFailureRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ProvenTerminalOutcome>> + Send + 'a>> {
        Box::pin(async move {
            self.admitted(request)?;
            let transaction_id = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
                .fetch_one(&mut **tx)
                .await
                .map_err(DurabilityError::from)?;
            Ok(ProvenTerminalOutcome {
                transaction_id,
                authority_binding: request.authority_binding()?,
            })
        })
    }
}
