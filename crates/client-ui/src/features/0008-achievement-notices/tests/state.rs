use super::*;

fn earned(name: &str, facts: u32) -> oteryn_client_runtime::achievement_notices::AchievementEarned {
    oteryn_client_runtime::achievement_notices::AchievementEarned {
        key: "oteryn:achievement/allow_cookies".into(),
        name: name.into(),
        watermark: oteryn_client_runtime::achievement_notices::AchievementWatermark {
            fact_count: facts,
            total_points: facts,
        },
    }
}

#[test]
fn pushed_achievement_shows_popup_and_one_advancement_line() {
    let mut notices = AchievementNotices::default();
    notices.selected = true;
    apply(&mut notices, earned("Allow Cookies?!", 1));
    let expected = "Congratulations! You earned the achievement \"Allow Cookies?!\".";
    assert_eq!(popup_text(&notices).as_deref(), Some(expected));
    assert_eq!(panel_text(&notices), expected);
    assert_eq!(notices.advancement_log().count(), 1);
    notices.dismiss_popup();
    assert_eq!(popup_text(&notices), None);
    assert_eq!(
        panel_text(&AchievementNotices::default()),
        "Achievement notices unavailable in this session"
    );
}

#[test]
fn advancement_log_is_bounded_and_keeps_the_newest_lines() {
    let mut notices = AchievementNotices::default();
    notices.selected = true;
    for index in 0..=MAX_ADVANCEMENT_LOG_LINES {
        apply(
            &mut notices,
            earned(
                &format!("Achievement {index}"),
                u32::try_from(index).unwrap_or(u32::MAX),
            ),
        );
    }
    assert_eq!(notices.advancement_log().len(), MAX_ADVANCEMENT_LOG_LINES);
    let first = advancement_text("Achievement 1");
    let last = advancement_text("Achievement 100");
    assert_eq!(notices.advancement_log().next(), Some(first.as_str()));
    assert_eq!(notices.advancement_log().last(), Some(last.as_str()));
}
