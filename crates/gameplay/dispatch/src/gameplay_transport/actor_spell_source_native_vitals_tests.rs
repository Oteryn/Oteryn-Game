#![allow(
    clippy::panic,
    reason = "test-only native outcome destructuring asserts impossible fixture variants; same scope as adjacent canonical owner tests"
)]
use super::tests::{runtime_with_player, wound};
use super::*;
use crate::player_lethal::PlayerLethalVitals;
fn ready(
    tag: u8,
) -> (
    ChannelRuntimeV1,
    ChannelSpellStates,
    ExactActorRef,
    GameSessionId,
) {
    let (mut runtime, actor, session) = runtime_with_player(tag);
    assert!(
        runtime
            .initialize_movement_test_position(
                actor,
                MovementLocalPosition {
                    x: 10,
                    y: 10,
                    floor: 7
                }
            )
            .is_ok()
    );
    let mut states = ChannelSpellStates::default();
    let facts = CharacterCastFacts {
        vocation: crate::spell::Vocation::Druid,
        level: 30,
        magic_level: 20,
        max_health: 185,
        max_mana: 90,
        max_soul: 100,
    };
    assert!(
        states
            .initialize(
                &runtime,
                actor,
                session,
                facts,
                (0, 0),
                SemanticTimeMicros::from_micros(0)
            )
            .is_some()
    );
    (runtime, states, actor, session)
}
#[test]
fn source_lethal_receipt_reuses_native_death_and_respawn_clears_source_replay() {
    let (mut r, mut s, a, g) = ready(0x41);
    wound(&mut s, a, g, 8);
    let Some(hit) = s.apply_attack_damage(
        &mut r,
        a,
        g,
        8,
        "source:lethal:0",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("native source hit")
    };
    let Some(native) = s.player_death(&r, a, g) else {
        panic!("native death")
    };
    let Some(receipt) = hit.death else {
        panic!("receipt")
    };
    assert_eq!(receipt.occurrence_bytes(), *native.occurrence.as_bytes());
    assert_eq!(receipt.position(), native.cell);
    assert_eq!(hit.health_after, 0);
    assert_eq!(
        s.apply_attack_damage(
            &mut r,
            a,
            g,
            8,
            "source:lethal:0",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        Some(hit)
    );
    assert_eq!(
        s.apply_attack_damage(
            &mut r,
            a,
            g,
            9,
            "source:lethal:0",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(s.deaths.len(), 1);
    assert_eq!(
        s.apply_source_player_heal(
            &mut r,
            a,
            g,
            1,
            "heal:dead",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(
        s.apply_attack_mana_drain(
            &mut r,
            a,
            g,
            1,
            "mana:dead",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert!(
        s.respawn(
            &r,
            a,
            g,
            native.occurrence,
            SemanticTimeMicros::from_micros(1)
        )
        .is_some()
    );
    assert!(s.source_damage.is_empty());
    assert!(s.source_mana.is_empty());
    assert!(s.source_heal.is_empty());
    assert!(
        s.apply_attack_damage(
            &mut r,
            a,
            g,
            8,
            "source:lethal:1",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        )
        .is_some()
    );
    assert!(!s.is_dead(a));
}
#[test]
fn source_failed_native_mint_and_wrong_session_have_no_hp_or_memo_write() {
    let (mut r, mut s, a, g) = ready(0x42);
    wound(&mut s, a, g, 8);
    s.mint_death = || None;
    let before = s.get(&r, a, g).cloned();
    assert_eq!(
        s.apply_attack_damage(
            &mut r,
            a,
            g,
            8,
            "mint:failed",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(s.get(&r, a, g).cloned(), before);
    assert!(s.source_damage.is_empty());
    assert!(s.deaths.is_empty());
    let (_, _, other) = runtime_with_player(0x43);
    assert_eq!(
        s.apply_attack_damage(
            &mut r,
            a,
            other,
            1,
            "session:wrong",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(
        s.apply_source_player_heal(
            &mut r,
            a,
            other,
            1,
            "session:wrong",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(
        s.apply_attack_mana_drain(
            &mut r,
            a,
            other,
            1,
            "session:wrong",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
    assert_eq!(s.get(&r, a, g).cloned(), before);
}
#[test]
fn source_heal_uses_current_maximum_native_credit_noop_revision_and_replay() {
    let (mut r, mut s, a, g) = ready(0x44);
    wound(&mut s, a, g, 100);
    let before = s.get(&r, a, g).cloned();
    let Some(heal) = s.apply_source_player_heal(
        &mut r,
        a,
        g,
        u32::MAX,
        "heal:0",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("native credit")
    };
    assert_eq!(
        (
            heal.applied,
            heal.health_after,
            heal.max_health,
            heal.vitals_revision
        ),
        (85, 185, 185, 2)
    );
    let Some(next) = s.get(&r, a, g).cloned() else {
        panic!("current")
    };
    let Some(before) = before else {
        panic!("before")
    };
    assert_eq!(
        (before.vitals().mana, before.vitals().soul),
        (next.vitals().mana, next.vitals().soul)
    );
    assert_eq!(
        s.apply_source_player_heal(
            &mut r,
            a,
            g,
            u32::MAX,
            "heal:0",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        Some(heal)
    );
    assert_eq!(s.get(&r, a, g), Some(&next));
    let Some(full) = s.apply_source_player_heal(
        &mut r,
        a,
        g,
        1,
        "heal:1",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("full credit")
    };
    assert_eq!((full.applied, full.vitals_revision), (0, 2));
    let Some(zero) = s.apply_source_player_heal(
        &mut r,
        a,
        g,
        0,
        "heal:2",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("zero")
    };
    assert_eq!((zero.applied, zero.vitals_revision), (0, 2));
}
#[test]
fn source_mp_only_drain_clamps_current_pool_replays_and_exhaustion_keeps_revision() {
    let (mut r, mut s, a, g) = ready(0x45);
    let Some(before) = s.get(&r, a, g).cloned() else {
        panic!("before")
    };
    let Some(drain) = s.apply_attack_mana_drain(
        &mut r,
        a,
        g,
        u32::MAX,
        "mana:0",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("MP debit")
    };
    assert_eq!(
        (
            drain.applied,
            drain.mana_before,
            drain.mana_after,
            drain.vitals_revision
        ),
        (90, 90, 0, 2)
    );
    assert_eq!(
        s.apply_attack_mana_drain(
            &mut r,
            a,
            g,
            u32::MAX,
            "mana:0",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        Some(drain)
    );
    let Some(next) = s.get(&r, a, g) else {
        panic!("current")
    };
    assert_eq!(
        (before.vitals().health, before.vitals().soul),
        (next.vitals().health, next.vitals().soul)
    );
    assert!(!s.is_dead(a));
    let Some(empty) = s.apply_attack_mana_drain(
        &mut r,
        a,
        g,
        1,
        "mana:1",
        crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0),
    ) else {
        panic!("empty MP")
    };
    assert_eq!((empty.applied, empty.vitals_revision), (0, 2));
    assert_eq!(
        s.apply_attack_mana_drain(
            &mut r,
            a,
            g,
            2,
            "mana:1",
            crate::foundation::owner_timer::SemanticTimeMicros::from_micros(0)
        ),
        None
    );
}
