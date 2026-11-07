//! S15a: Crystal 201189f8 `config.lua.dist:61` `loginProtectionPeriod = 10 * 1000` and
//! `player.cpp:6434-6436` `checkLoginDelay` (`OTSYS_TIME() <= lastLoad + LOGIN_PROTECTION`),
//! read by monster target selection only (`monster.cpp:475`, `:1014`). `lastLoad` is set at a
//! fresh login (`protocolgame.cpp:834`).
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
use super::*;
use crate::ControlLossMark;
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
use oteryn_gameplay_foundation::foundation::{ChannelId, NodeId, WorldId};

fn id(n: u8) -> [u8; 16] {
    [1, 0, 0, 0, 0, n, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, n]
}

fn runtime() -> ChannelRuntimeV1 {
    let world = WorldId::decode(&id(1)).unwrap();
    ChannelRuntimeV1::from_committed_assignment(
        world,
        ChannelId::decode(&id(2)).unwrap(),
        NodeId::decode(&id(3)).unwrap(),
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        4,
        ChannelContentPin::test(world),
    )
    .unwrap()
}

#[test]
fn fresh_login_hides_the_player_from_monsters_for_ten_seconds_only() {
    let mut runtime = runtime();
    let session = GameSessionId::decode(&id(4)).unwrap();
    let reserved = runtime.reserve_fresh_session(session).unwrap();
    let actor = runtime
        .commit_fresh_session_at(reserved, 1_000_000)
        .unwrap();
    assert_eq!(FRESH_LOGIN_PROTECTION_MICROS, 10_000_000);
    // Monster target selection skips the player through the whole period.
    for now in [1_000_000, 10_999_999] {
        assert_eq!(
            runtime.current_player_monster_target_protection(actor, session, now),
            Ok(true)
        );
    }
    // Crystal compares `<=`; Oteryn owner times are microseconds, so the first excluded
    // microsecond is the boundary.
    assert_eq!(
        runtime.current_player_monster_target_protection(actor, session, 11_000_001),
        Ok(false)
    );
    // The login period is not the reconnect window: the player's own attacks, spells and
    // fields are not refused (Crystal reads `checkLoginDelay` only in monster targeting).
    assert_eq!(
        runtime.current_player_reentry_protection(actor, session, 1_000_000),
        Ok(false)
    );
    let wrong = GameSessionId::decode(&id(5)).unwrap();
    assert!(
        runtime
            .current_player_monster_target_protection(actor, wrong, 1_000_000)
            .is_err()
    );
}

#[test]
fn reconnect_keeps_its_four_second_window_beside_the_login_period() {
    let mut runtime = runtime();
    let session = GameSessionId::decode(&id(4)).unwrap();
    let reserved = runtime.reserve_fresh_session(session).unwrap();
    // A plain commit (no login clock) grants nothing.
    let actor = runtime.commit_fresh_session(reserved).unwrap();
    assert_eq!(
        runtime.current_player_monster_target_protection(actor, session, 0),
        Ok(false)
    );
    runtime
        .record_control_loss(
            actor,
            session,
            ControlLossMark {
                epoch: 1,
                grace_deadline: 100,
            },
        )
        .unwrap();
    runtime
        .restore_control_after_committed_reentry(actor, session, 1, 100)
        .unwrap();
    // S15b stays four seconds (owner decision).
    assert_eq!(
        runtime.current_player_monster_target_protection(actor, session, 4_000_099),
        Ok(true)
    );
    assert_eq!(
        runtime.current_player_monster_target_protection(actor, session, 4_000_100),
        Ok(false)
    );
}
