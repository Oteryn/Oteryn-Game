//! Bounded house-rent collection for the node's serve lifetime.
use crate::durability::{
    DurabilityRoot, character_authority::ReconciledCharacterAuthority,
    runtime_scope_assignment::NodeIncarnationProof,
};
use crate::foundation::RuntimeScopeRefV1;
use oteryn_foundation::CancellationToken;
use std::time::Duration;

pub(super) async fn run(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    scope: RuntimeScopeRefV1,
    generation: u64,
    stop: &CancellationToken,
) {
    loop {
        let mut span = super::scheduler_diagnostics::job("house_rent_world_pass");
        let mut advanced = 0_u64;
        let mut failed = false;
        for _ in 0..100 {
            let Some(result) = super::cancellation::first(
                root.advance_house_rent(authority, node, scope, generation),
                stop.cancelled(),
            )
            .await
            else {
                span.set_outcome("cancelled");
                return;
            };
            match result {
                Ok(true) => advanced += 1,
                Ok(false) => break,
                Err(_) => {
                    failed = true;
                    span.set_outcome("retry");
                    span.set_code(
                        super::scheduler_diagnostics::SchedulerErrorKind::JobFailed.code(),
                    );
                    break;
                }
            }
        }
        span.field("advanced", advanced);
        if !failed {
            span.set_outcome("ok");
        }
        if super::cancellation::first(
            tokio::time::sleep(Duration::from_secs(60)),
            stop.cancelled(),
        )
        .await
        .is_none()
        {
            return;
        }
    }
}
