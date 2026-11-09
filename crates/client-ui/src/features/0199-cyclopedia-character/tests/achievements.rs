use super::*;
use crate::account_achievements::{AchievementFilter, AchievementSort, catalog_rows};
use oteryn_client_runtime::account_achievements::{
    AccountAchievementRow, AccountAchievementsResult,
};
use oteryn_client_runtime::cyclopedia_character::achievements::AchievementPage;
fn reply(
    character: [u8; 16],
    page: u32,
    rows: Vec<AccountAchievementRow>,
    more: bool,
) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: AchievementPage {
            character,
            name: "Test Hero".into(),
            page: AccountAchievementsResult {
                total_points: 65,
                fact_count: 65,
                page,
                rows,
                has_more: more,
            },
        }
        .encode()
        .unwrap_or_else(|_| unreachable!()),
    }
}
fn row(i: usize) -> AccountAchievementRow {
    AccountAchievementRow {
        key: format!("oteryn:achievement/entry_{i}"),
        name: format!("Entry {i}"),
        description: "A confirmed feat".into(),
        grade: 1,
        points: 1,
        earned_at: i as i64 + 1,
        secret: i.is_multiple_of(2),
    }
}
#[test]
fn character_catalog_loads_all_pages_before_global_filtering_and_preserves_identity()
-> Result<(), &'static str> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        target: Some([2; 16]),
        ..Default::default()
    };
    assert!(request(&mut p, Intent::Achievements).is_some());
    assert!(
        request(&mut p, Intent::Tab).is_none(),
        "same Character command lane is serialized"
    );
    assert!(apply_result(
        &mut p,
        &reply([2; 16], 0, (0..64).map(row).collect(), true)
    ));
    assert!(!p.achievements.catalog.ready);
    p.tab = CharacterTab::Defence;
    assert!(super::super::achievement_next_request(&mut p).is_some());
    assert!(apply_result(
        &mut p,
        &reply([2; 16], 1, vec![row(64)], false)
    ));
    assert!(p.achievements.catalog.ready);
    assert_eq!(
        p.tab,
        CharacterTab::Defence,
        "background catalogue pages must not change main navigation"
    );
    assert_eq!(p.achievements.catalog.rows.len(), 65);
    p.achievements.catalog.filter = AchievementFilter::Secret;
    p.achievements.catalog.sort = AchievementSort::Earned;
    let rows = catalog_rows(&p.achievements.catalog);
    assert_eq!(rows.len(), 33);
    assert!(rows.iter().all(|r| r.secret));
    p.achievements.catalog.search = "Entry 64".into();
    assert_eq!(
        catalog_rows(&p.achievements.catalog).len(),
        1,
        "search includes rows beyond the first server page"
    );
    let generation = p.profile_generation;
    p.change_target(Some([3; 16]));
    assert!(p.profile_generation > generation);
    assert!(p.achievement_profile.is_none());
    assert!(p.achievements.catalog.rows.is_empty());
    Ok(())
}
#[test]
fn foreign_or_mixed_profile_pages_fail_closed_without_publishing_partial_rows()
-> Result<(), &'static str> {
    for initial_target in [None, Some([2; 16])] {
        let mut p = CyclopediaCharacterPanel {
            selected: true,
            target: initial_target,
            ..Default::default()
        };
        assert!(request(&mut p, Intent::Achievements).is_some());
        assert!(apply_result(
            &mut p,
            &reply([2; 16], 0, (0..64).map(row).collect(), true)
        ));
        assert!(super::super::achievement_next_request(&mut p).is_some());
        assert!(!apply_result(
            &mut p,
            &reply([3; 16], 1, vec![row(64)], false)
        ));
        assert!(!p.pending);
        assert!(!p.achievements.catalog.ready);
        assert!(p.achievements.catalog.rows.is_empty());
        assert!(super::super::achievement_next_request(&mut p).is_none());
    }
    Ok(())
}
