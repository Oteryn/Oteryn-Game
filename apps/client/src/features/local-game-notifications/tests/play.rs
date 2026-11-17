use super::*;
#[test]
fn game_notices_follow_real_delivery_configuration_expiry_and_reconnect() -> Result<(), String> {
    let (link, _commands, outbox) = play_channel();
    let mut view = view().map_err(|e| e.to_string())?;
    for (sequence, message) in [
        (
            1,
            oteryn_session::ServerMessage::StatusText("World event begins".into()),
        ),
        (
            2,
            oteryn_session::ServerMessage::Warning("Seek shelter".into()),
        ),
    ] {
        outbox.push(vec![SessionEvent::ServerMessage(AppliedDelta {
            server_sequence: sequence,
            base_revision: sequence - 1,
            new_revision: sequence,
            value: message,
        })]);
    }
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    assert_eq!(view.notification_text(false), "World event begins\r\n");
    assert_eq!(view.notification_text(true), "Seek shelter\r\n");
    view.configure_notifications(false, true);
    assert!(view.notification_text(false).is_empty());
    assert!(view.notification_text(true).contains("Seek shelter"));
    let chord = oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
        .with(oteryn_input_actions::Modifier::Alt)
        .with(oteryn_input_actions::Modifier::Shift);
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(68).map_err(|e| e.to_string())?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: chord,
        repeat: false,
    };
    assert!(view.notification_input(&[event]));
    assert!(view.notification_text(true).is_empty());
    view.configure_notifications(true, true);
    let later = std::time::Instant::now() + Duration::from_secs(61);
    assert!(view.game_notifications.text_at(false, later).is_empty());
    assert!(view.game_notifications.text_at(true, later).is_empty());
    let next =
        PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[]).map_err(|e| e.to_string())?;
    assert!(next.notification_text(false).is_empty());
    assert!(next.notification_text(true).is_empty());
    Ok(())
}
