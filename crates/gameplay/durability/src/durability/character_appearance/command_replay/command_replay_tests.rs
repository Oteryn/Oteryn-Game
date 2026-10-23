use crate::durability::character_appearance::AppearanceChangeOccurrence;
use crate::foundation::{CommandId, CommandRef, GameSessionId};

#[test]
fn appearance_command_replay_identity_is_stable_and_scoped()
-> Result<(), Box<dyn std::error::Error>> {
    let command = |session, number| -> Result<_, Box<dyn std::error::Error>> {
        let mut bytes = [session; 16];
        bytes[6] = 0x70;
        bytes[8] = 0x80;
        Ok(CommandRef::new(
            GameSessionId::decode(&bytes)?,
            CommandId::new(number)?,
        ))
    };
    let first = AppearanceChangeOccurrence::for_command(command(1, 7)?);
    assert_eq!(
        first,
        AppearanceChangeOccurrence::for_command(command(1, 7)?)
    );
    assert_ne!(
        first,
        AppearanceChangeOccurrence::for_command(command(1, 8)?)
    );
    assert_ne!(
        first,
        AppearanceChangeOccurrence::for_command(command(2, 7)?)
    );
    assert_eq!(
        AppearanceChangeOccurrence::from_bytes(*first.as_bytes())?,
        first
    );
    Ok(())
}
