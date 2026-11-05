use crate::*;
use oteryn_protocol_oteryn::pvp_expert as wire;
pub(crate) fn apply<S: SessionStream>(
    session: &mut Session<S>,
    sequence: u64,
    payload: &[u8],
) -> Result<SessionEvent, SessionError> {
    if !session.is_selected(wire::CAPABILITY_PVP_EXPERT_V1) {
        return Err(SessionError::UnselectedDomain {
            domain_id: wire::STATE_DOMAIN_EXPERT_FIGHT_MODES,
        });
    }
    let delta = decode_state_delta(payload)?;
    let entry = session
        .gated_revisions
        .iter_mut()
        .find(|(id, _)| *id == wire::STATE_DOMAIN_EXPERT_FIGHT_MODES)
        .ok_or(SessionError::MissingDomain(
            wire::STATE_DOMAIN_EXPERT_FIGHT_MODES,
        ))?;
    check_delta(&delta, 1, entry.1)?;
    let value = wire::decode_expert_fight_modes(delta.payload)
        .map_err(|_| SessionError::SessionUnusable)?;
    entry.1 = delta.new_revision;
    Ok(SessionEvent::ExpertFightModes(AppliedDelta {
        server_sequence: sequence,
        base_revision: delta.base_revision,
        new_revision: delta.new_revision,
        value,
    }))
}

pub(crate) fn validate_join(
    selected: &[u32],
    snapshots: &[GatedSnapshot],
) -> Result<(), SessionError> {
    if !selected.contains(&wire::CAPABILITY_PVP_EXPERT_V1) {
        return Ok(());
    }
    let snapshot = snapshots
        .iter()
        .find(|s| s.domain_id == wire::STATE_DOMAIN_EXPERT_FIGHT_MODES)
        .ok_or(SessionError::MissingDomain(
            wire::STATE_DOMAIN_EXPERT_FIGHT_MODES,
        ))?;
    if snapshot.snapshot_type != 1 || snapshot.revision == 0 {
        return Err(SessionError::SessionUnusable);
    }
    wire::decode_expert_fight_modes(&snapshot.payload)
        .map_err(|_| SessionError::SessionUnusable)?;
    Ok(())
}
