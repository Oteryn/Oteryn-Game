use super::*;
use oteryn_client_runtime::player_typing;

#[test]
fn server_wire_updates_add_and_clear_the_exact_actor_generation() -> Result<(), player_typing::Error>
{
    let bytes = [
        10, 16, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 16, 9, 24, 1,
    ];
    let mut panel = PlayerTypingPanel::default();
    protocol::reduce(&mut panel, player_typing::decode_update(&bytes)?);
    assert_eq!(panel.indicators.len(), 1);
    assert_eq!(panel.indicators[0].identity, [7; 16]);
    assert!(panel_text(&panel).contains("Visible typing players: 1"));
    let mut off = bytes;
    off[21] = 0;
    protocol::reduce(&mut panel, player_typing::decode_update(&off)?);
    assert!(panel.indicators.is_empty());
    assert!(panel_text(&panel).contains("Visible typing players: 0"));
    Ok(())
}

#[test]
fn composer_coalesces_text_and_stops_even_with_start_in_flight() {
    let mut panel = PlayerTypingPanel {
        selected: true,
        ..Default::default()
    };
    let now = std::time::Instant::now();
    assert_eq!(
        composer(&mut panel, true, "h", now).map(|c| c.typing),
        Some(true)
    );
    assert!(composer(&mut panel, true, "hello", now).is_none());
    assert_eq!(
        composer(&mut panel, false, "hello", now).map(|c| c.typing),
        Some(false)
    );
    assert!(composer(&mut panel, true, "hello", now).is_none());
    assert_eq!(
        composer(
            &mut panel,
            true,
            "hello",
            now + std::time::Duration::from_millis(500)
        )
        .map(|c| c.typing),
        Some(true)
    );
    assert_eq!(
        composer(&mut panel, true, "", now).map(|c| c.typing),
        Some(false)
    );
    panel.selected = false;
    assert!(composer(&mut panel, true, "hi", now).is_none());
}

#[test]
fn composer_results_preserve_an_in_flight_manual_request() -> Result<(), &'static str> {
    let mut panel = PlayerTypingPanel::default();
    panel.initialize(true);
    let manual = request(&mut panel, false).ok_or("manual request")?;
    panel.feedback = Some("manual feedback");
    let command =
        composer(&mut panel, true, "hello", std::time::Instant::now()).ok_or("composer request")?;
    for status in [
        oteryn_client_runtime::CommandStatus::Accepted,
        oteryn_client_runtime::CommandStatus::Rejected,
    ] {
        apply_result(&mut panel, command, PlayerTypingResponse { status });
        assert!(panel.pending);
        assert_eq!(panel.feedback, Some("manual feedback"));
    }
    assert!(panel.own_typing);
    apply_result(
        &mut panel,
        manual,
        PlayerTypingResponse {
            status: oteryn_client_runtime::CommandStatus::Accepted,
        },
    );
    assert!(!panel.pending);
    assert_eq!(panel.feedback, None);
    Ok(())
}

#[test]
fn initialization_resets_composer_state_and_rate_gate() {
    let mut panel = PlayerTypingPanel::default();
    panel.initialize(true);
    let now = std::time::Instant::now();
    assert!(composer(&mut panel, true, "hello", now).is_some());
    panel.initialize(false);
    assert!(!panel.composer_typing);
    assert!(!panel.start_wanted);
    assert_eq!(panel.last_start, None);
    panel.initialize(true);
    assert!(composer(&mut panel, true, "hello", now).is_some());
}

#[test]
fn tick_retries_a_rate_gated_start_and_blur_cancels_it() {
    let mut panel = PlayerTypingPanel::default();
    panel.initialize(true);
    let now = std::time::Instant::now();
    assert!(composer(&mut panel, true, "a", now).is_some());
    assert!(composer(&mut panel, false, "a", now).is_some());
    assert!(composer(&mut panel, true, "b", now).is_none());
    assert!(panel.start_wanted);
    assert!(tick(&mut panel, now + std::time::Duration::from_millis(499)).is_none());
    assert_eq!(
        tick(&mut panel, now + std::time::Duration::from_millis(500)).map(|command| command.typing),
        Some(true)
    );
    assert!(tick(&mut panel, now + std::time::Duration::from_millis(501)).is_none());
    assert!(composer(&mut panel, false, "b", now).is_some());
    assert!(composer(&mut panel, true, "c", now).is_none());
    assert!(composer(&mut panel, false, "c", now).is_none());
    assert!(tick(&mut panel, now + std::time::Duration::from_secs(2)).is_none());
}
