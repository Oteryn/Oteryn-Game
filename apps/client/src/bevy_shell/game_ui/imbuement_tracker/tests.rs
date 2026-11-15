use super::*;
use oteryn_client_runtime::equipment::{EquipmentSlot, EquippedItem};
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
use oteryn_client_ui::inspection::{InspectedItem, Reply, Request, decode_request, encode_reply};
use oteryn_session::{CharacterInventory, ItemEntry};
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn tracker_projects_equipped_inspection_filters_closes_and_rejects_stale_controls() -> TestResult {
    let mut owner = HostOwner::new(None)?;
    let (link, mut requests, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    link.bevy_item_details_selected(true);
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    let item = ItemEntry {
        appearance_id: 0,
        handle: 7.try_into()?,
        item_definition_ref: 42.try_into()?,
        count: 1.try_into()?,
        sub_type: 0,
    };
    outbox.bevy_imbue_equipment(CharacterInventory {
        equipment: vec![EquippedItem {
            slot: EquipmentSlot::Head,
            item,
        }],
        ..Default::default()
    });
    link.open_imbue_tracker();
    let crate::combat_input::PlayCommand::ServerMechanic(command) = requests.try_recv()? else {
        return Err("inspection missing".into());
    };
    assert_eq!((command.capability, command.command_type), (132, 153));
    assert_eq!(
        decode_request(&command.payload).map_err(|e| format!("{e:?}"))?,
        Request::Item(item.handle)
    );
    link.bevy_item_details_response(&ServerMechanicResponse {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: encode_reply(&Reply {
            catalogue: None,
            status: 1,
            requests: vec![],
            items: vec![InspectedItem {
                slot: 0,
                definition_ref: 42,
                count: 1,
                tier: 0,
                name: "Helmet".into(),
                attributes: vec![(
                    "Imbuement Slot 1".into(),
                    "Basic Strike, lasts 00:10h while fighting.".into(),
                )],
            }],
        })
        .map_err(|e| format!("{e:?}"))?,
    });
    owner.phase = Phase::Playing;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .add_systems(Startup, crate::bevy_shell::ui::setup)
    .insert_non_send(owner);
    install(&mut app);
    app.update();
    assert!(texts(app.world_mut()).contains("Helmet"));
    let root = app.world().resource::<State>().root.ok_or("tracker root")?;
    assert!(app.world().get::<super::super::GameModal>(root).is_none());
    trigger(&mut app, |kind| matches!(kind, Kind::Filter("5")))?;
    app.update();
    assert!(!texts(app.world_mut()).contains("Helmet"));
    trigger(&mut app, |kind| matches!(kind, Kind::Filter("0")))?;
    app.update();
    assert!(texts(app.world_mut()).contains("10 minutes"));
    let stale = Control {
        generation: u64::MAX,
        serial: app.world().resource::<State>().serial,
        kind: Kind::Close,
    };
    perform(
        app.world().non_send::<HostOwner>(),
        app.world().resource::<State>(),
        &stale,
    );
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .imbue_panel()
            .tracker_open
    );
    trigger(&mut app, |kind| matches!(kind, Kind::Refresh))?;
    assert!(matches!(
        requests.try_recv()?,
        crate::combat_input::PlayCommand::ServerMechanic(_)
    ));
    app.update();
    assert!(
        requests.try_recv().is_err(),
        "one equipment inspection at a time"
    );
    trigger(&mut app, |kind| matches!(kind, Kind::Close))?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}
fn texts(world: &mut World) -> String {
    world
        .query::<&Text>()
        .iter(world)
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}
fn trigger(app: &mut App, matches: impl Fn(Kind) -> bool) -> TestResult {
    let world = app.world_mut();
    let entity = world
        .query::<(Entity, &Control)>()
        .iter(world)
        .find(|(_, control)| matches(control.kind))
        .map(|(entity, _)| entity)
        .ok_or("control")?;
    world.trigger(Activate { entity });
    Ok(())
}

#[test]
fn imbuement_tracker_and_items_shortcuts_route_to_distinct_owners() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut owner = HostOwner::new(None)?;
    let (link, mut commands, _) = crate::play::play_channel();
    link.use_shared_frontend();
    link.bevy_item_details_selected(true);
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    let code = KeyCode::new(12)?;
    let chord = |shift| NormalizedInputEvent::Key {
        code,
        state: ButtonState::Pressed,
        modifiers: if shift {
            Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift)
        } else {
            Modifiers::one(Modifier::Control).with(Modifier::Alt)
        },
        repeat: false,
    };
    owner.route_input(&[chord(true)], false);
    let link = &owner.play.as_ref().ok_or("play")?.link;
    assert!(link.imbue_panel().tracker_open);
    assert!(!link.inspection_panel().catalogue_open);
    assert!(commands.try_recv().is_err());
    owner.route_input(
        &[NormalizedInputEvent::Key {
            code,
            state: ButtonState::Released,
            modifiers: Modifiers::one(Modifier::Control)
                .with(Modifier::Alt)
                .with(Modifier::Shift),
            repeat: false,
        }],
        false,
    );
    owner.route_input(&[chord(false)], false);
    assert!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .inspection_panel()
            .catalogue_open
    );
    assert!(matches!(
        commands.try_recv()?,
        crate::combat_input::PlayCommand::ServerMechanic(_)
    ));
    Ok(())
}
