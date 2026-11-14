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

#[test]
fn server_text_labels_show_each_kind_in_its_own_colour() -> Result {
    use crate::play::feature_local_game_notifications::state::ScreenPlace;
    use oteryn_session::{AppliedDelta, ServerMessage, SessionEvent};
    let (link, _commands, outbox) = play_channel();
    outbox.push(vec![SessionEvent::ServerMessage(AppliedDelta {
        server_sequence: 1,
        base_revision: 0,
        new_revision: 1,
        value: ServerMessage::Warning("Fire!".into()),
    })]);
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    let Some(crate::play::PlayEvent::Pushed(delta)) = link.poll() else {
        return Err("missing message".into());
    };
    view.apply_pushed(&delta)?;
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(super::super::super::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(
            Startup,
            (crate::bevy_shell::ui::setup, notices::setup).chain(),
        )
        .add_systems(Update, notices::project);
    app.update();
    app.update();
    let mut labels = app
        .world_mut()
        .query::<(&notices::ScreenMessage, &Text, &TextColor)>();
    let shown = labels
        .iter(app.world())
        .map(|(label, text, color)| (label.0, text.0.clone(), color.0))
        .collect::<Vec<_>>();
    assert_eq!(shown.len(), 3);
    for (place, text, color) in shown {
        let [r, g, b] = place.rgb();
        assert_eq!(color, Color::srgb_u8(r, g, b));
        assert_eq!(
            text,
            if place == ScreenPlace::LowCenter {
                "Fire!"
            } else {
                ""
            }
        );
    }
    Ok(())
}
