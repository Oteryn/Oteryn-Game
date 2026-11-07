//! SHIELD-DEFENCE-1: a player's block budget on creature hits, on the attack entries.

use super::*;

#[test]
fn a_player_blocks_two_creature_hits_then_refills_one_per_second() {
    let mut owner = owner();
    let (player, session) = (owner.player, owner.session);
    let modes = FightModes {
        fight_mode: WireFightMode::Defensive,
        ..FightModes::DEFAULT
    };
    assert!(
        owner
            .attack
            .set_modes(&owner.runtime, player, session, modes)
    );
    let mut take = |now| {
        owner
            .attack
            .take_player_block(&owner.runtime, player, session, at(now))
    };
    let blocked = Some((FightMode::Defensive, false));
    assert_eq!(take(T0), blocked);
    assert_eq!(take(T0), blocked);
    assert_eq!(take(T0 + 999_999), None, "two blocks, then armor only");
    assert_eq!(take(T0 + 1_000_000), blocked);
    assert_eq!(take(T0 + 1_000_000), None);
}

#[test]
fn a_session_that_does_not_control_the_actor_takes_no_block() {
    let mut owner = owner();
    let stranger = GameSessionId::decode(&uuid(9)).unwrap();
    assert_eq!(
        owner
            .attack
            .take_player_block(&owner.runtime, owner.player, stranger, at(T0)),
        None
    );
}
