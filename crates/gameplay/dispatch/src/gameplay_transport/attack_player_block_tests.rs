//! SHIELD-DEFENCE-1: a player's block budget on creature hits, on the attack entries.

use super::*;

#[test]
fn global_parity_player_defence_uses_crystals_offensive_swing_interval() {
    use oteryn_gameplay_combat_rules::attack::formulas::defence_mode_factor;

    let mut owner = owner();
    let (player, session, rat) = (owner.player, owner.session, owner.rat);
    let constants = AttackConstants::checked_in().unwrap();
    let interval = constants.attack_interval_micros();
    assert_eq!(owner.target(T0, Some(rat), 1), AttackIntentDisposition::Ok);
    let rat = owner.runtime.visible_entities().creatures[0].actor;
    let facts = target_facts(&owner.runtime, &owner.room, player, session, rat, at(T0)).unwrap();
    let entry = owner.attack.entry(&owner.runtime, player, session).unwrap();
    let _ = entry.state.poll_swing(at(T0), interval, &facts);

    for (fight_mode, now, expected) in [
        (WireFightMode::Defensive, T0 + interval - 1, 0.5),
        (WireFightMode::Balanced, T0 + interval, 1.0),
    ] {
        assert!(owner.attack.set_modes(
            &owner.runtime,
            player,
            session,
            FightModes {
                fight_mode,
                ..FightModes::DEFAULT
            },
        ));
        let (mode, swung) = owner
            .attack
            .take_player_block(&owner.runtime, player, session, at(now))
            .unwrap();
        assert_eq!(defence_mode_factor(constants, mode, swung), expected);
    }
}

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
    let blocked = Some((FightMode::Offensive, false));
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
