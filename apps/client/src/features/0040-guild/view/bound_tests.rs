use super::*;
use crate::play::play_channel;
use oteryn_client_ui::guild::guild;
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn panel() -> GuildPanel {
    GuildPanel {
        selected: true,
        open: true,
        own: Some(guild::GuildSummary {
            guild: [1; 16],
            name: "Owner guild".into(),
            state: 1,
            revision: 8,
            own_rank: 1,
            title: String::new(),
            message: String::new(),
            ranks: vec!["Leader".into(), "Vice".into(), "Member".into()],
        }),
        roster: vec![guild::GuildMember {
            character: [2; 16],
            name: "Member".into(),
            level: 3,
            title: String::new(),
        }],
        ..Default::default()
    }
}
#[test]
fn guild_bound_owner_rejects_unconfirmed_changed_member_pending_and_closed() -> TestResult {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_guild_panel(panel());
    let view = link.guild_window_view();
    let kick = || GuildWindowAction::Request(Intent::Kick([2; 16]));
    assert!(!link.guild_window_dispatch(&view, kick(), false));
    assert_eq!(link.guild_panel(), view.panel);
    let mut changed = panel();
    changed.roster[0].character = [3; 16];
    outbox.bevy_guild_panel(changed.clone());
    assert!(!link.guild_window_dispatch(&view, kick(), true));
    assert_eq!(link.guild_panel(), changed);
    outbox.bevy_guild_panel(panel());
    assert!(link.guild_window_dispatch(&view, kick(), true));
    assert!(matches!(commands.try_recv()?, PlayCommand::Guild(_)));
    let pending = link.guild_window_view();
    assert!(!link.guild_window_dispatch(&pending, kick(), true));
    assert_eq!(link.guild_panel(), pending.panel);
    assert!(commands.try_recv().is_err());
    outbox.bevy_guild_panel(panel());
    drop(commands);
    assert!(!link.guild_window_dispatch(&view, kick(), true));
    assert_eq!(link.guild_panel(), view.panel);
    Ok(())
}
#[test]
fn guild_bound_owner_checks_server_modal_and_session_end_without_mutation() {
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_guild_panel(panel());
    let expected = link.guild_window_view();
    let _modal = link.bevy_server_modal_fixture();
    for intent in [
        Intent::RefreshOwn,
        Intent::RefreshInvitations,
        Intent::RefreshRoster,
        Intent::Invite("Member".into()),
        Intent::Kick([2; 16]),
        Intent::SetRank {
            character: [2; 16],
            level: 2,
        },
        Intent::Leave,
    ] {
        assert!(!link.guild_window_dispatch(&expected, GuildWindowAction::Request(intent), true));
        assert_eq!(link.guild_panel(), expected.panel);
    }
    assert!(!link.guild_window_dispatch(&expected, GuildWindowAction::Close, true));
    assert!(commands.try_recv().is_err());
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_guild_panel(panel());
    let expected = link.guild_window_view();
    outbox.bevy_guild_end();
    let ended = link.guild_panel();
    assert!(!link.guild_window_dispatch(
        &expected,
        GuildWindowAction::Request(Intent::Kick([2; 16])),
        true
    ));
    assert!(!link.guild_window_open());
    assert_eq!(link.guild_panel(), ended);
    assert!(commands.try_recv().is_err());
}
