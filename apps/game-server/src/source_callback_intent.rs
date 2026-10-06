//! Canonical AI proposal grammar, checked before callback timer/memo/speech mutation.
use crate::ai_think::profile_schedule::{ProfileAbilityProposal, ScheduleList};
use crate::creature_auto_attack::AttackError;
use crate::foundation::ChannelRuntimeV1;
pub(crate) fn validate(
    r: &ChannelRuntimeV1,
    p: &ProfileAbilityProposal,
) -> Result<(), AttackError> {
    let issuer = format!("actor:{}", hex(p.issuer.placement_identity()));
    let target = format!("actor:{}", hex(p.target.placement_identity()));
    if p.intent.proposal_source() != crate::ability::ProposalSource::Ai
        || p.intent.actor() != issuer
        || p.intent.candidate_count() != 1
        || p.intent.resolved_targets().len() != 1
        || p.intent.resolved_targets()[0].as_str() != target
    {
        return Err(AttackError::InvalidPlan);
    }
    let from = r
        .read_actor_position(p.issuer)
        .map_err(|_| AttackError::StaleIssuer)?
        .position();
    let to = r
        .read_actor_position(p.target)
        .map_err(|_| AttackError::StaleTarget)?
        .position();
    match p.list {
        ScheduleList::Defence if p.target != p.issuer => Err(AttackError::InvalidPlan),
        ScheduleList::Attack
            if from.floor != to.floor
                || (p.range_tiles != 0
                    && (i64::from(from.x) - i64::from(to.x))
                        .abs()
                        .max((i64::from(from.y) - i64::from(to.y)).abs())
                        > i64::from(p.range_tiles)) =>
        {
            Err(AttackError::OutOfRange)
        }
        _ => Ok(()),
    }
}
fn hex(v: [u8; 16]) -> String {
    v.iter().map(|b| format!("{b:02x}")).collect()
}
