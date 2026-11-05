use super::*;
use oteryn_client_runtime::account_achievements::{AccountAchievementsResult, Response};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};

#[test]
fn input_ignores_repeat_and_requires_the_exact_chord() -> Result<(), Box<dyn std::error::Error>> {
    let code = KeyCode::new(4)?;
    let key = |repeat, modifiers| NormalizedInputEvent::Key {
        code,
        state: ButtonState::Pressed,
        modifiers,
        repeat,
    };
    let modifiers = Modifiers::one(Modifier::Control).with(Modifier::Shift);
    assert!(open_key(&[key(false, modifiers)]));
    assert!(!open_key(&[key(true, modifiers)]));
    assert!(!open_key(&[key(false, Modifiers::one(Modifier::Control))]));
    Ok(())
}

#[test]
fn paging_waits_for_confirmed_data_and_refusal_keeps_it() {
    let mut panel = AchievementsPanel {
        selected: true,
        ..Default::default()
    };
    assert_eq!(request(&mut panel, Intent::Previous), None);
    assert_eq!(request(&mut panel, Intent::Refresh), Some(0));
    assert_eq!(request(&mut panel, Intent::Refresh), None);
    let first = AccountAchievementsResult {
        total_points: 25,
        fact_count: 65,
        has_more: true,
        ..Default::default()
    };
    apply_result(&mut panel, &Response::Page(first.clone()));
    assert!(panel_text(&panel).contains("25 points"));
    assert_eq!(request(&mut panel, Intent::Next), Some(1));
    apply_result(&mut panel, &Response::Refused);
    assert_eq!(panel.page, Some(first));
    assert_eq!(panel.pending, None);
    assert_eq!(request(&mut panel, Intent::Next), Some(1));
    apply_result(
        &mut panel,
        &Response::Page(AccountAchievementsResult {
            page: 1,
            ..Default::default()
        }),
    );
    assert_eq!(request(&mut panel, Intent::Next), None);
    assert_eq!(request(&mut panel, Intent::Previous), Some(0));
}

#[test]
fn unavailable_and_mismatched_results_never_replace_confirmed_data() {
    let mut panel = AchievementsPanel::default();
    assert_eq!(request(&mut panel, Intent::Refresh), None);
    panel.selected = true;
    assert_eq!(request(&mut panel, Intent::Refresh), Some(0));
    apply_result(
        &mut panel,
        &Response::Page(AccountAchievementsResult {
            page: 1,
            ..Default::default()
        }),
    );
    assert!(panel.page.is_none());
    assert!(panel.error.is_some());
    assert_eq!(request(&mut panel, Intent::Refresh), Some(0));
    apply_result(&mut panel, &Response::Page(Default::default()));
    assert!(panel_text(&panel).contains("No earned achievements"));
}
