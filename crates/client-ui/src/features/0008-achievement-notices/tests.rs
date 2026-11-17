use super::*;
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn pushed_achievement_notices_are_bounded_and_visible() {
    let mut panel = AchievementNoticesPanel::default();
    let notice = |n| AchievementEarned {
        key: format!("oteryn:achievement/grant-{n}"),
        name: format!("Award {n}"),
        watermark: AchievementWatermark {
            fact_count: n,
            total_points: n * 5,
        },
    };
    reduce(&mut panel, notice(1));
    assert!(panel.recent.is_empty());
    assert!(notices_panel_text(&panel).contains("unavailable"));
    panel.selected = true;
    panel.watermark = Some(AchievementWatermark {
        fact_count: 10,
        total_points: 50,
    });
    assert!(notices_panel_text(&panel).contains("No achievements earned this session"));
    for n in 11..=43 {
        reduce(&mut panel, notice(n));
    }
    assert_eq!(panel.recent.len(), 32);
    assert_eq!(
        panel.recent.front().map(|n| n.name.as_str()),
        Some("Award 12")
    );
    assert_eq!(
        panel.recent.back().map(|n| n.name.as_str()),
        Some("Award 43")
    );
    let text = notices_panel_text(&panel);
    assert!(text.contains("Achievements: 43 · Total points: 215"));
    assert!(text.contains("Achievement earned: Award 43"));
    assert!(!text.contains("oteryn:achievement/"));
}

#[test]
fn achievement_notice_input_requires_exact_chord_and_fresh_press() -> Result<(), &'static str> {
    let code = KeyCode::new(16).map_err(|_| "M key")?;
    let mut modifiers = Modifiers::one(Modifier::Control).with(Modifier::Shift);
    let key = |state, repeat, modifiers| NormalizedInputEvent::Key {
        code,
        state,
        repeat,
        modifiers,
    };
    assert!(open_key(&[key(ButtonState::Pressed, false, modifiers)]));
    assert!(!open_key(&[key(ButtonState::Pressed, true, modifiers)]));
    assert!(!open_key(&[key(ButtonState::Released, false, modifiers)]));
    assert!(!open_key(&[key(
        ButtonState::Pressed,
        false,
        Modifiers::NONE
    )]));
    modifiers = modifiers.with(Modifier::Alt);
    assert!(!open_key(&[key(ButtonState::Pressed, false, modifiers)]));
    Ok(())
}
