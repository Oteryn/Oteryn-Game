use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayLink, play_channel},
};
use oteryn_session::container_tree::{ContainerView, ContainerViews};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn views() -> std::result::Result<ContainerViews, Box<dyn std::error::Error>> {
    Ok(ContainerViews {
        views: vec![
            ContainerView {
                view_id: 0,
                container_handle: 10.try_into()?,
                parent_handle: None,
                capacity: 20,
                entries: vec![tests::entry(11, 7)?, tests::entry(12, 3)?],
                browse: None,
            },
            ContainerView {
                view_id: 1,
                container_handle: 20.try_into()?,
                parent_handle: None,
                capacity: 20,
                entries: vec![],
                browse: None,
            },
        ],
    })
}
fn app() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    use bevy::asset::AssetPlugin;
    let (link, commands, outbox) = play_channel();
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(Default::default()),
        Some(views()?),
        true,
        true,
    );
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Font>()
    .init_asset::<Image>()
    .init_resource::<bevy::input_focus::InputFocus>()
    .init_resource::<bevy::text::RemSize>()
    .init_resource::<UiScale>()
    .init_resource::<ButtonInput<KeyCode>>()
    .init_resource::<ButtonInput<MouseButton>>()
    .insert_non_send(tests::host(link)?)
    .add_systems(
        Startup,
        (
            crate::bevy_shell::ui::setup,
            crate::bevy_shell::game_ui::setup,
        )
            .chain(),
    )
    .add_systems(Update, crate::bevy_shell::game_ui::project);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, name: &str) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find(|(_, value)| value.as_str() == name)
        .map(|(entity, _)| entity)
        .ok_or_else(|| format!("missing actual control: {name}"))?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
#[test]
fn container_header_and_destination_choice_dispatch_ordered_real_handles_once() -> Result {
    let (mut app, mut commands, _) = app()?;
    click(&mut app, "inventory.bulk.open")?;
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .container_activate(1)
    );
    click(&mut app, "inventory.bulk.destination")?;
    let PlayCommand::MoveContainerContents(requests) = commands.try_recv()? else {
        return Err("bulk move".into());
    };
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].source, 11.try_into()?);
    assert_eq!(requests[1].source, 12.try_into()?);
    let destination = ItemMoveDestination::Container(20.try_into()?);
    assert!(
        requests
            .iter()
            .all(|request| request.destination == destination && request.count.is_none())
    );
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .container_panel()
            .views,
        Some(views()?)
    );
    click(&mut app, "inventory.bulk.destination")?;
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn changed_source_or_destination_snapshot_never_uses_recycled_open_view() -> Result {
    for source in [true, false] {
        let (mut app, mut commands, outbox) = app()?;
        click(&mut app, "inventory.bulk.open")?;
        app.update();
        let mut changed = views()?;
        if source {
            changed.views[0].entries.pop();
        } else {
            changed.views[1].container_handle = 30.try_into()?;
        }
        outbox.push(vec![oteryn_session::SessionEvent::ContainerViews(
            oteryn_session::AppliedDelta {
                server_sequence: 2,
                base_revision: 0,
                new_revision: 1,
                value: changed.clone(),
            },
        )]);
        click(&mut app, "inventory.bulk.destination")?;
        assert!(commands.try_recv().is_err());
        assert_eq!(
            app.world()
                .non_send::<HostOwner>()
                .play
                .as_ref()
                .ok_or("play")?
                .link
                .container_panel()
                .views,
            Some(changed)
        );
    }
    Ok(())
}
#[test]
fn higher_modal_pending_container_and_expired_session_block_real_destination_choice() -> Result {
    for mode in 0..3 {
        let (mut app, mut commands, _) = app()?;
        click(&mut app, "inventory.bulk.open")?;
        app.update();
        match mode {
            0 => {
                app.world_mut().spawn((
                    super::super::GameModal,
                    GlobalZIndex(180),
                    Node::default(),
                ));
            }
            1 => {
                assert!(
                    app.world()
                        .non_send::<HostOwner>()
                        .play
                        .as_ref()
                        .ok_or("play")?
                        .link
                        .container_open(10.try_into()?)
                );
                let _ = commands.try_recv()?;
            }
            _ => app.world_mut().non_send_mut::<HostOwner>().close(),
        }
        click(&mut app, "inventory.bulk.destination")?;
        assert!(commands.try_recv().is_err());
    }
    Ok(())
}
#[test]
fn bound_owner_rejects_actual_server_modal_and_closed_command_channel() -> Result {
    let (link, commands, outbox) = play_channel();
    PlayLink::bevy_items_fixture(
        &outbox,
        Some(Default::default()),
        Some(views()?),
        true,
        true,
    );
    let views = views()?;
    {
        let _modal = link.bevy_server_modal_fixture();
        assert!(link.server_modal_active());
        assert!(!link.container_move_all_bound(&views.views[0], &views.views[1]));
        assert!(!link.item_movement().pending);
    }
    assert!(!link.server_modal_active());
    drop(commands);
    assert!(!link.container_move_all_bound(&views.views[0], &views.views[1]));
    assert!(!link.item_movement().pending);
    assert_eq!(link.container_panel().views, Some(views));
    assert!(
        link.item_movement()
            .feedback
            .is_some_and(|message| message.contains("Session ended"))
    );
    Ok(())
}
