use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use oteryn_client_runtime::server_mechanic::{ServerMechanicCommand, ServerMechanicResponse};
use oteryn_client_ui::team_finder::*;
use oteryn_client_ui::team_finder_leader::{LeaderListing, TeamTarget};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture(selected: bool) -> Result<Fixture> {
    let (link, rx, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_team_selected(selected);
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
    owner.play.as_ref().ok_or("play")?.link.discovery_open(232);
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, rx, outbox))
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, c)| c.kind == kind)
        .map(|(e, _)| e)
        .ok_or("team control absent")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn command(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<ServerMechanicCommand> {
    match rx.try_recv()? {
        PlayCommand::ServerMechanic(c) => Ok(c),
        _ => Err("wrong command".into()),
    }
}
fn reply(outbox: &Outbox, payload: Vec<u8>) {
    outbox.bevy_team_response(&ServerMechanicResponse {
        command_id: 1,
        status: oteryn_session::CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    });
}
fn page(status: MemberStatus) -> TeamFinderResult {
    let listing = LeaderListing {
        min_level: 10,
        max_level: 100,
        vocation_mask: 15,
        slots: 4,
        free_slots: 3,
        party_members_join: false,
        timestamp: 0,
        target: TeamTarget::Boss(1),
    };
    TeamFinderResult::Browse {
        listings: vec![
            BrowseListing {
                leader_handle: 7,
                name: "First".into(),
                listing,
                occupied_slots: 1,
                status,
            },
            BrowseListing {
                leader_handle: 9,
                name: "Second".into(),
                listing,
                occupied_slots: 2,
                status,
            },
        ],
        next: Some(9),
    }
}
#[test]
fn team_search_filter_application_cancel_and_paging() -> Result {
    let (mut app, mut rx, outbox) = fixture(true)?;
    click(&mut app, Kind::Browse(false))?;
    assert_eq!(
        decode_query(&command(&mut rx)?.payload).map_err(|error| format!("{error:?}"))?,
        TeamFinderQuery::Browse { after: None }
    );
    reply(
        &outbox,
        encode_result(&page(MemberStatus::None)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    let filter = TeamFilter {
        name: "second".into(),
        ..default()
    };
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if matches!(field, Field::Name) {
            *text = EditableText::new("second");
        }
    }
    click(&mut app, Kind::Filter(TeamFilter::default()))?;
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("Second") && t.0.contains("2 free"))
    );
    assert_eq!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .ok_or("snapshot")?
            .1
            .filter,
        filter
    );
    click(&mut app, Kind::Apply(false))?;
    let sent = command(&mut rx)?;
    assert_eq!((sent.capability, sent.command_type), (232, 445));
    assert_eq!(
        decode_member_action(&sent.payload).map_err(|error| format!("{error:?}"))?,
        MemberAction::Request { leader: 9 }
    );
    // The old UI is still present until projection; duplicate activation must not send.
    click(&mut app, Kind::Apply(false))?;
    assert!(rx.try_recv().is_err());
    reply(&outbox, vec![]);
    app.update();
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&page(MemberStatus::Pending)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    click(&mut app, Kind::Apply(true))?;
    assert_eq!(
        decode_member_action(&command(&mut rx)?.payload).map_err(|error| format!("{error:?}"))?,
        MemberAction::Cancel { leader: 9 }
    );
    reply(&outbox, vec![]);
    app.update();
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&page(MemberStatus::None)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    click(&mut app, Kind::Browse(true))?;
    assert_eq!(
        decode_query(&command(&mut rx)?.payload).map_err(|error| format!("{error:?}"))?,
        TeamFinderQuery::Browse { after: Some(9) }
    );
    app.update();
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}
#[test]
fn team_unavailable_session_and_stale_snapshot_send_nothing() -> Result {
    let (mut app, mut rx, _) = fixture(false)?;
    click(&mut app, Kind::Browse(false))?;
    assert!(rx.try_recv().is_err());
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    let (mut app, mut rx, outbox) = fixture(true)?;
    click(&mut app, Kind::Browse(false))?;
    command(&mut rx)?;
    reply(
        &outbox,
        encode_result(&page(MemberStatus::None)).map_err(|error| format!("{error:?}"))?,
    );
    app.update();
    click(&mut app, Kind::Select(1))?;
    click(&mut app, Kind::Apply(false))?;
    assert!(rx.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Apply(false))?;
    assert_eq!(
        decode_member_action(&command(&mut rx)?.payload).map_err(|error| format!("{error:?}"))?,
        MemberAction::Request { leader: 9 }
    );
    Ok(())
}
