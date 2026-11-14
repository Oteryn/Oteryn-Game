use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_ui::guild::{GuildCommand, GuildPanel, GuildReply, Intent, guild as wire};
use oteryn_session::CommandStatus;
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn id(byte: u8) -> [u8; 16] {
    let mut id = [byte; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}
fn server_panel(rank: u8) -> GuildPanel {
    GuildPanel {
        selected: true,
        open: true,
        own: Some(wire::GuildSummary {
            guild: id(1),
            name: "Server guild".into(),
            state: 1,
            revision: 9,
            own_rank: rank,
            title: "Server title".into(),
            message: "Server MOTD".into(),
            ranks: vec![
                "Server leader".into(),
                "Server vice".into(),
                "Server member".into(),
                "Server novice".into(),
            ],
        }),
        roster: vec![wire::GuildMember {
            character: id(2),
            name: "Server member name".into(),
            level: 3,
            title: "Server member title".into(),
        }],
        invitations: vec![wire::GuildInvitation {
            guild: id(3),
            name: "Server invitation".into(),
            expires_at_unix_ms: 123456,
        }],
        ..Default::default()
    }
}
fn fixture(rank: u8) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_guild_panel(server_panel(rank));
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { link, view });
    owner.phase = Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn control(app: &mut App, action: Action) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, c)| (c.action == action).then_some(entity))
        .ok_or("guild control absent".into())
}
fn click(app: &mut App, action: Action) -> TestResult {
    let confirm = action == Action::Confirm;
    let entity = control(app, action)?;
    app.world_mut().trigger(Activate { entity });
    if confirm {
        app.world_mut().trigger(Activate { entity });
    }
    app.update();
    Ok(())
}
fn text(app: &mut App) -> String {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
fn edit(app: &mut App, field: Field, value: &str) -> TestResult {
    let world = app.world_mut();
    let mut query = world.query::<(&Field, &mut EditableText)>();
    let (_, mut editable) = query
        .iter_mut(world)
        .find(|(f, _)| **f == field)
        .ok_or("guild field absent")?;
    *editable = EditableText::new(value);
    Ok(())
}
#[test]
fn guild_window_shows_server_roster_and_permitted_actions() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(1)?;
    let body = text(&mut app);
    for expected in [
        "Server guild",
        "Server MOTD",
        "Server leader",
        "Server vice",
        "Server member name",
        "Server member title",
        "Server invitation",
    ] {
        assert!(body.contains(expected), "missing {expected}");
    }
    assert!(control(&mut app, Action::Invite).is_ok());
    assert!(control(&mut app, Action::Kick(id(2))).is_ok());
    assert!(control(&mut app, Action::Rank(id(2), 2)).is_ok());
    outbox.bevy_guild_panel(server_panel(3));
    app.update();
    assert!(control(&mut app, Action::Invite).is_err());
    assert!(control(&mut app, Action::Kick(id(2))).is_err());
    assert!(control(&mut app, Action::Rank(id(2), 2)).is_err());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn guild_actions_are_member_bound_and_confirmed() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(1)?;
    edit(&mut app, Field::Name, "Invited character")?;
    click(&mut app, Action::Invite)?;
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Guild(GuildCommand::Intent(wire::GuildIntent::Invite { guild, name })) if guild==id(1) && name=="Invited character")
    );
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .guild_panel()
            .roster,
        server_panel(1).roster
    );
    for action in [
        Action::Kick(id(2)),
        Action::Rank(id(2), 2),
        Action::Rank(id(2), 4),
    ] {
        outbox.bevy_guild_panel(server_panel(1));
        app.update();
        click(&mut app, action.clone())?;
        assert!(commands.try_recv().is_err());
        click(&mut app, Action::Confirm)?;
        let PlayCommand::Guild(GuildCommand::Intent(intent)) = commands.try_recv()? else {
            return Err("guild intent absent".into());
        };
        match action {
            Action::Kick(character) => assert_eq!(
                intent,
                wire::GuildIntent::Kick {
                    guild: id(1),
                    character
                }
            ),
            Action::Rank(character, level) => assert_eq!(
                intent,
                wire::GuildIntent::SetRank {
                    guild: id(1),
                    character,
                    level,
                    expected_revision: 9
                }
            ),
            _ => return Err("unexpected test action".into()),
        }
        assert!(commands.try_recv().is_err());
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .guild_panel()
                .roster,
            server_panel(1).roster
        );
    }
    outbox.bevy_guild_panel(server_panel(2));
    app.update();
    click(&mut app, Action::Leave)?;
    assert!(commands.try_recv().is_err());
    click(&mut app, Action::Confirm)?;
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Guild(GuildCommand::Intent(wire::GuildIntent::Leave {guild})) if guild==id(1))
    );
    outbox.bevy_guild_reply(&GuildReply::Intent {
        status: CommandStatus::Accepted,
        result: Some(wire::GuildIntentResult {
            code: 1,
            guild: id(1),
            revision: Some(10),
        }),
    });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .guild_panel()
            .own
            .is_some()
    );
    outbox.bevy_guild_reply(&GuildReply::Query {
        status: CommandStatus::Accepted,
        result: Some(wire::GuildQueryResult::Own(None)),
    });
    app.update();
    assert!(!text(&mut app).contains("Server MOTD"));
    Ok(())
}
#[test]
fn guild_bank_actions_are_amount_bound_and_confirmed() -> TestResult {
    use oteryn_client_ui::guild_bank::Intent;
    use oteryn_session::property_services::guild_bank as bank;
    let (mut app, mut commands, outbox) = fixture(1)?;
    outbox.bevy_guild_bank_panel(oteryn_client_ui::guild_bank::Panel {
        selected: true,
        open: true,
        fields: [hex_id(), "".into()],
        ..Default::default()
    });
    app.update();
    click(&mut app, Action::Bank(Intent::Refresh))?;
    let PlayCommand::Feature(query) = commands.try_recv()? else {
        return Err("bank query absent".into());
    };
    outbox.bevy_guild_bank_reply(
        &query,
        &bank::GuildBankReply {
            code: 1,
            bank_balance: 900,
            guild_balance: 500,
            claim: 0,
            entries: vec![bank::GuildBankEntry {
                ordinal: 7,
                kind: 1,
                amount: 25,
                before: 475,
                after: 500,
                actor: id(2),
            }],
            next: None,
            claims: vec![],
            next_claim: None,
        },
    )?;
    app.update();
    assert!(text(&mut app).contains("Guild bank: 500"));
    assert!(text(&mut app).contains("#7"));
    let confirmed = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .guild_bank_panel();
    for intent in [Intent::Deposit, Intent::Withdraw] {
        outbox.bevy_guild_bank_panel(confirmed.clone());
        app.update();
        edit(&mut app, Field::Amount, "25")?;
        click(&mut app, Action::Bank(intent))?;
        assert!(commands.try_recv().is_err());
        assert!(text(&mut app).contains("25"));
        click(&mut app, Action::Confirm)?;
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("bank intent absent".into());
        };
        let request =
            bank::decode_guild_bank_intent(&command.payload).map_err(|e| format!("{e:?}"))?;
        assert_eq!(request.guild, id(1));
        assert_eq!(
            request.action,
            if intent == Intent::Deposit {
                bank::GuildBankAction::Deposit(25)
            } else {
                bank::GuildBankAction::Withdraw(25)
            }
        );
        assert!(commands.try_recv().is_err());
        assert!(text(&mut app).contains("Guild bank: 500"));
        outbox.bevy_guild_bank_reply(
            &command,
            &bank::GuildBankReply {
                code: 4,
                bank_balance: 0,
                guild_balance: 0,
                claim: 0,
                entries: vec![],
                next: None,
                claims: vec![],
                next_claim: None,
            },
        )?;
        app.update();
        assert!(text(&mut app).contains("Insufficient balance"));
    }
    outbox.bevy_guild_bank_panel(confirmed);
    app.update();
    edit(&mut app, Field::Amount, "25")?;
    click(&mut app, Action::Bank(Intent::Deposit))?;
    edit(&mut app, Field::Amount, "26")?;
    click(&mut app, Action::Confirm)?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
fn hex_id() -> String {
    id(1).iter().map(|byte| format!("{byte:02x}")).collect()
}
#[test]
fn guild_actions_rejected_when_session_replaced() -> TestResult {
    let (mut app, mut commands, _outbox) = fixture(1)?;
    let stale = control(&mut app, Action::Kick(id(2)))?;
    let old_view = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .guild_window_view();
    let (new_link, mut new_commands, new_outbox) = play_channel();
    new_link.use_shared_frontend();
    new_outbox.bevy_guild_panel(server_panel(1));
    for intent in [
        Intent::RefreshOwn,
        Intent::RefreshInvitations,
        Intent::RefreshRoster,
        Intent::Invite("Invite target".into()),
        Intent::Kick(id(2)),
        Intent::SetRank {
            character: id(2),
            level: 2,
        },
        Intent::SetRank {
            character: id(2),
            level: 4,
        },
        Intent::Leave,
    ] {
        assert!(!new_link.guild_window_dispatch(
            &old_view,
            GuildWindowAction::Request(intent),
            true
        ));
        assert_eq!(new_link.guild_panel(), server_panel(1));
    }
    assert!(!new_link.guild_window_dispatch(&old_view, GuildWindowAction::Close, true));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = new_link;
    app.world_mut().trigger(Activate { entity: stale });
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert!(new_commands.try_recv().is_err());
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, Action::Kick(id(2)))?;
    let controls: Vec<_> = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .map(|(entity, _)| entity)
        .collect();
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(999), Node::default()))
        .id();
    for entity in controls {
        app.world_mut().trigger(Activate { entity });
    }
    assert!(new_commands.try_recv().is_err());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .guild_panel(),
        server_panel(1)
    );
    assert!(app.world().resource::<State>().confirmation.is_some());
    app.world_mut().despawn(modal);
    app.update();
    // Change the target after the confirmation is displayed, before the next projection.
    let confirm = control(&mut app, Action::Confirm)?;
    let mut changed = server_panel(1);
    changed.roster[0].character = id(4);
    new_outbox.bevy_guild_panel(changed.clone());
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(new_commands.try_recv().is_err());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .guild_panel(),
        changed
    );
    app.update();
    let expected = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .guild_window_view();
    changed.open = false;
    new_outbox.bevy_guild_panel(changed.clone());
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .guild_window_dispatch(
                &expected,
                GuildWindowAction::Request(Intent::RefreshOwn),
                true
            )
    );
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .guild_panel(),
        changed
    );
    assert!(new_commands.try_recv().is_err());
    Ok(())
}

#[test]
fn guild_ended_session_cannot_even_stage_a_confirmation() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(1)?;
    outbox.bevy_guild_end();
    app.update();
    let ended = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .guild_panel();
    click(&mut app, Action::Kick(id(2)))?;
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .guild_panel(),
        ended
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
