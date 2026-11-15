use super::super::*;
use oteryn_client_runtime::server_modal::{
    Dialog,
    input::{DialogInput, InputKind, InputValue},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn render(input: Option<DialogInput>, message: &str) -> Result<World, Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    app.update();
    let skin = app.world_mut().remove_resource::<Skins>().ok_or("skins")?;
    let mut world = World::new();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, &world);
    let overlay = commands.spawn_empty().id();
    build::panel(
        &mut commands,
        &skin,
        overlay,
        Key {
            generation: 1,
            revision: 1,
        },
        &oteryn_client_ui::server_modal::Panel {
            active: Some(Dialog {
                id: 7,
                title: "Text".into(),
                message: message.into(),
                choices: vec![],
                buttons: vec![(1, "Ok".into())],
                cancel_button: None,
                image: None,
            }),
            active_input: input,
            ..default()
        },
        &mut Assets::default(),
    );
    queue.apply(&mut world);
    Ok(world)
}

#[test]
fn text_window_read_only() -> TestResult {
    // Metadata is supplied in the server message; the client must preserve it verbatim.
    for message in [
        "You read the following, written by Ada on 2026-11-15:\nA sign.",
        "It is empty.",
    ] {
        let mut world = render(None, message)?;
        assert!(
            world
                .query::<&Text>()
                .iter(&world)
                .any(|text| text.0 == message)
        );
        assert_eq!(world.query::<&EditableText>().iter(&world).count(), 0);
        assert_eq!(
            world
                .query::<&Action>()
                .iter(&world)
                .filter(|a| matches!(a.kind, ActionKind::Button(_)))
                .count(),
            1
        );
        assert!(
            !world
                .query::<&Text>()
                .iter(&world)
                .any(|text| text.0 == "Save")
        );
    }
    Ok(())
}

#[test]
fn text_window_writable_unchanged() -> TestResult {
    let mut world = render(
        Some(DialogInput {
            kind: InputKind::Edit,
            minimum: 0,
            maximum: 120,
            initial: Some(InputValue::Text("First\nSecond".into())),
        }),
        "You can enter new text.",
    )?;
    let editor = world.query::<&EditableText>().single(&world)?;
    assert_eq!(editor.value().to_string(), "First\nSecond");
    assert!(editor.allow_newlines);
    assert_eq!(editor.max_characters, Some(120));
    Ok(())
}

#[test]
fn house_list_window() -> TestResult {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    app.update();
    let skin = app.world_mut().remove_resource::<Skins>().ok_or("skins")?;
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    super::super::text_window::build(
        &mut commands,
        &skin,
        1,
        &[
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            "Ada\nBob".into(),
            String::new(),
            String::new(),
        ],
        "",
    );
    queue.apply(app.world_mut());
    let world = app.world_mut();
    assert!(
        world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.0 == "Enter one name per line.")
    );
    let editor = world
        .query::<(&super::super::text_window::Field, &EditableText)>()
        .iter(world)
        .find(|(field, _)| field.0 == 4)
        .ok_or("list editor")?
        .1;
    assert_eq!(editor.value().to_string(), "Ada\nBob");
    assert!(editor.allow_newlines);
    Ok(())
}

#[test]
fn house_list_window_sends_existing_list_intent() -> TestResult {
    use crate::{combat_input::PlayCommand, play::play_channel};
    use oteryn_protocol_oteryn::house as wire;
    let (link, mut receiver, _) = play_channel();
    link.bevy_house_list_fixture();
    let expected = link.house_panel();
    let mut fields = expected.fields.clone();
    fields[4] = "02020202-0202-7202-8202-020202020202\n03030303-0303-7303-8303-030303030303".into();
    fields[6] = "CONFIRM".into();
    super::super::text_window::submit(&link, &expected.model, &expected.fields, fields, 3);
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("list intent".into());
    };
    assert_eq!((command.capability, command.kind), (49, 56));
    let intent = wire::decode_house_intent(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(intent.house, "oteryn:house.test");
    assert!(
        matches!(intent.action, wire::HouseAction::Acl { subowners: false, revision: 7, members } if members.len() == 2)
    );
    // A response invalidates the old descriptor; it cannot submit a second mutation.
    super::super::text_window::submit(
        &link,
        &expected.model,
        &expected.fields,
        expected.fields.clone(),
        3,
    );
    assert!(receiver.try_recv().is_err());
    Ok(())
}

#[test]
fn house_list_window_shortcut_save_and_session_cleanup() -> TestResult {
    use crate::{
        combat_input::PlayCommand,
        play::{PlayView, play_channel},
    };
    use bevy::input::keyboard::{Key as LogicalKey, KeyboardInput};
    let (link, mut receiver, _) = play_channel();
    link.use_shared_frontend();
    link.bevy_house_list_fixture();
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .add_message::<KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::super::modal::install(&mut app);
    super::super::text_window::install(&mut app);
    app.update();
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::ShiftLeft);
    }
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyH,
        logical_key: LogicalKey::Character("H".into()),
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    let save = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == "Save guests").then_some(parent.parent()))
        .ok_or("save button")?;
    for (field, mut text) in app
        .world_mut()
        .query::<(&super::super::text_window::Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 4 {
            text.editor_mut()
                .set_text("02020202-0202-7202-8202-020202020202");
        }
        if field.0 == 6 {
            text.editor_mut().set_text("CONFIRM");
        }
    }
    app.world_mut().trigger(Activate { entity: save });
    app.update();
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("house intent".into());
    };
    assert_eq!((command.capability, command.kind), (49, 56));
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&super::super::text_window::Field>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(receiver.try_recv().is_err());
    Ok(())
}
