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
