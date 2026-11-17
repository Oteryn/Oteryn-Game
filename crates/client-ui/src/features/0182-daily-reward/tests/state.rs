use super::*;
#[test]
fn reward_streak_bonus_is_one_point_per_claim_capped_at_seven_percent() {
    let mut panel = DailyRewardPanel::default();
    assert!(!panel_text(&panel).contains("Reward Streak Bonus"));
    for (streak, chance) in [
        (0, 0),
        (1, 1),
        (6, 6),
        (7, 7),
        (8, 7),
        (28, 7),
        (100_000, 7),
    ] {
        panel.wall = Some(daily_reward::DailyRewardWall {
            lane_day: 1,
            streak,
            jokers: 0,
            claimed: false,
            claim_enabled: false,
        });
        let text = panel_text(&panel);
        assert!(text.contains("Reward Streak Bonus"));
        if chance == 0 {
            assert!(text.contains("You currently have no Reward Streak Bonus."));
        } else {
            assert!(text.contains(&format!(
                "Your reward streak currently grants you a {chance}% chance to gain double weapon proficiency XP."
            )));
        }
        assert!(text.contains("chance of up to 7%"));
        assert!(!text.contains("resting"));
    }
}

#[test]
fn wall_repeats_every_seven_days_and_lists_the_new_reward_kinds() {
    let mut details = daily_reward::details::Details::empty(1);
    details.days[0].free = daily_reward::details::Tier {
        cosmetic_tokens: 2,
        prey_wildcards: 1,
        temple_scrolls: 1,
        xp_percent: 10,
        xp_minutes: 30,
        bounty_tokens: 1,
    };
    details.days[0].premium.cosmetic_tokens = 4;
    let mut panel = DailyRewardPanel {
        details: Some(details),
        wall: Some(daily_reward::DailyRewardWall {
            lane_day: 9,
            streak: 3,
            jokers: 0,
            claimed: false,
            claim_enabled: true,
        }),
        ..Default::default()
    };
    assert_eq!(
        [cycle_day(1), cycle_day(7), cycle_day(8), cycle_day(30)],
        [1, 7, 1, 2]
    );
    let text = panel_text(&panel);
    assert!(text.contains("Day 2 of 7"));
    assert!(text.contains("Seven-day reward cycle"));
    assert!(!text.contains("30-day"));
    assert!(text.contains(
        "2 Cosmetic Tokens, 1x Prey Wildcards, 1 Temple Teleport Scroll, 30 minutes XP Boost 10%, Bounty Reroll Token"
    ));
    assert!(text.contains("Premium: 4 Cosmetic Tokens"));
    assert!(text.contains("Day 2 today"));
    assert!(text.contains("Premium accounts receive larger rewards"));
    assert!(!text.contains("Day 08"));
    panel.details = None;
    assert!(panel_text(&panel).contains("Rewards are loading"));
}

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
    assert!(panel_text(&panel).contains("Day 2 of 7"));
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
    let mut details = daily_reward::details::Details::empty(1);
    details.days[4].free.cosmetic_tokens = 3;
    panel.details = Some(details);
    let before = panel_text(&panel);
    assert!(before.contains("Day 5 of 7"));
    assert!(before.contains("Day 5 today"));
    assert!(!before.contains("Day 4 claimed"));
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
    assert!(after.contains("Day 5 claimed"));
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
