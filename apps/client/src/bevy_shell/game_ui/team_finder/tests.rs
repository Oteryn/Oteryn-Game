use super::*;
use crate::play::{PlayView, play_channel};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture() -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
        crate::play::Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let (link, rx, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_team_finder_state(
        oteryn_client_ui::team_finder::Panel {
            selected: true,
            ..default()
        },
        oteryn_client_ui::team_finder_leader::Panel {
            selected: true,
            ..default()
        },
    );
    let mut owner = HostOwner::new(None)?;
    let view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
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
    Ok((app, rx, outbox))
}
#[test]
fn team_finder_window_registered() -> TestResult {
    let (mut app, _rx, _outbox) = fixture()?;
    assert_eq!(
        crate::bevy_shell::ui::Control::TeamFinder.id(),
        "shell.team_finder"
    );
    let button = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::TeamFinder).then_some(e))
        .ok_or("toolbar entry")?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        1
    );
    let close = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.action == Action::Close).then_some(e))
        .ok_or("close")?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        0
    );
    use bevy::input::keyboard::{Key, KeyboardInput};
    let f8 = || KeyboardInput {
        key_code: KeyCode::F8,
        logical_key: Key::F8,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut().write_message(f8());
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        1
    );
    app.world_mut().write_message(f8());
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}
#[test]
fn team_finder_window_shows_server_data_only() -> TestResult {
    let (mut app, mut rx, outbox) = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    use oteryn_client_ui::team_finder::*;
    use oteryn_client_ui::team_finder_leader::{LeaderListing, TeamTarget};
    let listing = LeaderListing {
        min_level: 11,
        max_level: 77,
        vocation_mask: 3,
        slots: 4,
        free_slots: 3,
        party_members_join: true,
        timestamp: 1234,
        target: TeamTarget::Quest(7),
    };
    let result = TeamFinderResult::Browse {
        listings: vec![BrowseListing {
            leader_handle: 99,
            name: "Server scouts".into(),
            listing,
            occupied_slots: 2,
            status: MemberStatus::Pending,
        }],
        next: Some(99),
    };
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(link.team_finder_dispatch_bound(
            &link.team_finder_snapshot(),
            Action::Browse {
                applicants: false,
                next: false
            },
            false
        ));
    }
    outbox.bevy_team_finder_response(
        232,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: encode_result(&result).map_err(|error| format!("{error:?}"))?,
        },
    );
    app.update();
    let snapshot = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .team_finder_snapshot();
    let expected = super::panel_text(&snapshot);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == expected)
    );
    assert_eq!(snapshot.teams.result, Some(result));
    for name in snapshot.teams.visible_names() {
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains(&name))
        );
    }
    assert!(expected.contains("Server scouts"));
    assert!(expected.contains("2 free"));
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(link.team_finder_dispatch_bound(
            &snapshot,
            Action::Filter(TeamFilter {
                name: "absent".into(),
                ..default()
            }),
            false
        ));
    }
    app.update();
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("Server scouts"))
    );
    {
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("play")?.link;
        assert!(link.team_finder_dispatch_bound(
            &link.team_finder_snapshot(),
            Action::Browse {
                applicants: true,
                next: false
            },
            false
        ));
    }
    let result = TeamFinderResult::Applicants {
        listing: Some(listing),
        occupied_slots: 2,
        members: vec![Applicant {
            handle: 17,
            name: "Server applicant".into(),
            level: 33,
            vocation: 2,
            status: MemberStatus::Pending,
        }],
        next: None,
    };
    outbox.bevy_team_finder_response(
        232,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 2,
            server_sequence: 2,
            status: oteryn_session::CommandStatus::Accepted,
            payload: encode_result(&result).map_err(|error| format!("{error:?}"))?,
        },
    );
    app.update();
    let snapshot = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .team_finder_snapshot();
    assert_eq!(snapshot.teams.result, Some(result));
    let expected = super::panel_text(&snapshot);
    assert!(expected.contains("Server applicant"));
    assert!(expected.contains("vocation 2"));
    assert!(expected.contains("start 1234"));
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == expected)
    );
    for _ in 0..2 {
        assert!(
            matches!(rx.try_recv(), Ok(crate::combat_input::PlayCommand::ServerMechanic(command)) if command.command_type == 444)
        );
    }
    assert!(rx.try_recv().is_err());
    let deny = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.action == Action::Decide(4)).then_some(entity))
        .ok_or("deny control")?;
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: deny });
    assert!(app.world().resource::<State>().confirmation.is_none());
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    app.world_mut().trigger(Activate { entity: deny });
    assert_eq!(
        app.world().resource::<State>().confirmation,
        Some(Action::Decide(4))
    );
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .team_finder_snapshot(),
        snapshot
    );
    app.update();
    let confirm = app
        .world_mut()
        .query::<(Entity, &FormControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control.action, FormAction::Confirm).then_some(entity)
        })
        .ok_or("confirm control")?;
    app.world_mut().trigger(Activate { entity: confirm });
    app.world_mut().trigger(Activate { entity: confirm });
    let after = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .team_finder_snapshot();
    assert_eq!(after.teams.pending, Some(446));
    assert_eq!(after.teams.result, snapshot.teams.result);
    let crate::combat_input::PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!(command.command_type, 446);
    assert_eq!(
        decode_leader_action(&command.payload).map_err(|error| format!("{error:?}"))?,
        LeaderAction::Deny { member: 17 }
    );
    assert!(rx.try_recv().is_err());
    outbox.bevy_team_finder_response(
        232,
        &oteryn_client_runtime::server_mechanic::ServerMechanicResponse {
            command_id: 3,
            server_sequence: 3,
            status: oteryn_session::CommandStatus::Rejected,
            payload: vec![],
        },
    );
    app.update();
    let fields: Vec<_> = app
        .world_mut()
        .query::<(Entity, &Field)>()
        .iter(app.world())
        .map(|(entity, field)| (entity, field.0))
        .collect();
    for (entity, index) in &fields {
        if let Some(value) = ["11", "77", "3", "4", "3", "1", "3", "7", "0", "1234"].get(*index) {
            app.world_mut()
                .entity_mut(*entity)
                .insert(EditableText::new(*value));
        }
    }
    let field = fields
        .iter()
        .find_map(|(entity, index)| (*index == 0).then_some(*entity))
        .ok_or("minimum level")?;
    app.world_mut()
        .get_mut::<EditableText>(field)
        .ok_or("editable")?
        .queue_edit(bevy::text::TextEdit::Insert("2".into()));
    let publish = app
        .world_mut()
        .query::<(Entity, &FormControl)>()
        .iter(app.world())
        .find_map(|(entity, control)| {
            matches!(control.action, FormAction::Publish).then_some(entity)
        })
        .ok_or("publish")?;
    app.world_mut().trigger(Activate { entity: publish });
    app.world_mut().trigger(Activate { entity: publish });
    app.update();
    assert!(rx.try_recv().is_err());
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("12"));
    app.update();
    let crate::combat_input::PlayCommand::ServerMechanic(command) = rx.try_recv()? else {
        return Err("wrong publication".into());
    };
    assert_eq!((command.capability, command.command_type), (200, 390));
    assert_eq!(
        oteryn_client_ui::team_finder_leader::decode_upsert(&command.payload)
            .map_err(|error| format!("{error:?}"))?
            .min_level,
        12
    );
    assert!(rx.try_recv().is_err());
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .team_finder_snapshot()
            .leader
            .confirmed
            .is_none()
    );
    Ok(())
}
