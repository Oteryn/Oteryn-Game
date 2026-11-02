use super::*;
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
        status: CommandStatus::Accepted,
        reply: Some(DailyRewardReply::Result(daily_reward::DailyRewardResult {
            disposition: 4,
            transaction_id: None,
        })),
    };
    assert!(!apply_result(&mut panel, &response));
    assert_eq!(panel.feedback, Some("Daily reward shrine is out of reach"));
}
