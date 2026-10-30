//! Hotkey equipment through the session's capability-gated command sequencing.
use oteryn_session::{ItemMoveOutcome, Session, SessionError, SessionStream};
use std::num::NonZeroU32;

pub async fn equip_definition<S: SessionStream>(
    session: &mut Session<S>,
    definition: NonZeroU32,
) -> Result<ItemMoveOutcome, SessionError> {
    Ok(session.equip_item_definition(definition).await?.outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_session::hotkey_equip;

    #[test]
    fn hotkey_equip_client_decodes_public_refusal_and_success() -> Result<(), SessionError> {
        for (wire, expected) in [
            (1, ItemMoveOutcome::Moved),
            (5, ItemMoveOutcome::NoRoom),
            (10, ItemMoveOutcome::SlotMismatch),
            (11, ItemMoveOutcome::RequirementNotMet),
            (9, ItemMoveOutcome::Rejected),
        ] {
            assert_eq!(
                hotkey_equip::decode_reply(&[8, wire]).map_err(SessionError::ItemView)?,
                expected
            );
        }
        assert!(hotkey_equip::decode_reply(&[8, 0]).is_err());
        assert!(hotkey_equip::decode_reply(&[8, 1, 8, 1]).is_err());
        Ok(())
    }
}
