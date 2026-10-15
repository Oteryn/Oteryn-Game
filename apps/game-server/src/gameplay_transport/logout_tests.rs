//! LOGOUT-WIRE-1 (ARCH-KILL-REWARD-LOGOUT-1 §1.6, §2.2): the in-fight retry and the terminal
//! release of a logout.

use super::logout::in_fight_retry_after_ms;

#[test]
fn the_in_fight_retry_rounds_up_to_whole_milliseconds_within_1_to_60000() {
    assert_eq!(in_fight_retry_after_ms(1_000_001, 1_000_000), 1);
    assert_eq!(in_fight_retry_after_ms(1_001_000, 1_000_000), 1);
    assert_eq!(in_fight_retry_after_ms(1_001_001, 1_000_000), 2);
    assert_eq!(in_fight_retry_after_ms(61_000_000, 1_000_000), 60_000);
    assert_eq!(in_fight_retry_after_ms(u64::MAX, 0), 60_000);
    // A deadline that already ran out between the read and the reply still says 1.
    assert_eq!(in_fight_retry_after_ms(1_000_000, 1_000_000), 1);
}

/// #2127 Codex P1 4278730847: a logout's familiar save is proven only by an ACTIVE row on
/// the exact current transport; a grace-expiry save keeps its exact-epoch proof.
#[test]
fn a_familiar_actor_end_save_is_proven_only_for_its_own_end() -> Result<(), String> {
    use super::actor_spell::{FamiliarActorEnd, familiar_end_proven};
    use crate::foundation::{AuthenticatedTransportRefV1, GameSessionState};

    let ours = AuthenticatedTransportRefV1::decode(&[1; 16]).map_err(|e| format!("{e:?}"))?;
    let other = AuthenticatedTransportRefV1::decode(&[2; 16]).map_err(|e| format!("{e:?}"))?;
    let logout = FamiliarActorEnd::Logout(ours, 1);
    let active = GameSessionState::Active;
    assert!(familiar_end_proven(logout, active, None, Some(ours)));
    assert!(!familiar_end_proven(logout, active, None, Some(other)));
    assert!(!familiar_end_proven(logout, active, None, None));
    // A resumed ACTIVE row keeps its last control-loss epoch.
    assert!(familiar_end_proven(logout, active, Some(1), Some(ours)));
    assert!(!familiar_end_proven(logout, active, Some(1), Some(other)));
    assert!(!familiar_end_proven(
        logout,
        GameSessionState::Reconnectable,
        None,
        Some(ours)
    ));
    assert!(!familiar_end_proven(
        logout,
        GameSessionState::Terminal,
        None,
        Some(ours)
    ));

    let grace = FamiliarActorEnd::GraceExpiry(3);
    let lost = GameSessionState::Reconnectable;
    assert!(familiar_end_proven(grace, lost, Some(3), None));
    assert!(!familiar_end_proven(grace, lost, Some(4), None));
    assert!(!familiar_end_proven(
        grace,
        GameSessionState::Terminal,
        Some(3),
        None
    ));
    Ok(())
}

/// #2127 Codex P2 4279467193: each logout attempt saves the familiar under its own occurrence,
/// so a retry after a `BUSY` whose save committed is no conflicting replay.
#[test]
fn each_logout_attempt_has_its_own_familiar_occurrence() -> Result<(), String> {
    use super::actor_spell::FamiliarActorEnd;
    use crate::foundation::{
        AuthenticatedTransportRefV1, ChannelId, ExactActorRef, GameSessionId, WorldId,
    };

    let uuid_v7 = |tag: u8| {
        let mut bytes = [tag; 16];
        bytes[6] = 0x70 | (tag & 0x0f);
        bytes[8] = 0x80 | (tag & 0x3f);
        bytes
    };
    let actor = ExactActorRef::transport_fixture(
        WorldId::decode(&uuid_v7(0x33)).map_err(|e| format!("{e:?}"))?,
        ChannelId::decode(&uuid_v7(0x44)).map_err(|e| format!("{e:?}"))?,
    );
    let session = GameSessionId::decode(&uuid_v7(0x55)).map_err(|e| format!("{e:?}"))?;
    let ours = AuthenticatedTransportRefV1::decode(&[1; 16]).map_err(|e| format!("{e:?}"))?;
    let first = FamiliarActorEnd::Logout(ours, 1).occurrence(actor, session);
    assert_eq!(
        first,
        FamiliarActorEnd::Logout(ours, 1).occurrence(actor, session)
    );
    assert_ne!(
        first,
        FamiliarActorEnd::Logout(ours, 2).occurrence(actor, session)
    );
    Ok(())
}
