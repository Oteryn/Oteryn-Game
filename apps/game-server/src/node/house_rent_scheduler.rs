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
        for _ in 0..100 {
            let Some(result) = super::cancellation::first(
                root.advance_house_rent(authority, node, scope, generation),
                stop.cancelled(),
            )
            .await
            else {
                return;
            };
            match result {
                Ok(true) => {}
                Ok(false) => break,
                Err(error) => {
                    eprintln!("event=house_rent_world_pass state=retry error={error}");
                    break;
                }
            }
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
