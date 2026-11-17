use super::*;
use oteryn_session::ServerMessage;

fn message_event(sequence: u64, value: ServerMessage) -> SessionEvent {
    SessionEvent::ServerMessage(AppliedDelta {
        server_sequence: sequence,
        base_revision: sequence - 1,
        new_revision: sequence,
        value,
    })
}

#[test]
fn raids_global_client_dispatch_preserves_messages_across_step_and_renders_each_kind()
-> Result<(), String> {
    let (link, _commands, outbox) = play_channel();
    let status = ServerMessage::StatusText("Welcome".into());
    let warning = ServerMessage::Warning("A raid approaches.".into());
    let advance = ServerMessage::Advance("The creatures have arrived.".into());
    outbox.push(vec![message_event(1, status.clone())]);
    outbox.stepped(moved_to(100, 199, 0));
    outbox.push(vec![
        message_event(2, warning.clone()),
        message_event(3, advance.clone()),
    ]);
    let mut view = view().map_err(|error| error.to_string())?;
    let Some(PlayEvent::Pushed(before)) = link.poll() else {
        return Err("missing messages before step".into());
    };
    view.apply_pushed(&before)
        .map_err(|error| error.to_string())?;
    assert_eq!(view.server_message_status().as_deref(), Some("Welcome"));
    assert!(matches!(link.poll(), Some(PlayEvent::Stepped(_))));
    let Some(PlayEvent::Pushed(after)) = link.poll() else {
        return Err("missing messages after step".into());
    };
    assert_eq!(
        after
            .server_messages
            .iter()
            .map(|delta| &delta.value)
            .collect::<Vec<_>>(),
        vec![&warning, &advance]
    );
    view.apply_pushed(&after)
        .map_err(|error| error.to_string())?;
    assert_eq!(
        view.server_messages()
            .map(|delta| &delta.value)
            .collect::<Vec<_>>(),
        vec![&status, &warning, &advance]
    );
    assert_eq!(
        view.server_messages()
            .map(|delta| delta.server_sequence)
            .collect::<Vec<_>>(),
        vec![1, 2, 3]
    );
    assert_eq!(
        view.combat_status(),
        "spells 1-4 — Advance: The creatures have arrived."
    );
    assert!(link.poll().is_none());
    Ok(())
}

#[test]
fn raids_global_client_warning_is_visible_alongside_combat_feedback() -> Result<(), String> {
    let (link, _commands, outbox) = play_channel();
    let mut view = view().map_err(|error| error.to_string())?;
    view.apply_combat(CombatReport::Cast(SpellCastDisposition::NotEnoughMana));
    outbox.push(vec![message_event(
        1,
        ServerMessage::Warning("Beware!".into()),
    )]);
    assert_eq!(view.tick(&link), Ok(()));
    assert_eq!(
        view.combat_status(),
        "spells 1-4 Not enough mana — Warning: Beware!"
    );
    Ok(())
}

#[test]
fn raids_global_client_undrained_messages_fail_closed_at_the_existing_event_budget()
-> Result<(), String> {
    let limit = u64::try_from(MAX_QUEUED_EVENTS).map_err(|error| error.to_string())?;
    for crosses_step in [false, true] {
        let (link, _commands, outbox) = play_channel();
        for sequence in 1..=limit {
            outbox.push(vec![message_event(
                sequence,
                ServerMessage::Warning("Pending".into()),
            )]);
        }
        if crosses_step {
            outbox.stepped(moved_to(100, 199, 0));
        }
        outbox.push(vec![message_event(
            limit + 1,
            ServerMessage::Advance("Overflow".into()),
        )]);
        if crosses_step {
            assert!(matches!(link.poll(), Some(PlayEvent::Stepped(_))));
        }
        assert!(matches!(
            link.poll(),
            Some(PlayEvent::Ended(PublicClass::SessionUnavailable))
        ));
        assert!(link.poll().is_none());
    }
    Ok(())
}

#[test]
fn raids_global_client_history_is_bounded_and_new_connection_starts_empty() -> Result<(), String> {
    let limit = u64::try_from(MAX_QUEUED_EVENTS).map_err(|error| error.to_string())?;
    let (link, _commands, outbox) = play_channel();
    let mut current = view().map_err(|error| error.to_string())?;
    for sequence in 1..=limit + 1 {
        outbox.push(vec![message_event(
            sequence,
            ServerMessage::StatusText(sequence.to_string()),
        )]);
        assert_eq!(current.tick(&link), Ok(()));
    }
    assert_eq!(current.server_messages().count(), MAX_QUEUED_EVENTS);
    assert_eq!(
        current
            .server_messages()
            .next()
            .map(|delta| delta.server_sequence),
        Some(2)
    );
    assert_eq!(
        current.server_message_status(),
        Some((limit + 1).to_string())
    );
    let next = view().map_err(|error| error.to_string())?;
    assert_eq!(next.server_messages().count(), 0);
    assert_eq!(next.server_message_status(), None);
    Ok(())
}

#[test]
fn server_warning_wire_reaches_interactive_notice_and_changed_text_gets_new_key()
-> Result<(), String> {
    use oteryn_protocol_oteryn::zones_nologout::{decode_server_message, encode_server_message};
    let (link, _commands, outbox) = play_channel();
    let mut view = view().map_err(|error| error.to_string())?;
    let wire = encode_server_message(&ServerMessage::Warning("Balancing rules changed".into()))
        .map_err(|_| "encode")?;
    let message = decode_server_message(&wire).map_err(|_| "decode")?;
    outbox.push(vec![message_event(1, message)]);
    let Some(PlayEvent::Pushed(batch)) = link.poll() else {
        return Err("missing notice batch".into());
    };
    view.apply_pushed(&batch)
        .map_err(|error| error.to_string())?;
    let notices: Vec<_> = batch.server_messages.interactive_notices().collect();
    assert_eq!(notices.len(), 1);
    let first = notices.first().ok_or("missing notice")?;
    assert_eq!(first.message, "Balancing rules changed");
    let key = first.key.clone();
    outbox.push(vec![
        message_event(2, ServerMessage::Warning("Balancing rules changed".into())),
        message_event(3, ServerMessage::Warning("New balancing rules".into())),
        message_event(4, ServerMessage::StatusText("Ordinary HUD status".into())),
    ]);
    let Some(PlayEvent::Pushed(batch)) = link.poll() else {
        return Err("missing notice batch".into());
    };
    view.apply_pushed(&batch)
        .map_err(|error| error.to_string())?;
    let notices: Vec<_> = batch.server_messages.interactive_notices().collect();
    assert_eq!(notices.len(), 2);
    assert_eq!(notices[0].key, key);
    assert_ne!(notices[1].key, key);
    Ok(())
}

#[test]
fn t057_banner_filter_retains_server_notice() -> Result<(), String> {
    let (link, _, outbox)=play_channel();
    outbox.push(vec![message_event(1,ServerMessage::StatusText("Welcome".into()))]);
    let mut view=view().map_err(|error|error.to_string())?;
    let Some(PlayEvent::Pushed(delta))=link.poll() else {return Err("missing message".into());};
    view.apply_pushed(&delta).map_err(|error|error.to_string())?;
    let mut preferences=view.preferences().clone();
    preferences.game_window.banner=false;
    view.set_preferences(preferences).map_err(|error|error.to_string())?;
    assert!(view.server_message_status().is_none());
    assert_eq!(view.server_messages().count(),1);
    let mut preferences=view.preferences().clone();
    preferences.game_window.banner=true;
    view.set_preferences(preferences).map_err(|error|error.to_string())?;
    assert_eq!(view.server_message_status().as_deref(),Some("Welcome"));
    Ok(())
}
