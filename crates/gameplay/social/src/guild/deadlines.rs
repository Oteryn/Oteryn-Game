use super::*;
#[derive(Debug, Default, PartialEq, Eq)]
pub struct GuildDeadlinePass {
    pub started: Vec<GuildId>,
    pub already_started: usize,
    pub no_longer_due: usize,
}

/// Recovery/admission are the accepted World-job boundary, not a Character session.
/// `node` proves the process only; no new World ownership grant is inferred from it.
pub async fn run_guild_deadline_pass(
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    world: WorldId,
) -> Result<GuildDeadlinePass, GuildError> {
    let candidates = root
        .guild_deadline_candidates(authority, node, world)
        .await?;
    let mut pass = GuildDeadlinePass::default();
    for guild in candidates {
        match root
            .start_guild_deadline_disband(authority, node, world, guild)
            .await?
        {
            GuildDeadlineOutcome::Started => pass.started.push(guild),
            GuildDeadlineOutcome::AlreadyStarted => pass.already_started += 1,
            GuildDeadlineOutcome::NoLongerDue => pass.no_longer_due += 1,
        }
    }
    Ok(pass)
}
