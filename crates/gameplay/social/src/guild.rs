//! World-scoped Guild jobs. Each candidate revalidates current recovery and node assignment.
use crate::domain::guild::GuildId;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::guild::{
    GuildDeadlineOutcome, GuildDisbandJobRequest, GuildDisbandOutcome, GuildError,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::WorldId;

mod deadlines;
mod disband;
pub use deadlines::{GuildDeadlinePass, run_guild_deadline_pass};
pub use disband::{GuildDisbandPass, run_guild_disband_pass};

/// One scheduler tick with bounded discovery and a fair pending cursor. Jobs do not acquire
/// Character session authority or infer it from retained operation receipts.
pub async fn run_guild_world_pass(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    world: WorldId,
    cursor: &mut Option<GuildId>,
) -> Result<(), GuildError> {
    run_guild_deadline_pass(root, authority, node, world).await?;
    let pending = root
        .pending_guild_disbands_after(authority, node, world, *cursor)
        .await?;
    if pending.is_empty() {
        *cursor = None;
        return Ok(());
    }
    let last = pending.last().copied();
    let requests = pending
        .into_iter()
        .map(|guild_id| GuildDisbandJobRequest {
            world_id: world,
            guild_id,
        })
        .collect();
    run_guild_disband_pass(root, authority, node, world, requests).await?;
    *cursor = last;
    Ok(())
}
