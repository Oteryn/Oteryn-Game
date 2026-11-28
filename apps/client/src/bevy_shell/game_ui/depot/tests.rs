use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayLink, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_ui::depot::{DepotAction, DepotOutcome, DepotReply, Intent};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn reply() -> DepotReply {
    DepotReply {
        outcome: DepotOutcome::Opened,
        town: Some(2),
        box_number: 17,
        page: 0,
        total: 33,
        entries: vec![oteryn_client_runtime::depot::DepotEntry {
            ordinal: 81,
            definition: 9,
            count: 3,
        }],
    }
}
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn depot_app(link: PlayLink) -> Result<App, Box<dyn std::error::Error>> {
    link.use_shared_frontend();
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = Phase::Playing;
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

fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    outbox.bevy_depot_initialize(true, false);
    let mut app = depot_app(link)?;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    app.world_mut().non_send_mut::<HostOwner>().route_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(15)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE
                .with(Modifier::Control)
                .with(Modifier::Shift),
            repeat: false,
        }],
        false,
    );
    app.update();
    Ok((app, commands, outbox))
}
fn control(app: &mut App, kind: Kind) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, c)| (c.kind == kind).then_some(entity))
        .ok_or_else(|| "missing control".into())
}
fn click(app: &mut App, kind: Kind) -> TestResult {
    let entity = control(app, kind)?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
#[test]
fn depot_shortcut_projects_seventeen_boxes_and_pages_confirmed_contents() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let PlayCommand::Depot(oteryn_session::DepotRequest {
        intent: initial,
        slot: None,
        cross_town: false,
    }) = commands.try_recv()?
    else {
        return Err("missing initial browse".into());
    };
    assert_eq!(
        (
            initial.box_number,
            initial.page,
            initial.town,
            initial.action
        ),
        (1, 0, None, DepotAction::Browse)
    );
    assert_eq!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .filter(|c| matches!(c.kind, Kind::Request(Intent::Browse { page: 0, .. })))
            .count(),
        17
    );
    // Pending requests cannot be duplicated even by a synthetic Activate.
    click(
        &mut app,
        Kind::Request(Intent::Browse {
            box_number: 17,
            page: 0,
        }),
    )?;
    assert!(commands.try_recv().is_err());
    outbox.bevy_depot_result(Some(reply()));
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("Item 9 ×3"))
    );
    click(
        &mut app,
        Kind::Request(Intent::Browse {
            box_number: 17,
            page: 1,
        }),
    )?;
    let PlayCommand::Depot(oteryn_session::DepotRequest {
        intent: next,
        slot: None,
        cross_town: false,
    }) = commands.try_recv()?
    else {
        return Err("missing next page".into());
    };
    assert_eq!((next.box_number, next.page, next.town), (17, 1, Some(2)));
    let mut last = reply();
    last.page = 1;
    last.entries.clear();
    outbox.bevy_depot_result(Some(last));
    app.update();
    let next = control(
        &mut app,
        Kind::Request(Intent::Browse {
            box_number: 17,
            page: 2,
        }),
    )?;
    assert!(
        app.world()
            .get::<bevy::ui::InteractionDisabled>(next)
            .is_some()
    );
    click(&mut app, Kind::Close)?;
    assert_eq!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}
#[test]
fn depot_withdraw_uses_displayed_ordinal_and_rejects_changed_snapshot() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    while commands.try_recv().is_ok() {}
    outbox.bevy_depot_result(Some(reply()));
    app.update();
    let old = control(&mut app, Kind::Request(Intent::Withdraw(0)))?;
    let mut changed = reply();
    changed.entries[0].ordinal = 82;
    outbox.bevy_depot_result(Some(changed));
    app.world_mut().trigger(Activate { entity: old });
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Request(Intent::Withdraw(0)))?;
    let PlayCommand::Depot(oteryn_session::DepotRequest {
        intent: withdraw,
        slot: None,
        cross_town: false,
    }) = commands.try_recv()?
    else {
        return Err("missing withdrawal".into());
    };
    assert_eq!(withdraw.action, DepotAction::Withdraw(82));
    outbox.bevy_depot_result(None);
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("refused"))
    );
    assert!(control(&mut app, Kind::Request(Intent::Withdraw(0))).is_err());
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}

#[test]
fn depot_withdraw_rejects_replacement_session_with_the_same_snapshot() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let initial = commands.try_recv()?;
    assert!(matches!(initial, PlayCommand::Depot(..)));
    outbox.bevy_depot_result(Some(reply()));
    app.update();
    let kind = Kind::Request(Intent::Withdraw(0));
    let stale = control(&mut app, kind)?;

    let (replacement, mut replacement_commands, replacement_outbox) = play_channel();
    replacement.use_shared_frontend();
    replacement_outbox.bevy_depot_initialize(true, false);
    replacement.open_depot();
    let initial = replacement_commands.try_recv()?;
    assert!(matches!(initial, PlayCommand::Depot(..)));
    replacement_outbox.bevy_depot_result(Some(reply()));
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let active = owner.play.as_mut().ok_or("missing depot session")?;
        assert_eq!(active.link.depot_panel(), replacement.depot_panel());
        active.link = replacement;
        owner.enter_character(Some([7; 16]));
    }

    app.world_mut().trigger(Activate { entity: stale });
    assert!(replacement_commands.try_recv().is_err());
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, kind)?;
    let PlayCommand::Depot(oteryn_session::DepotRequest {
        intent: withdraw,
        slot: None,
        cross_town: false,
    }) = replacement_commands.try_recv()?
    else {
        return Err("missing withdrawal from replacement session".into());
    };
    assert_eq!(withdraw.action, DepotAction::Withdraw(81));
    Ok(())
}

/// Tibia Global's Manual Sort Mode (Shift+S, capability 510) in the depot: the deposit button
/// holds the item, and the clicked box row sends the deposit with that row's box slot.
#[test]
fn manual_sort_mode_deposits_at_the_clicked_depot_row_slot() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    while commands.try_recv().is_ok() {}
    crate::play::PlayLink::bevy_manual_sort_fixture(&outbox, true);
    let handle = std::num::NonZeroU64::MIN;
    let mut items = oteryn_client_runtime::depot::Items::default();
    items.inventory.main_backpack = Some(oteryn_client_runtime::depot::ItemEntry {
        appearance_id: 0,
        handle,
        item_definition_ref: std::num::NonZeroU32::MIN,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    });
    outbox.bevy_depot_items(items);
    let mut page = reply();
    page.page = 1;
    outbox.bevy_depot_result(Some(page));
    app.update();
    // Without the mode a deposit goes out at once, at the first slot.
    click(&mut app, Kind::Request(Intent::Deposit(handle)))?;
    let PlayCommand::Depot(oteryn_session::DepotRequest {
        intent: plain,
        slot: None,
        cross_town: false,
    }) = commands.try_recv()?
    else {
        return Err("missing plain deposit".into());
    };
    assert_eq!(plain.action, DepotAction::Deposit(handle));
    let mut page = reply();
    page.page = 1;
    outbox.bevy_depot_result(Some(page));
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(link.toggle_container_manual_sort());
    app.update();
    click(&mut app, Kind::Request(Intent::Deposit(handle)))?;
    assert!(commands.try_recv().is_err());
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains("Item 9 ×3 — place here"))
    );
    click(&mut app, Kind::Request(Intent::Withdraw(0)))?;
    let PlayCommand::Depot(oteryn_session::DepotRequest {
        intent: slotted,
        slot: Some(slot),
        cross_town: false,
    }) = commands.try_recv()?
    else {
        return Err("missing slotted deposit".into());
    };
    assert_eq!(
        (slotted.action, slotted.page, slot),
        (DepotAction::Deposit(handle), 1, 32)
    );
    Ok(())
}

/// Turning Manual Sort Mode off (Shift+S, the same toggle as the sort menu's checkbox) drops a
/// held deposit: the rows offer withdrawal again and no row still says "place here".
#[test]
fn turning_manual_sort_off_drops_the_held_depot_deposit() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    while commands.try_recv().is_ok() {}
    crate::play::PlayLink::bevy_manual_sort_fixture(&outbox, true);
    let handle = std::num::NonZeroU64::MIN;
    let mut items = oteryn_client_runtime::depot::Items::default();
    items.inventory.main_backpack = Some(oteryn_client_runtime::depot::ItemEntry {
        appearance_id: 0,
        handle,
        item_definition_ref: std::num::NonZeroU32::MIN,
        count: std::num::NonZeroU32::MIN,
        sub_type: 0,
    });
    outbox.bevy_depot_items(items);
    outbox.bevy_depot_result(Some(reply()));
    app.update();
    let toggle = |app: &App| -> Result<bool, Box<dyn std::error::Error>> {
        Ok(app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .utility_action(crate::play::UtilityAction::ToggleManualSortMode))
    };
    let rows = |app: &mut App, caption: &str| {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0.contains(caption))
    };
    assert!(toggle(&app)?);
    app.update();
    click(&mut app, Kind::Request(Intent::Deposit(handle)))?;
    assert!(commands.try_recv().is_err());
    app.update();
    assert!(rows(&mut app, "Item 9 ×3 — place here"));
    assert!(toggle(&app)?);
    app.update();
    assert!(
        !rows(&mut app, "place here"),
        "no held deposit after the mode is off"
    );
    assert!(rows(&mut app, "Item 9 ×3 — withdraw stack"));
    assert!(toggle(&app)?);
    app.update();
    click(&mut app, Kind::Request(Intent::Deposit(handle)))?;
    let mut far = reply();
    far.page = 16;
    far.total = 513;
    outbox.bevy_depot_result(Some(far));
    app.update();
    assert!(!rows(&mut app, "place here"));
    outbox.bevy_depot_items(Default::default());
    app.update();
    assert!(rows(&mut app, "Item 9 ×3 — withdraw stack"));
    click(&mut app, Kind::Request(Intent::Withdraw(0)))?;
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Depot(oteryn_session::DepotRequest { intent, slot: None, cross_town: false }) if matches!(intent.action, DepotAction::Withdraw(_)))
    );
    Ok(())
}

#[test]
fn depot_town_picker_projects_and_enqueues_named_towns() -> TestResult {
    let (mut app, mut commands, outbox) = fixture()?;
    let carlin = Kind::Request(Intent::SelectTown(Some(6)));
    assert!(control(&mut app, carlin).is_err());
    commands.try_recv()?;
    outbox.bevy_depot_initialize(true, true);
    outbox.bevy_depot_result(Some(reply()));
    app.update();
    control(&mut app, Kind::Request(Intent::SelectTown(None)))?;
    click(&mut app, carlin)?;
    let PlayCommand::Depot(request) = commands.try_recv()? else {
        return Err("missing town browse".into());
    };
    assert!(request.cross_town);
    assert_eq!(request.intent.town, Some(6));
    assert_eq!(request.intent.action, DepotAction::Browse);
    Ok(())
}

#[test]
fn cross_town_depot_round_trip_through_bevy_control()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use crate::bevy_shell::controls::ContainerTool;
    use crate::play::feature_0037_depot::protocol::test_wire;
    use std::time::Duration;

    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), test_wire::Error> {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            runtime.block_on(async {
                tokio::time::timeout(Duration::from_secs(5), async {
                    let (session, mut peer) = test_wire::admit(true).await?;
                    let selected = session.selected_capabilities().to_vec();
                    let (link, commands, outbox) = play_channel();
                    let task = tokio::spawn(crate::play::run_session(session, commands, outbox));
                    let mut app = depot_app(link).map_err(|error| error.to_string())?;
                    assert!(
                        depot_panel_until(&app, |panel| panel.selected)
                            .await?
                            .cross_town_selected
                    );
                    assert!(
                        app.world_mut()
                            .non_send_mut::<HostOwner>()
                            .request_container_tool(ContainerTool::Depot, None, None)
                    );
                    let local =
                        test_wire::reply(&mut peer, &selected, 1, &[8, 1, 16, 1, 24, 0, 40, 0], 8)
                            .await?;
                    let settled = |panel: &DepotPanel| !panel.pending;
                    assert_eq!(depot_panel_until(&app, settled).await?.reply, Some(local));
                    app.update();
                    click(&mut app, Kind::Request(Intent::SelectTown(Some(6))))
                        .map_err(|error| error.to_string())?;
                    let remote = test_wire::reply(
                        &mut peer,
                        &selected,
                        2,
                        &[8, 1, 16, 1, 24, 0, 40, 0, 32, 6, 56, 1],
                        6,
                    )
                    .await?;
                    assert_eq!(depot_panel_until(&app, settled).await?.reply, Some(remote));
                    drop(app);
                    task.await?;
                    Ok::<(), test_wire::Error>(())
                })
                .await?
            })
        })?
        .join()
        .map_err(|_| "native Depot round trip thread panicked")?
}

async fn depot_panel_until(
    app: &App,
    ready: impl Fn(&DepotPanel) -> bool,
) -> Result<DepotPanel, crate::play::feature_0037_depot::protocol::test_wire::Error> {
    loop {
        let owner = app.world().non_send::<HostOwner>();
        let panel = owner
            .play
            .as_ref()
            .ok_or("missing play owner")?
            .link
            .depot_panel();
        if ready(&panel) {
            return Ok(panel);
        }
        tokio::task::yield_now().await;
    }
}
