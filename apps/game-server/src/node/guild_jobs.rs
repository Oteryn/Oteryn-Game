//! GUILD0 §3.3 World deadline pass, awaiting CP scheduler registration.
//!
//! Discovery is unlocked; each candidate obtains fresh recovery/admission evidence and its
//! own Guild lock in a separate native transaction. Checkpoints and private publication DATA
//! survive restart. D1098 resumes the actual Hall→Bank→bounded cleanup controller, committing
//! each Guild independently. Missing genuine House disposition is a typed wait, never closure.
use crate::domain::guild::GuildId;
use crate::durability::DurabilityRoot;
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::guild::{
    GuildDeadlineOutcome, GuildDisbandJobRequest, GuildDisbandOutcome, GuildError,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::WorldId;

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct GuildDeadlinePass {
    pub started: Vec<GuildId>,
    pub already_started: usize,
    pub no_longer_due: usize,
}

/// Recovery/admission are the accepted World-job boundary, not a Character session.
/// `node` proves the process only; no new World ownership grant is inferred from it.
pub(crate) async fn run_guild_deadline_pass(
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

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct GuildDisbandPass {
    pub outcomes: Vec<(GuildId, GuildDisbandOutcome)>,
}

/// The scheduler supplies stable operation IDs before the first attempt, from the bounded
/// pending page. These IDs are DATA: every candidate independently obtains current authority
/// and all actual owner facts. A failure preserves earlier committed Guild steps for restart.
pub(crate) async fn run_guild_disband_pass(
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
