use crate::combat_input::PlayCommand;
use crate::play::play_channel;
use oteryn_client_runtime::{
    CommandStatus,
    cyclopedia_character::{CharacterTab, KillStatus, TabView},
    raw_feature::GatedCommandResult,
};
use oteryn_client_ui::cyclopedia_character::{Intent, history_rows};
#[test]
fn character_histories_use_real_requests_results_and_local_status_filters()
-> Result<(), &'static str> {
    let (link, mut commands, outbox) = play_channel();
    outbox.cyclopedia_character_initialize(true);
    for (tab, view) in [
        (
            CharacterTab::RecentDeaths,
            TabView::RecentDeaths(vec![(42, "A creature defeated this character".into())]),
        ),
        (
            CharacterTab::RecentPvpKills,
            TabView::RecentPvpKills(vec![
                (43, KillStatus::GuildWar, "Opponent A".into()),
                (44, KillStatus::Unjustified, "Opponent B".into()),
            ]),
        ),
    ] {
        link.cyclopedia_character_request(Intent::SelectTab(tab));
        let Ok(PlayCommand::Feature(command)) = commands.try_recv() else {
            return Err("history command");
        };
        assert_eq!(command.kind, 388);
        assert_eq!(command.payload, vec![tab as u8, 0, 0, 20]);
        outbox.cyclopedia_character_result(&GatedCommandResult {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: view.encode().map_err(|_| "history payload")?,
        });
        let mut panel = link.cyclopedia_character_panel();
        assert_eq!(panel.history_tab, Some(tab));
        assert!(panel.feedback.is_none());
        if tab == CharacterTab::RecentDeaths {
            let rows = history_rows(&panel);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].at, 42);
            assert!(rows[0].description.contains("creature"));
        } else {
            let snapshot = panel.history.clone();
            assert!(
                crate::play::feature_0199_cyclopedia_character::view::native::submit(
                    &mut panel,
                    116,
                    ["0", "20", ""]
                )
                .is_none()
            );
            let rows = history_rows(&panel);
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].status, Some(KillStatus::GuildWar));
            assert_eq!(panel.history, snapshot);
            assert!(
                commands.try_recv().is_err(),
                "local filters must not issue game commands"
            );
        }
    }
    Ok(())
}
