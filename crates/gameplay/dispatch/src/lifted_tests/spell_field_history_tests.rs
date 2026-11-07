#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crate::spell::field_history::*;
#[test]
fn genuine_actor_history_obeys_owned_timeout_and_different_session_is_not_same_attacker() {
    let (_, source, session) =
        crate::gameplay_transport::actor_spell::tests::runtime_with_player(81);
    let mut history = FieldAttackHistory::default();
    assert_eq!(
        history.has_been_attacked(source, session, 1, 1000, 1000),
        Some(false)
    );
    history
        .record_staged_player_hit(source, session, 1, 1000)
        .unwrap();
    assert_eq!(
        history.has_been_attacked(source, session, 1, 1_001_000, 1000),
        Some(true)
    );
    assert_eq!(
        history.has_been_attacked(source, session, 1, 1_001_001, 1000),
        Some(false)
    );
    assert_eq!(
        history.has_been_attacked(source, session, 2, 1000, 1000),
        Some(false)
    );
    assert_eq!(
        history.has_been_attacked(source, session, 1, 999, 1000),
        None
    );
    history.clear_on_lifecycle();
    assert_eq!(
        history.has_been_attacked(source, session, 1, 1000, 1000),
        Some(false)
    );
}
