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
        assert!(state.feedback.contains("1 to 100"));
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
