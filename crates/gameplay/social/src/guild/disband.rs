use super::*;
#[derive(Debug, Default, PartialEq, Eq)]
pub struct GuildDisbandPass {
    pub outcomes: Vec<(GuildId, GuildDisbandOutcome)>,
}

/// The scheduler supplies stable operation IDs before the first attempt, from the bounded
/// pending page. These IDs are DATA: every candidate independently obtains current authority
/// and all actual owner facts. Root allocates each fresh C reservation with the native UUIDv7
/// allocator and recovers the stable C work operation/cursor from its own committed journal.
/// IDs/cursors are DATA only. Errors (including C resnapshot/NOWAIT) roll back the whole current
/// transaction; any retry re-enters the same occurrence→recipient roots→Guild order.
/// A failure preserves earlier committed Guild steps for restart.
pub async fn run_guild_disband_pass(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    world: WorldId,
    requests: Vec<GuildDisbandJobRequest>,
) -> Result<GuildDisbandPass, GuildError> {
    if requests.len() > 100
        || requests.iter().any(|request| request.world_id != world)
        || requests
            .windows(2)
            .any(|pair| pair[0].guild_id >= pair[1].guild_id)
    {
        return Err(GuildError::InvalidInput);
    }
    let mut pass = GuildDisbandPass::default();
    for request in requests {
        let guild = request.guild_id;
        let outcome = root.advance_guild_disband(authority, node, request).await?;
        pass.outcomes.push((guild, outcome));
    }
    Ok(pass)
}
