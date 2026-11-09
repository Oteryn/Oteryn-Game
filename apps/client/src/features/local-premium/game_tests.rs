use super::game::{Overview, Panel, State};
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn current() -> Overview {
    Overview {
        state: State::Active,
        authority_revision: 7,
        observed_at_us: 1_800_000_000_000_000,
        expires_at_us: Some(1_800_003_600_000_000),
        valid_for_us: 10_000_000,
        features: vec![oteryn_client_runtime::premium_overview::Feature {
            title: "Premium channels".into(),
            description: "Other channel rules still apply.".into(),
            enabled: true,
        }],
    }
}
fn result(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    }
}
#[test]
fn premium_game_real_controller_sends_existing_command_and_refuses_stale_data() -> TestResult {
    let (link, mut commands, outbox) = crate::play::play_channel();
    outbox.premium_game_initialize(true);
    assert!(
        link.premium_overview_input(&[oteryn_input_actions::NormalizedInputEvent::Key {
            code: oteryn_input_actions::KeyCode::new(19)?,
            state: oteryn_input_actions::ButtonState::Pressed,
            modifiers: oteryn_input_actions::Modifiers::one(
                oteryn_input_actions::Modifier::Control
            )
            .with(oteryn_input_actions::Modifier::Alt)
            .with(oteryn_input_actions::Modifier::Shift),
            repeat: false,
        }])
    );
    let command = match commands.try_recv()? {
        crate::combat_input::PlayCommand::Feature(c) => c,
        _ => return Err("wrong command".into()),
    };
    assert_eq!(
        (command.capability, command.kind, command.payload.as_slice()),
        (97, 113, [5].as_slice())
    );
    let observed = current();
    outbox.premium_game_result(&command, &result(observed.encode().ok_or("encode")?));
    let text = link.premium_overview_text();
    assert!(text.contains("1800003600 UTC"));
    assert!(text.contains("Premium channels [available]"));
    link.premium_overview_refresh();
    let _ = commands.try_recv()?;
    outbox.premium_game_result(&command, &result(vec![1]));
    assert!(!link.premium_overview_text().contains("[available]"));
    assert!(link.premium_overview_text().contains("Invalid Premium"));
    Ok(())
}
#[test]
fn premium_game_ui_expires_lease_independently_of_actual_subscription_end() -> TestResult {
    let mut panel = Panel {
        selected: true,
        ..Default::default()
    };
    let command = panel.request().ok_or("request")?;
    let snapshot = current();
    panel.apply(&command, &result(snapshot.encode().ok_or("encode")?));
    assert!(panel.view.status_text().contains("[available]"));
    panel.view.fresh_until = Some(Instant::now());
    assert!(!panel.view.status_text().contains("[available]"));
    assert!(panel.view.status_text().contains("1800003600 UTC"));
    panel.pending = true;
    let mut backwards = current();
    backwards.authority_revision = 6;
    panel.apply(&command, &result(backwards.encode().ok_or("encode")?));
    assert!(panel.view.feedback.contains("Inconsistent"));
    assert_eq!(
        panel
            .view
            .game_status
            .as_ref()
            .map(|s| s.authority_revision),
        Some(7)
    );
    Ok(())
}
