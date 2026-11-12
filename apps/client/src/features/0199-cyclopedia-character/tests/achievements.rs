use super::*;
use crate::combat_input::PlayCommand;
use crate::play::play_channel;
use oteryn_client_runtime::account_achievements::{
    AccountAchievementRow, AccountAchievementsResult,
};
use oteryn_client_runtime::cyclopedia_character::achievements::{AchievementPage, decode_request};

#[test]
fn character_achievements_use_the_targeted_adapter_and_server_metadata() -> Result<(), &'static str>
{
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    link.cyclopedia_character_request(Intent::Achievements);
    let Ok(PlayCommand::Feature(command)) = commands.try_recv() else {
        return Err("command");
    };
    assert_eq!(command.kind, 388);
    assert_eq!(
        decode_request(&command.payload)
            .map_err(|_| "request")?
            .target,
        None
    );
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: AchievementPage {
            character: [1; 16],
            name: "Test Hero".into(),
            page: AccountAchievementsResult {
                total_points: 7,
                fact_count: 1,
                page: 0,
                has_more: false,
                rows: vec![AccountAchievementRow {
                    key: "oteryn:achievement/explorer".into(),
                    name: "Explorer".into(),
                    description: "Explore a new place".into(),
                    grade: 1,
                    points: 7,
                    earned_at: 1234,
                    secret: true,
                }],
            },
        }
        .encode()
        .map_err(|_| "page")?,
    });
    let panel = link.cyclopedia_character_panel();
    assert!(panel.achievements.catalog.ready);
    let rows = oteryn_client_ui::account_achievements::catalog_rows(&panel.achievements.catalog);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].earned_at, 1234);
    assert!(rows[0].secret);
    assert_eq!(
        panel.achievement_profile,
        Some(([1; 16], "Test Hero".into()))
    );
    Ok(())
}

#[test]
fn t092_selecting_achievement_tab_loads_filterable_server_catalog() -> Result<(), &'static str> {
    use oteryn_client_runtime::cyclopedia_character::CharacterTab;
    use oteryn_client_ui::account_achievements::{
        AchievementFilter, AchievementSort, catalog_rows,
    };
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    link.cyclopedia_character_request(Intent::SelectTab(CharacterTab::Achievements));
    let Ok(PlayCommand::Feature(command)) = commands.try_recv() else {
        return Err("command");
    };
    assert_eq!(
        decode_request(&command.payload)
            .map_err(|_| "targeted catalog request")?
            .page,
        0
    );
    let rows = [
        ("Regular", false, 1),
        ("Secret", true, 9),
        ("Hidden trail", true, 4),
    ]
    .into_iter()
    .map(|(name, secret, points)| AccountAchievementRow {
        key: format!(
            "oteryn:achievement/{}",
            name.to_lowercase().replace(' ', "_")
        ),
        name: name.into(),
        description: "Server achievement".into(),
        grade: 1,
        points,
        earned_at: i64::from(points),
        secret,
    })
    .collect();
    outbox.cyclopedia_character_result(&GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: AchievementPage {
            character: [1; 16],
            name: "Hero".into(),
            page: AccountAchievementsResult {
                total_points: 14,
                fact_count: 3,
                page: 0,
                has_more: false,
                rows,
            },
        }
        .encode()
        .map_err(|_| "encode")?,
    });
    let mut panel = link.cyclopedia_character_panel();
    assert!(panel.achievements.catalog.ready);
    let catalog = &mut panel.achievements.catalog;
    catalog.filter = AchievementFilter::Secret;
    catalog.sort = AchievementSort::Points;
    assert_eq!(
        catalog_rows(catalog)
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["Secret", "Hidden trail"]
    );
    catalog.filter = AchievementFilter::Regular;
    assert_eq!(catalog_rows(catalog)[0].name, "Regular");
    catalog.filter = AchievementFilter::All;
    catalog.search = "trail".into();
    assert_eq!(catalog_rows(catalog)[0].name, "Hidden trail");
    Ok(())
}
