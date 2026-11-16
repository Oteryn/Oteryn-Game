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
                default_button: None,
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
    super::super::houses::build(
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
        &oteryn_client_ui::house::Panel::default(),
        "",
    );
    queue.apply(app.world_mut());
    let world = app.world_mut();
    assert!(
        world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.0 == "Enter one member UUID per line.")
    );
    for caption in [
        "Bid",
        "Owned / incoming",
        "Review transfer",
        "Review accept transfer",
        "Review reject transfer",
        "Confirm reviewed action",
        "Cancel review",
    ] {
        assert!(
            world
                .query::<&Text>()
                .iter(world)
                .any(|text| text.0 == caption)
        );
    }
    assert!(
        world
            .query::<(&super::super::houses::Field, &EditableText)>()
            .iter(world)
            .any(|(field, _)| field.0 == 3)
    );
    let editor = world
        .query::<(&super::super::houses::Field, &EditableText)>()
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
    super::super::houses::submit(&link, &expected.model, &expected.fields, fields, 3);
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
    super::super::houses::submit(
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
    house_window_action("Save guests", 4, "02020202-0202-7202-8202-020202020202")
}
#[test]
fn house_browser_shortcut_bid_and_session_cleanup() -> TestResult {
    house_window_action("Bid", 3, "1250")
}
#[test]
fn house_browser_opens_owned_and_incoming_catalogue() -> TestResult {
    house_window_action("Owned / incoming", 3, "")
}
#[test]
fn house_browser_reviews_and_confirms_transfer() -> TestResult {
    house_window_action("Review transfer", 3, "200")
}
fn house_window_action(caption: &str, field_index: usize, value: &str) -> TestResult {
    use crate::{
        combat_input::PlayCommand,
        play::{PlayView, play_channel},
    };
    use bevy::input::keyboard::{Key as LogicalKey, KeyboardInput};
    let (link, mut receiver, _) = play_channel();
    link.use_shared_frontend();
    link.bevy_house_list_fixture();
    if caption == "Review transfer" {
        use oteryn_client_runtime::{
            house::house_sale as wire,
            raw_feature::{CommandStatus, GatedCommandResult},
        };
        link.house_ownership_open();
        let PlayCommand::Feature(query) = receiver.try_recv()? else {
            return Err("query".into());
        };
        let reply = wire::HouseSaleReply {
            now_unix_ms: 1_800_000_000_000,
            has_more: false,
            rows: vec![wire::HouseSaleRow {
                house: "oteryn:content.house.harbour_home".into(),
                name: "Harbour Home".into(),
                revision: 7,
                owner: "Elara Vale".into(),
                recipient: String::new(),
                transfer_state: 0,
                due_at_unix_ms: 0,
                price: 0,
                rent: 50,
                allowed: 1 << 3,
                move_out_state: 0,
            }],
        };
        link.bevy_house_ownership_result(
            &query,
            &GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: wire::encode_reply(&reply).map_err(|e| format!("{e:?}"))?,
            },
        );
    }

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
    super::super::houses::install(&mut app);
    app.update();
    let press_h = |app: &mut App| {
        app.world_mut().write_message(KeyboardInput {
            key_code: KeyCode::KeyH,
            logical_key: LogicalKey::Character("H".into()),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
        app.update();
    };
    {
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.press(KeyCode::ControlLeft);
        keys.press(KeyCode::ShiftLeft);
        keys.press(KeyCode::AltLeft);
    }
    // Ctrl+Alt+Shift+H belongs to the cast chat binding; the house window stays closed.
    press_h(&mut app);
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Enter one member UUID per line."),
        "Ctrl+Alt+Shift+H opened the house window"
    );
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release(KeyCode::AltLeft);
    press_h(&mut app);
    for expected in [
        "Edit House List — oteryn:house.test",
        "Enter one member UUID per line.",
    ] {
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0 == expected),
            "missing house list descriptor: {expected}"
        );
    }
    let save = app
        .world_mut()
        .query::<(&Text, &ChildOf)>()
        .iter(app.world())
        .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
        .ok_or("save button")?;
    for (field, mut text) in app
        .world_mut()
        .query::<(&super::super::houses::Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == field_index {
            text.editor_mut().set_text(value);
        }
        if caption == "Review transfer" {
            if field.0 == 4 {
                text.editor_mut().set_text("Mira Vale");
            }
            if field.0 == 5 {
                text.editor_mut().set_text("2027-01-20 12:00:00 UTC");
            }
        }
        if field.0 == 6 {
            text.editor_mut().set_text("CONFIRM");
        }
    }
    app.world_mut().trigger(Activate { entity: save });
    app.update();
    if caption == "Review transfer" {
        assert!(receiver.try_recv().is_err(), "review sends no mutation");
        app.update();
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains("Offer Harbour Home to Mira Vale")
                    && text.0.contains("2027-01-20 12:00:00 UTC")
                    && text.0.contains("for 200 gold"))
        );
        let confirm = app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| {
                (text.0 == "Confirm reviewed action").then_some(parent.parent())
            })
            .ok_or("confirm button")?;
        app.world_mut().trigger(Activate { entity: confirm });
        app.update();
    }
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("house intent".into());
    };
    let expected_kind = if matches!(caption, "Owned / incoming" | "Review transfer") {
        oteryn_client_runtime::house::house_sale::COMMAND
    } else {
        56
    };
    assert_eq!((command.capability, command.kind), (49, expected_kind));
    if caption == "Review transfer" {
        let intent = oteryn_client_runtime::house::house_sale::decode_request(&command.payload)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            (intent.action, intent.price, intent.expected_revision),
            (3, 200, 7)
        );
        assert_eq!(intent.recipient, "Mira Vale");
        assert_eq!(intent.house, "oteryn:content.house.harbour_home");
    }
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&super::super::houses::Field>()
            .iter(app.world())
            .count(),
        0
    );
    assert!(receiver.try_recv().is_err());
    Ok(())
}

#[test]
fn house_list_window_uses_selected_house_name() -> TestResult {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    app.update();
    let skin = app.world_mut().remove_resource::<Skins>().ok_or("skins")?;
    let mut model = oteryn_client_ui::house::Panel::default();
    model.page.rows = vec![
        oteryn_client_runtime::house::Row {
            name: "First house".into(),
            ..default()
        },
        oteryn_client_runtime::house::Row {
            name: "Harbour Place 2".into(),
            house: "oteryn:house.harbour-2".into(),
            ..default()
        },
    ];
    model.index = 1;
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    super::super::houses::build(&mut commands, &skin, 1, &Default::default(), &model, "");
    queue.apply(app.world_mut());
    let world = app.world_mut();
    assert!(
        world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.0 == "Edit House List — Harbour Place 2")
    );
    assert!(
        !world
            .query::<&Text>()
            .iter(world)
            .any(|text| text.0 == "Enter one name per line.")
    );
    Ok(())
}

#[test]
fn house_browser_bid_sends_existing_auction_intent() -> TestResult {
    use crate::{combat_input::PlayCommand, play::play_channel};
    use oteryn_protocol_oteryn::house as wire;
    let (link, mut receiver, _) = play_channel();
    link.bevy_house_list_fixture();
    let expected = link.house_panel();
    let mut fields = expected.fields.clone();
    fields[3] = "1250".into();
    super::super::houses::submit(&link, &expected.model, &expected.fields, fields.clone(), 2);
    assert!(receiver.try_recv().is_err());
    let expected = link.house_panel();
    fields[6] = "CONFIRM".into();
    super::super::houses::submit(&link, &expected.model, &expected.fields, fields, 2);
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("auction intent".into());
    };
    let intent = wire::decode_house_intent(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(intent.house, "oteryn:house.test");
    assert!(matches!(intent.action, wire::HouseAction::Bid(1250)));
    Ok(())
}
