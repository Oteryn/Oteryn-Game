use super::*;
use crate::durability::{
    DurabilityError,
    character_stash::container::{Request, ResultView},
};
use oteryn_protocol_oteryn::container_batch::{
    COMMAND_TYPE_STASH_CONTAINER, Request as WireRequest,
};
use std::num::NonZeroU64;

struct ReceiptAuthority(Cell<Option<DurabilityError>>);

impl FreshAdmissionAuthority for ReceiptAuthority {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        Some(at(0))
    }

    async fn stash_container(
        &self,
        _: AdmittedSession,
        _: u64,
        request: Request,
    ) -> Result<Option<ResultView>, DurabilityError> {
        assert!(matches!(request, Request::Confirm { quote: None, .. }));
        self.0.take().map_or(Ok(None), Err)
    }
}

#[test]
fn stash_container_receipt_commit_unknown_without_quote_stays_outstanding()
-> Result<(), Box<dyn Error>> {
    run(async {
        let admitted = positioned()?;
        let payload = WireRequest::Confirm {
            preview: NonZeroU64::MIN,
        }
        .encode();
        let command = oteryn_protocol_oteryn::ClientCommandView {
            command_id: 2,
            command_type: COMMAND_TYPE_STASH_CONTAINER,
            payload: &payload,
        };
        for error in [
            DurabilityError::CommitOutcomeUnknown,
            DurabilityError::CommitTransactionTimeout,
        ] {
            let authority = ReceiptAuthority(Cell::new(Some(error)));
            let result =
                routing::stash_container::route(&authority, admitted, command, None, &mut None)
                    .await;
            assert!(
                result.is_none(),
                "an unresolved receipt must stay outstanding"
            );
        }

        let authority = ReceiptAuthority(Cell::new(Some(DurabilityError::Unavailable)));
        let result =
            routing::stash_container::route(&authority, admitted, command, None, &mut None).await;
        assert!(matches!(result, Some(routing::Dispatch::Unregistered)));
        Ok(())
    })
}
