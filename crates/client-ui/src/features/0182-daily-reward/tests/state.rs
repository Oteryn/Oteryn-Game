use super::*;
#[test]
fn confirmed_claim_disables_repeat_claim_and_projects_receipt() {
    let mut panel = DailyRewardPanel {
        selected: true,
        wall: Some(daily_reward::DailyRewardWall {
            lane_day: 12,
            streak: 19,
            jokers: 2,
            claimed: false,
            claim_enabled: true,
        }),
        ..Default::default()
    };
    assert_eq!(
        request(&mut panel, Intent::Claim),
        Some(DailyRewardCommand::Claim)
    );
    assert_eq!(request(&mut panel, Intent::Claim), None);
    apply_result(
        &mut panel,
        &DailyRewardResponse {
            status: CommandStatus::Accepted,
            reply: Some(DailyRewardReply::Result(daily_reward::DailyRewardResult {
                disposition: 2,
                transaction_id: None,
            })),
        },
    );
    assert!(panel_text(&panel).contains("Status: claimed"));
    assert_eq!(request(&mut panel, Intent::Claim), None);
}

#[test]
fn shortcut_requires_fresh_control_d() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let code = KeyCode::new(7)?;
    let event = |state, modifiers, repeat| NormalizedInputEvent::Key {
        code,
        state,
        modifiers,
        repeat,
    };
    let control = Modifiers::one(Modifier::Control);
    assert!(open_key(&[event(ButtonState::Pressed, control, false)]));
    for modifiers in [
        Modifiers::NONE,
        control.with(Modifier::Alt),
        control.with(Modifier::Shift),
        control.with(Modifier::Super),
    ] {
        assert!(!open_key(&[event(ButtonState::Pressed, modifiers, false)]));
    }
    assert!(!open_key(&[event(ButtonState::Released, control, false)]));
    assert!(!open_key(&[event(ButtonState::Pressed, control, true)]));
    let mut panel = DailyRewardPanel::default();
    assert_eq!(request(&mut panel, Intent::Refresh), None);
    assert!(panel_text(&panel).contains("unavailable"));
    Ok(())
}
#[test]
fn wall_encoded_like_server_reduces_into_panel() {
    let wall = daily_reward::DailyRewardWall {
        lane_day: 30,
        streak: 42,
        jokers: 3,
        claimed: false,
        claim_enabled: true,
    };
    let bytes = wall.encode().unwrap_or_default();
    let decoded = decode_reply(DailyRewardCommand::Query, &bytes);
    assert!(matches!(decoded, Ok(DailyRewardReply::Wall(value)) if value == wall));
    let response = DailyRewardResponse {
        status: CommandStatus::Accepted,
        reply: Some(DailyRewardReply::Wall(wall)),
    };
    let mut panel = DailyRewardPanel {
        selected: true,
        pending: true,
        ..Default::default()
    };
    assert!(apply_result(&mut panel, &response));
    assert_eq!(panel.wall, Some(wall));
    assert!(panel_text(&panel).contains("Day 30 of 30"));
}
#[test]
fn claim_refusal_is_visible_and_not_inferred_as_state() {
    let mut panel = DailyRewardPanel {
        selected: true,
        wall: Some(daily_reward::DailyRewardWall {
            lane_day: 2,
            streak: 1,
            jokers: 0,
            claimed: false,
            claim_enabled: true,
        }),
        pending: true,
        ..Default::default()
    };
    let response = DailyRewardResponse {
        status: CommandStatus::Rejected,
        reply: Some(DailyRewardReply::Result(daily_reward::DailyRewardResult {
            disposition: 4,
            transaction_id: None,
        })),
    };
    assert!(!apply_result(&mut panel, &response));
    assert_eq!(
        panel.feedback,
        Some("Open the daily reward shrine within reach, then refresh")
    );
}

#[test]
fn reward_calendar_records_only_server_confirmed_claims() {
    let mut panel = DailyRewardPanel {
        selected: true,
        wall: Some(daily_reward::DailyRewardWall {
            lane_day: 5,
            streak: 12,
            jokers: 1,
            claimed: false,
            claim_enabled: true,
        }),
        ..Default::default()
    };
    let before = panel_text(&panel);
    assert!(before.contains("30-day calendar"));
    assert!(before.contains("05: available"));
    assert!(!before.contains("04: claimed"));
    assert!(request(&mut panel, Intent::Claim).is_some());
    assert!(apply_result(
        &mut panel,
        &DailyRewardResponse {
            status: CommandStatus::Accepted,
            reply: Some(DailyRewardReply::Result(daily_reward::DailyRewardResult {
                disposition: 1,
                transaction_id: None
            }))
        }
    ));
    let after = panel_text(&panel);
    assert!(after.contains("05: claimed"));
    assert!(after.contains("Confirmed this session: day 5, streak 12"));
}

#[test]
fn returner_reward_notification_never_infers_entitlement_from_daily_streak() {
    let mut panel = ReturnerRewardPanel::default();
    assert!(panel.claim(100).is_none());
    panel.offer = Some(ReturnerOffer {
        id: [7; 16],
        expires_epoch: 200,
        description: "Welcome back pack".into(),
    });
    assert!(panel.notification().contains("Welcome back pack"));
    assert_eq!(panel.claim(100), Some([7; 16]));
    assert!(panel.claim(100).is_none());
    assert!(!panel.finish([8; 16], true));
    assert!(panel.pending);
    assert!(panel.finish([7; 16], false));
    assert_eq!(panel.claim(100), Some([7; 16]));
    assert!(panel.finish([7; 16], true));
    assert!(panel.claim(100).is_none());
    panel.offer = Some(ReturnerOffer {
        id: [9; 16],
        expires_epoch: 100,
        description: "Expired".into(),
    });
    assert!(panel.claim(100).is_none());
}
