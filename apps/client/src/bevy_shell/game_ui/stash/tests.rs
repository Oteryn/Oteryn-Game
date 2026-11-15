use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::stash::{StashResponse, stash};
use oteryn_session::CommandStatus;
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn reply() -> StashResponse {
    StashResponse {
        status: CommandStatus::Accepted,
        reply: stash::StashReply {
            outcome: stash::StashOutcome::Opened,
            stowed_quantity: 0,
            transaction: None,
            page: 0,
            has_more: true,
            rows: [("item.apple", 32, 7), ("item.berry", 4, 8)]
                .into_iter()
                .map(|(name, quantity, id)| stash::StashRow {
                    handle: [id; 16],
                    definition_key: name.into(),
                    definition_revision: "r1".into(),
                    quantity,
                    migration_required: false,
                })
                .collect(),
        },
    }
}
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_stash_initialize(true);
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
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(22)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    outbox.bevy_stash_result(&reply());
    app.update();
    Ok((app, commands, outbox))
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("control missing")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn stash_window_lists_visible_rows() -> TestResult {
    let (mut app, _, _) = fixture()?;
    let labels: Vec<_> = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.clone())
        .collect();
    assert!(
        labels
            .iter()
            .any(|t| t.contains("item.apple") && t.contains("32"))
    );
    assert!(
        labels
            .iter()
            .any(|t| t.contains("item.berry") && t.contains('4'))
    );
    assert_eq!(app.world_mut().query::<&Row>().iter(app.world()).count(), 2);
    Ok(())
}
#[test]
fn stash_search_filters_rows() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    while commands.try_recv().is_ok() {}
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        *text = EditableText::new(match field {
            Field::Search => "apple",
            Field::Category => "other",
            Field::Trader => "Sam",
        });
    }
    click(&mut app, Kind::Filter)?;
    let PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("stash command missing".into());
    };
    let (_, filter) = stash::decode_stash_query(&command.payload).map_err(|e| format!("{e:?}"))?;
    let filter = filter.ok_or("filter missing")?;
    assert_eq!(
        (
            filter.search.as_str(),
            filter.category.as_str(),
            filter.trader.as_str()
        ),
        ("apple", "other", "Sam")
    );
    let mut response = reply();
    response.reply.rows.truncate(1);
    outbox.bevy_stash_result(&response);
    app.update();
    assert_eq!(app.world_mut().query::<&Row>().iter(app.world()).count(), 1);
    click(&mut app, Kind::Sort)?;
    let PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("sort missing".into());
    };
    assert!(
        stash::decode_stash_query(&command.payload)
            .map_err(|e| format!("{e:?}"))?
            .1
            .ok_or("filter")?
            .stock_order
    );
    Ok(())
}
#[test]
fn stash_amount_dialog_keys() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    while commands.try_recv().is_ok() {}
    click(&mut app, Kind::Row(0))?;
    for (key, expected) in [
        (KeyCode::ArrowUp, 11),
        (KeyCode::ArrowUp, 21),
        (KeyCode::PageUp, 32),
        (KeyCode::ArrowUp, 32),
        (KeyCode::ArrowDown, 22),
        (KeyCode::PageDown, 1),
        (KeyCode::ArrowDown, 1),
    ] {
        press(&mut app, key);
        assert_eq!(
            app.world()
                .resource::<State>()
                .amount
                .as_ref()
                .ok_or("amount")?
                .value,
            expected
        );
    }
    press(&mut app, KeyCode::Escape);
    assert!(app.world().resource::<State>().amount.is_none());
    assert!(commands.try_recv().is_err());
    click(&mut app, Kind::Row(0))?;
    press(&mut app, KeyCode::Enter);
    let PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("withdraw missing".into());
    };
    assert_eq!(
        command.intent.action,
        stash::StashAction::Withdraw {
            row: [7; 16],
            amount: 1
        }
    );
    Ok(())
}

fn press(app: &mut App, key_code: KeyCode) {
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code,
            logical_key: bevy::input::keyboard::Key::Unidentified(
                bevy::input::keyboard::NativeKey::Unidentified,
            ),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
}
#[test]
fn stash_stale_row_cannot_withdraw() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    while commands.try_recv().is_ok() {}
    click(&mut app, Kind::Row(0))?;
    let mut response = reply();
    response.reply.rows.reverse();
    outbox.bevy_stash_result(&response);
    // Receipt before projection must invalidate the old consent.
    press(&mut app, KeyCode::Enter);
    assert!(commands.try_recv().is_err());
    assert!(app.world().resource::<State>().amount.is_none());
    Ok(())
}
#[test]
fn stash_modal_and_session_end_fence_withdraw() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    while commands.try_recv().is_ok() {}
    click(&mut app, Kind::Row(0))?;
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(250), Node::default()))
        .id();
    press(&mut app, KeyCode::Enter);
    assert!(commands.try_recv().is_err());
    assert!(app.world().resource::<State>().amount.is_some());
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    press(&mut app, KeyCode::Enter);
    assert!(commands.try_recv().is_err());
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

#[test]
fn stash_amount_text_is_clamped_through_confirm_caller() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    while commands.try_recv().is_ok() {}
    click(&mut app, Kind::Row(0))?;
    let mut query = app
        .world_mut()
        .query_filtered::<&mut EditableText, With<Quantity>>();
    for mut text in query.iter_mut(app.world_mut()) {
        *text = EditableText::new("999");
    }
    click(&mut app, Kind::Confirm)?;
    let PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("withdraw missing".into());
    };
    assert_eq!(
        command.intent.action,
        stash::StashAction::Withdraw {
            row: [7; 16],
            amount: 32
        }
    );
    Ok(())
}
#[test]
fn stash_shortcut_requests_data_and_escape_closes() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    let PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("browse missing".into());
    };
    assert_eq!(command.intent.action, stash::StashAction::Browse);
    assert!(app.world().resource::<State>().root.is_some());
    press(&mut app, KeyCode::Escape);
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}

#[test]
fn stash_amount_keys_in_one_frame_confirm_accumulated_amount() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    while commands.try_recv().is_ok() {}
    click(&mut app, Kind::Row(0))?;
    for key_code in [KeyCode::ArrowUp, KeyCode::ArrowUp, KeyCode::Enter] {
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code,
                logical_key: bevy::input::keyboard::Key::Unidentified(
                    bevy::input::keyboard::NativeKey::Unidentified,
                ),
                state: bevy::input::ButtonState::Pressed,
                text: None,
                repeat: false,
                window: Entity::PLACEHOLDER,
            });
    }
    app.update();
    let PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("withdraw missing".into());
    };
    assert_eq!(
        command.intent.action,
        stash::StashAction::Withdraw {
            row: [7; 16],
            amount: 21
        }
    );
    Ok(())
}

#[test]
fn stash_numeric_correction_preserves_editor_configuration() -> TestResult {
    let (mut app, _commands, _) = fixture()?;
    click(&mut app, Kind::Row(0))?;
    for mut text in app
        .world_mut()
        .query_filtered::<&mut EditableText, With<Quantity>>()
        .iter_mut(app.world_mut())
    {
        text.cursor_width = 0.7;
        text.max_characters = Some(7);
        text.visible_lines = Some(3.);
    }
    press(&mut app, KeyCode::ArrowUp);
    let text = app
        .world_mut()
        .query_filtered::<&EditableText, With<Quantity>>()
        .single(app.world())?;
    assert_eq!(text.value().to_string(), "11");
    assert_eq!(text.cursor_width, 0.7);
    assert_eq!(text.max_characters, Some(7));
    assert_eq!(text.visible_lines, Some(3.));
    Ok(())
}
