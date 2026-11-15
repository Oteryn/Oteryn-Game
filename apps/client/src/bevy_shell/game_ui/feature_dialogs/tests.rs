use super::*;
use crate::play::{PlayView, play_channel};
use oteryn_renderer::TileCoord;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;

#[test]
fn modal_submit_refuses_old_generation_and_no_longer_admitted_owner() -> Result {
    let mut owner = HostOwner::new(None)?;
    let (link, _commands, _outbox) = play_channel();
    owner.play = Some(super::super::super::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    let key = Key {
        generation: owner.generation(),
        revision: 1,
    };
    let mut state = State {
        key: Some(key),
        ..default()
    };
    assert!(!submit(
        &owner,
        &mut state,
        Key {
            generation: key.generation + 1,
            ..key
        },
        1,
        ""
    ));
    owner.close();
    assert!(!submit(&owner, &mut state, key, 1, ""));
    Ok(())
}

#[test]
fn server_input_fields_preserve_multiline_and_source_font_on_first_mount() -> Result {
    use oteryn_client_runtime::server_modal::{
        Dialog,
        input::{DialogInput, InputKind, InputValue},
    };
    let mut world = World::new();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);
    let overlay = commands.spawn_empty().id();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .add_systems(
            Startup,
            (crate::bevy_shell::ui::setup, notices::setup).chain(),
        );
    // Independent feature windows can also register the shared modal marker.
    app.world_mut().spawn((
        GameModal,
        Node {
            display: Display::None,
            ..default()
        },
    ));
    app.update();
    let death = app
        .world_mut()
        .query_filtered::<&Node, (With<GameModal>, With<notices::DeathOverlay>)>()
        .iter(app.world())
        .map(|node| node.display)
        .collect::<Vec<_>>();
    let notices = app
        .world_mut()
        .query_filtered::<&Node, (With<GameModal>, With<notices::NoticeOverlay>)>()
        .iter(app.world())
        .map(|node| node.display)
        .collect::<Vec<_>>();
    assert_eq!(death, [Display::None], "one initially hidden death overlay");
    assert_eq!(
        notices,
        [Display::None],
        "one initially hidden notice overlay"
    );
    let skin = app.world_mut().remove_resource::<Skins>().ok_or("skins")?;
    let model = oteryn_client_ui::server_modal::Panel {
        selected: true,
        input: true,
        active: Some(Dialog {
            id: 73,
            title: "Edit".into(),
            message: "Server text".into(),
            choices: vec![],
            buttons: vec![(1, "Ok".into())],
            default_button: None,
            cancel_button: None,
            image: None,
        }),
        active_input: Some(DialogInput {
            kind: InputKind::Edit,
            minimum: 0,
            maximum: 120,
            initial: Some(InputValue::Text("First\nSecond".into())),
        }),
        ..default()
    };
    let (_, focus) = build::panel(
        &mut commands,
        &skin,
        overlay,
        Key {
            generation: 1,
            revision: 1,
        },
        &model,
        &mut Assets::default(),
    );
    queue.apply(&mut world);
    let focused = focus.ok_or("field focus")?;
    let input = world.get::<EditableText>(focused).ok_or("editable")?;
    assert_eq!(input.value().to_string(), "First\nSecond");
    assert!(input.allow_newlines);
    assert_eq!(input.max_characters, Some(120));
    assert_eq!(
        world.get::<TextFont>(focused).ok_or("font")?.font_size,
        bevy::text::FontSize::Px(11.)
    );
    Ok(())
}

#[test]
fn shared_notices_close_preserves_server_owned_history() {
    let (link, _commands, outbox) = play_channel();
    outbox.push(vec![oteryn_session::SessionEvent::ServerSaveCountdown(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::ServerSaveCountdown {
                ending_epoch: 3,
                minutes_remaining: 5,
            },
        },
    )]);
    link.notices_set_open(true);
    assert!(link.notices_panel().open);
    assert_eq!(link.notices_panel().save_minutes, Some(5));
    link.notices_set_open(false);
    assert!(!link.notices_panel().open);
    assert_eq!(link.notices_panel().save_minutes, Some(5));
}

mod text_input;
mod text_window;

#[test]
fn death_acknowledgement_enter_waits_for_respawn_and_respects_frontmost_modal() -> Result {
    use bevy::input::{
        ButtonState,
        keyboard::{Key as LogicalKey, KeyboardInput},
    };
    let mut owner = HostOwner::new(None)?;
    let (link, mut commands, outbox) = play_channel();
    let mut view = PlayView::new(TileCoord::new(1, 1), 0, &[])?;
    let vitals = |health| {
        vec![oteryn_session::SessionEvent::ActorVitals(
            oteryn_session::AppliedDelta {
                server_sequence: 1,
                base_revision: 0,
                new_revision: 1,
                value: oteryn_session::ActorVitals {
                    health,
                    max_health: 150,
                    ..default()
                },
            },
        )]
    };
    outbox.push(vitals(0));
    view.tick(&link).map_err(|e| format!("{e:?}"))?;
    owner.play = Some(super::super::super::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_message::<KeyboardInput>()
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    install_notice_input_for_test(&mut app);
    app.update();
    let enter = || KeyboardInput {
        key_code: KeyCode::Enter,
        logical_key: LogicalKey::Enter,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut().write_message(enter());
    app.update();
    let notice = |app: &App| {
        app.world()
            .get_non_send::<HostOwner>()
            .and_then(|host| host.play.as_ref())
            .and_then(|play| play.view.death_modal_notice())
    };
    assert!(
        notice(&app).is_some(),
        "Enter cannot invent a server respawn"
    );
    outbox.push(vitals(150));
    {
        let mut host = app
            .world_mut()
            .get_non_send_mut::<HostOwner>()
            .ok_or("host")?;
        let play = host.play.as_mut().ok_or("play")?;
        play.view.tick(&play.link).map_err(|e| format!("{e:?}"))?;
    }
    app.update();
    assert!(
        notice(&app).is_some(),
        "respawn must retain the death notice"
    );
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(200), Node::default()))
        .id();
    app.world_mut().write_message(enter());
    app.update();
    assert!(notice(&app).is_some(), "another modal owns Enter");
    app.world_mut().despawn(higher);
    app.world_mut().write_message(enter());
    app.update();
    assert!(
        notice(&app).is_none(),
        "Enter acknowledges the visible death"
    );
    assert!(
        commands.try_recv().is_err(),
        "acknowledgement sends no fake respawn command"
    );
    Ok(())
}
