use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_ui::chat_private_channel::ChatPanel;

type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn owner_panel() -> ChatPanel {
    ChatPanel {
        selected: true,
        list: Some(oteryn_session::ChatChannelList {
            own_channel: Some(42),
            invited: vec![oteryn_session::InvitedChannel {
                channel_id: 87,
                owner_name: "Ada".into(),
            }],
            ..default()
        }),
        channel: Some(42),
        name: "Lin".into(),
        text: "Hello".into(),
        ..default()
    }
}
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.private_channels_test_panel(owner_panel());
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
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
    // Open through the registered toolbar observer, as a user does.
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| {
            matches!(c, crate::bevy_shell::ui::Control::PrivateChannels).then_some(e)
        })
        .ok_or("private channels toolbar missing")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    Ok((app, commands, outbox))
}
fn control(app: &mut App, action: Action) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.action == action).then_some(e))
        .ok_or("control missing".into())
}
fn panel(app: &App) -> Result<ChatPanel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .chat_private_panel())
}
fn click(app: &mut App, action: Action) -> TestResult {
    let entity = control(app, action)?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
#[test]
fn private_channels_window_shows_owner_data_only() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains(&panel_text(&owner_panel())));
    for expected in [
        "Own channel (42)",
        "Ada (87)",
        "Create/Open own",
        "Refresh",
        "Join",
        "Close own channel",
        "Invite player",
        "Exclude player",
        "Send",
        "Close",
    ] {
        assert!(text.contains(expected), "missing {expected}");
    }
    assert_eq!(panel(&app)?, owner_panel());
    assert!(commands.try_recv().is_err());
    let fields = app
        .world_mut()
        .query::<(&Field, &EditableText)>()
        .iter(app.world())
        .map(|(f, t)| (f.0, t.value().to_string()))
        .collect::<Vec<_>>();
    assert!(fields.contains(&(0, "Lin".into())));
    assert!(fields.contains(&(1, "Hello".into())));
    Ok(())
}
#[test]
fn private_channels_window_registered() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    assert!(app.world().resource::<State>().root.is_some());
    click(&mut app, Action::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert!(commands.try_recv().is_err());
    let registrations = include_str!("../mod.rs");
    assert!(registrations.contains("private_channels::install(app);"));
    let toolbar = include_str!("../../ui/toolbar.rs");
    assert!(toolbar.contains("| Control::PrivateChannels"));
    assert!(!include_str!("../../controls.rs").contains("KeyCode::F8"));
    Ok(())
}
#[test]
fn private_channels_window_confirmation_invalidated_by_edits() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    click(&mut app, Action::Request(Intent::Exclude))?;
    app.update();
    let confirm = control(&mut app, Action::Confirm(Intent::Exclude))?;
    assert!(commands.try_recv().is_err());
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 0 {
            text.editor_mut().set_text("Changed");
        }
    }
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(commands.try_recv().is_err());
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert_eq!(panel(&app)?, owner_panel());
    Ok(())
}
#[test]
fn private_channels_window_close_preserves_pending_when_unavailable_and_blocked() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    click(&mut app, Action::Request(Intent::Create))?;
    let _ = commands.try_recv()?;
    let mut pending = panel(&app)?;
    assert!(pending.pending);
    pending.selected = false;
    outbox.private_channels_test_panel(pending.clone());
    outbox.private_channels_test_block(true);
    // Close deliberately accepts the old rendered target while its request is in flight.
    click(&mut app, Action::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert_eq!(panel(&app)?, pending);
    assert!(commands.try_recv().is_err());
    outbox.private_channels_test_block(false);
    let generation = app.world().non_send::<HostOwner>().generation();
    let bound = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .chat_private_bound();
    app.world_mut().resource_mut::<State>().session = Some((generation, bound));
    app.update();
    assert_eq!(panel(&app)?, pending);
    assert!(app.world().resource::<State>().root.is_some());
    outbox.private_channels_test_end();
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn private_channels_window_other_modal_rejects_without_mutation() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    app.world_mut()
        .spawn((GameModal, GlobalZIndex(900), Node::default()));
    click(&mut app, Action::Request(Intent::Create))?;
    assert_eq!(panel(&app)?, owner_panel());
    assert!(commands.try_recv().is_err());
    click(&mut app, Action::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

#[test]
fn private_channels_window_confirmation_sends_frozen_target_once() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    click(&mut app, Action::Request(Intent::Exclude))?;
    assert!(commands.try_recv().is_err());
    app.update();
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("Exclude Lin from own channel 42?"));
    let confirm = control(&mut app, Action::Confirm(Intent::Exclude))?;
    app.world_mut().trigger(Activate { entity: confirm });
    app.world_mut().trigger(Activate { entity: confirm });
    assert!(matches!(commands.try_recv()?, PlayCommand::ChatPublic(
        oteryn_session::ChatIntent::PrivateChannel(
            oteryn_client_runtime::chat_private_channel::PrivateChannelIntent::Exclude { name }
        )
    ) if name == "Lin"));
    assert!(commands.try_recv().is_err());
    assert!(panel(&app)?.pending);
    assert_eq!(panel(&app)?.list, owner_panel().list);
    Ok(())
}

#[test]
fn private_channels_window_rejects_replaced_session_with_identical_panel() -> TestResult {
    let (mut app, mut old_commands, _) = fixture()?;
    let stale = control(&mut app, Action::Request(Intent::Create))?;
    let (link, mut commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.private_channels_test_panel(owner_panel());
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = link;
    app.world_mut().trigger(Activate { entity: stale });
    assert_eq!(panel(&app)?, owner_panel());
    assert!(commands.try_recv().is_err());
    assert!(old_commands.try_recv().is_err());
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}
