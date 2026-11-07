use super::*;
use crate::gameplay_transport::actor_spell::tests::runtime_with_capacity;
use crate::gameplay_transport::chat_intent::ChatRuntime;
use oteryn_simulation_determinism::SemanticTimeMicros;
use std::error::Error;

fn session(tag: u8) -> Result<GameSessionId, Box<dyn Error>> {
    Ok(GameSessionId::decode(&[
        0x01, 0x90, 0x00, 0x00, 0x00, tag, 0x70, 0x00, 0x80, 0x00, 0, 0, 0, 0, 0, tag,
    ])?)
}

fn at(x: i32, y: i32, floor: i16) -> ActorPosition {
    ActorPosition { x, y, floor }
}

#[test]
fn player_typing_geometry_uses_donor_multifloor_ranges_and_projected_coordinates() {
    for floor in 0..=15 {
        let source = at(100, 100, floor);
        let (low, high) = match floor {
            0..=5 => (0, 7),
            6 => (0, 8),
            7 => (0, 9),
            _ => (floor - 2, (floor + 2).min(15)),
        };
        for observed_floor in 0..=15 {
            let offset = i32::from(floor - observed_floor);
            let included = (low..=high).contains(&observed_floor);
            assert_eq!(
                observes(source, at(100 + offset, 100 + offset, observed_floor)),
                included
            );
            for dx in [-11, 11] {
                for dy in [-11, 11] {
                    assert_eq!(
                        observes(
                            source,
                            at(100 + offset + dx, 100 + offset + dy, observed_floor)
                        ),
                        included
                    );
                }
            }
            assert!(!observes(
                source,
                at(112 + offset, 100 + offset, observed_floor)
            ));
            assert!(!observes(
                source,
                at(100 + offset, 88 + offset, observed_floor)
            ));
        }
        assert!(observes(source, source));
    }
}

#[test]
fn player_typing_geometry_rejects_invalid_floors_without_coordinate_overflow() {
    assert!(!observes(at(0, 0, -1), at(0, 0, 0)));
    assert!(!observes(at(0, 0, 0), at(0, 0, 16)));
    assert!(!observes(at(i32::MIN, 0, 7), at(i32::MAX, 0, 7)));
    assert!(observes(
        at(i32::MAX, i32::MIN, 7),
        at(i32::MAX, i32::MIN, 7)
    ));
}

#[test]
fn player_typing_broadcast_includes_speaker_and_selected_nearby_players_only()
-> Result<(), Box<dyn Error>> {
    let (mut runtime, source, source_session) = runtime_with_capacity(0x71, 4);
    runtime.initialize_first_entry_position(source)?;
    let near_session = session(0x72)?;
    let far_session = session(0x73)?;
    for recipient in [near_session, far_session] {
        let reservation = runtime.reserve_fresh_session(recipient)?;
        let actor = runtime.commit_fresh_session(reservation)?;
        runtime.initialize_first_entry_position(actor)?;
        if recipient == far_session {
            for _ in 0..12 {
                let position = runtime.read_actor_position(actor)?;
                let mut target = position.position();
                target.x += 1;
                runtime
                    .borrow_movement_position()
                    .commit_cardinal(position, target)?;
            }
        }
    }
    let typing = TypingRuntime::default();
    for recipient in [source_session, near_session, far_session] {
        typing.select(recipient, 1, true);
    }
    assert!(typing.broadcast(&runtime, source, source_session, 1, true));
    let own = typing.next(source_session, 1).ok_or("speaker update")?;
    assert!(own.typing);
    assert_eq!(own.speaker.identity, source.placement_identity());
    assert_eq!(typing.next(near_session, 1), Some(own));
    assert_eq!(typing.next(far_session, 1), None);
    assert_eq!(typing.next(source_session, 1), None);
    typing.select(near_session, 1, false);
    assert!(typing.broadcast(&runtime, source, source_session, 1, false));
    assert!(!typing.next(source_session, 1).ok_or("speaker stop")?.typing);
    assert_eq!(typing.next(near_session, 1), None);
    Ok(())
}

#[test]
fn player_typing_refuses_foreign_session_unselected_and_stale_connection()
-> Result<(), Box<dyn Error>> {
    let (mut runtime, actor, known) = runtime_with_capacity(0x71, 3);
    runtime.initialize_first_entry_position(actor)?;
    let foreign = session(0x72)?;
    let reservation = runtime.reserve_fresh_session(foreign)?;
    let other = runtime.commit_fresh_session(reservation)?;
    runtime.initialize_first_entry_position(other)?;
    let typing = TypingRuntime::default();
    assert!(!typing.broadcast(&runtime, actor, known, 1, true));
    typing.select(known, 2, true);
    typing.select(foreign, 2, true);
    assert!(!typing.broadcast(&runtime, actor, foreign, 2, true));
    assert!(!typing.broadcast(&runtime, other, known, 2, true));
    assert!(!typing.broadcast(&runtime, actor, known, 1, true));
    assert!(typing.broadcast(&runtime, actor, known, 2, true));
    assert_eq!(typing.next(known, 1), None);
    assert!(typing.next(known, 2).is_some());
    Ok(())
}

#[test]
fn player_typing_refuses_control_lost_speaker_without_broadcast() -> Result<(), Box<dyn Error>> {
    let (mut runtime, actor, known) = runtime_with_capacity(0x71, 3);
    runtime.initialize_first_entry_position(actor)?;
    let nearby = session(0x72)?;
    let reservation = runtime.reserve_fresh_session(nearby)?;
    let recipient = runtime.commit_fresh_session(reservation)?;
    runtime.initialize_first_entry_position(recipient)?;
    let typing = TypingRuntime::default();
    typing.select(known, 1, true);
    typing.select(nearby, 1, true);
    assert!(typing.broadcast(&runtime, actor, known, 1, true));
    assert!(
        typing
            .next(known, 1)
            .ok_or("speaker before control loss")?
            .typing
    );
    assert!(
        typing
            .next(nearby, 1)
            .ok_or("recipient before control loss")?
            .typing
    );

    runtime.record_control_loss(
        actor,
        known,
        crate::foundation::ControlLossMark {
            epoch: 1,
            grace_deadline: 100,
        },
    )?;
    for value in [false, true] {
        assert!(!typing.broadcast(&runtime, actor, known, 1, value));
        assert_eq!(typing.next(known, 1), None);
        assert_eq!(typing.next(nearby, 1), None);
    }

    runtime.restore_control(actor, known, 1)?;
    assert!(typing.broadcast(&runtime, actor, known, 1, false));
    assert!(
        !typing
            .next(known, 1)
            .ok_or("speaker after control recovery")?
            .typing
    );
    assert!(
        !typing
            .next(nearby, 1)
            .ok_or("recipient after control recovery")?
            .typing
    );
    Ok(())
}

#[test]
fn player_typing_coalesces_latest_state_and_reconnect_drops_pending_events()
-> Result<(), Box<dyn Error>> {
    let (mut runtime, actor, known) = runtime_with_capacity(0x71, 2);
    runtime.initialize_first_entry_position(actor)?;
    let typing = TypingRuntime::default();
    typing.select(known, 1, true);
    for value in [true, false, true, false] {
        assert!(typing.broadcast(&runtime, actor, known, 1, value));
    }
    assert!(!typing.next(known, 1).ok_or("latest stop")?.typing);
    assert_eq!(typing.next(known, 1), None);
    assert!(typing.broadcast(&runtime, actor, known, 1, true));
    typing.select(known, 2, true);
    assert_eq!(typing.next(known, 2), None);
    typing.select(known, 1, true);
    assert!(typing.broadcast(&runtime, actor, known, 2, false));
    assert!(
        !typing
            .next(known, 2)
            .ok_or("stale admission did not replace current")?
            .typing
    );
    Ok(())
}

#[test]
fn player_typing_chat_retirement_clears_recipients_and_departed_speaker_updates()
-> Result<(), Box<dyn Error>> {
    let (mut runtime, actor, first) = runtime_with_capacity(0x71, 3);
    runtime.initialize_first_entry_position(actor)?;
    let second = session(0x72)?;
    let reservation = runtime.reserve_fresh_session(second)?;
    let second_actor = runtime.commit_fresh_session(reservation)?;
    runtime.initialize_first_entry_position(second_actor)?;
    let chat = ChatRuntime::default();
    let now = SemanticTimeMicros::from_micros(1);
    chat.admit(first, "Aela", now);
    chat.admit(second, "Bryn", now);
    chat.typing.select(first, 1, true);
    chat.typing.select(second, 1, true);
    assert!(chat.typing.broadcast(&runtime, actor, first, 1, true));
    chat.forget(first);
    assert_eq!(chat.typing.next(first, 1), None);
    assert_eq!(chat.typing.next(second, 1), None);
    assert!(
        chat.typing
            .broadcast(&runtime, second_actor, second, 1, true)
    );
    chat.refresh(second);
    chat.typing.select(second, 2, true);
    assert_eq!(chat.typing.next(second, 2), None);
    assert!(
        !chat
            .typing
            .broadcast(&runtime, second_actor, second, 1, true)
    );
    Ok(())
}

#[test]
fn player_typing_delayed_chat_snapshot_cannot_downgrade_current_generation()
-> Result<(), Box<dyn Error>> {
    let (mut runtime, actor, known) = runtime_with_capacity(0x71, 2);
    runtime.initialize_first_entry_position(actor)?;
    let chat = ChatRuntime::default();
    let now = SemanticTimeMicros::from_micros(1);
    chat.admit(known, "Aela", now);
    chat.typing.select(known, 2, true);
    assert!(chat.typing.broadcast(&runtime, actor, known, 2, true));

    // House-transfer recovery can replace an active destination before its first
    // command. The old snapshot may reach generic chat admission after generation 2.
    chat.refresh(known);
    chat.typing.select(known, 1, true);
    assert!(
        chat.typing
            .next(known, 2)
            .ok_or("current update survived stale refresh")?
            .typing
    );
    assert!(!chat.typing.broadcast(&runtime, actor, known, 1, true));
    assert!(chat.typing.broadcast(&runtime, actor, known, 2, false));

    chat.admit(known, "Aela", now);
    chat.typing.select(known, 1, true);
    assert!(
        !chat
            .typing
            .next(known, 2)
            .ok_or("current update survived stale admission")?
            .typing
    );
    assert!(!chat.typing.broadcast(&runtime, actor, known, 1, true));
    assert!(chat.typing.broadcast(&runtime, actor, known, 2, true));

    // The actual new generation owns queue cleanup and replaces the old selection.
    chat.refresh(known);
    chat.typing.select(known, 3, true);
    assert_eq!(chat.typing.next(known, 3), None);
    assert!(!chat.typing.broadcast(&runtime, actor, known, 2, true));
    assert!(chat.typing.broadcast(&runtime, actor, known, 3, false));
    Ok(())
}

#[test]
fn player_typing_deselection_fences_delayed_older_selection_and_disables_delivery()
-> Result<(), Box<dyn Error>> {
    let (mut runtime, actor, known) = runtime_with_capacity(0x71, 3);
    runtime.initialize_first_entry_position(actor)?;
    let nearby = session(0x72)?;
    let reservation = runtime.reserve_fresh_session(nearby)?;
    let recipient = runtime.commit_fresh_session(reservation)?;
    runtime.initialize_first_entry_position(recipient)?;
    let typing = TypingRuntime::default();
    typing.admit(&runtime, actor, known, 1, true);
    typing.admit(&runtime, recipient, nearby, 1, true);
    assert!(typing.broadcast(&runtime, actor, known, 1, true));

    typing.admit(&runtime, actor, known, 2, false);
    assert_eq!(typing.next(known, 1), None);
    assert_eq!(typing.next(known, 2), None);
    assert_eq!(typing.next(nearby, 1), None);
    typing.admit(&runtime, actor, known, 1, true);
    assert!(!typing.broadcast(&runtime, actor, known, 1, true));
    assert!(!typing.broadcast(&runtime, actor, known, 2, true));
    assert!(typing.broadcast(&runtime, recipient, nearby, 1, true));
    assert!(
        typing
            .next(nearby, 1)
            .ok_or("selected recipient still receives")?
            .typing
    );
    assert_eq!(typing.next(known, 1), None);
    assert_eq!(typing.next(known, 2), None);
    {
        let sessions = typing.0.lock().map_err(|_| "typing lock")?;
        assert_eq!(sessions.len(), 2);
        let disabled = sessions.get(&known).ok_or("disabled generation entry")?;
        assert_eq!(disabled.connection_generation, 2);
        assert!(!disabled.selected);
        assert!(disabled.pending.is_empty());
    }
    typing.forget(known);
    let sessions = typing.0.lock().map_err(|_| "typing lock")?;
    assert_eq!(sessions.len(), 1);
    assert!(!sessions.contains_key(&known));
    Ok(())
}

#[test]
fn player_typing_admission_refuses_lost_control_and_late_retired_snapshot()
-> Result<(), Box<dyn Error>> {
    let (mut runtime, actor, known) = runtime_with_capacity(0x71, 2);
    runtime.initialize_first_entry_position(actor)?;
    let chat = ChatRuntime::default();
    runtime.record_control_loss(
        actor,
        known,
        crate::foundation::ControlLossMark {
            epoch: 1,
            grace_deadline: 100,
        },
    )?;
    for selected in [false, true] {
        chat.typing.admit(&runtime, actor, known, 1, selected);
    }
    assert!(chat.typing.0.lock().map_err(|_| "typing lock")?.is_empty());

    runtime.restore_control(actor, known, 1)?;
    chat.typing.admit(&runtime, actor, known, 2, true);
    assert!(chat.typing.broadcast(&runtime, actor, known, 2, true));
    // Production retirement removes the exact actor before forgetting session state.
    runtime.remove_terminal_session(known, actor)?;
    chat.forget(known);
    for generation in [1, 2] {
        for selected in [false, true] {
            chat.typing
                .admit(&runtime, actor, known, generation, selected);
        }
        assert_eq!(chat.typing.next(known, generation), None);
        assert!(
            !chat
                .typing
                .broadcast(&runtime, actor, known, generation, true)
        );
    }
    assert!(chat.typing.0.lock().map_err(|_| "typing lock")?.is_empty());
    Ok(())
}
