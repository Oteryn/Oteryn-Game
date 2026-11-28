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
    outbox.bevy_stash_initialize(true);
    let mut app = stash_app(link)?;
    open_stash(&mut app)?;
    outbox.bevy_stash_result(&reply());
    app.update();
    Ok((app, commands, outbox))
}
fn stash_app(link: crate::play::PlayLink) -> Result<App, Box<dyn std::error::Error>> {
    link.use_shared_frontend();
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
    Ok(app)
}
fn open_stash(app: &mut App) -> TestResult {
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(22)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    Ok(())
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

#[test]
fn stash_stow_controls_are_disabled_while_request_pending() -> TestResult {
    let (mut app, _, _) = fixture()?;
    let mut panel = app
        .world()
        .resource::<State>()
        .panel
        .clone()
        .ok_or("panel")?;
    panel.carried.push(oteryn_client_runtime::depot::ItemEntry {
        appearance_id: 0,
        handle: std::num::NonZeroU64::MIN,
        item_definition_ref: std::num::NonZeroU32::MIN,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    });
    panel.pending = true;

    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, app.world());
    widgets::build(
        &mut commands,
        app.world().resource::<Skins>(),
        &State::default(),
        &panel,
        &mut InputFocus::default(),
    );
    queue.apply(app.world_mut());
    assert!(
        app.world_mut()
            .query::<(&Control, &bevy::ui::InteractionDisabled)>()
            .iter(app.world())
            .any(|(c, _)| c.kind == Kind::Stow(0))
    );
    Ok(())
}

#[test]
fn stash_left_right_adjust_and_confirm_through_keyboard() -> TestResult {
    let (mut app, mut commands, _) = fixture()?;
    while commands.try_recv().is_ok() {}
    click(&mut app, Kind::Row(0))?;
    for (key, expected) in [
        (KeyCode::ArrowLeft, 1),
        (KeyCode::ArrowRight, 2),
        (KeyCode::ArrowRight, 3),
        (KeyCode::ArrowLeft, 2),
        (KeyCode::PageUp, 32),
        (KeyCode::ArrowRight, 32),
    ] {
        press(&mut app, key);
        let text = app
            .world_mut()
            .query_filtered::<&EditableText, With<Quantity>>()
            .single(app.world())?;
        assert_eq!(text.value().to_string(), expected.to_string());
    }
    press(&mut app, KeyCode::Enter);
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
fn manage_containers_opens_quick_loot_from_the_stash() -> TestResult {
    let (mut app, _commands, outbox) = fixture()?;
    click(&mut app, Kind::ManageContainers)?;
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .stash_panel()
            .open
    );
    outbox.bevy_quick_loot_selected(true);
    click(&mut app, Kind::ManageContainers)?;
    let play = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?;
    assert!(!play.link.stash_panel().open);
    assert!(play.link.quick_loot_panel().open);
    Ok(())
}

#[test]
fn reverse_button_requests_opposite_name_and_quantity_orders() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    commands.try_recv()?;
    for stock in [false, true] {
        if stock {
            click(&mut app, Kind::Sort)?;
            commands.try_recv()?;
            outbox.bevy_stash_result(&reply());
            app.update();
        }
        click(&mut app, Kind::Reverse)?;
        let PlayCommand::Stash(command) = commands.try_recv()? else {
            return Err("browse".into());
        };
        assert_eq!(command.intent.page, 0);
        let panel = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .stash_panel();
        assert!(panel.reverse);
        assert_eq!(panel.sort == oteryn_client_ui::stash::Sort::Stock, stock);
        outbox.bevy_stash_result(&reply());
        app.update();
        click(&mut app, Kind::Reverse)?;
        commands.try_recv()?;
        outbox.bevy_stash_result(&reply());
        app.update();
    }
    Ok(())
}

#[path = "tests/wire.rs"]
pub(in crate::bevy_shell::game_ui) mod wire;

#[test]
fn gap_0591_stash_row_actions_and_stow_intents() -> TestResult {
    let (mut peer, link) = wire::Peer::new(Some(591))?;
    let mut app = stash_app(link)?;
    wire::wait(&mut app, |link| !link.stash_panel().carried.is_empty())?;
    open_stash(&mut app)?;
    peer.command(79)?;
    peer.stash(&reply().reply)?;
    wire::wait(&mut app, |link| !link.stash_panel().rows.is_empty())?;

    click_caption(&mut app, "Add to loot list")?;
    let bytes = peer.command(6345)?;
    assert_eq!(
        stash::decode_stash_row_intent(&bytes).map_err(wire::invalid)?,
        stash::StashRowIntent {
            row: [7; 16],
            action: stash::StashRowAction::LootAdd,
        }
    );
    peer.result(
        CommandStatus::Accepted,
        &stash::encode_stash_row_result(std::num::NonZeroU32::MIN),
    )?;
    peer.command(79)?;
    peer.stash(&reply().reply)?;
    wire::wait(&mut app, |link| !link.stash_panel().pending)?;

    use stash::StashAction;
    for (caption, action) in [
        (
            "Stow amount…",
            StashAction::StowQuantity {
                handle: 11.try_into()?,
                amount: 1,
            },
        ),
        (
            "Stow all of this type",
            StashAction::StowMatching(11.try_into()?),
        ),
        ("Stow contents", StashAction::StowContents(10.try_into()?)),
    ] {
        click_caption(&mut app, caption)?;
        if matches!(action, StashAction::StowQuantity { .. }) {
            press(&mut app, KeyCode::Enter);
        }
        let bytes = peer.command(79)?;
        assert_eq!(
            stash::decode_stash_query(&bytes)
                .map_err(wire::invalid)?
                .0
                .action,
            action
        );
        peer.stash(&reply().reply)?;
        wire::wait(&mut app, |link| !link.stash_panel().pending)?;
    }
    Ok(())
}

#[test]
fn gap_0601_stash_container() -> TestResult {
    use oteryn_protocol_oteryn::container_batch::{Reply, Request};
    let (mut peer, link) = wire::Peer::new(Some(601))?;
    let mut app = stash_app(link)?;
    wire::wait(&mut app, |link| !link.stash_panel().carried.is_empty())?;
    open_stash(&mut app)?;
    peer.command(79)?;
    peer.stash(&reply().reply)?;
    wire::wait(&mut app, |link| !link.stash_panel().rows.is_empty())?;

    for status in [CommandStatus::Rejected, CommandStatus::Accepted] {
        click_caption(&mut app, "Preview stash contents")?;
        assert_eq!(
            Request::decode(&peer.command(6364)?),
            Some(Request::Preview {
                source: 10.try_into()?,
                recursive: true,
            })
        );
        let quote = peer.result(CommandStatus::Accepted, &Reply::Preview(4).encode())?;
        wire::wait_text(&mut app, "Stow 4 units")?;
        click_caption(&mut app, "Stow contents (Enter)")?;
        assert_eq!(
            Request::decode(&peer.command(6364)?),
            Some(Request::Confirm {
                preview: quote.try_into()?
            })
        );
        let payload = Reply::Committed(4).encode();
        peer.result(
            status,
            if status == CommandStatus::Accepted {
                &payload
            } else {
                &[]
            },
        )?;
        peer.command(79)?;
        let mut refreshed = reply().reply;
        refreshed.rows[0].quantity = 36;
        peer.stash(&refreshed)?;
        wire::wait(&mut app, |link| {
            !link.stash_panel().pending && link.stash_panel().rows == refreshed.rows
        })?;
        let expected = if status == CommandStatus::Accepted {
            "Stowed 4 units."
        } else {
            "The preview is no longer valid."
        };
        wire::wait_text(&mut app, expected)?;
    }
    Ok(())
}

#[test]
fn stash_legacy_stow_retains_its_wire_bytes() -> TestResult {
    let (mut peer, link) = wire::Peer::new(None)?;
    let mut app = stash_app(link)?;
    wire::wait(&mut app, |link| link.stash_panel().selected)?;
    open_stash(&mut app)?;
    peer.command(79)?;
    peer.stash(&reply().reply)?;
    wire::wait(&mut app, |link| !link.stash_panel().rows.is_empty())?;
    assert!(click_caption(&mut app, "Stow amount…").is_err());
    let panel = app
        .world()
        .resource::<State>()
        .panel
        .as_ref()
        .ok_or("stash panel")?;
    let row = panel
        .carried
        .iter()
        .position(|item| item.handle.get() == 11)
        .ok_or("stack")?;
    click(&mut app, Kind::Stow(row))?;
    assert_eq!(
        peer.command(79)?,
        [8, 0, 16, 2, 24, 11, 50, 8, 10, 0, 18, 0, 26, 0, 32, 0]
    );
    Ok(())
}

fn click_caption(app: &mut App, caption: &str) -> TestResult {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control, &Children)>()
        .iter(app.world())
        .find(|(_, _, children)| {
            children.iter().any(|child| {
                app.world()
                    .get::<Text>(child)
                    .is_some_and(|text| text.0 == caption)
            })
        })
        .map(|(entity, _, _)| entity)
        .ok_or_else(|| format!("control missing: {caption}"))?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
