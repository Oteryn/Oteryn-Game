use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_client_runtime::party::{PartyResponse, party};
use oteryn_renderer::TileCoord;
use oteryn_session::CommandStatus;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    HostOwner,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn host() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let mut host = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    host.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    host.phase = crate::bevy_shell::owner::Phase::Playing;
    outbox.bevy_party_selected(true, true, "Leader");
    Ok((host, commands, outbox))
}
fn state() -> party::PartyResult {
    let entry = |handle, name: &str, leader| party::PartyEntry {
        handle,
        name: name.into(),
        leader,
        expires_at: 0,
        list: vec![],
        party_spell_remaining_ms: None,
    };
    party::PartyResult {
        disposition: party::PartyDisposition::Ok,
        shared_xp_enabled: false,
        members: vec![entry(1, "Leader", true), entry(2, "Member", false)],
        invitations: vec![party::PartyEntry {
            expires_at: 4_000_000_000,
            ..entry(3, "Inviter", false)
        }],
    }
}
fn publish(
    outbox: &crate::play::Outbox,
    state: &party::PartyResult,
    disposition: party::PartyDisposition,
) -> Result {
    outbox.bevy_party_response(&PartyResponse {
        status: CommandStatus::Accepted,
        operation: party::encode_party_result(&party::PartyResult::refused(disposition))
            .map_err(|_| "operation")?,
        view: Some(party::encode_party_result(state).map_err(|_| "view")?),
    });
    Ok(())
}
#[test]
fn canonical_party_shortcut_reaches_query_and_preserves_modal_input_fence() -> Result {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let (mut owner, mut commands, _) = host()?;
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(10)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    owner.route_input(std::slice::from_ref(&key), true);
    assert!(commands.try_recv().is_err());
    owner.open_graphics();
    owner.route_input(std::slice::from_ref(&key), false);
    assert!(commands.try_recv().is_err());
    owner.cancel();
    owner.route_input(&[key], false);
    let PlayCommand::Party(command) = commands.try_recv()? else {
        return Err("party query missing".into());
    };
    assert_eq!(command.kind, party::COMMAND_TYPE_PARTY_QUERY);
    assert!(command.payload.is_empty());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn real_server_members_are_presented_in_nonmodal_party_dock() -> Result {
    let (owner, mut commands, outbox) = host()?;
    owner.play.as_ref().ok_or("play")?.link.open_party();
    assert!(matches!(commands.try_recv()?, PlayCommand::Party(_)));
    publish(&outbox, &state(), party::PartyDisposition::Ok)?;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.world_mut().spawn((PartyDock, Node::default()));
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str() == "party.member")
            .count(),
        2,
        "canonical members need actual source-shaped rows"
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
fn app() -> std::result::Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        crate::play::Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let (owner, mut commands, outbox) = host()?;
    owner.play.as_ref().ok_or("play")?.link.open_party();
    let _ = commands.try_recv()?;
    publish(&outbox, &state(), party::PartyDisposition::Ok)?;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.world_mut().spawn((PartyDock, Node::default()));
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, kind: impl Fn(Kind) -> bool) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| kind(control.kind))
        .map(|(entity, _)| entity)
        .ok_or("control")?;
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity });
    Ok(())
}
fn command(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> std::result::Result<oteryn_client_runtime::party::PartyCommand, Box<dyn std::error::Error>> {
    let PlayCommand::Party(command) = commands.try_recv()? else {
        return Err("party command".into());
    };
    Ok(command)
}
#[test]
fn actual_member_and_invitation_controls_send_exact_handles_without_optimistic_membership() -> Result
{
    for action in [
        Kind::Pass,
        Kind::Kick,
        Kind::Join,
        Kind::Leave,
        Kind::Shared,
    ] {
        let (mut app, mut commands, outbox) = app()?;
        let before = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .party_panel()
            .state;
        if matches!(action, Kind::Pass | Kind::Kick) {
            click(&mut app, |kind| matches!(kind, Kind::Member(2)))?;
            app.update();
        }
        if matches!(action, Kind::Join) {
            click(&mut app, |kind| matches!(kind, Kind::Invitation(3)))?;
            app.update();
        }
        click(&mut app, |kind| {
            matches!(
                (kind, action),
                (Kind::Pass, Kind::Pass)
                    | (Kind::Kick, Kind::Kick)
                    | (Kind::Join, Kind::Join)
                    | (Kind::Leave, Kind::Leave)
                    | (Kind::Shared, Kind::Shared)
            )
        })?;
        let sent = command(&mut commands)?;
        if matches!(action, Kind::Shared) {
            assert_eq!(sent.shared_setting(), Some(true));
        } else {
            assert_eq!(
                party::decode_party_intent(&sent.payload).map_err(|_| "intent")?,
                match action {
                    Kind::Pass => party::PartyIntent::PassLeadership(2),
                    Kind::Kick => party::PartyIntent::Kick(2),
                    Kind::Join => party::PartyIntent::Accept(3),
                    _ => party::PartyIntent::Leave,
                }
            );
        }
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .party_panel()
                .state,
            before
        );
        assert!(commands.try_recv().is_err());
        publish(&outbox, &state(), party::PartyDisposition::NotLeader)?;
        app.update();
        let panel = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .party_panel();
        assert_eq!(panel.state, before);
        assert!(panel.pending.is_none());
        assert!(
            panel
                .feedback
                .is_some_and(|message| message.contains("leader"))
        );
    }
    Ok(())
}
#[test]
fn closing_pending_party_preserves_data_and_late_response_stays_closed() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    click(&mut app, |kind| matches!(kind, Kind::Refresh))?;
    assert_eq!(
        command(&mut commands)?.kind,
        party::COMMAND_TYPE_PARTY_QUERY
    );
    click(&mut app, |kind| matches!(kind, Kind::Close))?;
    app.update();
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .party_panel();
    assert!(!panel.open);
    assert!(panel.pending.is_some());
    assert_eq!(panel.state, Some(state()));
    publish(&outbox, &state(), party::PartyDisposition::Ok)?;
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str() == "party.sidebar")
            .count(),
        0
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn stale_revision_selection_pending_capability_and_top_modal_never_dispatch() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    click(&mut app, |kind| matches!(kind, Kind::Member(2)))?;
    app.update();
    let mut changed = state();
    changed.members[1].name = "Changed member".into();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .party_request(oteryn_client_ui::party::Intent::Refresh);
    let _ = command(&mut commands)?;
    publish(&outbox, &changed, party::PartyDisposition::Ok)?;
    click(&mut app, |kind| matches!(kind, Kind::Kick))?;
    assert!(commands.try_recv().is_err());
    app.update();
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(180), Node::default()))
        .id();
    click(&mut app, |kind| matches!(kind, Kind::Leave))?;
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    outbox.bevy_party_selected(false, true, "Leader");
    click(&mut app, |kind| matches!(kind, Kind::Leave))?;
    assert!(commands.try_recv().is_err());
    outbox.bevy_party_selected(true, true, "Leader");
    app.world_mut().non_send_mut::<HostOwner>().close();
    click(&mut app, |kind| matches!(kind, Kind::Leave))?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
mod context;
