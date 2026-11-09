use super::*;
use oteryn_client_ui::account_achievements::{AchievementFilter, AchievementSort, catalog_rows};

#[test]
fn achievement_browser_filters_regular_secret_and_sorts_server_rows() -> Result<(), Error> {
    let (link, mut commands, outbox) = play_channel();
    outbox.achievements_initialize();
    crate::play::mailbox::lock(&link.mailbox)
        .achievements
        .catalog
        .enabled = true;
    link.achievements_request(Intent::Refresh);
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::AccountAchievements(0)
    ));
    let row = |key: &str, points, secret| AccountAchievementRow {
        key: format!("oteryn:achievement/{key}"),
        name: key.into(),
        description: "From server".into(),
        grade: 1,
        points,
        earned_at: i64::from(points),
        secret,
    };
    outbox.achievements_result(&Response::Page(AccountAchievementsResult {
        page: 0,
        fact_count: 3,
        total_points: 16,
        has_more: false,
        rows: vec![
            row("Scout", 1, false),
            row("Explorer", 5, false),
            row("Hidden path", 10, true),
        ],
    }));
    link.achievements_browse("", AchievementFilter::Regular, AchievementSort::Points);
    let p = link.achievements_panel();
    assert_eq!(
        catalog_rows(&p.catalog)
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["Explorer", "Scout"]
    );
    link.achievements_browse(" PATH ", AchievementFilter::Secret, AchievementSort::Earned);
    let p = link.achievements_panel();
    assert_eq!(
        catalog_rows(&p.catalog)
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["Hidden path"]
    );
    assert_eq!(p.catalog.points, 16);
    assert_eq!(p.catalog.rows.len(), 3);
    assert!(
        commands.try_recv().is_err(),
        "local options do not synthesize account progress"
    );
    Ok(())
}
