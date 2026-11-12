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

#[test]
fn full_account_list_collects_all_server_pages_before_global_secret_filter_and_sort() {
    use oteryn_client_runtime::account_achievements::AccountAchievementRow;
    let row = |i: u32, secret| AccountAchievementRow {
        key: format!("oteryn:achievement/a{i}"),
        name: format!("Award {i:03}"),
        description: "Earned in play".into(),
        grade: if secret { 4 } else { 1 },
        points: if secret { 100 } else { 1 },
        earned_at: i64::from(i),
        secret,
    };
    let mut p = AchievementsPanel {
        selected: true,
        ..Default::default()
    };
    p.catalog.enabled = true;
    assert_eq!(request(&mut p, Intent::Refresh), Some(0));
    apply_result(
        &mut p,
        &Response::Page(AccountAchievementsResult {
            total_points: 164,
            fact_count: 65,
            page: 0,
            has_more: true,
            rows: (0..64).map(|i| row(i, false)).collect(),
        }),
    );
    assert!(!p.catalog.ready);
    assert_eq!(catalog_next_request(&mut p), Some(1));
    apply_result(
        &mut p,
        &Response::Page(AccountAchievementsResult {
            total_points: 164,
            fact_count: 65,
            page: 1,
            has_more: false,
            rows: vec![row(64, true)],
        }),
    );
    assert!(p.catalog.ready);
    assert_eq!(p.catalog.rows.len(), 65);
    assert_eq!(catalog_next_request(&mut p), None);
    p.catalog.filter = AchievementFilter::Secret;
    assert_eq!(catalog_rows(&p.catalog).len(), 1);
    assert_eq!(catalog_rows(&p.catalog)[0].key, "oteryn:achievement/a64");
    p.catalog.filter = AchievementFilter::All;
    p.catalog.sort = AchievementSort::Points;
    assert_eq!(catalog_rows(&p.catalog)[0].points, 100);
    p.catalog.search = "award 064".into();
    assert_eq!(catalog_rows(&p.catalog).len(), 1);
}

#[test]
fn changing_account_pages_never_replace_a_confirmed_full_list() {
    use oteryn_client_runtime::account_achievements::AccountAchievementRow;
    let row = |i| AccountAchievementRow {
        key: format!("oteryn:achievement/a{i}"),
        name: format!("Award {i}"),
        description: "Earned".into(),
        grade: 1,
        points: 1,
        earned_at: 1,
        secret: false,
    };
    let mut p = AchievementsPanel {
        selected: true,
        ..Default::default()
    };
    p.catalog.enabled = true;
    assert_eq!(request(&mut p, Intent::Refresh), Some(0));
    apply_result(
        &mut p,
        &Response::Page(AccountAchievementsResult::default()),
    );
    assert!(p.catalog.ready);
    let confirmed = p.catalog.rows.clone();
    assert_eq!(request(&mut p, Intent::Refresh), Some(0));
    apply_result(
        &mut p,
        &Response::Page(AccountAchievementsResult {
            fact_count: 65,
            total_points: 65,
            page: 0,
            has_more: true,
            rows: (0..64).map(row).collect(),
        }),
    );
    assert_eq!(catalog_next_request(&mut p), Some(1));
    apply_result(
        &mut p,
        &Response::Page(AccountAchievementsResult {
            page: 1,
            fact_count: 66,
            total_points: 66,
            has_more: false,
            rows: vec![row(64), row(65)],
        }),
    );
    assert!(p.error.is_some());
    assert_eq!(p.catalog.rows, confirmed);
    assert!(!p.catalog.collecting);
    assert_eq!(catalog_next_request(&mut p), None);
}

#[test]
fn achievement_browser_search_trims_input_and_finds_confirmed_keys() {
    use oteryn_client_runtime::account_achievements::AccountAchievementRow;
    let mut p = AchievementsPanel::default();
    p.catalog.rows.push(AccountAchievementRow {
        key: "oteryn:achievement/hunter".into(),
        name: "Hunter".into(),
        description: "Hunt creatures".into(),
        grade: 1,
        points: 5,
        earned_at: 100,
        secret: true,
    });
    p.catalog.search = " OTERYN:ACHIEVEMENT/HUNTER ".into();
    assert_eq!(catalog_rows(&p.catalog).len(), 1);
    p.catalog.filter = AchievementFilter::Regular;
    assert!(catalog_rows(&p.catalog).is_empty());
    p.catalog.filter = AchievementFilter::Secret;
    p.catalog.search = " HUNT CREATURES ".into();
    assert_eq!(catalog_rows(&p.catalog).len(), 1);
}
