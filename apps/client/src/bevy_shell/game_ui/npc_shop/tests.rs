use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_renderer::TileCoord;
use oteryn_session::{AppliedDelta, EntityRef, SessionEvent, npc_service};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn fixture() -> std::result::Result<
    (
        HostOwner,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        crate::play::Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let mut host = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_npc_shop_supported();
    host.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(TileCoord::new(100, 100), 7, &[])?,
        link,
    });
    host.phase = crate::bevy_shell::owner::Phase::Playing;
    publish(&outbox, 7, 12);
    Ok((host, commands, outbox))
}
fn publish(outbox: &crate::play::Outbox, revision: u64, price: u32) {
    outbox.push(vec![SessionEvent::NpcTradeWindow(AppliedDelta {
        server_sequence: revision,
        base_revision: 0,
        new_revision: revision,
        value: npc_service::NpcTradeWindow {
            details: Vec::new(),
            open: Some((
                EntityRef {
                    identity: [9; 16],
                    generation: 2,
                },
                revision,
            )),
            offers: vec![npc_service::NpcOffer {
                item_definition_ref: 42,
                side: NpcTradeSide::Buy,
                count: 3,
                unit_price: price,
            }],
        },
    })]);
}
#[test]
fn npc_shop_actual_activate_emits_exact_retained_trade_and_pending_blocks_repeats() -> Result {
    let (host, mut commands, _) = fixture()?;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<InputFocus>()
    .insert_non_send(host)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    let select = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find(|(_, action)| matches!(action.kind, Kind::Select(_)))
        .map(|(e, _)| e)
        .ok_or("offer row")?;
    app.world_mut().trigger(Activate { entity: select });
    app.update();
    let buy = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find(|(_, action)| matches!(action.kind, Kind::Trade))
        .map(|(e, _)| e)
        .ok_or("trade button")?;
    app.world_mut().trigger(Activate { entity: buy });
    let PlayCommand::NpcTrade(intent) = commands.try_recv()? else {
        return Err("actual trade missing".into());
    };
    assert_eq!(intent.catalogue_revision, 7);
    assert_eq!(intent.offer_index, 0);
    assert_eq!(intent.expected_unit_price, 12);
    assert_eq!(intent.quantity, 1);
    app.world_mut().trigger(Activate { entity: buy });
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn refreshed_quote_and_session_generation_reject_old_selection_and_buttons() -> Result {
    let (mut host, mut commands, outbox) = fixture()?;
    let quote = host
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .npc_shop_panel()
        .live
        .quotes()[0];
    let mut state = State {
        selection: Some(quote),
        generation: Some(host.generation()),
        ..default()
    };
    let action = Action {
        generation: host.generation(),
        kind: Kind::Trade,
    };
    publish(&outbox, 8, 15);
    assert!(!perform(&mut host, &mut state, action, "1"));
    assert!(commands.try_recv().is_err());
    let stale = Action {
        generation: host.generation() + 1,
        kind: Kind::Select(quote),
    };
    assert!(!perform(&mut host, &mut state, stale, "1"));
    state.selection = Some(
        host.play
            .as_ref()
            .ok_or("play")?
            .link
            .npc_shop_panel()
            .live
            .quotes()[0],
    );
    for invalid in ["0", "101", "abc"] {
        assert!(!perform(&mut host, &mut state, action, invalid));
        assert!(state.feedback.contains("limit"));
    }
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn close_updates_retained_owner_without_sending_or_inventing_trade() -> Result {
    let (mut host, mut commands, _) = fixture()?;
    let mut state = State::default();
    let action = Action {
        generation: host.generation(),
        kind: Kind::Close,
    };
    assert!(perform(&mut host, &mut state, action, "1"));
    assert!(!host.play.as_ref().ok_or("play")?.link.npc_shop_is_open());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn floating_shop_remains_nonmodal_and_actual_controls_respect_higher_modal() -> Result {
    let (host, mut commands, _) = fixture()?;
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<InputFocus>()
    .insert_non_send(host)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    let panel = app
        .world_mut()
        .query_filtered::<Entity, With<Panel>>()
        .single(app.world())?;
    assert!(app.world().get::<super::super::GameModal>(panel).is_none());
    let option = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find(|(_, action)| matches!(action.kind, Kind::IgnoreCapacity))
        .map(|(e, _)| e)
        .ok_or("capacity option")?;
    app.world_mut().trigger(Activate { entity: option });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .npc_shop_panel()
            .live
            .options
            .ignore_capacity
    );
    app.world_mut()
        .spawn((super::super::GameModal, GlobalZIndex(180), Node::default()));
    app.world_mut().trigger(Activate { entity: option });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .npc_shop_panel()
            .live
            .options
            .ignore_capacity,
        "higher modal must prevent underlying option mutation"
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[path = "conversation_tests.rs"]
mod conversation_tests;
#[path = "text_tests.rs"]
mod text_tests;

#[test]
fn sell_only_catalogue_initially_selects_available_side() -> Result {
    let (host, _, outbox) = fixture()?;
    outbox.push(vec![SessionEvent::NpcTradeWindow(AppliedDelta {
        server_sequence: 8,
        base_revision: 7,
        new_revision: 8,
        value: npc_service::NpcTradeWindow {
            details: Vec::new(),
            open: Some((
                EntityRef {
                    identity: [9; 16],
                    generation: 2,
                },
                8,
            )),
            offers: vec![npc_service::NpcOffer {
                item_definition_ref: 42,
                side: NpcTradeSide::Sell,
                count: 1,
                unit_price: 12,
            }],
        },
    })]);
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<InputFocus>()
    .insert_non_send(host)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    install(&mut app);
    app.update();
    assert_eq!(
        app.world().resource::<State>().side,
        Some(NpcTradeSide::Sell)
    );
    assert!(app.world_mut().query::<&Action>().iter(app.world()).any(
        |action| matches!(action.kind, Kind::Select(quote) if quote.side == NpcTradeSide::Sell)
    ));
    Ok(())
}

#[test]
fn npc_dialog_uses_global_geometry_without_help_prose() -> Result {
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<crate::bevy_shell::ui::window_layout::WindowLayouts>()
        .add_systems(Startup, crate::bevy_shell::ui::setup)
        .add_systems(Update, |mut commands: Commands, skins: Res<Skins>| {
            widgets::build(&mut commands, &skins, 1);
        })
        .add_systems(PostUpdate, crate::bevy_shell::ui::window_layout::project);
    app.update();
    let dialog = app
        .world_mut()
        .query::<(&Name, &Node)>()
        .iter(app.world())
        .find_map(|(name, node)| (name.as_str() == "npc-shop.dialog").then(|| node.clone()))
        .ok_or("npc dialog")?;
    // NpcDialog.qml: width 600, resizable from a 350px minimum height.
    assert_eq!(
        (dialog.width, dialog.height, dialog.min_height),
        (px(600.), Val::Auto, px(350.))
    );
    let trade = app
        .world_mut()
        .query::<(&Name, &Node)>()
        .iter(app.world())
        .find_map(|(name, node)| (name.as_str() == "npc-shop.trade").then_some(node.width))
        .ok_or("trade addon")?;
    assert_eq!(trade, px(180.));
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| { text.0.starts_with("Buy in backpacks") || text.0 == "NPC response" })
    );
    Ok(())
}

#[test]
fn npc_shop_decoded_names_and_extended_amounts_reach_rows_and_commands() -> Result {
    let (host, mut commands, outbox) = fixture()?;
    let mut window = host
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .npc_shop_panel()
        .live
        .window;
    window.details = vec![npc_service::NpcQuoteDetails {
        name: "meat".into(),
        maximum_quantity: 150,
    }];
    let payload = npc_service::encode_npc_trade_window_for(&window, true)
        .map_err(|error| format!("{error:?}"))?;
    let value = npc_service::decode_npc_trade_window_for(&payload, true)
        .map_err(|error| format!("{error:?}"))?;
    outbox.push(vec![SessionEvent::NpcTradeWindow(AppliedDelta {
        server_sequence: 8,
        base_revision: 7,
        new_revision: 8,
        value,
    })]);
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        source::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<InputFocus>()
    .insert_non_send(host)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "meat")
    );
    let select = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find(|(_, action)| matches!(action.kind, Kind::Select(_)))
        .map(|(entity, _)| entity)
        .ok_or("select")?;
    app.world_mut().trigger(Activate { entity: select });
    app.update();
    let selection = app
        .world()
        .resource::<State>()
        .selection
        .ok_or("selection")?;
    let mut state = State {
        selection: Some(selection),
        ..default()
    };
    let generation = app.world().non_send::<HostOwner>().generation();
    let action = Action {
        generation,
        kind: Kind::Trade,
    };
    assert!(!perform(
        &mut app.world_mut().non_send_mut::<HostOwner>(),
        &mut state,
        action,
        "151"
    ));
    assert!(perform(
        &mut app.world_mut().non_send_mut::<HostOwner>(),
        &mut state,
        action,
        "150"
    ));
    let PlayCommand::NpcTrade(intent) = commands.try_recv()? else {
        return Err("trade command".into());
    };
    assert_eq!(intent.quantity, 150);
    Ok(())
}
