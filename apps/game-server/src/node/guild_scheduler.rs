//! GUILD-1: run the real bounded World pass for the node serve lifetime.
use crate::durability::{
    DurabilityRoot, character_authority::ReconciledCharacterAuthority,
    runtime_scope_assignment::NodeIncarnationProof,
};
use crate::foundation::WorldId;
use oteryn_foundation::CancellationToken;
use std::time::Duration;

pub(super) async fn run(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    world: WorldId,
    stop: &CancellationToken,
) {
    let mut cursor = None;
    loop {
        if let Some(result) = super::cancellation::first(
            super::guild_jobs::run_guild_world_pass(root, authority, node, world, &mut cursor),
            stop.cancelled(),
        )
        .await
        {
            if let Err(error) = result {
                eprintln!("event=guild_world_pass state=retry error={error}");
            }
        } else {
            return;
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
