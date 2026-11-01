use super::*;
use crate::domain::{CharacterId, CharacterRevision};
use crate::foundation::{
    ChannelId, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration, WorldId,
};

fn id(seed: u8) -> [u8; 16] {
    let mut id = [seed; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
fn member(seed: u8) -> Result<CurrentCharacterGameplayFence> {
    Ok(CurrentCharacterGameplayFence {
        character_id: CharacterId::from_bytes(id(seed))
            .map_err(|_| EndgameTerminalError::InvalidInput)?,
        game_session_id: GameSessionId::decode(&id(seed + 10))
            .map_err(|_| EndgameTerminalError::InvalidInput)?,
        connection_generation: ConnectionGeneration::new(1)
            .map_err(|_| EndgameTerminalError::InvalidInput)?,
        character_lease_generation: 2,
        runtime_scope: RuntimeScopeRefV1::channel(
            WorldId::decode(&id(50)).map_err(|_| EndgameTerminalError::InvalidInput)?,
            ChannelId::decode(&id(51)).map_err(|_| EndgameTerminalError::InvalidInput)?,
        ),
        scope_ownership_generation: ScopeOwnershipGeneration::new(1)
            .map_err(|_| EndgameTerminalError::InvalidInput)?,
        expected_character_revision: CharacterRevision::new(7)
            .map_err(|_| EndgameTerminalError::InvalidInput)?,
    })
}
fn request() -> Result<EndgameFailureRequest> {
    let mut request = EndgameFailureRequest {
        attempt: [73; 32],
        encounter: FinalEncounter::Megalomania,
        participants: vec![member(1)?, member(2)?],
    };
    request.canonicalize()?;
    Ok(request)
}

#[test]
fn endgame_terminal_pair_is_one_retained_distinct_subset_for_the_group() -> Result<()> {
    for encounter in [FinalEncounter::Megalomania, FinalEncounter::Bakragore] {
        let mut seen = std::collections::BTreeSet::new();
        for seed in 1..=255 {
            let pair = choose_pair(&[seed; 32], encounter, encounter.mask())?
                .ok_or(EndgameTerminalError::InvalidInput)?;
            assert_eq!(pair.count_ones(), 2);
            assert_eq!(pair & encounter.mask(), pair);
            assert_eq!(
                choose_pair(&[seed; 32], encounter, encounter.mask())?,
                Some(pair)
            );
            seen.insert(pair);
        }
        assert_eq!(
            seen.len(),
            if encounter == FinalEncounter::Megalomania {
                10
            } else {
                6
            }
        );
    }
    let group = [15_u8, 11, 7];
    let common = group.into_iter().fold(15, |all, flags| all & flags);
    let pair = choose_pair(&[7; 32], FinalEncounter::Bakragore, common)?
        .ok_or(EndgameTerminalError::InvalidInput)?;
    assert_eq!(
        pair, 3,
        "the same pair must have been completed by every participant"
    );
    assert!(group.iter().all(|flags| flags & pair == pair));
    Ok(())
}

#[test]
fn endgame_terminal_empty_current_chain_is_a_retained_noop_decision() -> Result<()> {
    for common in [0, 1, 3, 7, 15] {
        assert_eq!(
            choose_pair(&[5; 32], FinalEncounter::Megalomania, common)?,
            None
        );
    }
    assert_eq!(choose_pair(&[5; 32], FinalEncounter::Bakragore, 0)?, None);
    assert_eq!(choose_pair(&[5; 32], FinalEncounter::Bakragore, 8)?, None);
    assert!(matches!(
        choose_pair(&[5; 32], FinalEncounter::Bakragore, 16),
        Err(EndgameTerminalError::InvalidInput)
    ));
    Ok(())
}

#[test]
fn endgame_terminal_owner_seam_refuses_absence_death_disconnect_and_victory() -> Result<()> {
    assert!(matches!(
        UnavailableSharedTerminalOwner.admission(),
        Err(EndgameTerminalError::ContentUnavailable)
    ));
    let request = request()?;
    for state in [
        TestOwnerState::Active,
        TestOwnerState::IndividualDeath,
        TestOwnerState::Disconnected,
        TestOwnerState::Victory,
    ] {
        let owner = TestSharedTerminalOwner::new(request.clone(), state);
        assert!(matches!(
            owner.admitted(&request),
            Err(EndgameTerminalError::NotTerminalUnsuccessful)
        ));
    }
    let owner = TestSharedTerminalOwner::new(request.clone(), TestOwnerState::TerminalUnsuccessful);
    owner.admitted(&request)?;
    let mut foreign = request.clone();
    foreign.attempt = [74; 32];
    assert!(matches!(
        owner.admitted(&foreign),
        Err(EndgameTerminalError::AuthorityRejected)
    ));
    foreign = request.clone();
    foreign.participants[0].character_lease_generation += 1;
    assert!(matches!(
        owner.admitted(&foreign),
        Err(EndgameTerminalError::AuthorityRejected)
    ));
    Ok(())
}

#[test]
fn endgame_terminal_roster_is_bounded_unique_and_same_scope() -> Result<()> {
    let base = request()?;
    let mut wrong = base.clone();
    wrong.participants.clear();
    assert!(matches!(
        wrong.canonicalize(),
        Err(EndgameTerminalError::InvalidInput)
    ));
    wrong = base.clone();
    wrong.attempt = [0; 32];
    assert!(matches!(
        wrong.canonicalize(),
        Err(EndgameTerminalError::InvalidInput)
    ));
    wrong = base.clone();
    wrong.participants.push(wrong.participants[0]);
    assert!(matches!(
        wrong.canonicalize(),
        Err(EndgameTerminalError::InvalidInput)
    ));
    wrong = base.clone();
    wrong.participants = (1..=6).map(member).collect::<Result<Vec<_>>>()?;
    assert!(matches!(
        wrong.canonicalize(),
        Err(EndgameTerminalError::InvalidInput)
    ));
    wrong = base.clone();
    wrong.participants[0].scope_ownership_generation =
        ScopeOwnershipGeneration::new(2).map_err(|_| EndgameTerminalError::InvalidInput)?;
    assert!(matches!(
        wrong.canonicalize(),
        Err(EndgameTerminalError::InvalidInput)
    ));
    wrong = base.clone();
    wrong.participants[0].runtime_scope = RuntimeScopeRefV1::channel(
        WorldId::decode(&id(50)).map_err(|_| EndgameTerminalError::InvalidInput)?,
        ChannelId::decode(&id(52)).map_err(|_| EndgameTerminalError::InvalidInput)?,
    );
    assert!(matches!(
        wrong.canonicalize(),
        Err(EndgameTerminalError::InvalidInput)
    ));
    Ok(())
}

#[test]
fn endgame_terminal_historical_binding_never_becomes_current_authority() -> Result<()> {
    let original = request()?;
    let mut retry = original.clone();
    retry.participants.reverse();
    retry.canonicalize()?;
    assert_eq!(original.binding()?, retry.binding()?);
    assert_eq!(original.authority_binding()?, retry.authority_binding()?);
    retry.participants[0].character_lease_generation += 1;
    retry.participants[0].expected_character_revision =
        CharacterRevision::new(10).map_err(|_| EndgameTerminalError::InvalidInput)?;
    assert_eq!(
        original.binding()?,
        retry.binding()?,
        "replay returns its original recorded decision"
    );
    assert_ne!(
        original.authority_binding()?,
        retry.authority_binding()?,
        "a fresh mutation needs current owner proof"
    );
    retry.participants[0] = member(3)?;
    retry.canonicalize()?;
    assert_ne!(
        original.binding()?,
        retry.binding()?,
        "a different roster cannot reuse an attempt"
    );
    Ok(())
}
