use super::super::connection::{FirstEntryOutcome, SessionContinuity};
use super::*;
use crate::foundation::{ChannelId, GameSessionId, WorldId};

#[test]
fn party_membership_handles_are_bounded_and_session_scoped()
-> Result<(), Box<dyn std::error::Error>> {
    let mut id = [0; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    let mut admitted = AdmittedSession {
        game_session_id: GameSessionId::decode(&id)?,
        world_id: WorldId::decode(&id)?,
        channel_id: ChannelId::decode(&id)?,
        runtime_actor: None,
        first_entry: FirstEntryOutcome::NotApplicable,
        controller: None,
        continuity: SessionContinuity::FRESH,
        item_fence: None,
    };
    let mut view = SessionPartyView::default();
    let first = view.handle(admitted, 1, id).ok_or("member handle")?;
    assert_eq!(view.resolve(1, first), Some(id));
    assert_eq!(view.resolve(2, first), None);
    assert_eq!(SessionPartyView::default().resolve(1, first), None);
    let invite = view.handle(admitted, 2, id).ok_or("invitation handle")?;
    assert_ne!(first, invite);
    admitted.continuity.connection_generation += 1;
    let resumed = SessionPartyView::default()
        .handle(admitted, 1, id)
        .ok_or("new generation")?;
    assert_ne!(first, resumed);
    admitted.continuity.connection_generation -= 1;
    let mut saturated = SessionPartyView::default();
    for value in 0..MAX_HANDLES {
        let key = (value as u128).to_be_bytes();
        assert!(saturated.handle(admitted, 1, key).is_some());
    }
    assert!(
        saturated
            .handle(admitted, 1, u128::MAX.to_be_bytes())
            .is_none()
    );
    assert!(
        saturated
            .handle(admitted, 1, 0_u128.to_be_bytes())
            .is_some()
    );
    assert_eq!(saturated.handles.len(), MAX_HANDLES);
    Ok(())
}
