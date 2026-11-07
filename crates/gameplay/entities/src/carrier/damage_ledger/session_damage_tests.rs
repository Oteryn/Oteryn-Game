use super::*;

fn uuid(seed: u8) -> [u8; 16] {
    let mut bytes = [0; 16];
    bytes[0] = 1;
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes[15] = seed;
    bytes
}

#[test]
fn current_session_damage_reconnect_resets_ai_without_losing_rewards() -> Result<(), String> {
    let character = CharacterId::decode(&uuid(1)).map_err(|error| format!("{error:?}"))?;
    let first = GameSessionId::decode(&uuid(2)).map_err(|error| format!("{error:?}"))?;
    let second = GameSessionId::decode(&uuid(3)).map_err(|error| format!("{error:?}"))?;
    let mut ledger = DamageContributors::default();
    ledger.record(character, 100, 1, Some((1, first, 1, 0)), Some(10));
    ledger.record(character, 20, 2, Some((1, first, 2, 0)), Some(20));
    assert_eq!(
        (
            ledger.entries[0].total,
            ledger.entries[0].current_session_total
        ),
        (120, 120)
    );
    ledger.record(character, 7, 3, Some((2, second, 1, 0)), Some(30));
    assert_eq!(
        (
            ledger.entries[0].total,
            ledger.entries[0].current_session_total
        ),
        (127, 7)
    );
    ledger.record(character, 3, 4, Some((2, second, 2, 0)), Some(40));
    assert_eq!(
        (
            ledger.entries[0].total,
            ledger.entries[0].current_session_total
        ),
        (130, 10)
    );
    assert_eq!(ledger.high_water(character), Some((2, second, 2, 0)));
    Ok(())
}

#[test]
fn current_session_damage_does_not_invent_a_session_for_unsequenced_credit() -> Result<(), String> {
    let character = CharacterId::decode(&uuid(1)).map_err(|error| format!("{error:?}"))?;
    let session = GameSessionId::decode(&uuid(2)).map_err(|error| format!("{error:?}"))?;
    let mut ledger = DamageContributors::default();
    ledger.record(character, 80, 1, None, Some(10));
    assert_eq!(
        (
            ledger.entries[0].total,
            ledger.entries[0].current_session_total
        ),
        (80, 0)
    );
    ledger.record(character, 5, 2, Some((1, session, 1, 0)), Some(20));
    ledger.record(character, 40, 3, None, Some(30));
    assert_eq!(
        (
            ledger.entries[0].total,
            ledger.entries[0].current_session_total
        ),
        (125, 5)
    );
    assert_eq!(ledger.high_water(character), Some((1, session, 1, 0)));
    Ok(())
}

#[test]
fn current_session_subtotal_does_not_change_reward_winner() -> Result<(), String> {
    let first_character = CharacterId::decode(&uuid(1)).map_err(|error| format!("{error:?}"))?;
    let second_character = CharacterId::decode(&uuid(2)).map_err(|error| format!("{error:?}"))?;
    let session = GameSessionId::decode(&uuid(3)).map_err(|error| format!("{error:?}"))?;
    let reconnected = GameSessionId::decode(&uuid(4)).map_err(|error| format!("{error:?}"))?;
    let mut ledger = DamageContributors::default();
    ledger.record(first_character, 100, 1, Some((1, session, 1, 0)), Some(10));
    ledger.record(second_character, 60, 2, None, Some(20));
    ledger.record(
        first_character,
        1,
        3,
        Some((2, reconnected, 1, 0)),
        Some(30),
    );
    assert_eq!(ledger.top_damage_character(), Some(first_character));
    assert_eq!(
        (
            ledger.entries[0].total,
            ledger.entries[0].current_session_total
        ),
        (101, 1)
    );
    Ok(())
}
