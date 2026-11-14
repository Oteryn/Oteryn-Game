use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Receiver = tokio::sync::mpsc::UnboundedReceiver<PlayCommand>;
fn fixture(window: usize) -> Result<(App, Receiver, Outbox), Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_depot_fixture();
    outbox.bevy_depot_window(window);
    let mut owner = HostOwner::new(None)?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    super::super::inventory::amount::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn control(app: &mut App, kind: Kind) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("control missing".into())
}
fn contains(app: &mut App, value: &str) -> bool {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .any(|text| text.0.contains(value))
}
fn link(app: &App) -> Result<&crate::play::PlayLink, Box<dyn std::error::Error>> {
    Ok(&app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link)
}
#[test]
fn depot_window_lists_owner_contents_and_dispatches_move() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(0)?;
    assert!(contains(&mut app, "Retrieve item 42 × 20"));
    let expected = link(&app)?.depot_panel();
    let retrieve = control(&mut app, Kind::Retrieve(0))?;
    app.world_mut().trigger(Activate { entity: retrieve });
    app.world_mut().trigger(Activate { entity: retrieve });
    let PlayCommand::Depot(command) = commands.try_recv()? else {
        return Err("command".into());
    };
    assert_eq!(command.action, depot::DepotAction::Withdraw(91));
    assert!(commands.try_recv().is_err());
    assert_eq!(link(&app)?.depot_panel().reply, expected.reply);
    let mut reply = expected.reply.ok_or("reply")?;
    reply.outcome = depot::DepotOutcome::Moved;
    reply.entries.clear();
    reply.total = 0;
    outbox.bevy_depot_reply(Some(reply));
    app.update();
    assert!(!contains(&mut app, "Retrieve item 42 × 20"));
    let deposit = control(&mut app, Kind::Deposit(std::num::NonZeroU64::MIN))?;
    app.world_mut().trigger(Activate { entity: deposit });
    let PlayCommand::Depot(command) = commands.try_recv()? else {
        return Err("deposit command".into());
    };
    assert_eq!(
        command.action,
        depot::DepotAction::Deposit(std::num::NonZeroU64::MIN)
    );
    Ok(())
}
#[test]
fn depot_search_shows_server_results_only() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(2)?;
    assert!(!contains(&mut app, "depot 5"));
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<Field>>()
        .single(app.world())?;
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("42"));
    let search = control(&mut app, Kind::Search)?;
    app.world_mut().trigger(Activate { entity: search });
    app.world_mut().trigger(Activate { entity: search });
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("command".into());
    };
    assert_eq!(
        oteryn_client_runtime::cui3::depot_search::decode_request(&command.payload)
            .map_err(|e| format!("{e:?}"))?,
        oteryn_client_runtime::cui3::depot_search::Request::Search { client_id: 42 }
    );
    assert!(commands.try_recv().is_err());
    app.update();
    assert!(!contains(&mut app, "depot 5"));
    outbox.bevy_depot_search_reply()?;
    app.update();
    assert!(contains(&mut app, "Item 42: depot 5, stash 3, inbox 0"));
    Ok(())
}
#[test]
fn depot_actions_rejected_when_session_replaced() -> TestResult {
    let (mut app, mut commands, _) = fixture(0)?;
    let entity = control(&mut app, Kind::Retrieve(0))?;
    let stale = app.world().get::<Control>(entity).ok_or("control")?.clone();
    let (replacement, mut next_commands, next_outbox) = play_channel();
    next_outbox.bevy_depot_fixture();
    next_outbox.bevy_depot_window(0);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = replacement;
    let expected = link(&app)?.depot_panel();
    app.world_mut().trigger(Activate { entity });
    assert_eq!(link(&app)?.depot_panel(), expected);
    assert!(next_commands.try_recv().is_err());
    app.update();
    assert!(!actions::perform(
        app.world().non_send::<HostOwner>(),
        app.world().resource::<State>(),
        &stale,
        ""
    ));
    let current = control(&mut app, Kind::Retrieve(0))?;
    let modal = link(&app)?.bevy_server_modal_fixture();
    app.world_mut().trigger(Activate { entity: current });
    assert_eq!(link(&app)?.depot_panel(), expected);
    assert!(next_commands.try_recv().is_err());
    drop(modal);
    app.world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(999), Node::default()));
    app.world_mut().trigger(Activate { entity: current });
    assert_eq!(link(&app)?.depot_panel(), expected);
    assert!(next_commands.try_recv().is_err());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn stash_browse_pages_and_searches_server_data() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(1)?;
    assert!(contains(&mut app, "Test supply"));
    let next = control(&mut app, Kind::Next)?;
    app.world_mut().trigger(Activate { entity: next });
    let PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("browse command".into());
    };
    assert_eq!(command.intent.page, 1);
    assert_eq!(link(&app)?.stash_panel().page, 0);
    outbox.bevy_stash_page(1);
    app.update();
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<Field>>()
        .single(app.world())?;
    app.world_mut()
        .entity_mut(field)
        .insert(EditableText::new("supply"));
    let filter = control(&mut app, Kind::Filter)?;
    app.world_mut().trigger(Activate { entity: filter });
    let PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("filter command".into());
    };
    let (intent, filter) =
        stash::stash::decode_stash_query(&command.payload).map_err(|error| format!("{error:?}"))?;
    assert_eq!(intent.page, 0);
    assert_eq!(filter.ok_or("filter")?.search, "supply");
    assert_eq!(link(&app)?.stash_panel().page, 1);
    Ok(())
}

#[test]
fn depot_shortcuts_bind_live_session_and_reject_closed_or_modal_opening() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    for (window, code, shift) in [(0, 15, true), (1, 22, false), (2, 4, true)] {
        let event = NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers: if shift {
                Modifiers::one(Modifier::Control).with(Modifier::Shift)
            } else {
                Modifiers::one(Modifier::Control)
            },
            repeat: false,
        };
        let (mut app, mut commands, _) = fixture(3)?;
        let expected = (
            link(&app)?.depot_panel(),
            link(&app)?.stash_panel(),
            link(&app)?.depot_search_panel(),
        );
        let modal = link(&app)?.bevy_server_modal_fixture();
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .route_input(std::slice::from_ref(&event), false);
        assert_eq!(
            (
                link(&app)?.depot_panel(),
                link(&app)?.stash_panel(),
                link(&app)?.depot_search_panel()
            ),
            expected
        );
        assert!(commands.try_recv().is_err());
        drop(modal);
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .route_input(std::slice::from_ref(&event), false);
        app.update();
        assert!(app.world().resource::<State>().windows[window].is_some());
        let (mut app, mut commands, _) = fixture(3)?;
        let expected = (
            link(&app)?.depot_panel(),
            link(&app)?.stash_panel(),
            link(&app)?.depot_search_panel(),
        );
        commands.close();
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .route_input(&[event], false);
        assert_eq!(
            (
                link(&app)?.depot_panel(),
                link(&app)?.stash_panel(),
                link(&app)?.depot_search_panel()
            ),
            expected
        );
    }
    Ok(())
}

fn named(app: &mut App, name: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, value)| (value.as_str() == name).then_some(entity))
        .ok_or_else(|| format!("missing {name}").into())
}
fn withdraw_prompt(app: &mut App) -> TestResult {
    let withdraw = named(app, "stash.withdraw.0")?;
    app.world_mut().trigger(Activate { entity: withdraw });
    app.update();
    let input = named(app, "amount.caller.input")?;
    app.world_mut()
        .trigger(bevy::ui_widgets::ValueChange::<String> {
            source: input,
            value: "13".into(),
            is_final: false,
        });
    Ok(())
}
#[test]
fn stash_withdraw_uses_quantity_and_rejects_stale_session() -> TestResult {
    let (mut app, mut commands, _) = fixture(1)?;
    let before = link(&app)?.stash_panel();
    withdraw_prompt(&mut app)?;
    assert!(
        commands.try_recv().is_err(),
        "opening the amount prompt must not withdraw"
    );
    let input = named(&mut app, "amount.caller.input")?;
    let confirm = named(&mut app, "amount.caller.confirm")?;
    for invalid in ["0", "81", "2.5", ""] {
        app.world_mut()
            .trigger(bevy::ui_widgets::ValueChange::<String> {
                source: input,
                value: invalid.into(),
                is_final: false,
            });
        app.world_mut().trigger(Activate { entity: confirm });
        app.update();
        assert!(commands.try_recv().is_err());
        assert_eq!(link(&app)?.stash_panel(), before);
    }
    app.world_mut()
        .trigger(bevy::ui_widgets::ValueChange::<String> {
            source: input,
            value: "13".into(),
            is_final: false,
        });
    app.world_mut().trigger(Activate { entity: confirm });
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    let PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("withdraw command".into());
    };
    assert_eq!(
        command.intent.action,
        stash::stash::StashAction::Withdraw {
            row: [3; 16],
            amount: 13
        }
    );
    assert_eq!(
        link(&app)?.stash_panel().rows,
        before.rows,
        "only a server reply changes stock"
    );
    assert!(commands.try_recv().is_err());

    let (mut app, mut commands, _) = fixture(1)?;
    withdraw_prompt(&mut app)?;
    let confirm = named(&mut app, "amount.caller.confirm")?;
    let (replacement, mut next_commands, next_outbox) = play_channel();
    next_outbox.bevy_depot_fixture();
    next_outbox.bevy_depot_window(1);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .play
        .as_mut()
        .ok_or("play")?
        .link = replacement;
    let expected = link(&app)?.stash_panel();
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert_eq!(link(&app)?.stash_panel(), expected);
    assert!(commands.try_recv().is_err());
    assert!(next_commands.try_recv().is_err());

    let (mut app, mut commands, _) = fixture(1)?;
    withdraw_prompt(&mut app)?;
    let confirm = named(&mut app, "amount.caller.confirm")?;
    let expected = link(&app)?.stash_panel();
    let modal = link(&app)?.bevy_server_modal_fixture();
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert_eq!(link(&app)?.stash_panel(), expected);
    assert!(commands.try_recv().is_err());
    drop(modal);
    commands.close();
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert_eq!(link(&app)?.stash_panel(), expected);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn stash_quantity_waits_for_editor_and_cancel_never_dispatches() -> TestResult {
    let (mut app, mut commands, _) = fixture(1)?;
    withdraw_prompt(&mut app)?;
    let input = named(&mut app, "amount.caller.input")?;
    let confirm = named(&mut app, "amount.caller.confirm")?;
    app.world_mut()
        .get_mut::<EditableText>(input)
        .ok_or("editor")?
        .queue_edit(bevy::text::TextEdit::Insert("7".into()));
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "pending edits cannot submit the old value"
    );
    // Model the upstream editor completing its queued edit before the deferred confirmation.
    app.world_mut()
        .entity_mut(input)
        .insert(EditableText::new("7"));
    app.update();
    let PlayCommand::Stash(command) = commands.try_recv()? else {
        return Err("withdraw".into());
    };
    assert_eq!(
        command.intent.action,
        stash::stash::StashAction::Withdraw {
            row: [3; 16],
            amount: 7
        }
    );
    assert!(commands.try_recv().is_err());

    let (mut app, mut commands, _) = fixture(1)?;
    withdraw_prompt(&mut app)?;
    let input = named(&mut app, "amount.caller.input")?;
    for (button, expected) in [
        ("amount.caller.all", "80"),
        ("amount.caller.decrease", "79"),
        ("amount.caller.increase", "80"),
    ] {
        let button = named(&mut app, button)?;
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(
            app.world()
                .get::<EditableText>(input)
                .ok_or("editor")?
                .value()
                .to_string(),
            expected
        );
        assert!(commands.try_recv().is_err());
    }
    let cancel = named(&mut app, "amount.caller.cancel")?;
    app.world_mut().trigger(Activate { entity: cancel });
    app.update();
    assert!(named(&mut app, "amount.caller.input").is_err());
    assert!(commands.try_recv().is_err());
    Ok(())
}
