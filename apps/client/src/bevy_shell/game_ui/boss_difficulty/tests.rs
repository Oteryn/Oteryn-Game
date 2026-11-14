use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::{
    boss_difficulty::{Reply, Request, decode_request, encode_reply},
    raw_feature::{CommandStatus, GatedCommandResult},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn fixture() -> Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_boss_difficulty_panel(Panel {
        selected: true,
        open: true,
        ..Default::default()
    });
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
    Ok((app, commands, outbox))
}
fn panel(app: &App) -> Panel {
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .map(|p| p.link.boss_difficulty_panel())
        .unwrap_or_default()
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("control missing")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn reply(outbox: &Outbox, value: &Reply) -> TestResult {
    outbox.bevy_boss_difficulty_result(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: encode_reply(value).map_err(|e| format!("{e:?}"))?,
    });
    Ok(())
}
fn window() -> Reply {
    Reply::Window {
        context: [7; 16],
        boss_key: "oteryn:dragon".into(),
        boss_creature_key: "oteryn:dragon".into(),
        content_revision: "server-r17".into(),
        selected_difficulty: 2,
        personal_highest: 4,
        unlocked_max: 5,
        maximum_difficulty: 25,
    }
}
fn request(
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<Request, Box<dyn std::error::Error>> {
    let PlayCommand::Feature(command) = rx.try_recv()? else {
        return Err("wrong dispatcher".into());
    };
    assert_eq!((command.capability, command.kind), (225, 431));
    decode_request(&command.payload).map_err(|e| format!("{e:?}").into())
}
#[test]
fn boss_difficulty_window_shows_server_data_only() -> TestResult {
    let (mut app, _rx, outbox) = fixture()?;
    assert!(panel(&app).reply.is_none());
    reply(&outbox, &window())?;
    app.update();
    let expected = oteryn_client_ui::boss_difficulty::panel_text(&panel(&app));
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == expected)
    );
    assert!(expected.contains("server-r17"));
    assert!(expected.contains("Personal highest: 4"));
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("07070707070707070707070707070707"))
    );
    let mut selected = panel(&app);
    selected.pity_selected = true;
    outbox.bevy_boss_difficulty_panel(selected);
    let tracker = oteryn_client_runtime::boss_difficulty::PityTracker {
        kills_without_drop: 40,
        bonus_permille: 200,
        tracker_active: true,
    };
    outbox.bevy_boss_difficulty_result(&GatedCommandResult {
        command_id: 2,
        server_sequence: 2,
        status: CommandStatus::Accepted,
        payload: oteryn_client_runtime::boss_difficulty::encode_reply_with_pity(
            &window(),
            Some(&tracker),
        )
        .map_err(|e| format!("{e:?}"))?,
    });
    app.update();
    let expected = oteryn_client_ui::boss_difficulty::panel_text(&panel(&app));
    assert_eq!(panel(&app).pity, Some(tracker));
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == expected)
    );
    reply(&outbox, &Reply::Closed { context: [7; 16] })?;
    app.update();
    let expected = oteryn_client_ui::boss_difficulty::panel_text(&panel(&app));
    assert!(panel(&app).context.is_none());
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == expected)
    );
    Ok(())
}
#[test]
fn boss_difficulty_actions_are_bound_and_confirmed() -> TestResult {
    let (mut app, mut rx, outbox) = fixture()?;
    app.world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
        .for_each(|(field, mut text)| {
            if *field == Field::Boss {
                *text = EditableText::new("oteryn:dragon");
            }
        });
    click(&mut app, Kind::Request(Intent::Open))?;
    assert_eq!(
        request(&mut rx)?,
        Request::OpenWithProgress {
            boss_key: "oteryn:dragon".into()
        }
    );
    assert!(panel(&app).reply.is_none());
    reply(&outbox, &window())?;
    app.update();
    let confirmed = panel(&app).reply;
    app.world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
        .for_each(|(field, mut text)| {
            if *field == Field::Difficulty {
                *text = EditableText::new("4");
            }
        });
    click(&mut app, Kind::Request(Intent::Preview))?;
    assert_eq!(
        request(&mut rx)?,
        Request::Preview {
            context: [7; 16],
            difficulty: 4
        }
    );
    assert_eq!(panel(&app).reply, confirmed);
    assert_eq!(panel(&app).difficulty, 2);
    reply(&outbox, &window())?;
    click(&mut app, Kind::Request(Intent::Preview))?;
    assert!(
        rx.try_recv().is_err(),
        "a consumed render cannot submit twice after a fast reply"
    );
    app.update();
    for intent in [Intent::Start, Intent::Cancel] {
        click(&mut app, Kind::Request(intent))?;
        assert!(rx.try_recv().is_err());
        app.update();
        click(&mut app, Kind::Confirm)?;
        click(&mut app, Kind::Confirm)?;
        let expected = if intent == Intent::Start {
            Request::Start {
                context: [7; 16],
                difficulty: 2,
            }
        } else {
            Request::Cancel { context: [7; 16] }
        };
        assert_eq!(request(&mut rx)?, expected);
        assert!(rx.try_recv().is_err());
        assert_eq!(panel(&app).reply, confirmed);
        reply(&outbox, &window())?;
        app.update();
    }
    Ok(())
}
#[test]
fn boss_difficulty_rejected_when_session_replaced_or_target_changed() -> TestResult {
    let (mut app, mut rx, outbox) = fixture()?;
    reply(&outbox, &window())?;
    app.update();
    click(&mut app, Kind::Request(Intent::Start))?;
    app.update();
    let mut changed = panel(&app);
    changed.context = Some([8; 16]);
    outbox.bevy_boss_difficulty_panel(changed.clone());
    click(&mut app, Kind::Confirm)?;
    assert!(rx.try_recv().is_err());
    assert_eq!(panel(&app), changed);
    app.update();
    let higher = app
        .world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    click(&mut app, Kind::Request(Intent::Preview))?;
    assert!(rx.try_recv().is_err());
    assert_eq!(panel(&app), changed);
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    let old = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == Kind::Request(Intent::Preview)).then_some(e))
        .ok_or("preview")?;
    outbox.bevy_boss_difficulty_end();
    {
        let owner = app.world().non_send::<HostOwner>();
        let play = owner.play.as_ref().ok_or("play")?;
        assert!(!play.link.boss_difficulty_dispatch_bound(
            &changed,
            Intent::Preview,
            "oteryn:dragon",
            2,
            false
        ));
        assert_eq!(play.link.boss_difficulty_panel(), changed);
    }
    let (link, mut replacement_rx, replacement_outbox) = play_channel();
    link.use_shared_frontend();
    replacement_outbox.bevy_boss_difficulty_panel(changed.clone());
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let previous = owner.generation();
        owner.poll([960, 640]);
        assert!(owner.generation() > previous);
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
    }
    app.world_mut().trigger(Activate { entity: old });
    assert!(rx.try_recv().is_err());
    assert!(replacement_rx.try_recv().is_err());
    assert_eq!(panel(&app), changed);
    Ok(())
}
#[test]
fn boss_difficulty_window_registered() -> TestResult {
    let (mut app, _rx, _outbox) = fixture()?;
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(!panel(&app).open);
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.id() == "shell.boss_difficulty").then_some(e))
        .ok_or("toolbar entry")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(panel(&app).open);
    click(&mut app, Kind::Close)?;
    app.update();
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(28)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    app.update();
    assert!(panel(&app).open);
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        1
    );
    let mut keys = ButtonInput::<KeyCode>::default();
    keys.press(KeyCode::ControlLeft);
    app.insert_resource(keys);
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::KeyY,
            logical_key: bevy::input::keyboard::Key::Character("y".into()),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(!panel(&app).open);
    let registry = include_str!("../mod.rs");
    assert!(registry.contains("boss_difficulty::install(app)"));
    Ok(())
}
